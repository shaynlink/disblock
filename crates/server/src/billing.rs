//! Stripe Checkout, Customer Portal, and webhook-reconciled Pro entitlements.
//!
//! The browser only receives redirect URLs. It never decides whether a bot has
//! paid access: handlers and the runtime read the subscription state persisted
//! from signed Stripe webhooks.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router, routing};
use ring::hmac;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::AppState;
use crate::auth::CurrentUser;
use crate::config::StripeConfig;
use crate::error::ApiError;

const STRIPE_API: &str = "https://api.stripe.com/v1";
const STRIPE_API_VERSION: &str = "2026-02-25.clover";
const WEBHOOK_TOLERANCE_SECONDS: i64 = 300;

/// Close the Stripe customer before account deletion so a paid subscription
/// cannot outlive the service account it belongs to.
pub async fn delete_customer(state: &AppState, user_id: Uuid) -> Result<(), ApiError> {
    let customer = sqlx::query_scalar::<_, String>(
        "SELECT stripe_customer_id FROM stripe_customers WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;
    let Some(customer) = customer else {
        return Ok(());
    };
    let stripe = stripe_config(state)?;
    let response = state
        .http
        .delete(format!("{STRIPE_API}/customers/{customer}"))
        .basic_auth(&stripe.secret_key, Some(""))
        .header("Stripe-Version", STRIPE_API_VERSION)
        .send()
        .await
        .map_err(|error| ApiError::BillingProvider(error.to_string()))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(ApiError::BillingProvider(format!(
            "Stripe returned HTTP {} while deleting a customer",
            response.status()
        )))
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/billing/bots/{id}", routing::get(status))
        .route("/billing/checkout", routing::post(checkout))
        .route("/billing/portal", routing::post(portal))
        // This route intentionally has no session extractor. Stripe signs the
        // exact raw request body before it reaches JSON deserialization.
        .route("/stripe/webhook", routing::post(webhook))
}

#[derive(Deserialize)]
struct CheckoutRequest {
    bot_id: Uuid,
    /// Monthly unless the customer picks yearly billing.
    #[serde(default)]
    interval: BillingInterval,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum BillingInterval {
    #[default]
    Month,
    Year,
}

impl BillingInterval {
    fn price_id(self, stripe: &StripeConfig) -> &str {
        match self {
            Self::Month => &stripe.pro_price_id,
            Self::Year => &stripe.pro_yearly_price_id,
        }
    }
}

#[derive(Serialize)]
struct RedirectUrl {
    url: String,
}

#[derive(Debug, Serialize)]
struct BillingStatus {
    plan: String,
    status: Option<String>,
    cancel_at_period_end: bool,
}

async fn status(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<BillingStatus>, ApiError> {
    load_status(&state.db, bot_id, user.id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

/// Billing state of one of `owner_id`'s bots; free bots have no subscription row.
async fn load_status(
    db: &PgPool,
    bot_id: Uuid,
    owner_id: Uuid,
) -> Result<Option<BillingStatus>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT bs.status, COALESCE(bs.cancel_at_period_end, false) AS cancel_at_period_end
         FROM bots b LEFT JOIN bot_subscriptions bs ON bs.bot_id = b.id
         WHERE b.id = $1 AND b.owner_id = $2",
    )
    .bind(bot_id)
    .bind(owner_id)
    .fetch_optional(db)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let subscription_status: Option<String> = row.try_get("status")?;
    let active = subscription_status.as_deref().is_some_and(is_pro_status);
    Ok(Some(BillingStatus {
        plan: if active { "pro" } else { "free" }.to_owned(),
        status: subscription_status,
        cancel_at_period_end: row.try_get("cancel_at_period_end")?,
    }))
}

async fn checkout(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CheckoutRequest>,
) -> Result<Json<RedirectUrl>, ApiError> {
    ensure_owned(&state.db, body.bot_id, user.id).await?;
    if has_pro_access(&state.db, body.bot_id).await? {
        return Err(ApiError::Conflict(
            "this bot already has Pro; manage it from the billing portal".to_owned(),
        ));
    }

    let stripe = stripe_config(&state)?;
    let customer = load_or_create_customer(&state, stripe, &user).await?;
    let bot_id = body.bot_id.to_string();
    let user_id = user.id.to_string();
    let success_url = format!(
        "{}/bots/{}/billing?billing=success&session_id={{CHECKOUT_SESSION_ID}}",
        state.config.public_url, bot_id
    );
    let cancel_url = format!(
        "{}/bots/{}/billing?billing=cancelled",
        state.config.public_url, bot_id
    );
    let session: StripeCheckoutSession = stripe_post(
        &state,
        stripe,
        "/checkout/sessions",
        vec![
            ("mode", "subscription".to_owned()),
            ("customer", customer),
            ("client_reference_id", bot_id.clone()),
            (
                "line_items[0][price]",
                body.interval.price_id(stripe).to_owned(),
            ),
            ("line_items[0][quantity]", "1".to_owned()),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("metadata[bot_id]", bot_id.clone()),
            ("metadata[user_id]", user_id.clone()),
            ("subscription_data[metadata][bot_id]", bot_id),
            ("subscription_data[metadata][user_id]", user_id),
        ],
        Some(format!("disblock-checkout-{}-{}", user.id, body.bot_id)),
    )
    .await?;
    Ok(Json(RedirectUrl { url: session.url }))
}

async fn portal(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<RedirectUrl>, ApiError> {
    let stripe = stripe_config(&state)?;
    let customer = sqlx::query_scalar::<_, String>(
        "SELECT stripe_customer_id FROM stripe_customers WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("there is no subscription to manage yet".to_owned()))?;
    let session: StripePortalSession = stripe_post(
        &state,
        stripe,
        "/billing_portal/sessions",
        vec![
            ("customer", customer),
            ("return_url", format!("{}/", state.config.public_url)),
        ],
        None,
    )
    .await?;
    Ok(Json(RedirectUrl { url: session.url }))
}

async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ApiError> {
    let stripe = state.config.stripe.as_ref().ok_or(ApiError::NotFound)?;
    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("invalid Stripe webhook signature".to_owned()))?;
    verify_webhook_signature(&stripe.webhook_secret, signature, &body)
        .map_err(|()| ApiError::BadRequest("invalid Stripe webhook signature".to_owned()))?;
    let event: StripeEvent = serde_json::from_slice(&body)
        .map_err(|_| ApiError::BadRequest("invalid Stripe webhook body".to_owned()))?;

    let mut transaction = state.db.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO stripe_webhook_events (stripe_event_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(&event.id)
    .execute(&mut *transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        transaction.commit().await?;
        return Ok(StatusCode::OK);
    }

    let bot_to_refresh = match event.kind.as_str() {
        "customer.subscription.created"
        | "customer.subscription.updated"
        | "customer.subscription.deleted" => upsert_subscription(&mut transaction, &event).await?,
        "checkout.session.completed" => upsert_checkout_customer(&mut transaction, &event).await?,
        _ => None,
    };
    transaction.commit().await?;

    // The cache has committed before we change a running program. Failure is
    // logged so Stripe does not repeatedly deliver an otherwise valid event.
    if let Some(bot_id) = bot_to_refresh
        && let Err(error) = state.runtime.refresh_program(bot_id).await
    {
        tracing::warn!(bot = %bot_id, %error, "could not refresh billing limits");
    }
    Ok(StatusCode::OK)
}

async fn ensure_owned(db: &PgPool, bot_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM bots WHERE id = $1 AND owner_id = $2)",
    )
    .bind(bot_id)
    .bind(user_id)
    .fetch_one(db)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(ApiError::NotFound)
    }
}

/// Free bots keep this many member and server variables at most.
pub const FREE_SAVED_VARIABLES: usize = 5;

/// Refuse programs that need Pro on a bot without an active subscription:
/// scheduled triggers, or more than [`FREE_SAVED_VARIABLES`] saved variables.
pub async fn check_program_limits(
    db: &PgPool,
    bot_id: Uuid,
    program: &disblock_program::CompiledProgram,
) -> Result<(), ApiError> {
    let scheduled = !program.schedules().is_empty();
    let variables = program.saved_variable_count();
    if !scheduled && variables <= FREE_SAVED_VARIABLES {
        return Ok(());
    }
    if has_pro_access(db, bot_id).await? {
        return Ok(());
    }
    Err(ApiError::BadRequest(if scheduled {
        "scheduled triggers need Pro: upgrade this bot from its Settings page".to_owned()
    } else {
        format!(
            "the free plan keeps up to {FREE_SAVED_VARIABLES} saved variables and this program uses {variables}: upgrade this bot to Pro"
        )
    }))
}

pub(crate) async fn has_pro_access(db: &PgPool, bot_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bot_subscriptions WHERE bot_id = $1 AND status IN ('active', 'trialing'))",
    )
    .bind(bot_id)
    .fetch_one(db)
    .await
}

