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

3. Run the API (migrations apply on start) and the web app:

   ```bash
   set -a; source .env; set +a; cargo run -p disblock-server
   ```

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

### 1. Neon

Create a project in an EU region and copy its **direct** connection string
(not the `-pooler` one), with `sslmode=require`.

### 2. Fly.io

```bash
fly launch --no-deploy --copy-config --name disblock-api --region cdg
fly secrets set \
  DATABASE_URL='postgres://…neon.tech/neondb?sslmode=require' \
  PUBLIC_URL='https://your-domain' \
  DISCORD_CLIENT_ID='…' \
  DISCORD_CLIENT_SECRET='…' \
  ENCRYPTION_KEY="$(openssl rand -base64 32)" \
  STRIPE_SECRET_KEY='sk_live_…' \
  STRIPE_WEBHOOK_SECRET='whsec_…' \
  STRIPE_PRO_PRICE_ID='price_…'
fly deploy
fly scale count 1
```

Keep `ENCRYPTION_KEY` somewhere safe: losing it makes every stored bot token
unreadable.

### 3. Stripe Billing

Create a recurring Pro Price in Stripe and set `STRIPE_PRO_PRICE_ID` to its
`price_…` id. Configure a Stripe webhook endpoint at
`https://your-domain/api/stripe/webhook` and give it the
`checkout.session.completed`, `customer.subscription.created`,
`customer.subscription.updated`, and `customer.subscription.deleted` events.
Store its signing secret as `STRIPE_WEBHOOK_SECRET`.

The API creates hosted subscription Checkout sessions and Customer Portal
sessions; it never exposes Stripe secret keys to the browser. Pro access is
only granted from signed, idempotently-recorded subscription webhooks. Leaving
the three Stripe variables unset keeps billing disabled for local development.

### 4. Cloudflare

Set `API_ORIGIN` in `web/wrangler.jsonc` to the Fly app URL, then:

```bash
cd web && pnpm run deploy
```

Attach your domain to the `disblock` worker, and add
`https://your-domain/api/auth/callback` to the Discord application's OAuth2
redirects.
