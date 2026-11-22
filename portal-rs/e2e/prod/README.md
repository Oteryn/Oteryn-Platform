# Production synthetic monitor

The monitor runs one Chromium worker with two sequential projects. `pl-1440` covers Polish desktop at 1440 × 900. `en-390` covers English mobile at 390 × 844. Both verify the localized home URL, document language and public news heading. The desktop project also checks that `/healthz` returns the exact body `ok`. These are the only page endpoints the production monitor may request. Browser telemetry is suppressed.

The sign-in journey runs on the disposable harness. It checks the login form, account overview, Privacy, Security, sessions, logout and the post-logout redirect. Preference writes run in a separate seeded test.

## Run locally

From `portal-rs/e2e`:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npx playwright install chromium
node prod/run.mjs
```

The browser defaults to `https://oteryn.com` and runs public checks only. The launcher accepts `--project=pl-1440` or `--project=en-390` for local diagnosis. Set `CHROMIUM_PATH` if using an installed Chromium.

### Seeded harness

The harness filters the parent environment. Use a temporary binary wrapper to pass the account flags directly to the serve process while keeping the harness migration and seeding steps. From `portal-rs`, with the disposable `PORTAL_E2E_ADMIN_DATABASE_URL` and an absolute `SEED_PLAN` path already set:

```bash
cargo build --locked -p oteryn-portal
monitor_binary="$(realpath "${CARGO_TARGET_DIR:-target}/debug/oteryn-portal")"
monitor_wrapper="$(mktemp)"
trap 'rm -f "$monitor_wrapper"' EXIT
{
  cat <<'WRAPPER'
#!/usr/bin/env bash
if [[ "$1" == serve ]]; then
  export OTERYN_ACCOUNT_LOGIN=true OTERYN_ACCOUNT_MFA=true
  export OTERYN_ACCOUNT_OVERVIEW_ENABLED=true OTERYN_ACCOUNT_PRIVACY_ENABLED=true
  export OTERYN_ACCOUNT_CREDENTIALS=true OTERYN_ACCOUNT_SESSIONS=true
fi
WRAPPER
  printf 'exec %q "$@"\n' "$monitor_binary"
} > "$monitor_wrapper"
chmod 700 "$monitor_wrapper"
PORTAL_E2E_BINARY="$monitor_wrapper" e2e/start.sh --seed-plan "$SEED_PLAN"
```

Use the `account` seed command for a disposable account and optionally `account-mfa` to enroll TOTP. In a second shell, set `MONITOR_BASE_URL` to the harness public origin, using the same hostname as `OTERYN_PUBLIC_ORIGIN` for CSRF. The harness default is `http://localhost:18080`. Set `PORTAL_E2E_DATABASE_URL` to the disposable database and supply the seeded credentials through `MONITOR_EMAIL` and `MONITOR_PASSWORD`. If the fixture uses TOTP, set `MONITOR_TOTP_SECRET` to its Base32 secret. Run `node prod/run.mjs --seeded-auth` from `portal-rs/e2e`. The explicit mode takes precedence over the legacy `MONITOR_ANONYMOUS_ONLY` environment variable. This mode requires a loopback portal and an active seeded account in a local `portal_e2e_` database. It authorizes the ordinary login, session and MFA writes on that fixture. The browser requires the exact serialized form for each permitted POST. The TOTP POST accepts only the CSRF token and current code, so it cannot request a remembered device.

### Seeded Privacy writes

Use a separate password-only `account` seed. With `MONITOR_BASE_URL` pointing to the loopback harness, pass its disposable database URL via `PORTAL_E2E_DATABASE_URL` and the seeded account credentials via `MONITOR_SEEDED_PRIVACY_EMAIL` and `MONITOR_SEEDED_PRIVACY_PASSWORD`. From `portal-rs/e2e`, run `node prod/run.mjs --seeded-privacy`. Each project saves the alternate locale through the real Privacy form, reloads it, and restores the observed starting preference. The check verifies both visibility flags and the privacy revision in PostgreSQL after each save.

