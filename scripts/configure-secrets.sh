#!/usr/bin/env bash
# Configure the secrets of one Disblock environment on GitHub, Fly.io and
# Cloudflare. Values are typed silently or generated locally: they never
# appear as command arguments, so they stay out of shell history.
#
# Requirements: gh, flyctl and wrangler logged in.
# Usage: scripts/configure-secrets.sh staging|production
set -euo pipefail

REPO="shaynlink/disblock"
target="${1:-}"
case "$target" in
  staging) app="disblock-api-staging"; worker_config="wrangler.staging.jsonc" ;;
  production) app="disblock-api"; worker_config="wrangler.jsonc" ;;
  *) echo "usage: $0 staging|production" >&2; exit 1 ;;
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

has_fly_secret() {
  flyctl secrets list --app "$app" --json | grep -q "\"Name\": *\"$1\""
}

echo "== $target: GitHub Actions secrets"
echo "Creating a Fly deploy token for $app and storing it as FLY_API_TOKEN..."
flyctl tokens create deploy --app "$app" --name "github-actions-$target" \
  | gh secret set FLY_API_TOKEN --env "$target" --repo "$REPO"

echo "CLOUDFLARE_API_TOKEN (Cloudflare dashboard > API tokens > 'Edit Cloudflare Workers'):"
ask_secret "CLOUDFLARE_API_TOKEN" | gh secret set CLOUDFLARE_API_TOKEN --env "$target" --repo "$REPO"

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
  echo "PUBLIC_URL=$(ask_visible "PUBLIC_URL (https://... where the app is served)")"
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

sentry_dsn="$(ask_secret "SENTRY_DSN (Enter to skip error tracking)")"
if [ -n "$sentry_dsn" ]; then
  {
    echo "SENTRY_DSN=$sentry_dsn"
    echo "SENTRY_ENVIRONMENT=$target"
  } >>"$env_file"
fi

# Staged: applied on the next deploy instead of restarting machines now.
flyctl secrets import --app "$app" --stage <"$env_file"
echo "Fly secrets staged for $app."

echo "Done: $target is configured."
