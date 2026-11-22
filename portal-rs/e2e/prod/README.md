# Production synthetic monitor

The monitor runs one Chromium worker at 1440 × 900. It checks the public Polish and English homepages, `/healthz`, and the form reached from `/login`. It then runs the account journey in Polish and English with the same dedicated test account. Use an account reserved for this monitor so no other process changes its settings.

The account journey opens the home page, follows the sign-in link, submits the login form and completes a TOTP challenge if prompted. It opens the account overview, Privacy settings, and the Security landing page. It then switches the stored account language between Polish and English and restores the original. The redirect and selected option confirm each save. Both privacy checkboxes must retain their original values. After checking the current session marker, the journey signs out from the account menu and verifies that the sessions page redirects to a working login form.

## Run

From `portal-rs/e2e`:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npx playwright install chromium
npx playwright test --config prod/playwright.prod.config.mjs
```

Supply `MONITOR_EMAIL` and `MONITOR_PASSWORD` through the environment. If the account uses TOTP, also supply its Base32 `MONITOR_TOTP_SECRET`. The monitor generates codes in memory and waits for a fresh time window when the second login needs it. Set `MONITOR_ANONYMOUS_ONLY=1` for the public checks alone.

The browser defaults to `https://oteryn.com`. For local validation, set `MONITOR_BASE_URL` to the isolated harness portal origin and use a disposable account from its `account` seed command. The optional `account-mfa` seed command enables the TOTP path. The harness portal needs `OTERYN_ACCOUNT_LOGIN`, `OTERYN_ACCOUNT_MFA`, `OTERYN_ACCOUNT_OVERVIEW_ENABLED`, `OTERYN_ACCOUNT_PRIVACY_ENABLED`, `OTERYN_ACCOUNT_CREDENTIALS`, and `OTERYN_ACCOUNT_SESSIONS` set to `true` for the serve process. Set `CHROMIUM_PATH` to use an installed browser.

## Budgets and request limits

| Step | Maximum |
| --- | ---: |
| Public homes, health and anonymous login form | 10 s each |
| Home to login form, overview, settings, security, reopening settings, current session and post-logout redirect | 10 s each |
| Password sign-in, each language save and logout | 15 s each |
| TOTP challenge, including the wait for a fresh code | 45 s |
| Entire Playwright test | 6 min |

A 5xx, browser console error, uncaught browser exception, CSP violation, or over-budget step fails the run. Chromium intercepts every outgoing navigation hop and limits the run to 60 requests, including redirects. The browser loads portal styles and scripts and the Cloudflare beacon script. It reuses successful asset responses within the run and suppresses network traffic for images, fonts, and browser telemetry. The only POSTs that reach the portal are password login, its optional TOTP challenge, the language saves, and logout. The monitor reserves request capacity for restoration and re-reads settings to recover if the alternate save fails.

Failure screenshots are limited to public home pages. Traces, video, and Playwright's automatic failure snapshots are disabled. Workflow diagnostics contain step names, status codes, and fixed error descriptions.

## Scheduled runs and alerts

The GitHub Actions workflow runs every 15 minutes on the default branch once this workflow lands, and supports manual dispatch. Add repository Actions secrets `MONITOR_EMAIL` and `MONITOR_PASSWORD`, plus `MONITOR_TOTP_SECRET` for a TOTP-enabled account. An empty `MONITOR_EMAIL` leaves the workflow idle with a notice.

For email, add `MONITOR_SMTP_HOST`, `MONITOR_SMTP_USER`, and `MONITOR_SMTP_PASSWORD`. The relay must accept the SMTP user as the sender when it is an email address, or `support@oteryn.com` when the user is a relay login. The host accepts a hostname, `smtp://host:port` for STARTTLS, or `smtps://host:port` for implicit TLS. The default ports are 587 and 465 respectively. The open `prod-monitor` issue records the outage across runs. Workflow runs comment on continued failures, close the issue after recovery, and attempt one email to support@oteryn.com per state change.
