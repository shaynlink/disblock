use std::env;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("environment variable {0} is not set")]
    Missing(&'static str),
    #[error("environment variable {name} is invalid: {reason}")]
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

#[derive(Debug)]
pub struct Config {
    pub database_url: String,
    /// Origin the app is served from, without trailing slash.
    pub public_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub encryption_key: [u8; 32],
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let public_url = required("PUBLIC_URL")?.trim_end_matches('/').to_owned();
        if !(public_url.starts_with("https://") || public_url.starts_with("http://localhost")) {
            return Err(ConfigError::Invalid {
                name: "PUBLIC_URL",
                reason: "must use https outside of localhost",
            });
        }

        let encryption_key = STANDARD
            .decode(required("ENCRYPTION_KEY")?.trim())
            .ok()
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or(ConfigError::Invalid {
                name: "ENCRYPTION_KEY",
                reason: "must be 32 bytes encoded in base64 (openssl rand -base64 32)",
            })?;

        let port = match env::var("PORT") {
            Ok(value) => value.parse().map_err(|_| ConfigError::Invalid {
                name: "PORT",
                reason: "must be a port number",
            })?,
            Err(_) => 8080,
        };

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            public_url,
            discord_client_id: required("DISCORD_CLIENT_ID")?,
            discord_client_secret: required("DISCORD_CLIENT_SECRET")?,
            encryption_key,
            port,
        })
    }

    /// Cookies are `Secure` everywhere except local development over http.
    pub fn secure_cookies(&self) -> bool {
        self.public_url.starts_with("https://")
    }

    pub fn oauth_redirect_uri(&self) -> String {
        format!("{}/api/auth/callback", self.public_url)
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing(name)),
    }
}
