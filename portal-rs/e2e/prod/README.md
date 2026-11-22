# Production synthetic monitor

The monitor runs one Chromium worker with two sequential projects. `pl-1440` covers the Polish desktop journey at 1440 × 900. `en-390` covers the English mobile journey at 390 × 844. Each verifies the final localized home URL, document language and public news heading, then signs in through the header, opens account overview, Privacy and Security, checks the current session, and signs out through the account menu. It verifies that revisiting Sessions redirects to the login form. The desktop project also checks `/healthz` and `/login` directly.

The desktop project alone writes the account language. It restores the configured baseline first if a previous run was interrupted, then verifies one switch to the other language and one restoration. The mobile project reads the settings. Before a privacy POST leaves the browser, its URL-encoded fields must match the intended locale, CSRF field and original visibility settings. Use a dedicated account so another process cannot change these values mid-run.

## Run

From `portal-rs/e2e`:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npx playwright install chromium
node prod/run.mjs
```

Supply `MONITOR_EMAIL` and `MONITOR_PASSWORD` through the environment. Set `MONITOR_BASELINE_LOCALE` to the dedicated account's intended preference (`pl` by default, or `en`). Supply the Base32 `MONITOR_TOTP_SECRET` for a TOTP-enabled account. The monitor requires a challenge whenever that secret is configured and uses a fresh code for the second project.

The browser defaults to `https://oteryn.com`. Set `MONITOR_ANONYMOUS_ONLY=1` for the public checks alone. For local verification set `MONITOR_BASE_URL` to the isolated harness portal origin, `CHROMIUM_PATH` if using an installed Chromium, and use a disposable account from the `account` seed command. The `account-mfa` seed enables the TOTP path. The harness serve process needs `OTERYN_ACCOUNT_LOGIN`, `OTERYN_ACCOUNT_MFA`, `OTERYN_ACCOUNT_OVERVIEW_ENABLED`, `OTERYN_ACCOUNT_PRIVACY_ENABLED`, `OTERYN_ACCOUNT_CREDENTIALS`, and `OTERYN_ACCOUNT_SESSIONS` set to `true`. The launcher accepts `--project=pl-1440` or `--project=en-390` for local diagnosis.

## Budgets and request limits

| Step | Maximum |
| --- | ---: |
| Public homes, health and anonymous login form | 10 s each |
| Home to login form, overview, settings, security, reopening settings, current session and post-logout redirect | 10 s each |
| Password sign-in, each language save and logout | 15 s each |
| TOTP challenge, including the wait for a fresh code | 45 s |
| Each Playwright project | 6 min |

Any 5xx, browser console error, uncaught browser exception, CSP violation, unexpected browser target, or over-budget step fails the run. The 60-request allowance is shared across both projects and includes redirects. Chromium pauses unexpected targets at startup and context routing also rejects requests from outside the monitor frame. The browser loads portal styles and scripts and the Cloudflare Insights beacon script, reuses successful asset responses within each project, and suppresses network traffic for images, fonts and browser telemetry. The POST allowlist covers sign-in and its optional TOTP challenge, the verified language saves, and logout.

Failure screenshots are limited to public home pages. Traces, video and Playwright's automatic failure snapshots are disabled. The launcher strips Playwright debug settings before startup and emits project names, step names, fixed diagnostics and the request count. Start the launcher with `NODE_DEBUG` unset because Node initializes that logger before loading the launcher.

## Scheduled runs and alerts

The workflow runs every 15 minutes on the default branch and supports manual dispatch. Add Actions secrets `MONITOR_EMAIL` and `MONITOR_PASSWORD`, plus `MONITOR_TOTP_SECRET` for a TOTP-enabled account. Set the Actions variable `MONITOR_BASELINE_LOCALE` to `en` if the account should retain English. An empty `MONITOR_EMAIL` leaves the workflow idle with a notice.

The probe has read-only contents access. A separate issues-write job creates or updates one `prod-monitor` issue and closes it after recovery. It records each email transition on the issue with a pending label. Later runs retry pending alerts, including on closed issues, and clear each label after SMTP accepts the message. Each transition uses a stable Message-ID across retries. If the runner stops after the relay accepts a message and before GitHub clears its label, the next run may deliver it again.

For email, add `MONITOR_SMTP_HOST`, `MONITOR_SMTP_USER`, and `MONITOR_SMTP_PASSWORD`. The relay must accept the SMTP user as sender when it is an email address, or `support@oteryn.com` when the user is a relay login. The host accepts a hostname, `smtp://host:port` for STARTTLS, or `smtps://host:port` for implicit TLS. The default ports are 587 and 465 respectively.
