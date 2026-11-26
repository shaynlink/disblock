# Disblock roadmap to a sellable V1

Today: Discord login, bot hosting, a block editor with 3 triggers and 5
actions, execution history. Enough for toy bots, not for paying customers.

## Defaults until decided otherwise

- **Market**: international, English first; French translation in P1.
- **Pricing**: per bot. Free (1 bot, capped executions, "Made with Disblock"
  in embeds) and Pro per bot (around €5-8/month, to check against
  competitors): unlimited triggers, persistent data, scheduled tasks, custom
  status.
- **First use case**: community bots (welcome, roles, levels), then
  moderation, then tickets.

## P0 — required to sell

| # | Feature | Status |
| --- | --- | --- |
| 1 | Stripe subscriptions: plans, checkout, customer portal, webhooks, limits enforced by the server | |
| 2 | Logic blocks: if/else, comparisons, and/or/not, has role, random, math, wait, stop | done |
| 3 | Persistent data: per-server and per-member variables (counters, points, levels) | done |
| 4 | Rich messages: embeds, buttons and select menus, with "on click" triggers | done |
| 5 | Moderation actions: timeout, kick, ban, delete message, DM a member | done |
| 6 | More events: member leaves, reaction added/removed, scheduled tasks | |
| 7 | Ready-made templates: welcome, moderation, reaction roles, tickets, levels | |
| 8 | Validation in the editor: errors on the faulty block, missing intents detected before saving | done |
| 9 | Legal and account: legal notice, terms, privacy policy, GDPR export and deletion | |
| 10 | Hosting reliability: resume Discord sessions across deploys, execution limits per plan | |
| 11 | Operations: CI/CD, staging, error tracking, alerts, database backups | |

## P1 — competitive at launch

| # | Feature |
| --- | --- |
| 12 | Program version history with one-click restore |
| 13 | Bot presence: custom status and activity |
| 14 | Command permissions and cooldowns |
| 15 | Replace the token of an existing bot |
| 16 | Usage stats: runs per command and per day, error rate |
| 17 | Marketing landing page with pricing and FAQ |
| 18 | Support: community Discord server, status page, block documentation |
| 19 | French and English interface |

## P2 — after launch

- Teams: workspaces and members (a "Team" plan)
- Community template marketplace
- Webhooks and external API calls from blocks
- Several runtime workers sharing the bots (dishard), for high availability
- Command simulator, to test without Discord

## Order

Engine first, since it shapes the program format: 2 → 3 → 4 → 8, then 7
(templates), then 1 (Stripe) and 9 (legal) right before opening sales.
