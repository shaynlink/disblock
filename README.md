# Disblock

Build Discord bots with blocks. Disblock hosts them.

A user signs in with Discord, pastes a bot token, and assembles the bot's
behaviour in a block editor: slash commands, member joins and message triggers,
with replies, messages, reactions and roles as actions, conditions, and
variables saved per member or per server. Saving pushes the slash
commands to Discord and swaps the new program into the running bot, without
reconnecting it.

## Architecture

```
Cloudflare Worker  ──  web app (React + Blockly), and /api/* proxied to Fly.io
Fly.io             ──  crates/server: Axum API + every bot's gateway connection
Neon               ──  Postgres
```

| Path | What it is |
| --- | --- |
| `crates/program` | Program format, validation against Discord's limits, and a pure interpreter that plans effects and variable writes. No I/O. |
| `crates/server` | API, Discord login, encrypted bot tokens, bot runtime (twilight), batched execution history. |
| `migrations` | SQL schema, applied at startup. |
| `web` | Vite+ React app, the Blockly editor, and `worker/` for Cloudflare. |

Notes on the runtime:

- One process hosts every bot. Each bot gets the gateway intents its program
  needs and nothing more; events nobody uses are skipped before parsing.
- Bot tokens are encrypted with AES-256-GCM, bound to the bot's id.
- User content can only mention users, never `@everyone` or roles.
- Run **one** Fly machine: a second one would answer every event twice.

## Local development

Requirements: Rust 1.98+, Node 26+, pnpm, Docker, `sqlx-cli`.

1. Create a Discord application for the login at
   <https://discord.com/developers/applications>, and add
   `http://localhost:5173/api/auth/callback` to **OAuth2 → Redirects**.
2. Configure and start the database:

   ```bash
   cp .env.example .env    # then fill DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET
   openssl rand -base64 32 # paste as ENCRYPTION_KEY
   docker compose up -d
   ```

3. Apply the migrations, then run the API and the web app:

   ```bash
   set -a; source .env; set +a; cargo run -p disblock-server -- migrate
   ```

   ```bash
   set -a; source .env; set +a; cargo run -p disblock-server
   ```

   The server refuses to start on a schema older than the build expects.

   ```bash
   cd web && pnpm install && pnpm dev
   ```

4. Open <http://localhost:5173>.

Checks (the server tests create throwaway databases on the local Postgres):

```bash
set -a; source .env; set +a
cargo clippy --workspace --all-targets -- -W clippy::pedantic && cargo test --workspace
cd web && pnpm exec vp check && pnpm exec vp test
```

After changing a SQL query, refresh the offline query data used by Docker
builds: `cargo sqlx prepare --workspace`.

## Deploy

For CI/CD, staging, Sentry, health alerts, encrypted backups and restoration
drills, see [the operations runbook](docs/operations.md) before the first
deployment.

### 1. Neon

Create a project in AWS US East, next to the Fly region (`iad`) and Discord.
Use **direct** connection strings (not the
`-pooler` ones) with `sslmode=require`, and two roles:

- the owner (`neondb_owner`) for `MIGRATION_DATABASE_URL`: only
  `disblock-server migrate` uses it, run by Fly before each deploy;
- `disblock_app` for `DATABASE_URL`: rows only, no schema changes.

```sql
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON DATABASE neondb FROM PUBLIC;
CREATE ROLE disblock_app LOGIN PASSWORD '<generated>' CONNECTION LIMIT 20;
GRANT CONNECT ON DATABASE neondb TO disblock_app;
GRANT USAGE ON SCHEMA public TO disblock_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO disblock_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO disblock_app;
ALTER DEFAULT PRIVILEGES FOR ROLE neondb_owner IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO disblock_app;
ALTER DEFAULT PRIVILEGES FOR ROLE neondb_owner IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO disblock_app;
ALTER ROLE disblock_app SET statement_timeout = '10s';
ALTER ROLE disblock_app SET idle_in_transaction_session_timeout = '30s';
```