## Budgets and request limits

| Step | Maximum |
| --- | ---: |
| Public homes, health and fixture login form | 10 s each |
| Home to login form, overview, settings, security, current session and post-logout redirect | 10 s each |
| Password sign-in and logout | 15 s each |
| TOTP challenge, including the wait for a fresh code | 45 s |
| Each Playwright project | 6 min |

Any 5xx, browser console error, uncaught browser exception, CSP violation, unexpected browser target, or over-budget step fails the run. Both projects share the 60-request allowance, including redirects. Chromium pauses unexpected targets at startup and context routing rejects requests from outside the monitor frame. Chromium loads portal styles and scripts and the Cloudflare Insights beacon script, reuses successful asset responses within each project, and suppresses network traffic for images, fonts and browser telemetry.

Failure screenshots are limited to public home pages. Traces, video and Playwright's automatic failure snapshots are disabled. The launcher strips Playwright debug settings before startup and emits project names, step names, fixed diagnostics and the request count. Start with `NODE_DEBUG` unset because Node initializes that logger before loading the launcher.

## Owner setup for scheduled runs

Create both GitHub Actions environments in repository settings before enabling the monitor or adding credentials. In each environment, set the deployment branch policy to **Selected branches and tags**, allow the **repository default branch only** (currently `main`), and add no tag rules. A preflight job checks both environment names and branch rules before the probe or incident job can start. Keep SMTP credentials in the alert environment.

| Environment | Environment secrets |
| --- | --- |
| `production-monitor-probe` | None |
| `production-monitor-alert` | `MONITOR_SMTP_HOST`, `MONITOR_SMTP_USER`, and `MONITOR_SMTP_PASSWORD` to enable email alerts |

The workflow runs every 15 minutes and supports default-branch manual dispatch. The probe has read-only contents access. The incident job alone has issues-write access and reads completed probe jobs from GitHub Actions history. Job outcomes remain available if the incident job fails to write to Issues. The incident job checks history from the saved checkpoint in bounded time windows, splitting any window that hits the API search cap. Continuity starts at the first unreconciled run. The incident job still checks newer attempts of the checkpoint run when it is retained. Missing unreconciled runs fail visibly and leave the checkpoint in place. Outstanding results are applied in run and attempt order, using the `prod-monitor-state` issue to track reconciliation. An incident-only rerun verifies the original probe attempt before reusing its result. Keep that issue, its label and the unprocessed Actions runs until they have been reconciled.

The incident job validates SMTP configuration before marking a transition `attempted`. Temporary SMTP rejections and disconnects confirmed before DATA authorization clear the attempt so the pending alert can retry. Permanent message rejections receive a separate `rejected` label, are removed from the retry queue, and do not block later alerts. Relay acceptance or an uncertain outcome keeps the attempt recorded to prevent duplicate mail. A crash between recording the attempt and relay acceptance can leave an alert undelivered. If both labels remain, check the relay before any manual resend. The Message-ID is stable for relay lookup.

The relay must accept the SMTP user as sender when it is an email address, or `support@oteryn.com` when the user is a relay login. The host accepts a hostname, `smtp://host:port` for STARTTLS, or `smtps://host:port` for implicit TLS. The default ports are 587 and 465 respectively.

### Recovering from a deleted run range

A history-gap error reports the first missing range as `FIRST..LAST`. Check the workflow history and the storage-retention run before proceeding. If the missing runs still exist, leave the checkpoint alone and rerun after the API can read them.

If the runs were deleted, manually dispatch **Production synthetic monitor** on the default branch with `recovery_checkpoint` set to the exact reported range, for example `102..105`. This confirms that those outcomes are unavailable and that reconciliation may resume after run 105. The workflow rejects a range that differs from the first gap, so retained runs preceding the gap are still reconciled. The `prod-monitor-state` issue records the range, dispatch actor, run ID, and attempt with the updated checkpoint. The skip itself does not open or close an incident. Retained probe outcomes determine incident state. If the job reports another gap, verify that range and submit a new dispatch.
