# Production synthetic monitor

The monitor runs one Chromium worker with two sequential projects. `pl-1440` covers Polish desktop at 1440 × 900. `en-390` covers English mobile at 390 × 844. Each verifies the final localized home URL, document language and public news heading, then signs in through the header, opens account overview, Privacy and Security, checks the current session, and signs out through the account menu. It verifies that revisiting Sessions redirects to the login form. The desktop project also checks `/healthz` and `/login` directly.

The desktop project previews the other language on the Privacy page, verifies it, and clears the preview to restore the observed starting language. The server stores the preview with a 30-second deadline and a restore token. The account's persisted locale and visibility preferences remain intact. An expired preview is ignored on every Privacy read, even if its write commits late or the runner stops. The mobile project reads settings. The portal must be running the server change and migration in this branch before enabling the production probe.

## Run locally

From `portal-rs/e2e`:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npx playwright install chromium
node prod/run.mjs
```

Supply `MONITOR_EMAIL` and `MONITOR_PASSWORD` through the environment. Supply the Base32 `MONITOR_TOTP_SECRET` for a TOTP-enabled account. The monitor requires a challenge whenever that secret is configured and uses a fresh code for the second project. Set `MONITOR_ALLOW_AUTH_WRITES=1` to authorize ordinary authentication writes for the dedicated account. These include login auditing, web session creation and revocation, new-device notification records, and TOTP challenge, counter and grant records. The browser requires the exact serialized form for each permitted POST. The TOTP POST accepts only the CSRF token and current code, so it cannot request a remembered device.

The browser defaults to `https://oteryn.com`. Set `MONITOR_ANONYMOUS_ONLY=1` for public checks alone. The launcher accepts `--project=pl-1440` or `--project=en-390` for local diagnosis. Set `CHROMIUM_PATH` if using an installed Chromium.

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

Use the `account` seed command for a disposable account and optionally `account-mfa` to enroll TOTP. In a second shell, set `MONITOR_BASE_URL` to the harness public origin, using the same hostname as `OTERYN_PUBLIC_ORIGIN` for CSRF. The harness default is `http://localhost:18080`. Supply the seeded credentials through the environment and run the launcher.

## Budgets and request limits

| Step | Maximum |
| --- | ---: |
| Public homes, health and anonymous login form | 10 s each |
| Home to login form, overview, settings, security, reopening settings, current session and post-logout redirect | 10 s each |
| Password sign-in, locale preview, restoration and logout | 15 s each |
| TOTP challenge, including the wait for a fresh code | 45 s |
| Each Playwright project | 6 min |

Any 5xx, browser console error, uncaught browser exception, CSP violation, unexpected browser target, or over-budget step fails the run. Both projects share the 60-request allowance, including redirects. Chromium pauses unexpected targets at startup and context routing rejects requests from outside the monitor frame. Chromium loads portal styles and scripts and the Cloudflare Insights beacon script, reuses successful asset responses within each project, and suppresses network traffic for images, fonts and browser telemetry.

Failure screenshots are limited to public home pages. Traces, video and Playwright's automatic failure snapshots are disabled. The launcher strips Playwright debug settings before startup and emits project names, step names, fixed diagnostics and the request count. Start with `NODE_DEBUG` unset because Node initializes that logger before loading the launcher.

## Owner setup for scheduled runs

Create these GitHub Actions environments in repository settings before enabling the monitor. In each environment, set the deployment branch policy to **Selected branches and tags**, allow the **repository default branch only** (currently `main`), and add no tag rules. Both workflow jobs also check the selected ref against the default branch. Put the credentials only in the environments below. Move any repository-level copies of these secrets to the matching environment.

| Environment | Environment secrets | Environment variable |
| --- | --- | --- |
| `production-monitor-probe` | `MONITOR_EMAIL`, `MONITOR_PASSWORD`, and `MONITOR_TOTP_SECRET` if the dedicated account uses TOTP | Set `MONITOR_ALLOW_AUTH_WRITES` to `1` to authorize the ordinary authentication writes listed above. |
| `production-monitor-alert` | `MONITOR_SMTP_HOST`, `MONITOR_SMTP_USER`, and `MONITOR_SMTP_PASSWORD` to enable email alerts | |

The workflow runs every 15 minutes and supports default-branch manual dispatch. An empty `MONITOR_EMAIL` leaves the workflow idle with a notice. The probe has read-only contents access. The incident job alone has issues-write access. It updates the outage issue and persists the newest probe run number and attempt in a separate `prod-monitor-state` issue. Keep that issue and its label so older reruns cannot change incident state.

The incident job records each transition on the outage issue. SMTP cannot guarantee idempotency, so the workflow writes a permanent per-transition `attempted` label before contacting the relay. Later runs skip that transition even if the pending label is still present. A crash between writing the label and relay acceptance can leave an alert undelivered. A retained pending label alongside an attempted label signals uncertain delivery. Check the relay before any manual resend. The Message-ID is stable for relay lookup.

The relay must accept the SMTP user as sender when it is an email address, or `support@oteryn.com` when the user is a relay login. The host accepts a hostname, `smtp://host:port` for STARTTLS, or `smtps://host:port` for implicit TLS. The default ports are 587 and 465 respectively.
