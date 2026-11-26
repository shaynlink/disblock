# Operations runbook

This repository has four operational workflows. They are intentionally
configuration-only until the required GitHub environments, credentials and
external accounts are configured; no production service is touched by merging
these files.

| Capability | Implementation | Trigger |
| --- | --- | --- |
| Verification | `.github/workflows/ci.yml` | Pull requests and `main` |
| Staging and promotion | `.github/workflows/deploy.yml` | Successful `main` CI; manual production promotion |
| Availability alerts | `.github/workflows/health.yml` | Every 15 minutes |
| Backup and restoration | `.github/workflows/database-backup.yml`, `restore-drill.yml` | Daily 03:17 UTC; manual restore drill |

## One-time setup

Create GitHub Environments named `staging`, `production` and `restore-drill`.
Require approval for `production` and `restore-drill`; this makes production
promotion and a destructive restore explicit actions.

For each deployment environment, configure these GitHub Environment values:

| Name | Kind | Meaning |
| --- | --- | --- |
| `FLY_API_TOKEN` | Secret | Deploy-only Fly token, scoped to the matching app. |
| `FLY_APP` | Variable | Fly application name (`disblock-api-staging` or `disblock-api`). |
| `HEALTH_URL` | Variable | Direct public API origin, without `/api/health`. |
| `CLOUDFLARE_API_TOKEN` | Secret | Token able to deploy only the matching Worker. |
| `CLOUDFLARE_ACCOUNT_ID` | Variable | Account hosting that Worker. |
| `OPS_ALERT_WEBHOOK_URL` | Secret | Slack-compatible or Discord webhook for operational failures. |

The staging app needs its own Neon branch/database, Discord OAuth application,
Discord bot tokens, Stripe **test-mode** credentials, and Sentry project/DSN.
Never copy production tokens or `ENCRYPTION_KEY` to staging. Configure all Fly
secrets in each app before the first deployment; `fly.staging.toml` only holds
non-secret runtime settings.

At repository scope, set `STAGING_HEALTH_URL` and `PRODUCTION_HEALTH_URL` for
the scheduled probes. They should be the direct Fly origins so an unhealthy
worker cannot mask a failed API.

## Error tracking

The API initializes Sentry only when `SENTRY_DSN` is set. Errors logged at the
`ERROR` level become issues; INFO-and-higher tracing spans are sampled at 10%
by default. `SENTRY_TRACES_SAMPLE_RATE` accepts a value from `0` to `1`, and
`RELEASE_VERSION` is baked into the deploy image from the Git revision.

Set the following Fly secrets for each environment:

```bash
fly secrets set SENTRY_DSN='https://public-key@o0.ingest.sentry.io/project' \
  SENTRY_TRACES_SAMPLE_RATE=0.1 --app disblock-api-staging
```

Create two Sentry issue alerts: one for a new production issue and one for a
regression. Route both to the same on-call destination as
`OPS_ALERT_WEBHOOK_URL`. The instrumentation deliberately excludes DEBUG
spans and does not send structured application logs, reducing the risk that
user-supplied Discord content reaches Sentry.

## Backups and restore drills

The daily job creates a PostgreSQL custom-format dump, encrypts it client-side
with `age`, and stores only the encrypted object plus its SHA-256 checksum in
S3 with SSE-KMS. Set these repository values:

| Name | Kind | Meaning |
| --- | --- | --- |
| `BACKUP_DATABASE_URL` | Secret | Direct Neon production connection string, backup role only. |
| `BACKUP_AGE_RECIPIENT` | Secret | Public `age1…` encryption recipient. |
| `BACKUP_S3_URI` | Variable | `s3://bucket/prefix`, in a dedicated backup account/prefix. |
| `BACKUP_KMS_KEY_ID` | Secret | KMS key ARN or ID used for object encryption. |
| `AWS_REGION` | Variable | Region of the backup bucket. |
| `AWS_BACKUP_ROLE_ARN` | Secret | GitHub OIDC role with only the required prefix permissions. |
| `BACKUP_AGE_IDENTITY` | Secret | Private age identity, available **only** to `restore-drill`. |
| `BACKUP_RESTORE_DATABASE_URL` | Secret | Isolated Neon restore-drill branch; never staging or production. |

Configure S3 versioning, Object Lock (or an immutable retention policy), and a
lifecycle rule that retains daily backups for at least 35 days and monthly
backups for at least 12 months. The IAM role needs `PutObject` for the backup
workflow and `GetObject` only for the `restore-drill` environment.

Run a restore drill monthly: select a recent encrypted `s3://…/*.dump.age`
object in **Database restore drill**, approve the protected environment, then
review the users/bots counts from the job log. The workflow runs `pg_restore
--clean` against the isolated target, so its target must be disposable.

## Incident response

1. A failed deployment, probe or backup sends a link to the exact GitHub run.
   A production deployment remains protected by the GitHub Environment gate.
2. For an API outage, check the Fly health check and Sentry issue/trace, then
   roll back with `fly releases rollback --app disblock-api`.
3. For data loss, first create a Neon branch from the relevant restore point.
   Use the restore drill workflow on that branch; do not restore directly over
   production. Validate row counts and application health before any cutover.