`scripts/configure-secrets.sh staging|production` then sets every secret on
GitHub, Fly and Cloudflare, asking for the values only you know.

### 2. Fly.io

```bash
fly launch --no-deploy --copy-config --name disblock-api --region iad
fly secrets set \
  DATABASE_URL='postgres://…neon.tech/neondb?sslmode=require' \
  PUBLIC_URL='https://your-domain' \
  DISCORD_CLIENT_ID='…' \
  DISCORD_CLIENT_SECRET='…' \
  ENCRYPTION_KEY="$(openssl rand -base64 32)" \
  ORIGIN_SECRET="$(openssl rand -hex 32)" \
  STRIPE_SECRET_KEY='sk_live_…' \
  STRIPE_WEBHOOK_SECRET='whsec_…' \
  STRIPE_PRO_PRICE_ID='price_…' \
  STRIPE_PRO_YEARLY_PRICE_ID='price_…'
fly deploy
fly scale count 1
```

Keep `ENCRYPTION_KEY` somewhere safe: losing it makes every stored bot token
unreadable.

### 3. Stripe Billing

Create the Pro product in Stripe with two recurring prices, €6 monthly
(`STRIPE_PRO_PRICE_ID`) and €50 yearly (`STRIPE_PRO_YEARLY_PRICE_ID`), tax
included; customers pick the period on the bot's Billing page. Configure a Stripe webhook endpoint at
`https://your-domain/api/stripe/webhook` and give it the
`checkout.session.completed`, `customer.subscription.created`,
`customer.subscription.updated`, and `customer.subscription.deleted` events.
Store its signing secret as `STRIPE_WEBHOOK_SECRET`.

The API creates hosted subscription Checkout sessions and Customer Portal
sessions; it never exposes Stripe secret keys to the browser. Pro access is
only granted from signed, idempotently-recorded subscription webhooks. Leaving
the three Stripe variables unset keeps billing disabled for local development.

### 4. Cloudflare

Set `API_ORIGIN` in `web/wrangler.jsonc` to the Fly app URL, and give the
Worker the same `ORIGIN_SECRET` as Fly so rate limits apply per browser
rather than per Cloudflare egress address (`cd web && pnpm exec wrangler
secret put ORIGIN_SECRET`). Then:

```bash
cd web && pnpm run deploy
```

Attach your domain to the `disblock` worker, and add
`https://your-domain/api/auth/callback` to the Discord application's OAuth2
redirects.

### 5. Legal identity

Before publishing the service, set these build-time variables for the web app:
`VITE_LEGAL_PUBLISHER`, `VITE_LEGAL_FORM`, `VITE_LEGAL_ADDRESS`,
`VITE_LEGAL_REGISTRATION`, `VITE_LEGAL_VAT`, `VITE_LEGAL_EMAIL`,
`VITE_LEGAL_PHONE`, `VITE_LEGAL_PUBLICATION_DIRECTOR`, and
`VITE_LEGAL_CONSUMER_MEDIATOR`. They populate `/legal` and deliberately show a
visible placeholder when absent; do not publish with placeholders.

### Sentry

Errors, traces, logs, metrics and Session Replay go to the `disblock-server`
and `disblock-web` projects of the `nvbes` organization.

- Fly: `fly secrets set SENTRY_DSN=<disblock-server DSN>`. The environment
  comes from `fly.toml` / `fly.staging.toml`, the release from the commit.
- GitHub: secret `SENTRY_AUTH_TOKEN` (an org token able to write releases),
  and variable `SENTRY_WEB_DSN` (the `disblock-web` DSN).

Each deploy builds the web app with hidden source maps, uploads them to Sentry
and deletes them before publishing, then records one release for both
projects, linked to its commits, with a deploy for the target environment.
Locally, put `SENTRY_DSN` in `.env` and `VITE_SENTRY_DSN` in `web/.env.local`.

