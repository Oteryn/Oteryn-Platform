# Portal worker qualification

This suite uses the PostgreSQL 18.6 and Chromium prerequisites in
[the E2E README](../README.md) and [environment guide](../ENVIRONMENT.md).
The worker launcher requires native Node.js 22.15 or later with `process.execve`
support and checks this before starting the harness.
Supply the existing loopback `PORTAL_E2E_ADMIN_DATABASE_URL` through the
environment and point `PORTAL_E2E_BINARY` at the exact built binary.

```sh
# From portal-rs, with the E2E dependencies already installed.
CARGO_BUILD_JOBS=2 cargo build --locked -p oteryn-portal
export PORTAL_E2E_BINARY="$(realpath "${CARGO_TARGET_DIR:-target}/debug/oteryn-portal")"
node e2e/worker-jobs/run.mjs
```

The launcher uses the harness's existing HTTPS browser proxy, ephemeral TLS
credentials, migrated database, and real `serve`, `worker` and `gateway`
roles. It drains the initial worker before Playwright takes control of worker
lifetimes. Account fixtures use the seed CLI. Historical retention inputs use
the existing PostgreSQL schema and database clock. All four normal Playwright
projects run, with zero retries. Failure diagnostics remain private to the
temporary harness directory.

| Worker job | Evidence |
| --- | --- |
| `cleanup.sessions` | Browser sessions, targeted PostgreSQL backend termination during a DELETE, persisted jittered backoff, second attempt started by the worker, SIGTERM drain, expired browser refused and current browser still signed in. |
| `cleanup.passkeys` | Two-attempt exhaustion of a persistent PostgreSQL error, unchanged dead letter after independent healthy work, and elapsed/fresh ceremony retention. |
| `identity.email-change` | Player-requested change, catcher stopped, 30-second per-message retry, invalid payload quarantined, two workers, SIGTERM during SMTP DATA, one actual message per destination, and confirmation through the localized browser. |
| `cleanup.support-tickets` | Expired closed ticket disappears from the owner browser and a recent closed ticket stays visible after concurrent workers run. |
| `admin.control.expire` | Two dispatches expire a queued request once. The system transition appears in the MFA-authenticated staff audit browser. |
| `referrals.process` | Browser request creates an invitation, concurrent workers deliver it once, and the recipient opens the localized landing page. |
| `cleanup.deaths`, `cleanup.game-logins`, `cleanup.game-ticket-budgets`, `cleanup.hunting-places`, `cleanup.mfa`, `cleanup.rate-limits`, `cleanup.tokens`, `cleanup.world-kills`, `community.today.retention`, `league.retention`, `stats.v2-retention`, `stats.world-sample-retention` | PostgreSQL effects with retained fresh data or ordering fences. Concurrent dispatches preserve the sample aggregate. These tests inspect retained storage directly. |
| `stats.poll` | Two dispatches consume a retained qualified projection and produce one sample and cursor. |
| `identity.finalize-terminations` | Worker visits a due termination. Its account and pending state remain intact when the required Game closure authority is absent. |
| `cleanup.journal-payloads` | Closed-account lifecycle handoff and login history are erased once, while an active account's login history is retained. |
| `identity.account-closure-audit`, `stats.death-retention` | Retention removes aged audit payloads and copied death details while preserving durable closure receipts and recent data. |
| `cleanup.support` | Scheduled claim and completion. The registered handler is a placeholder with no effect ([worker.rs](../../bin/oteryn-portal/src/worker.rs#L66)). |

The automatic schedule test starts both workers against an empty isolated queue
and checks that all 24 locally configured recurring kinds complete there.
Per-job transient failure, permanent failure and mid-job shutdown are only
verified where the table describes those paths. Successful closure finalization
requires Game receipts and a qualified empty character projection. Bazaar
authority and League ingestion require their Game producer boundaries. This
suite uses retained storage fixtures to qualify retention separately from Game
ingress.

[PR #2013](https://github.com/Oteryn/Oteryn-Platform/pull/2013) owns the native
security-revocation, character-service reconciliation, and Store delivery
journeys in [outbox.spec.mjs](../specs/outbox.spec.mjs). The wallet credit path
has its own [wallet-topup spec](../specs/wallet-topup.spec.mjs). Password-reset, registration, security-notification, wallet top-up, and commerce
refund delivery still need their own worker fault qualification.

## Defect

Malformed email-change mail previously returned before acknowledging the row,
so each sweep could pick the same message and block eligible mail behind it.
The identity worker retains a terminal unsent row and removes the sensitive
payload when the route or token is invalid. A structurally valid link for a
different public origin retains its payload and receives the same per-message
backoff as a transient SMTP failure. The per-message state is visible in PostgreSQL.
[email_change_mail.rs](../../crates/identity/src/email_change_mail.rs#L47)
