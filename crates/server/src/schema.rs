//! Database schema migrations.
//!
//! The server connects with a role that can only read and write rows. Schema
//! changes run separately, with the owner role in `MIGRATION_DATABASE_URL`,
//! through `disblock-server migrate` (Fly runs it before each deploy).

use std::env;
use std::time::Duration;

use sqlx::PgPool;
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Postgres error code of an undefined table.
const UNDEFINED_TABLE: &str = "42P01";

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("environment variable MIGRATION_DATABASE_URL is not set")]
    MissingMigrationUrl,
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migrations: {0}")]
    Migrate(#[from] MigrateError),
    #[error("the database has never been migrated: run `disblock-server migrate` first")]
    NotMigrated,
    #[error(
        "the database schema is behind ({0} pending migrations): run `disblock-server migrate` first"
    )]
    Pending(usize),
}

/// `disblock-server migrate`: apply pending migrations as the schema owner.
pub async fn migrate() -> Result<(), SchemaError> {
    let url = env::var("MIGRATION_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or(SchemaError::MissingMigrationUrl)?;
    let db = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&url)
        .await?;
    MIGRATOR.run(&db).await?;
    tracing::info!(
        migrations = MIGRATOR.iter().count(),
        "database schema is up to date"
    );
    Ok(())
}

/// Refuse to serve on a schema older than this build expects.
pub async fn ensure_up_to_date(db: &PgPool) -> Result<(), SchemaError> {
    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success")
            .fetch_all(db)
            .await
            .map_err(|error| match &error {
                sqlx::Error::Database(database)
                    if database.code().as_deref() == Some(UNDEFINED_TABLE) =>
                {
                    SchemaError::NotMigrated
                }
                _ => SchemaError::Database(error),
            })?;
    let pending = MIGRATOR
        .iter()
        .filter(|migration| migration.migration_type.is_up_migration())
        .filter(|migration| !applied.contains(&migration.version))
        .count();
    if pending == 0 {
        Ok(())
    } else {
        Err(SchemaError::Pending(pending))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = false)]
    async fn refuses_an_unmigrated_database(db: PgPool) {
        assert!(matches!(
            ensure_up_to_date(&db).await,
            Err(SchemaError::NotMigrated)
        ));
        MIGRATOR.run(&db).await.expect("migrate");
        assert!(ensure_up_to_date(&db).await.is_ok());
    }
}
