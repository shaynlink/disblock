use std::env;
use std::str::FromStr;

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

pub struct Config {
    pub database_url: String,
    /// Origin the app is served from, without trailing slash.
    pub public_url: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub encryption_key: [u8; 32],
    /// Stripe is optional locally, but its three values must be configured as a set.
    pub stripe: Option<StripeConfig>,
    /// Error tracking is disabled until a DSN is explicitly configured.
    pub sentry: Option<SentryConfig>,
    pub port: u16,
}

/// Server-only Stripe credentials and the recurring Pro Price to sell.
pub struct StripeConfig {
    pub secret_key: String,
    pub webhook_secret: String,
    pub pro_price_id: String,
}

/// Sentry is intentionally opt-in so local development never exports telemetry.
pub struct SentryConfig {
    pub dsn: String,
    pub environment: String,
    pub release: Option<String>,
    pub traces_sample_rate: f32,
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

        let stripe = stripe_from_env()?;
        let sentry = sentry_from_env()?;

        Ok(Self {
            database_url: required("DATABASE_URL")?,
            public_url,
            discord_client_id: required("DISCORD_CLIENT_ID")?,
            discord_client_secret: required("DISCORD_CLIENT_SECRET")?,
            encryption_key,
            stripe,
            sentry,
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

fn sentry_from_env() -> Result<Option<SentryConfig>, ConfigError> {
    let Some(dsn) = env::var("SENTRY_DSN")
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    sentry::types::Dsn::from_str(&dsn).map_err(|_| ConfigError::Invalid {
        name: "SENTRY_DSN",
        reason: "must be a valid Sentry DSN",
    })?;

    let traces_sample_rate = env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value.parse::<f32>().map_err(|_| ConfigError::Invalid {
                name: "SENTRY_TRACES_SAMPLE_RATE",
                reason: "must be a number between 0 and 1",
            })
        })
        .transpose()?
        .unwrap_or(0.1);
    if !(0.0..=1.0).contains(&traces_sample_rate) {
        return Err(ConfigError::Invalid {
            name: "SENTRY_TRACES_SAMPLE_RATE",
            reason: "must be a number between 0 and 1",
        });
    }

    Ok(Some(SentryConfig {
        dsn,
        environment: env::var("SENTRY_ENVIRONMENT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "development".to_owned()),
        release: env::var("RELEASE_VERSION")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        traces_sample_rate,
    }))
}

fn stripe_from_env() -> Result<Option<StripeConfig>, ConfigError> {
    let secret_key = env::var("STRIPE_SECRET_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let webhook_secret = env::var("STRIPE_WEBHOOK_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let pro_price_id = env::var("STRIPE_PRO_PRICE_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());

    match (secret_key, webhook_secret, pro_price_id) {
        (None, None, None) => Ok(None),
        (Some(secret_key), Some(webhook_secret), Some(pro_price_id)) => Ok(Some(StripeConfig {
            secret_key,
            webhook_secret,
            pro_price_id,
        })),
        _ => Err(ConfigError::Invalid {
            name: "STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET and STRIPE_PRO_PRICE_ID",
            reason: "must be set together",
        }),
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing(name)),
    }
}
