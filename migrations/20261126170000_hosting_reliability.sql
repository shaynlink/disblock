-- A Discord gateway session is reusable after a process restart. Persisting
-- its latest sequence lets the next process ask Discord to replay missed events.
CREATE TABLE gateway_sessions (
    bot_id       UUID NOT NULL REFERENCES bots (id) ON DELETE CASCADE,
    shard_id     INTEGER NOT NULL,
    session_id   TEXT NOT NULL,
    sequence     BIGINT NOT NULL CHECK (sequence >= 0),
    resume_url   TEXT NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (bot_id, shard_id)
);

-- One row per bot and UTC billing month. The conditional upsert used by the
-- runtime is the authorization point for an execution, not a reporting total.
CREATE TABLE bot_execution_usage (
    bot_id        UUID NOT NULL REFERENCES bots (id) ON DELETE CASCADE,
    period_start  DATE NOT NULL,
    executions    INTEGER NOT NULL DEFAULT 0 CHECK (executions >= 0),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (bot_id, period_start)
);
