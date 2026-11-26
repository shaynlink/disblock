//! Member and server variables, stored per bot and per server.

use std::collections::BTreeMap;

use disblock_program::{Execution, SavedVariables, Scope, Value, VariableWrite};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::quota;

/// Server variables are stored with this member id.
const SERVER: i64 = 0;

/// Snowflakes fit in `BIGINT`: Discord ids stay below 2^63 for centuries.
fn snowflake(id: u64) -> Result<i64, sqlx::Error> {
    i64::try_from(id).map_err(|error| sqlx::Error::Encode(Box::new(error)))
}

/// Everything the member and their server have saved, in one query.
#[allow(dead_code)] // Also kept for diagnostics and unit-level verification.
pub async fn load(
    db: &PgPool,
    bot_id: Uuid,
    guild_id: u64,
    user_id: u64,
) -> Result<SavedVariables, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT user_id, name, value FROM bot_variables
         WHERE bot_id = $1 AND guild_id = $2 AND user_id IN ($3, $4)",
        bot_id,
        snowflake(guild_id)?,
        snowflake(user_id)?,
        SERVER,
    )
    .fetch_all(db)
    .await?;

    let mut saved = SavedVariables::default();
    for row in rows {
        // A value that no longer parses is treated as unset.
        let Ok(value) = serde_json::from_value::<Value>(row.value) else {
            continue;
        };
        let scope = if row.user_id == SERVER {
            &mut saved.server
        } else {
            &mut saved.member
        };
        scope.insert(row.name, value);
    }
    Ok(saved)
}

pub async fn save(
    db: &PgPool,
    bot_id: Uuid,
    guild_id: u64,
    user_id: u64,
    writes: &[VariableWrite],
) -> Result<(), sqlx::Error> {
    let mut transaction = db.begin().await?;
    lock(&mut transaction, bot_id, guild_id, user_id).await?;
    save_tx(&mut transaction, bot_id, guild_id, user_id, writes).await?;
    transaction.commit().await
}

/// Read, plan, reserve all matching executions, and persist their writes in
/// one transaction. The advisory transaction lock prevents two events in the
/// same server from planning from the same stale member or server snapshot.
pub async fn plan_and_save(
    db: &PgPool,
    bot_id: Uuid,
    guild_id: u64,
    user_id: u64,
    plan: impl FnOnce(&SavedVariables) -> Vec<Execution>,
) -> Result<Option<Vec<Execution>>, sqlx::Error> {
    let mut transaction = db.begin().await?;
    lock(&mut transaction, bot_id, guild_id, user_id).await?;
    let saved = load_tx(&mut transaction, bot_id, guild_id, user_id).await?;
    let mut executions = plan(&saved);
    let count =
        i32::try_from(executions.len()).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
    if !quota::reserve_many_tx(&mut transaction, bot_id, count).await? {
        return Ok(None);
    }
    let writes: Vec<VariableWrite> = executions
        .iter()
        .flat_map(|execution| execution.writes.iter().cloned())
        .collect();
    save_tx(&mut transaction, bot_id, guild_id, user_id, &writes).await?;
    for execution in &mut executions {
        execution.writes.clear();
    }
    transaction.commit().await?;
    Ok(Some(executions))
}

async fn lock(
    transaction: &mut Transaction<'_, Postgres>,
    bot_id: Uuid,
    guild_id: u64,
    _user_id: u64,
) -> Result<(), sqlx::Error> {
    // Server-scoped values are shared by every member, so the lock must cover
    // the entire bot/server pair rather than a single member.
    let key = format!("{bot_id}:{guild_id}");
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(key)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn load_tx(
    transaction: &mut Transaction<'_, Postgres>,
    bot_id: Uuid,
    guild_id: u64,
    user_id: u64,
) -> Result<SavedVariables, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT user_id, name, value FROM bot_variables
         WHERE bot_id = $1 AND guild_id = $2 AND user_id IN ($3, $4)",
    )
    .bind(bot_id)
    .bind(snowflake(guild_id)?)
    .bind(snowflake(user_id)?)
    .bind(SERVER)
    .fetch_all(&mut **transaction)
    .await?;
    let mut saved = SavedVariables::default();
    for row in rows {
        let user_id: i64 = row.try_get("user_id")?;
        let value: serde_json::Value = row.try_get("value")?;
        let Ok(value) = serde_json::from_value::<Value>(value) else {
            continue;
        };
        let scope = if user_id == SERVER {
            &mut saved.server
        } else {
            &mut saved.member
        };
        scope.insert(row.try_get("name")?, value);
    }
    Ok(saved)
}

