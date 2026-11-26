//! "Sign in with Discord" (`OAuth2` authorization code flow) and cookie sessions.

use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Json, Router, routing};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use time::Duration;
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;

const SESSION_COOKIE: &str = "disblock_session";
const STATE_COOKIE: &str = "disblock_oauth_state";
const SESSION_DAYS: i64 = 30;
const DISCORD_API: &str = "https://discord.com/api/v10";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", routing::get(login))
        .route("/auth/callback", routing::get(callback))
        .route("/auth/logout", routing::post(logout))
        .route("/account/export", routing::get(export_account))
        .route("/account", routing::delete(delete_account))
        .route("/me", routing::get(me))
}

/// The signed-in user, extracted from the session cookie.
#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub discord_id: String,
    pub username: String,
    pub avatar: Option<String>,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SESSION_COOKIE).ok_or(ApiError::Unauthorized)?;
        let hash = hash_token(token.value());

        sqlx::query_as!(
            CurrentUser,
            r#"SELECT u.id, u.discord_id, u.username, u.avatar
               FROM sessions s JOIN users u ON u.id = s.user_id
               WHERE s.token_hash = $1 AND s.expires_at > now()"#,
            hash,
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(ApiError::Unauthorized)
    }
}

async fn me(user: CurrentUser) -> Json<CurrentUser> {
    Json(user)
}

/// Data portability export. Deliberately excludes encrypted bot tokens: they
/// are credentials, never returned to browsers, and cannot be safely exported.
async fn export_account(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Response, ApiError> {
    let data: serde_json::Value = sqlx::query_scalar(
        r"SELECT jsonb_build_object(
             'format', 'disblock-account-export-v1',
             'exported_at', now(),
             'account', jsonb_build_object(
                'discord_id', u.discord_id, 'username', u.username, 'avatar', u.avatar,
                'created_at', u.created_at, 'updated_at', u.updated_at
             ),
             'bots', COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                   'id', b.id, 'application_id', b.application_id, 'name', b.name,
                   'avatar', b.avatar, 'enabled', b.enabled, 'status', b.status,
                   'status_detail', b.status_detail, 'workspace', b.workspace, 'program', b.program,
                   'created_at', b.created_at, 'updated_at', b.updated_at,
                   'executions', COALESCE((
                     SELECT jsonb_agg(jsonb_build_object(
                       'trigger', e.trigger, 'success', e.success, 'error', e.error,
                       'duration_ms', e.duration_ms, 'created_at', e.created_at
                     ) ORDER BY e.created_at)
                     FROM executions e WHERE e.bot_id = b.id
                   ), '[]'::jsonb),
                   'variables', COALESCE((
                     SELECT jsonb_agg(jsonb_build_object(
                       'guild_id', v.guild_id, 'user_id', v.user_id, 'name', v.name,
                       'value', v.value, 'updated_at', v.updated_at
                     ) ORDER BY v.guild_id, v.user_id, v.name)
                     FROM bot_variables v WHERE v.bot_id = b.id
                   ), '[]'::jsonb)
                ) ORDER BY b.created_at)
                FROM bots b WHERE b.owner_id = u.id
             ), '[]'::jsonb)
           ) FROM users u WHERE u.id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    let mut response = Json(data).into_response();
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=disblock-account-export.json"),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct DeleteAccount {
    confirmation: String,
}

async fn delete_account(
    State(state): State<AppState>,
    jar: CookieJar,
    user: CurrentUser,
    Json(body): Json<DeleteAccount>,
) -> Result<Response, ApiError> {
    if body.confirmation != "DELETE" {
        return Err(ApiError::BadRequest(
            "type DELETE to confirm account deletion".to_owned(),
        ));
    }

    let bot_ids = sqlx::query_scalar::<_, Uuid>("SELECT id FROM bots WHERE owner_id = $1")
        .bind(user.id)
        .fetch_all(&state.db)
        .await?;
    for bot_id in bot_ids {
        state.runtime.stop(bot_id).await;
        if let Err(error) = state.runtime.clear_commands(bot_id).await {
            tracing::warn!(bot = %bot_id, %error, "could not remove commands during account deletion");
        }
    }

    crate::billing::delete_customer(&state, user.id).await?;
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&state.db)
        .await?;
    let jar = jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    Ok((jar, StatusCode::NO_CONTENT).into_response())
}

