# Production synthetic monitor

The monitor runs one Chromium worker with two sequential projects. `pl-1440` covers Polish desktop at 1440 × 900. `en-390` covers English mobile at 390 × 844. Both verify the localized home URL, document language and public news heading. The desktop project also checks that `/healthz` returns the exact body `ok`. These are the only page endpoints in public mode. The authenticated probe allows the canonical HTTPS portal and its sign-in, MFA, account, session, and sign-out routes. Browser telemetry is suppressed.

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

Use a separate password-only `account` seed. With `MONITOR_BASE_URL` pointing to the loopback harness, pass its disposable database URL via `PORTAL_E2E_DATABASE_URL` and the seeded account credentials via `MONITOR_SEEDED_PRIVACY_EMAIL` and `MONITOR_SEEDED_PRIVACY_PASSWORD`. From `portal-rs/e2e`, run `node prod/run.mjs --seeded-privacy`. Each project saves the alternate locale through the real Privacy form, reloads it, and restores the observed starting preference. The check verifies both visibility flags and the privacy revision in PostgreSQL after each save. Cleanup checks PostgreSQL and restores the starting preference even if an assertion fails after the first write.

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

## Scheduled runs

The workflow runs every 15 minutes on the default branch and supports manual dispatch there. It reads these repository secrets for the dedicated, least-privilege production account:

- `OTERYN_MONITOR_EMAIL`
- `OTERYN_MONITOR_PASSWORD`
- `OTERYN_MONITOR_TOTP_SEED`

When a secret is missing, the workflow reports `skipped: monitor account not configured` in the job log and summary, then runs the public checks. With all three secrets, it exercises sign-in, TOTP, account reads, and sign-out on `https://oteryn.com` in both projects. Preference writes run only in the seeded loopback suite.

The workflow token has contents-read access. Reconciliation reads public Actions history and writes the last completed probe outcome to the run summary. An empty, completed cancellation is verified by exact run ID and attempt and passed over. The current probe still controls the job's result. GitHub Actions exposes failures through its normal workflow notifications.