fn is_pro_status(status: &str) -> bool {
    matches!(status, "active" | "trialing")
}

fn stripe_config(state: &AppState) -> Result<&StripeConfig, ApiError> {
    state
        .config
        .stripe
        .as_ref()
        .ok_or(ApiError::BillingUnavailable)
}

async fn load_or_create_customer(
    state: &AppState,
    stripe: &StripeConfig,
    user: &CurrentUser,
) -> Result<String, ApiError> {
    if let Some(customer) = sqlx::query_scalar::<_, String>(
        "SELECT stripe_customer_id FROM stripe_customers WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(customer);
    }

    // A deterministic idempotency key prevents parallel browser requests from
    // creating two Stripe Customers for the same Disblock account.
    let customer: StripeCustomer = stripe_post(
        state,
        stripe,
        "/customers",
        vec![
            ("metadata[disblock_user_id]", user.id.to_string()),
            (
                "description",
                format!("Disblock account {}", user.discord_id),
            ),
        ],
        Some(format!("disblock-customer-{}", user.id)),
    )
    .await?;
    sqlx::query(
        "INSERT INTO stripe_customers (user_id, stripe_customer_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET stripe_customer_id = EXCLUDED.stripe_customer_id,
             updated_at = now()",
    )
    .bind(user.id)
    .bind(&customer.id)
    .execute(&state.db)
    .await?;
    Ok(customer.id)
}