async fn login(State(state): State<AppState>, jar: CookieJar) -> Response {
    let oauth_state = random_token();
    let authorize = reqwest::Url::parse_with_params(
        "https://discord.com/oauth2/authorize",
        [
            ("response_type", "code"),
            ("client_id", state.config.discord_client_id.as_str()),
            ("scope", "identify"),
            ("redirect_uri", state.config.oauth_redirect_uri().as_str()),
            ("state", oauth_state.as_str()),
            ("prompt", "none"),
        ],
    )
    .expect("static authorize URL is valid");

    let cookie = Cookie::build((STATE_COOKIE, oauth_state))
        .path("/api/auth")
        .http_only(true)
        .secure(state.config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(Duration::minutes(10));

    (jar.add(cookie), Redirect::to(authorize.as_str())).into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
    avatar: Option<String>,
}

async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, ApiError> {
    let expected_state = jar
        .get(STATE_COOKIE)
        .map(|cookie| cookie.value().to_owned());
    let jar = jar.remove(Cookie::build(STATE_COOKIE).path("/api/auth"));

    let (Some(code), Some(returned_state), Some(expected_state)) =
        (query.code, query.state, expected_state)
    else {
        // The user cancelled on Discord, or the state cookie expired.
        return Ok((jar, Redirect::to("/?login=cancelled")).into_response());
    };
    if returned_state != expected_state {
        return Err(ApiError::BadRequest(
            "sign in expired, please try again".to_owned(),
        ));
    }

    let discord_user = fetch_discord_user(&state, &code).await?;
    let user_id: Uuid = sqlx::query_scalar!(
        r#"INSERT INTO users (id, discord_id, username, avatar)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (discord_id) DO UPDATE
           SET username = EXCLUDED.username, avatar = EXCLUDED.avatar, updated_at = now()
           RETURNING id"#,
        Uuid::new_v4(),
        discord_user.id,
        discord_user.global_name.unwrap_or(discord_user.username),
        discord_user.avatar,
    )
    .fetch_one(&state.db)
    .await?;

    let token = random_token();
    sqlx::query!(
        "INSERT INTO sessions (token_hash, user_id, expires_at)
         VALUES ($1, $2, now() + make_interval(days => $3))",
        hash_token(&token),
        user_id,
        i32::try_from(SESSION_DAYS).expect("session length fits in i32"),
    )
    .execute(&state.db)
    .await?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(state.config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(Duration::days(SESSION_DAYS));

    Ok((jar.add(cookie), Redirect::to("/")).into_response())
}

async fn fetch_discord_user(state: &AppState, code: &str) -> Result<DiscordUser, ApiError> {
    let redirect_uri = state.config.oauth_redirect_uri();
    let token: TokenResponse = state
        .http
        .post(format!("{DISCORD_API}/oauth2/token"))
        .basic_auth(
            &state.config.discord_client_id,
            Some(&state.config.discord_client_secret),
        )
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| ApiError::Discord(format!("oauth token exchange: {error}")))?
        .json()
        .await
        .map_err(|error| ApiError::Discord(format!("oauth token body: {error}")))?;

    state
        .http
        .get(format!("{DISCORD_API}/users/@me"))
        .bearer_auth(token.access_token)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| ApiError::Discord(format!("fetch user: {error}")))?
        .json()
        .await
        .map_err(|error| ApiError::Discord(format!("user body: {error}")))
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> Result<Response, ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        sqlx::query!(
            "DELETE FROM sessions WHERE token_hash = $1",
            hash_token(cookie.value())
        )
        .execute(&state.db)
        .await?;
    }
    let jar = jar.remove(Cookie::build(SESSION_COOKIE).path("/"));
    Ok((jar, StatusCode::NO_CONTENT).into_response())
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
