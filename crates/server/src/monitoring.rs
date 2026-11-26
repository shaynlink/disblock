//! Error reporting and performance tracing. The SDK is a no-op until `SENTRY_DSN`
//! is configured, which keeps local development data on the developer machine.

use sentry::integrations::tracing::{ErrorLayer, SpanLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer as _;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

use crate::config::SentryConfig;

pub fn init(config: Option<&SentryConfig>) -> Option<sentry::ClientInitGuard> {
    config.map(|config| {
        let options = sentry::ClientOptions::new()
            .dsn(&config.dsn)
            .environment(config.environment.clone())
            .maybe_release(config.release.clone())
            // Production defaults to 10%; deployments may lower or raise this
            // deliberately with SENTRY_TRACES_SAMPLE_RATE.
            .traces_sample_rate(config.traces_sample_rate);
        sentry::init(options)
    })
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        // Never ship debug spans or error events: they can contain user-provided
        // Discord content and would also consume the error budget too quickly.
        .with(SpanLayer::new().with_filter(LevelFilter::INFO))
        .with(ErrorLayer::new().with_filter(LevelFilter::ERROR))
        .init();
}
