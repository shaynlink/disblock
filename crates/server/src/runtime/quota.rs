//! Atomic, per-bot execution reservations.

use sqlx::PgPool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Monthly runs for a beta bot without an active Pro subscription.
pub const FREE_MONTHLY_EXECUTIONS: i32 = 1_000;
/// Monthly runs for a bot with an active or trialing Pro subscription.
pub const PRO_MONTHLY_EXECUTIONS: i32 = 50_000;

pub async fn reserve_many(db: &PgPool, bot_id: Uuid, count: i32) -> Result<bool, sqlx::Error> {
    if count == 0 {
        return Ok(true);
    }
    reserve_with_limits(
        db,
        bot_id,
        count,
        FREE_MONTHLY_EXECUTIONS,
        PRO_MONTHLY_EXECUTIONS,
    )
    .await
}

pub async fn reserve_many_tx(
    transaction: &mut Transaction<'_, Postgres>,
    bot_id: Uuid,
    count: i32,
) -> Result<bool, sqlx::Error> {
    if count == 0 {
        return Ok(true);
    }
    reserve_with_limits_tx(
        transaction,
        bot_id,
        count,
        FREE_MONTHLY_EXECUTIONS,
        PRO_MONTHLY_EXECUTIONS,
    )
    .await
}

async fn reserve_with_limits(
    db: &PgPool,
    bot_id: Uuid,
    count: i32,
    free_limit: i32,
    pro_limit: i32,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r"WITH plan AS (
             SELECT CASE WHEN EXISTS(
                 SELECT 1 FROM bot_subscriptions
                 WHERE bot_id = $1 AND status IN ('active', 'trialing')
             ) THEN $3::int4 ELSE $2::int4 END AS execution_limit
           ), reserved AS (
             INSERT INTO bot_execution_usage (bot_id, period_start, executions)
             SELECT $1, date_trunc('month', now() AT TIME ZONE 'UTC')::date, $4 FROM plan
             WHERE $4 <= (SELECT execution_limit FROM plan)
             ON CONFLICT (bot_id, period_start) DO UPDATE
             SET executions = bot_execution_usage.executions + $4, updated_at = now()
             WHERE bot_execution_usage.executions + $4 <= (SELECT execution_limit FROM plan)
             RETURNING executions
           )
           SELECT EXISTS(SELECT 1 FROM reserved)",
    )
    .bind(bot_id)
    .bind(free_limit)
    .bind(pro_limit)
    .bind(count)
    .fetch_one(db)
    .await
}

async fn reserve_with_limits_tx(
    transaction: &mut Transaction<'_, Postgres>,
    bot_id: Uuid,
    count: i32,
    free_limit: i32,
    pro_limit: i32,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r"WITH plan AS (
             SELECT CASE WHEN EXISTS(
                 SELECT 1 FROM bot_subscriptions
                 WHERE bot_id = $1 AND status IN ('active', 'trialing')
             ) THEN $3::int4 ELSE $2::int4 END AS execution_limit
           ), reserved AS (
             INSERT INTO bot_execution_usage (bot_id, period_start, executions)
             SELECT $1, date_trunc('month', now() AT TIME ZONE 'UTC')::date, $4 FROM plan
             WHERE $4 <= (SELECT execution_limit FROM plan)
             ON CONFLICT (bot_id, period_start) DO UPDATE
             SET executions = bot_execution_usage.executions + $4, updated_at = now()
             WHERE bot_execution_usage.executions + $4 <= (SELECT execution_limit FROM plan)
             RETURNING executions
           )
           SELECT EXISTS(SELECT 1 FROM reserved)",
    )
    .bind(bot_id)
    .bind(free_limit)
    .bind(pro_limit)
    .bind(count)
    .fetch_one(&mut **transaction)
    .await
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
    async fn reserves_no_more_than_the_free_limit(db: PgPool) {
        let bot = bot(&db).await;
        assert!(reserve_with_limits(&db, bot, 1, 2, 4).await.expect("first"));
        assert!(
            reserve_with_limits(&db, bot, 1, 2, 4)
                .await
                .expect("second")
        );
        assert!(
            !reserve_with_limits(&db, bot, 1, 2, 4)
                .await
                .expect("limited")
        );
    }
}
