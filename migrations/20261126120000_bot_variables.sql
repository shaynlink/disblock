-- Member and server variables written by bot programs.
CREATE TABLE bot_variables (
    bot_id      UUID NOT NULL REFERENCES bots (id) ON DELETE CASCADE,
    guild_id    BIGINT NOT NULL,
    -- The member's Discord id, or 0 for a server variable.
    user_id     BIGINT NOT NULL,
    name        TEXT NOT NULL,
    value       JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (bot_id, guild_id, user_id, name)
);
