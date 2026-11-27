#!/usr/bin/env bash
# Configure the secrets of one Disblock environment on GitHub, Fly.io and
# Cloudflare. Values are typed silently or generated locally: they never
# appear as command arguments, so they stay out of shell history.
#
# Requirements: gh, flyctl and wrangler logged in for staging and production;
# the Sentry CLI (`sentry auth login`) to fetch DSNs instead of typing them.
# Usage: scripts/configure-secrets.sh development|staging|production
#
# Per environment, Sentry is set up as follows (see README "Sentry"):
#   development  .env SENTRY_DSN + web/.env.local VITE_SENTRY_DSN, every trace
#   CI           nothing: no DSN and no auth token, so builds send nothing
#   staging      Fly SENTRY_DSN, GitHub vars, every trace, source maps
#   production   Fly SENTRY_DSN, GitHub vars, 10% of traces, source maps
set -euo pipefail

REPO="shaynlink/disblock"
SENTRY_ORG="${SENTRY_ORG:-nvbes}"
target="${1:-}"
case "$target" in
  development) ;;
  staging) app="disblock-api-staging"; worker_config="wrangler.staging.jsonc" ;;
  production) app="disblock-api"; worker_config="wrangler.jsonc" ;;
  *) echo "usage: $0 development|staging|production" >&2; exit 1 ;;
esac
root="$(cd "$(dirname "$0")/.." && pwd)"

# Read a value without echoing it. Empty answers are allowed for optional ones.
ask_secret() {
  local value
  read -rsp "$1: " value </dev/tty
  echo >&2
  printf '%s' "$value"
}

ask_visible() {
  local value
  read -rp "$1: " value </dev/tty
  printf '%s' "$value"
}

# DSN of a Sentry project, from the Sentry CLI when it is logged in, else typed.
# DSNs are public keys (they ship in the web bundle), so they may be shown.
sentry_dsn() {
  local dsn=""
  if command -v sentry >/dev/null && sentry auth status >/dev/null 2>&1; then
    dsn="$(sentry dsn list "$SENTRY_ORG/$1" --json 2>/dev/null \
      | grep -oE 'https://[^"]+@[^"]+ingest[^"]+' | head -n 1 || true)"
  fi
  if [ -z "$dsn" ]; then
    read -rp "DSN of the Sentry project $SENTRY_ORG/$1 (Enter to skip): " dsn </dev/tty
  fi
  printf '%s' "$dsn"
}

# Set KEY=VALUE in a dotenv file, replacing an existing line.
set_dotenv() {
  local file="$1" key="$2" value="$3"
  touch "$file"
  chmod 600 "$file"
  grep -v "^$key=" "$file" >"$file.tmp" || true
  echo "$key=$value" >>"$file.tmp"
  mv "$file.tmp" "$file"
}

if [ "$target" = development ]; then
  echo "== development: local Sentry DSNs (.env and web/.env.local, both git-ignored)"
  server_dsn="$(sentry_dsn disblock-server)"
  web_dsn="$(sentry_dsn disblock-web)"
  [ -f "$root/.env" ] || cp "$root/.env.example" "$root/.env"
  set_dotenv "$root/.env" SENTRY_DSN "$server_dsn"
  set_dotenv "$root/.env" SENTRY_ENVIRONMENT development
  set_dotenv "$root/.env" SENTRY_TRACES_SAMPLE_RATE 1.0
  set_dotenv "$root/web/.env.local" VITE_SENTRY_DSN "$web_dsn"
  echo "Done: restart the API and the Vite dev server to pick them up."
  exit 0
fi

# flyctl's JSON field case differs between versions: match "Name" and "name".
has_fly_secret() {
  flyctl secrets list --app "$app" --json | grep -qiE "\"name\"[[:space:]]*:[[:space:]]*\"$1\""
}

has_github_secret() {
  gh secret list --env "$target" --repo "$REPO" --json name --jq '.[].name' | grep -qx "$1"
}

echo "== $target: GitHub Actions secrets"
if has_github_secret FLY_API_TOKEN; then
  echo "FLY_API_TOKEN already set: kept."
else
  echo "Creating a Fly deploy token for $app and storing it as FLY_API_TOKEN..."
  flyctl tokens create deploy --app "$app" --name "github-actions-$target" \
    | gh secret set FLY_API_TOKEN --env "$target" --repo "$REPO"
fi

cloudflare_token="$(ask_secret "CLOUDFLARE_API_TOKEN ('Edit Cloudflare Workers' permission, Enter to keep the current one)")"
if [ -n "$cloudflare_token" ]; then
  printf '%s' "$cloudflare_token" | gh secret set CLOUDFLARE_API_TOKEN --env "$target" --repo "$REPO"
