-- Stripe remains the source of truth; these tables are a webhook-reconciled
-- entitlement cache used by request handlers and the bot runtime.
CREATE TABLE stripe_customers (
    user_id             UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    stripe_customer_id  TEXT NOT NULL UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE bot_subscriptions (
    bot_id                  UUID PRIMARY KEY REFERENCES bots (id) ON DELETE CASCADE,
    stripe_subscription_id  TEXT NOT NULL UNIQUE,
    stripe_customer_id      TEXT NOT NULL,
    status                  TEXT NOT NULL,
    current_period_end      TIMESTAMPTZ,
    cancel_at_period_end    BOOLEAN NOT NULL DEFAULT false,
    source_event_created_at TIMESTAMPTZ NOT NULL,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX bot_subscriptions_customer_idx ON bot_subscriptions (stripe_customer_id);
CREATE INDEX bot_subscriptions_access_idx ON bot_subscriptions (bot_id) WHERE status IN ('active', 'trialing');

-- Stripe retries until a 2xx response. Keeping event ids makes retries and
-- overlapping deliveries harmless.
CREATE TABLE stripe_webhook_events (
    stripe_event_id  TEXT PRIMARY KEY,
    received_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
