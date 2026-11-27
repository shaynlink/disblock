//! Per-client request limits, so one client cannot exhaust the API or
//! Discord's limits for everyone else.

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::time::Duration;

use axum::Json;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use serde_json::json;

use crate::AppState;

/// Header the Cloudflare Worker sets with the shared origin secret.
const ORIGIN_SECRET_HEADER: &str = "x-disblock-origin-secret";
/// Header the Cloudflare Worker sets with the browser's IP address.
const CLIENT_IP_HEADER: &str = "x-disblock-client-ip";
/// Set by Fly.io's edge, which clients cannot forge.
const FLY_CLIENT_IP_HEADER: &str = "fly-client-ip";

/// Dashboards poll every few seconds, so ordinary reads get room to spare.
const GENERAL_PER_MINUTE: NonZeroU32 = NonZeroU32::new(300).expect("non-zero");
const GENERAL_BURST: NonZeroU32 = NonZeroU32::new(100).expect("non-zero");
/// Sign in, bot creation, program saves and billing call Discord or Stripe.
const SENSITIVE_PER_MINUTE: NonZeroU32 = NonZeroU32::new(20).expect("non-zero");
const SENSITIVE_BURST: NonZeroU32 = NonZeroU32::new(10).expect("non-zero");

pub struct RateLimits {
    general: DefaultKeyedRateLimiter<IpAddr>,
    sensitive: DefaultKeyedRateLimiter<IpAddr>,
    origin_secret: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    Exempt,
    General,
    Sensitive,
}

impl RateLimits {
    /// `origin_secret` is shared with the Cloudflare Worker; without it, the
    /// client IP header it forwards is ignored.
    pub fn new(origin_secret: Option<String>) -> Self {
        Self {
            general: RateLimiter::keyed(
                Quota::per_minute(GENERAL_PER_MINUTE).allow_burst(GENERAL_BURST),
            ),
            sensitive: RateLimiter::keyed(
                Quota::per_minute(SENSITIVE_PER_MINUTE).allow_burst(SENSITIVE_BURST),
            ),
            origin_secret,
        }
    }

    /// Forget clients whose limits have fully recovered, to bound memory.
    pub fn forget_idle_clients(&self) {
        self.general.retain_recent();
        self.sensitive.retain_recent();
    }

    /// The browser's address: the one the Worker vouches for, else Fly's edge
    /// view, else the TCP peer.
    fn client_ip(&self, headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
        let header_ip = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<IpAddr>().ok())
        };
        let from_worker = self.origin_secret.as_deref().is_some_and(|secret| {
            headers
                .get(ORIGIN_SECRET_HEADER)
                .is_some_and(|sent| constant_time_eq(sent.as_bytes(), secret.as_bytes()))
        });
        from_worker
            .then(|| header_ip(CLIENT_IP_HEADER))
            .flatten()
            .or_else(|| header_ip(FLY_CLIENT_IP_HEADER))
            .unwrap_or_else(|| peer.ip())
    }
}

fn bucket(method: &Method, path: &str) -> Bucket {
    let path = path.trim_end_matches('/');
    if path == "/api/health" || path == "/api/stripe/webhook" {
        return Bucket::Exempt;
    }
    let sensitive = matches!(
        path,
        "/api/auth/login" | "/api/auth/callback" | "/api/billing/checkout" | "/api/billing/portal"
    ) || (method == Method::POST && path == "/api/bots")
        || (method == Method::PUT && path.ends_with("/program"))
        || (method == Method::DELETE && path == "/api/account");
    if sensitive {
        Bucket::Sensitive
    } else {
        Bucket::General
    }
}

/// Compare secrets without leaking how many leading bytes match.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}

pub async fn limit(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let limits = &state.limits;
    let limiter = match bucket(request.method(), request.uri().path()) {
        Bucket::Exempt => return next.run(request).await,
        Bucket::General => &limits.general,
        Bucket::Sensitive => &limits.sensitive,
    };
    let ip = limits.client_ip(request.headers(), peer);

    match limiter.check_key(&ip) {
        Ok(()) => next.run(request).await,
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            too_many_requests(wait)
        }
    }
}

fn too_many_requests(wait: Duration) -> Response {
    let seconds = wait.as_secs().max(1);
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": format!("too many requests, try again in {seconds} s") })),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from(seconds));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_routes() {
        assert_eq!(bucket(&Method::GET, "/api/health"), Bucket::Exempt);
        assert_eq!(bucket(&Method::POST, "/api/stripe/webhook"), Bucket::Exempt);
        assert_eq!(bucket(&Method::GET, "/api/auth/login"), Bucket::Sensitive);
        assert_eq!(bucket(&Method::POST, "/api/bots"), Bucket::Sensitive);
        assert_eq!(bucket(&Method::GET, "/api/bots"), Bucket::General);
        assert_eq!(
            bucket(&Method::PUT, "/api/bots/42/program"),
            Bucket::Sensitive
        );
        assert_eq!(
            bucket(&Method::GET, "/api/bots/42/executions"),
            Bucket::General
        );
    }

    #[test]
    fn trusts_the_forwarded_ip_only_with_the_secret() {
        let peer: SocketAddr = "10.0.0.1:443".parse().expect("addr");
        let limits = RateLimits::new(Some("s".repeat(32)));
        let mut headers = HeaderMap::new();
        headers.insert(CLIENT_IP_HEADER, HeaderValue::from_static("203.0.113.7"));
        headers.insert(
            FLY_CLIENT_IP_HEADER,
            HeaderValue::from_static("198.51.100.2"),
        );

        assert_eq!(
            limits.client_ip(&headers, peer),
            "198.51.100.2".parse::<IpAddr>().expect("ip")
        );

        headers.insert(ORIGIN_SECRET_HEADER, HeaderValue::from_static("wrong"));
        assert_eq!(
            limits.client_ip(&headers, peer),
            "198.51.100.2".parse::<IpAddr>().expect("ip")
        );

        headers.insert(
            ORIGIN_SECRET_HEADER,
            HeaderValue::from_str(&"s".repeat(32)).expect("value"),
        );
        assert_eq!(
            limits.client_ip(&headers, peer),
            "203.0.113.7".parse::<IpAddr>().expect("ip")
        );
    }

    #[test]
    fn rejects_bursts_beyond_the_quota() {
        let limits = RateLimits::new(None);
        let ip: IpAddr = "203.0.113.9".parse().expect("ip");
        let allowed = (0..SENSITIVE_BURST.get() + 5)
            .filter(|_| limits.sensitive.check_key(&ip).is_ok())
            .count();
        assert_eq!(allowed, SENSITIVE_BURST.get() as usize);
    }
}