elif ! has_github_secret CLOUDFLARE_API_TOKEN; then
  echo "CLOUDFLARE_API_TOKEN is required for deploys." >&2
  exit 1
fi

webhook="$(ask_secret "OPS_ALERT_WEBHOOK_URL (Discord or Slack webhook, Enter to skip)")"
if [ -n "$webhook" ]; then
  printf '%s' "$webhook" | gh secret set OPS_ALERT_WEBHOOK_URL --repo "$REPO"
fi

echo "== $target: Fly.io app secrets ($app)"
env_file="$(mktemp)"
trap 'rm -f "$env_file"' EXIT
chmod 600 "$env_file"
{
  echo "DATABASE_URL=$(ask_secret "DATABASE_URL (Neon, disblock_app role: rows only, sslmode=require)")"
  echo "MIGRATION_DATABASE_URL=$(ask_secret "MIGRATION_DATABASE_URL (Neon, owner role: used only by 'migrate' before deploys)")"
  echo "PUBLIC_URL=$(ask_visible "PUBLIC_URL (the Worker's https URL; add PUBLIC_URL/api/auth/callback to Discord OAuth2 redirects)")"
  echo "DISCORD_CLIENT_ID=$(ask_visible "DISCORD_CLIENT_ID")"
  echo "DISCORD_CLIENT_SECRET=$(ask_secret "DISCORD_CLIENT_SECRET")"
} >>"$env_file"

# Regenerating the encryption key would make every stored bot token unreadable.
if has_fly_secret ENCRYPTION_KEY; then
  echo "ENCRYPTION_KEY already set: kept."
else
  echo "ENCRYPTION_KEY=$(openssl rand -base64 32)" >>"$env_file"
  echo "ENCRYPTION_KEY generated. Fly never shows it again: losing it means re-adding every bot."
fi

# The Worker and Fly must share ORIGIN_SECRET: set the Worker first, and give
# Fly the value only once the Worker has it.
if has_fly_secret ORIGIN_SECRET; then
  echo "ORIGIN_SECRET already set: kept."
else
  origin_secret="$(openssl rand -hex 32)"
  if printf '%s' "$origin_secret" \
    | (cd "$root/web" && pnpm exec wrangler secret put ORIGIN_SECRET --config "$worker_config"); then
    echo "ORIGIN_SECRET=$origin_secret" >>"$env_file"
    echo "ORIGIN_SECRET set on the Worker and staged on Fly."
  else
    echo "Worker not reachable yet (first deploy pending): ORIGIN_SECRET skipped." >&2
    echo "Rate limits use Fly-Client-IP until you rerun this script after the first deploy." >&2
  fi
fi

stripe_key="$(ask_secret "STRIPE_SECRET_KEY (Enter to skip billing)")"
if [ -n "$stripe_key" ]; then
  {
    echo "STRIPE_SECRET_KEY=$stripe_key"
    echo "STRIPE_WEBHOOK_SECRET=$(ask_secret "STRIPE_WEBHOOK_SECRET")"
    echo "STRIPE_PRO_PRICE_ID=$(ask_visible "STRIPE_PRO_PRICE_ID")"
  } >>"$env_file"
fi

echo "== $target: Sentry"
# The environment name and trace sample rate live in fly.toml / fly.staging.toml.
server_dsn="$(sentry_dsn disblock-server)"
if [ -n "$server_dsn" ]; then
  echo "SENTRY_DSN=$server_dsn" >>"$env_file"
fi
web_dsn="$(sentry_dsn disblock-web)"
if [ -n "$web_dsn" ]; then
  gh variable set SENTRY_WEB_DSN --env "$target" --repo "$REPO" --body "$web_dsn"
fi
gh variable set SENTRY_ORG --repo "$REPO" --body "$SENTRY_ORG"
if has_github_secret SENTRY_AUTH_TOKEN; then
  echo "SENTRY_AUTH_TOKEN already set: kept."
else
  sentry_token="$(ask_secret "SENTRY_AUTH_TOKEN (organization token with release access, Enter to skip releases and source maps)")"
  if [ -n "$sentry_token" ]; then
    printf '%s' "$sentry_token" | gh secret set SENTRY_AUTH_TOKEN --env "$target" --repo "$REPO"
  fi
fi

# Staged: applied on the next deploy instead of restarting machines now.
flyctl secrets import --app "$app" --stage <"$env_file"
echo "Fly secrets staged for $app."

echo "Done: $target is configured."
