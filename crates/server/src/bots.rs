use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing};
use disblock_program::Program;
use serde::{Deserialize, Serialize};
use sqlx::types::Json as DbJson;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::AppState;
use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::runtime::{RuntimeError, bot_client, is_unauthorized};

/// Hosting is free during the beta, so accounts are capped.
const MAX_BOTS_PER_USER: i64 = 5;
const MAX_EXECUTIONS: i64 = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/bots", routing::get(list).post(create))
        .route("/bots/{id}", routing::get(get).patch(update).delete(remove))
        .route("/bots/{id}/program", routing::put(save_program))
        .route("/bots/{id}/executions", routing::get(executions))
}

#[derive(Serialize)]
struct BotSummary {
    id: Uuid,
    application_id: String,
    name: String,
    avatar: Option<String>,
    enabled: bool,
    status: String,
    status_detail: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
struct BotDetail {
    #[serde(flatten)]
    summary: BotSummary,
    workspace: serde_json::Value,
    program: Program,
}

async fn list(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<Vec<BotSummary>>, ApiError> {
    let bots = sqlx::query_as!(
        BotSummary,
        "SELECT id, application_id, name, avatar, enabled, status, status_detail, created_at
         FROM bots WHERE owner_id = $1 ORDER BY created_at",
        user.id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(bots))
}

async fn get(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BotDetail>, ApiError> {
    let row = sqlx::query!(
        r#"SELECT id, application_id, name, avatar, enabled, status, status_detail, created_at,
                  workspace, program AS "program: DbJson<Program>"
           FROM bots WHERE id = $1 AND owner_id = $2"#,
        id,
        user.id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(BotDetail {
        summary: BotSummary {
            id: row.id,
            application_id: row.application_id,
            name: row.name,
            avatar: row.avatar,
            enabled: row.enabled,
            status: row.status,
            status_detail: row.status_detail,
            created_at: row.created_at,
        },
        workspace: row.workspace,
        program: row.program.0,
    }))
}

#[derive(Deserialize)]
struct CreateBot {
    token: String,
}

async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateBot>,
) -> Result<(StatusCode, Json<BotSummary>), ApiError> {
    let owned: i64 = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM bots WHERE owner_id = $1"#,
        user.id
    )
    .fetch_one(&state.db)
    .await?;
    if owned >= MAX_BOTS_PER_USER {
        return Err(ApiError::BadRequest(format!(
            "you can host up to {MAX_BOTS_PER_USER} bots during the beta"
        )));
    }

    let token = body.token.trim();
    let token = token
        .strip_prefix("Bot ")
        .unwrap_or(token)
        .trim()
        .to_owned();
    if token.is_empty() || token.contains(char::is_whitespace) {
        return Err(ApiError::BadRequest(
            "paste the bot token from the Discord developer portal".to_owned(),
        ));
    }

    let http = bot_client(&token);
    let (bot_user, application) =
        tokio::join!(http.current_user(), http.current_user_application());
    let invalid_token = |error: twilight_http::Error| {
        if is_unauthorized(&error) {
            ApiError::BadRequest(
                "Discord rejected this token. Reset it in the developer portal (Bot page) and paste the new one."
                    .to_owned(),
            )
        } else {
            ApiError::Discord(error.to_string())
        }
    };
    let bot_user = bot_user
        .map_err(invalid_token)?
        .model()
        .await
        .map_err(|error| ApiError::Discord(error.to_string()))?;
    let application = application
        .map_err(invalid_token)?
        .model()
        .await
        .map_err(|error| ApiError::Discord(error.to_string()))?;

    let id = Uuid::new_v4();
    let sealed = state.cipher.seal(id, &token)?;
    let inserted = sqlx::query_as!(
        BotSummary,
        "INSERT INTO bots (id, owner_id, application_id, name, avatar, token_nonce, token_ciphertext)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, application_id, name, avatar, enabled, status, status_detail, created_at",
        id,
        user.id,
        application.id.to_string(),
        bot_user.global_name.unwrap_or(bot_user.name),
        bot_user.avatar.map(|hash| hash.to_string()),
        sealed.nonce,
        sealed.ciphertext,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            ApiError::Conflict("this bot is already hosted on Disblock".to_owned())
        }
        _ => ApiError::Database(error),
    })?;

    // Disblock owns the bot's slash commands from now on.
    state.runtime.start(id, true).await.map_err(runtime_error)?;
    Ok((StatusCode::CREATED, Json(inserted)))
}

#[derive(Deserialize)]
struct UpdateBot {
    enabled: bool,
}

async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBot>,
) -> Result<StatusCode, ApiError> {
    let updated = sqlx::query!(
        "UPDATE bots SET enabled = $3, updated_at = now() WHERE id = $1 AND owner_id = $2",
        id,
        user.id,
        body.enabled,
    )
    .execute(&state.db)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    if body.enabled {
        state.runtime.start(id, true).await.map_err(runtime_error)?;
    } else {
        state.runtime.stop(id).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let owned = sqlx::query_scalar!(
        "SELECT id FROM bots WHERE id = $1 AND owner_id = $2",
        id,
        user.id
    )
    .fetch_optional(&state.db)
    .await?;
    if owned.is_none() {
        return Err(ApiError::NotFound);
    }

    state.runtime.stop(id).await;
    if let Err(error) = state.runtime.clear_commands(id).await {
        tracing::warn!(bot = %id, %error, "could not remove slash commands of a deleted bot");
    }
    sqlx::query!("DELETE FROM bots WHERE id = $1", id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SaveProgram {
    workspace: serde_json::Value,
    program: Program,
}

async fn save_program(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(body): Json<SaveProgram>,
) -> Result<StatusCode, ApiError> {
    body.program
        .compile()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    let enabled = sqlx::query_scalar!(
        "UPDATE bots SET workspace = $3, program = $4, updated_at = now()
         WHERE id = $1 AND owner_id = $2
         RETURNING enabled",
        id,
        user.id,
        body.workspace,
        DbJson(&body.program) as _,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound)?;

    state
        .runtime
        .apply_program(id, enabled)
        .await
        .map_err(runtime_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct ExecutionEntry {
    id: i64,
    trigger: String,
    success: bool,
    error: Option<String>,
    duration_ms: i32,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

async fn executions(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ExecutionEntry>>, ApiError> {
    let entries = sqlx::query_as!(
        ExecutionEntry,
        "SELECT e.id, e.trigger, e.success, e.error, e.duration_ms, e.created_at
         FROM executions e JOIN bots b ON b.id = e.bot_id
         WHERE e.bot_id = $1 AND b.owner_id = $2
         ORDER BY e.created_at DESC
         LIMIT $3",
        id,
        user.id,
        MAX_EXECUTIONS,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(entries))
}

fn runtime_error(error: RuntimeError) -> ApiError {
    match error {
        RuntimeError::NotFound => ApiError::NotFound,
        RuntimeError::Database(error) => ApiError::Database(error),
        RuntimeError::Crypto(error) => ApiError::Crypto(error),
        RuntimeError::Program(error) => ApiError::BadRequest(error.to_string()),
        RuntimeError::Discord(message) => ApiError::BadRequest(message),
    }
}
