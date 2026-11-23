# Staging commerce simulation findings

## Run

Playwright ran Chromium at `/usr/bin/chromium` against `https://staging.oteryn.com` with two workers and zero Playwright retries. The locked dependency install used a writable temporary npm cache.

Run from `portal-rs/e2e`.

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/m-commerce/playwright.config.mjs
```

The route matrix covers 84 method and path cases. The shared account guard runs before module routing for anonymous GETs under `/account/` and `/panel/`. The suite checks the localized login redirect for anonymous callers and retains feature-disabled status assertions for signed-in callers. [Route and actor matrix](COVERAGE.md) · [Chrome guard](../../../../crates/web/src/chrome.rs#L28)

The last full run had 24 passes, one failure and 15 skips. All public cases passed in all four projects. The staff reset defect failed the Platform Administrator setup test. An earlier focused run reproduced the same reset failure for Moderator and Support and the registration failure in both English projects. Staff account setup runs once per role in `en-1440` to avoid overlapping resets across projects.

## Product defects

- **Registration unavailable.** Open the English registration form, enter a fresh email and matching generated passwords, accept the terms and privacy notice and submit. Expected HTTP 303 to the check-inbox page. Actual HTTP 503 reproduced at desktop and mobile widths after retrying the shared rate limit. No registration screenshot was captured. The focused screenshot rerun also ended at the shared 429. [Expected success](../../../../crates/web/src/routes/register.rs#L151) · [Unavailable response](../../../../crates/identity/src/abuse/http.rs#L41)
- **Staff password reset unavailable.** Request reset for the Platform Administrator, Moderator or Support staging account. Open the newest matching email and submit a new matching password. Expected HTTP 302 to sign-in. Actual HTTP 503 on the reset submission for each role after Mailpit recovered. The response blocks password reset completion and MFA enrollment. Local screenshot `test-results/commerce-commerce-disabled-for-staff-platform-admin-en-1440/reset-unavailable.png`. [Expected success and unavailable branch](../../../../crates/identity/src/reset_http.rs#L330)

The browser evidence establishes the failing stage. The server-side cause remains unverified.

## Staging limitations

- Readiness returned HTTP 200 during the runs. Authenticated Mailpit requests returned 404 at 11:41, 11:51 and 12:01 UTC on November 23, 2026. The endpoint returned 200 at 12:11 UTC and all three staff reset emails arrived afterwards.
- Shared-source 429s persisted after a 65-second retry in all four registration projects in the last run. They also blocked the last repeat of the Moderator and Support reset requests. Earlier requests reached the 503 responses recorded above.
- Bazaar fallback pages rendered their localized 503 messages. Game-backed success, persisted commerce records and comparisons between a record owner and another player require an active commerce service and authenticated accounts.
- Private service routes were checked through the public staging host.
- Before Mailpit recovered, an oversized inert form submission to `POST /pl/bazaar/items/{id}/publish` returned one HTTP 502 in `pl-390`. The unchanged scenario passed on its focused rerun, including the same submission and its expected localized 503. The one-off HTTP response had no browser screenshot.

## Test fixes

- The first Payments and Bazaar tests assumed module handlers answered anonymous account GETs. The revised tests assert the outer guard's exact 303 and localized login target and retain the signed-in disabled checks.
- The first navigation test opened the home page with an interior menu selector. It now opens News, uses the Community menu at both widths and switches locale with the footer link.
- Account setup failures now identify the stage and HTTP status using messages that omit credentials, form values and mail content.