async fn save_tx(
    transaction: &mut Transaction<'_, Postgres>,
    bot_id: Uuid,
    guild_id: u64,
    user_id: u64,
    writes: &[VariableWrite],
) -> Result<(), sqlx::Error> {
    let member = snowflake(user_id)?;
    let mut unique = BTreeMap::new();
    for write in writes {
        unique.insert((write.scope, write.name.clone()), write.value.clone());
    }
    let mut owners = Vec::with_capacity(writes.len());
    let mut names = Vec::with_capacity(writes.len());
    let mut values = Vec::with_capacity(writes.len());
    for ((scope, name), value) in unique {
        owners.push(match scope {
            Scope::Server => SERVER,
            Scope::Member | Scope::Local => member,
        });
        names.push(name);
        values.push(
            serde_json::to_value(value).map_err(|error| sqlx::Error::Encode(Box::new(error)))?,
        );
    }

    if owners.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO bot_variables (bot_id, guild_id, user_id, name, value)
         SELECT $1, $2, w.user_id, w.name, w.value
         FROM UNNEST($3::int8[], $4::text[], $5::jsonb[]) AS w (user_id, name, value)
         ON CONFLICT (bot_id, guild_id, user_id, name)
         DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
        bot_id,
        snowflake(guild_id)?,
        &owners,
        &names,
        &values,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn bot(db: &PgPool) -> Uuid {
        let owner = Uuid::new_v4();
        let bot = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, discord_id, username) VALUES ($1, '1', 'owner')",
            owner
        )
        .execute(db)
        .await
        .expect("user");
        sqlx::query!(
            "INSERT INTO bots (id, owner_id, application_id, name, token_nonce, token_ciphertext)
             VALUES ($1, $2, '2', 'bot', '\\x00', '\\x00')",
            bot,
            owner
        )
        .execute(db)
        .await
        .expect("bot");
        bot
    }

    fn write(scope: Scope, name: &str, value: Value) -> VariableWrite {
        VariableWrite {
            scope,
            name: name.to_owned(),
            value,
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn saves_and_loads_member_and_server_variables(db: PgPool) {
        let bot = bot(&db).await;
        let (guild, ada, bob) = (10, 20, 30);

        save(
            &db,
            bot,
            guild,
            ada,
            &[
                write(Scope::Member, "xp", Value::Number(5)),
                write(Scope::Server, "joins", Value::Number(1)),
            ],
        )
        .await
        .expect("first save");
        save(
            &db,
            bot,
            guild,
            ada,
            &[write(Scope::Member, "xp", Value::Number(12))],
        )
        .await
        .expect("update");
        save(
            &db,
            bot,
            guild,
            bob,
            &[write(Scope::Member, "xp", Value::Text("lots".to_owned()))],
        )
        .await
        .expect("other member");

        let saved = load(&db, bot, guild, ada).await.expect("load");
        assert_eq!(saved.member.get("xp"), Some(&Value::Number(12)));
        assert_eq!(saved.server.get("joins"), Some(&Value::Number(1)));
        assert_eq!(saved.member.len(), 1, "another member's variables leaked");

        let other_server = load(&db, bot, guild + 1, ada).await.expect("load");
        assert!(other_server.member.is_empty() && other_server.server.is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn serializes_variable_plans_for_a_server(db: PgPool) {
        let bot = bot(&db).await;
        let increment = |saved: &SavedVariables| {
            let xp = match saved.member.get("xp") {
                Some(Value::Number(value)) => *value,
                _ => 0,
            };
            vec![Execution {
                trigger: "increment".to_owned(),
                effects: vec![],
                writes: vec![write(Scope::Member, "xp", Value::Number(xp + 1))],
                error: None,
            }]
        };

        let (first, second) = tokio::join!(
            plan_and_save(&db, bot, 10, 20, increment),
            plan_and_save(&db, bot, 10, 20, increment),
        );
        assert!(first.expect("first transaction").is_some());
        assert!(second.expect("second transaction").is_some());

        let saved = load(&db, bot, 10, 20).await.expect("load");
        assert_eq!(saved.member.get("xp"), Some(&Value::Number(2)));
    }
}