async fn stripe_post<T: DeserializeOwned>(
    state: &AppState,
    stripe: &StripeConfig,
    path: &str,
    form: Vec<(&str, String)>,
    idempotency_key: Option<String>,
) -> Result<T, ApiError> {
    let mut request = state
        .http
        .post(format!("{STRIPE_API}{path}"))
        .basic_auth(&stripe.secret_key, Some(""))
        .header("Stripe-Version", STRIPE_API_VERSION)
        .form(&form);
    if let Some(idempotency_key) = idempotency_key {
        request = request.header("Idempotency-Key", idempotency_key);
    }
    let response = request
        .send()
        .await
        .map_err(|error| ApiError::BillingProvider(error.to_string()))?;
    if !response.status().is_success() {
        return Err(ApiError::BillingProvider(format!(
            "Stripe returned HTTP {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map_err(|error| ApiError::BillingProvider(format!("invalid Stripe response: {error}")))
}

#[derive(Deserialize)]
struct StripeCheckoutSession {
    url: String,
}

#[derive(Deserialize)]
struct StripePortalSession {
    url: String,
}

#[derive(Deserialize)]
struct StripeCustomer {
    id: String,
}

#[derive(Deserialize)]
struct StripeEvent {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    created: i64,
    data: StripeEventData,
}

#[derive(Deserialize)]
struct StripeEventData {
    object: serde_json::Value,
}

#[derive(Deserialize)]
struct StripeSubscription {
    id: String,
    customer: String,
    status: String,
    #[serde(default)]
    metadata: HashMap<String, String>,
    #[serde(default)]
    cancel_at_period_end: bool,
    #[serde(default)]
    items: StripeSubscriptionItems,
}

#[derive(Default, Deserialize)]
struct StripeSubscriptionItems {
    #[serde(default)]
    data: Vec<StripeSubscriptionItem>,
}

#[derive(Deserialize)]
struct StripeSubscriptionItem {
    current_period_end: Option<i64>,
}

#[derive(Deserialize)]
struct StripeCheckoutCompleted {
    customer: Option<String>,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

async fn upsert_subscription(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &StripeEvent,
) -> Result<Option<Uuid>, ApiError> {
    let subscription: StripeSubscription = serde_json::from_value(event.data.object.clone())
        .map_err(|_| ApiError::BadRequest("invalid Stripe subscription webhook".to_owned()))?;
    let Some(bot_id) = subscription
        .metadata
        .get("bot_id")
        .and_then(|id| Uuid::parse_str(id).ok())
    else {
        return Ok(None);
    };
    let owner_id = sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_optional(&mut **transaction)
        .await?;
    let Some(owner_id) = owner_id else {
        return Ok(None);
    };
    let created_at = OffsetDateTime::from_unix_timestamp(event.created)
        .map_err(|_| ApiError::BadRequest("invalid Stripe event timestamp".to_owned()))?;
    let period_end = subscription
        .items
        .data
        .iter()
        .filter_map(|item| item.current_period_end)
        .filter_map(|seconds| OffsetDateTime::from_unix_timestamp(seconds).ok())
        .max();

    sqlx::query(
        "INSERT INTO stripe_customers (user_id, stripe_customer_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET stripe_customer_id = EXCLUDED.stripe_customer_id,
             updated_at = now()",
    )
    .bind(owner_id)
    .bind(&subscription.customer)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO bot_subscriptions
             (bot_id, stripe_subscription_id, stripe_customer_id, status, current_period_end,
              cancel_at_period_end, source_event_created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (bot_id) DO UPDATE SET
             stripe_subscription_id = EXCLUDED.stripe_subscription_id,
             stripe_customer_id = EXCLUDED.stripe_customer_id,
             status = EXCLUDED.status,
             current_period_end = EXCLUDED.current_period_end,
             cancel_at_period_end = EXCLUDED.cancel_at_period_end,
             source_event_created_at = EXCLUDED.source_event_created_at,
             updated_at = now()
         WHERE bot_subscriptions.source_event_created_at <= EXCLUDED.source_event_created_at",
    )
    .bind(bot_id)
    .bind(&subscription.id)
    .bind(&subscription.customer)
    .bind(&subscription.status)
    .bind(period_end)
    .bind(subscription.cancel_at_period_end)
    .bind(created_at)
    .execute(&mut **transaction)
    .await?;
    Ok(Some(bot_id))
}

async fn upsert_checkout_customer(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &StripeEvent,
) -> Result<Option<Uuid>, ApiError> {
    let checkout: StripeCheckoutCompleted = serde_json::from_value(event.data.object.clone())
        .map_err(|_| ApiError::BadRequest("invalid Stripe checkout webhook".to_owned()))?;
    let Some(bot_id) = checkout
        .metadata
        .get("bot_id")
        .and_then(|id| Uuid::parse_str(id).ok())
    else {
        return Ok(None);
    };
    let Some(customer) = checkout.customer else {
        return Ok(None);
    };
    let owner_id = sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM bots WHERE id = $1")
        .bind(bot_id)
        .fetch_optional(&mut **transaction)
        .await?;
    let Some(owner_id) = owner_id else {
        return Ok(None);
    };
    sqlx::query(
        "INSERT INTO stripe_customers (user_id, stripe_customer_id)
         VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET stripe_customer_id = EXCLUDED.stripe_customer_id,
             updated_at = now()",
    )
    .bind(owner_id)
    .bind(customer)
    .execute(&mut **transaction)
    .await?;
    Ok(None)
}

fn verify_webhook_signature(secret: &str, signature: &str, body: &[u8]) -> Result<(), ()> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in signature.split(',') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" => signatures.push(value),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or(())?;
    if (OffsetDateTime::now_utc().unix_timestamp() - timestamp).abs() > WEBHOOK_TOLERANCE_SECONDS {
        return Err(());
    }
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(body);
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    signatures
        .into_iter()
        .any(|candidate| {
            let Ok(candidate) = hex::decode(candidate) else {
                return false;
            };
            hmac::verify(&key, &signed_payload, &candidate).is_ok()
        })
        .then_some(())
        .ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_stripe_signature() {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        let body = br#"{"id":"evt_test"}"#;
        let mut payload = format!("{timestamp}.").into_bytes();
        payload.extend_from_slice(body);
        let key = hmac::Key::new(hmac::HMAC_SHA256, b"whsec_test");
        let signature = format!(
            "t={timestamp},v1={}",
            hex::encode(hmac::sign(&key, &payload))
        );
        assert!(verify_webhook_signature("whsec_test", &signature, body).is_ok());
    }

    #[test]
    fn rejects_an_invalid_stripe_signature() {
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        assert!(
            verify_webhook_signature("whsec_test", &format!("t={timestamp},v1=00"), b"{}").is_err()
        );
    }

    #[test]
    fn only_active_or_trialing_subscriptions_have_pro_access() {
        assert!(is_pro_status("active"));
        assert!(is_pro_status("trialing"));
        assert!(!is_pro_status("past_due"));
        assert!(!is_pro_status("canceled"));
    }
}

#[cfg(test)]
mod plan_limit_tests {
    use super::*;

    async fn free_bot(db: &PgPool) -> Uuid {
        let (owner, bot) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query("INSERT INTO users (id, discord_id, username) VALUES ($1, '1', 'owner')")
            .bind(owner)
            .execute(db)
            .await
            .expect("user");
        sqlx::query(
            "INSERT INTO bots (id, owner_id, application_id, name, token_nonce, token_ciphertext)
             VALUES ($1, $2, '2', 'bot', '\\x00', '\\x00')",
        )
        .bind(bot)
        .bind(owner)
        .execute(db)
        .await
        .expect("bot");
        bot
    }

    /// A program whose trigger changes `variables` distinct server variables,
    /// or sends a message when there are none.
    fn compiled(trigger: &str, variables: usize) -> disblock_program::CompiledProgram {
        let actions: Vec<String> = if variables == 0 {
            vec![
                r#"{"type":"send_message","channel_id":"123456789012345678","content":"hi"}"#
                    .to_owned(),
            ]
        } else {
            (0..variables)
                .map(|index| {
                    format!(
                        r#"{{"type":"change_variable","scope":"server","name":"v{index}","by":{{"type":"number","value":1}}}}"#
                    )
                })
                .collect()
        };
        let json = format!(
            r#"{{"handlers":[{{"trigger":{trigger},"actions":[{}]}}]}}"#,
            actions.join(",")
        );
        serde_json::from_str::<disblock_program::Program>(&json)
            .expect("valid json")
            .compile()
            .expect("valid program")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn free_bots_keep_free_limits(db: PgPool) {
        let bot = free_bot(&db).await;
        let join = r#"{"type":"member_join"}"#;

        assert!(
            check_program_limits(&db, bot, &compiled(join, FREE_SAVED_VARIABLES))
                .await
                .is_ok()
        );
        assert!(matches!(
            check_program_limits(&db, bot, &compiled(join, FREE_SAVED_VARIABLES + 1)).await,
            Err(ApiError::BadRequest(_))
        ));
        let hourly = r#"{"type":"schedule","interval":1,"unit":"hours"}"#;
        assert!(matches!(
            check_program_limits(&db, bot, &compiled(hourly, 0)).await,
            Err(ApiError::BadRequest(_))
        ));
    }
}

#[cfg(test)]
mod interval_tests {
    use super::*;

    #[test]
    fn picks_the_price_of_the_chosen_interval() {
        let stripe = StripeConfig {
            secret_key: "sk".to_owned(),
            webhook_secret: "whsec".to_owned(),
            pro_price_id: "price_month".to_owned(),
            pro_yearly_price_id: "price_year".to_owned(),
        };
        let parse = |json: &str| serde_json::from_str::<CheckoutRequest>(json).expect("valid");
        let bot = r#""bot_id":"00000000-0000-4000-8000-000000000001""#;

        assert_eq!(
            parse(&format!("{{{bot}}}")).interval.price_id(&stripe),
            "price_month"
        );
        assert_eq!(
            parse(&format!(r#"{{{bot},"interval":"year"}}"#))
                .interval
                .price_id(&stripe),
            "price_year"
        );
        assert!(
            serde_json::from_str::<CheckoutRequest>(&format!(r#"{{{bot},"interval":"week"}}"#))
                .is_err()
        );
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;

    async fn bot(db: &PgPool) -> (Uuid, Uuid) {
        let (owner, bot) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query("INSERT INTO users (id, discord_id, username) VALUES ($1, '1', 'owner')")
            .bind(owner)
            .execute(db)
            .await
            .expect("user");
        sqlx::query(
            "INSERT INTO bots (id, owner_id, application_id, name, token_nonce, token_ciphertext)
             VALUES ($1, $2, '2', 'bot', '\\x00', '\\x00')",
        )
        .bind(bot)
        .bind(owner)
        .execute(db)
        .await
        .expect("bot");
        (owner, bot)
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn reports_free_and_pro_bots(db: PgPool) {
        let (owner, bot) = bot(&db).await;

        let free = load_status(&db, bot, owner)
            .await
            .expect("query")
            .expect("owned");
        assert_eq!(free.plan, "free");
        assert_eq!(free.status, None);
        assert!(!free.cancel_at_period_end);

        sqlx::query(
            "INSERT INTO bot_subscriptions
               (bot_id, stripe_subscription_id, stripe_customer_id, status, cancel_at_period_end, source_event_created_at)
             VALUES ($1, 'sub_1', 'cus_1', 'active', true, now())",
        )
        .bind(bot)
        .execute(&db)
        .await
        .expect("subscription");
        let pro = load_status(&db, bot, owner)
            .await
            .expect("query")
            .expect("owned");
        assert_eq!(pro.plan, "pro");
        assert!(pro.cancel_at_period_end);

        assert!(
            load_status(&db, bot, Uuid::new_v4())
                .await
                .expect("query")
                .is_none()
        );
    }
}
