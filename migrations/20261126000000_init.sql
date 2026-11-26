CREATE TABLE users (
    id          UUID PRIMARY KEY,
    discord_id  TEXT NOT NULL UNIQUE,
    username    TEXT NOT NULL,
    avatar      TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Only a SHA-256 of the session token is stored, never the token itself.
CREATE TABLE sessions (
    token_hash  BYTEA PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

CREATE TABLE bots (
    id               UUID PRIMARY KEY,
    owner_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    application_id   TEXT NOT NULL UNIQUE,
    name             TEXT NOT NULL,
    avatar           TEXT,
    -- AES-256-GCM, with the bot id as associated data.
    token_nonce      BYTEA NOT NULL,
    token_ciphertext BYTEA NOT NULL,
    -- Block editor state, opaque to the server.
    workspace        JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Program compiled from the workspace by the editor, validated by the server.
    program          JSONB NOT NULL DEFAULT '{"handlers": []}'::jsonb,
    enabled          BOOLEAN NOT NULL DEFAULT true,
    status           TEXT NOT NULL DEFAULT 'stopped'
                     CHECK (status IN ('stopped', 'starting', 'online', 'error')),
    status_detail    TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX bots_owner_id_idx ON bots (owner_id);

CREATE TABLE executions (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    bot_id       UUID NOT NULL REFERENCES bots (id) ON DELETE CASCADE,
    trigger      TEXT NOT NULL,
    success      BOOLEAN NOT NULL,
    error        TEXT,
    duration_ms  INTEGER NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX executions_bot_id_created_at_idx ON executions (bot_id, created_at DESC);
