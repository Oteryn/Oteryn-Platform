# Content editor staging simulation

The target is `https://staging.oteryn.com`. The suite uses system Chromium at 1440 px and 390 px in Polish and English. Each test waits for `GET /readyz` to return HTTP 200.

Run from `portal-rs/e2e`.

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/content-editor/playwright.config.mjs
```

## Staging results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Anonymous language switching and missing content | Pass | Pass | Pass | Pass |
| Anonymous CMS host refusal | Pass | Pass | Pass | Pass |
| Registration input and localized unavailable state | Pass | Pass | Pass | Pass |
| Verified player CMS refusal | Blocked | Blocked | Blocked | Blocked |
| Editor recovery and MFA sign-in | Blocked | Blocked | Blocked | Blocked |
| Support CMS refusal | Blocked | Blocked | Blocked | Blocked |
| News lifecycle and scheduling | Blocked | Blocked | Blocked | Blocked |
| News validation | Blocked | Blocked | Blocked | Blocked |
| Localized page lifecycle | Blocked | Blocked | Blocked | Blocked |
| Wiki preview, lifecycle and revision history | Blocked | Blocked | Blocked | Blocked |

No product defects were confirmed in the completed scenarios. Registration screenshots remain in the local Playwright output directory at `/tmp/oteryn-content-editor-results/`.

## Staging limitations

- Mailpit initially returned HTTP 404 for `GET /__mailpit/api/v1/messages` while readiness returned HTTP 200. Mailpit began returning HTTP 200 at 12:14 UTC.
- The editor requested a reset through the public form and received a reset email. The link opened successfully. Submitting a generated password returned HTTP 503, so sign-in and MFA could not proceed. The reset handler returns 503 when the password checking service is unavailable. `portal-rs/crates/identity/src/reset.rs:170`, `portal-rs/crates/identity/src/reset_http.rs:339`
- `GET /admin/news/create`, `/admin/pages` and `/admin/wiki/articles/create` returned HTTP 404 to anonymous browsers. The staff host guard returns 404 when the panel is disabled or the configured host differs from the request host. This establishes public host refusal. Authenticated CMS workflows and the Support permission check remained blocked. `portal-rs/crates/admin/src/web.rs:300`
- The registration form accepted input, then a normal valid submission returned HTTP 503 with the localized registration unavailable message. The suite asserts that state and the browser’s return to the form. A working registration still requires HTTP 303, the check-inbox URL and its localized message. The handler maps an unavailable registration dependency to 503 (`portal-rs/crates/identity/src/abuse/http.rs:45`, `:170`). The operations notes describe how an inaccessible password range service with a cache miss produces this outcome (`portal-rs/crates/identity/src/abuse/OPERATIONS.md:34`). The registration response does not identify the unavailable dependency. Fresh player verification and the authenticated permission check therefore remained blocked.
- The editor role can read wiki revision history. The platform audit console checks separate audit permissions (`portal-rs/crates/admin/src/console/audit.rs:74`). Its entries could not be verified from an authorized actor in this simulation.

A Cloudflare 502 appeared on a registration-page history navigation. The runner retries a Cloudflare 502 on a staging GET once. The test still requires the registration form to render.

## Test bugs fixed

- Chromium rejected the execution proxy certificate on same-origin redirects. The runner pins the execution proxy certificate’s public key when the environment provides one.

- Readiness now uses the browser context’s request transport, which reaches staging through the execution environment’s proxy.
- The registration assertion now waits for the portal’s own browser challenge to finish. The first run timed out while the form script was still working.
- The registration unavailable assertion checks the shared error template’s localized home link and uses browser Back to reopen the original form.

- Password-reset mail uses the account’s stored language. The runner searches for both localized reset subjects and continues in the language of the email link.
