//! Durable Discord gateway resume state.

use std::collections::HashMap;

use sqlx::{PgPool, Row};
use twilight_gateway::Session;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub session_id: String,
    pub sequence: u64,
    pub resume_url: String,
}

pub async fn load(db: &PgPool, bot_id: Uuid) -> Result<HashMap<u32, StoredSession>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT shard_id, session_id, sequence, resume_url FROM gateway_sessions WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let shard_id = u32::try_from(row.try_get::<i32, _>("shard_id").ok()?).ok()?;
            let sequence = u64::try_from(row.try_get::<i64, _>("sequence").ok()?).ok()?;
            Some((
                shard_id,
                StoredSession {
                    session_id: row.try_get("session_id").ok()?,
                    sequence,
                    resume_url: row.try_get("resume_url").ok()?,
                },
            ))
        })
        .collect())
}

pub async fn save(
    db: &PgPool,
    bot_id: Uuid,
    shard_id: u32,
    session: Option<&Session>,
    resume_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    let (Some(session), Some(resume_url)) = (session, resume_url) else {
        return clear(db, bot_id, shard_id).await;
    };
    let sequence =
        i64::try_from(session.sequence()).map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
    sqlx::query(
        "INSERT INTO gateway_sessions (bot_id, shard_id, session_id, sequence, resume_url)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (bot_id, shard_id) DO UPDATE SET
           session_id = EXCLUDED.session_id, sequence = EXCLUDED.sequence,
           resume_url = EXCLUDED.resume_url, updated_at = now()",
    )
    .bind(bot_id)
    .bind(i32::try_from(shard_id).map_err(|error| sqlx::Error::Encode(Box::new(error)))?)
    .bind(session.id())
    .bind(sequence)
    .bind(resume_url)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn clear(db: &PgPool, bot_id: Uuid, shard_id: u32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM gateway_sessions WHERE bot_id = $1 AND shard_id = $2")
        .bind(bot_id)
        .bind(i32::try_from(shard_id).map_err(|error| sqlx::Error::Encode(Box::new(error)))?)
        .execute(db)
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
            "INSERT INTO users (id, discord_id, username) VALUES ($1, $2, 'owner')",
            owner,
            owner.to_string(),
        )
        .execute(db)
        .await
        .expect("user");
        sqlx::query!(
            "INSERT INTO bots (id, owner_id, application_id, name, token_nonce, token_ciphertext)
             VALUES ($1, $2, $3, 'bot', '\\x00', '\\x00')",
            bot,
            owner,
            bot.to_string(),
        )
        .execute(db)
        .await
        .expect("bot");
        bot
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn saves_and_clears_a_resume_session(db: PgPool) {
        let bot = bot(&db).await;
        let session = Session::new(42, "session-id".to_owned());
        save(
            &db,
            bot,
            0,
            Some(&session),
            Some("wss://gateway.discord.gg"),
        )
        .await
        .expect("save");

        let saved = load(&db, bot).await.expect("load");
        let saved = saved.get(&0).expect("shard session");
        assert_eq!(saved.session_id, "session-id");
        assert_eq!(saved.sequence, 42);
        assert_eq!(saved.resume_url, "wss://gateway.discord.gg");

        clear(&db, bot, 0).await.expect("clear");
        assert!(load(&db, bot).await.expect("reload").is_empty());
    }
}
