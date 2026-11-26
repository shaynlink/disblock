use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use disblock_program::ErrorLocation;
use serde_json::json;
use thiserror::Error;

use crate::crypto::CryptoError;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("you need to sign in")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("{message}")]
    ProgramError {
        message: String,
        location: Box<ErrorLocation>,
    },
    #[error("{0}")]
    Conflict(String),
    #[error("Discord did not answer, try again in a moment")]
    Discord(String),
    #[error("internal error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Crypto(#[from] CryptoError),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) | Self::ProgramError { .. } => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Discord(_) => StatusCode::BAD_GATEWAY,
            Self::Database(_) | Self::Crypto(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match &self {
            Self::Database(error) => tracing::error!(%error, "database error"),
            Self::Crypto(error) => tracing::error!(%error, "crypto error"),
            Self::Discord(error) => tracing::warn!(%error, "discord error"),
            _ => {}
        }
        let body = match &self {
            Self::ProgramError { message, location } => json!({
                "error": message,
                "location": location,
            }),
            _ => json!({ "error": self.to_string() }),
        };
        (self.status(), Json(body)).into_response()
    }
}
