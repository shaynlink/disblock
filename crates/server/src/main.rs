mod auth;
mod bots;
mod config;
mod crypto;
mod error;
mod runtime;

use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::{Router, routing};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::{Config, ConfigError};
use crypto::TokenCipher;
use runtime::Runtime;
use runtime::log::ExecutionLog;

/// Request bodies carry at most a block workspace.
const MAX_BODY_BYTES: usize = 1024 * 1024;
const CLEANUP_INTERVAL: Duration = Duration::from_hours(1);
const LOG_FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub cipher: TokenCipher,
    pub runtime: Runtime,
}

#[derive(Debug, Error)]
enum StartupError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migrations: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("http client: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server: {0}")]
    Io(#[from] std::io::Error),
    #[error("runtime: {0}")]
    Runtime(#[from] runtime::RuntimeError),
    #[error("a TLS crypto provider was already installed")]
    Tls,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if let Err(error) = run().await {
        tracing::error!(%error, "disblock stopped");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), StartupError> {
    // Every TLS client (Discord, Neon) uses the same ring-based provider.
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| StartupError::Tls)?;

    let config = Arc::new(Config::from_env()?);
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("../../migrations").run(&db).await?;

    let cipher = TokenCipher::new(&config.encryption_key);
    let (log, log_writer) = ExecutionLog::spawn(db.clone());
    let runtime = Runtime::new(db.clone(), cipher.clone(), log);
    runtime.start_enabled().await?;
    tokio::spawn(cleanup(db.clone()));

    let state = AppState {
        db,
        config: config.clone(),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?,
        cipher,
        runtime: runtime.clone(),
    };
    let api = Router::new()
        .merge(auth::routes())
        .merge(bots::routes())
        .route("/health", routing::get(|| async { "ok" }));
    let app = Router::new()
        .nest("/api", api)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Close gateway connections cleanly, then flush the execution history.
    runtime.shutdown().await;
    drop(runtime);
    if tokio::time::timeout(LOG_FLUSH_TIMEOUT, log_writer)
        .await
        .is_err()
    {
        tracing::warn!("execution history was not fully flushed");
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "cannot listen for ctrl-c");
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "cannot listen for SIGTERM"),
        }
    };
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutting down");
}

/// Drop expired sessions and execution history older than 30 days.
async fn cleanup(db: PgPool) {
    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
    loop {
        interval.tick().await;
        let sessions = sqlx::query!("DELETE FROM sessions WHERE expires_at < now()")
            .execute(&db)
            .await;
        let executions =
            sqlx::query!("DELETE FROM executions WHERE created_at < now() - interval '30 days'")
                .execute(&db)
                .await;
        for result in [sessions, executions] {
            if let Err(error) = result {
                tracing::error!(%error, "cleanup failed");
            }
        }
    }
}
