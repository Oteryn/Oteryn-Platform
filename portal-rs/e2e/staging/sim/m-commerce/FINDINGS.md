# Staging commerce simulation findings

## Run

`PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts` installed the locked dependencies after redirecting the npm cache to a writable temporary path. The suite runs Chromium at `/usr/bin/chromium` against `https://staging.oteryn.com` with two workers and zero Playwright retries.

Run the suite from `portal-rs/e2e`.

```sh
npx playwright test -c staging/sim/m-commerce/playwright.config.mjs
```

The shared account guard runs before module routing for anonymous GETs under `/account/` and `/panel/`. The suite checks the localized login redirect for anonymous callers and retains the feature-disabled status assertion for signed-in callers. Source [chrome.rs](../../../../crates/web/src/chrome.rs#L28).

The final full run produced 23 passes, 16 skips and one transient failure. The unchanged failed scenario passed on its focused rerun. Each of the 24 public scenario and project combinations passed at least once on the final suite.

## Product defects

No product defects were reproduced in the completed anonymous checks.

## Staging limitations

- `GET /readyz` was 200. `GET /__mailpit/api/v1/messages` returned 404 with staging access at 11:41, 11:51 and 12:01 UTC on November 23, 2026. Staff reset requires this mailbox. Player verification also requires a mailbox link.
- Game-backed Bazaar pages serve the documented localized 503 copy. The fallback can show a public catalogue refusal, and sale, bid and control cannot create a listing. Route-specific statuses are recorded in [COVERAGE.md](COVERAGE.md).
- Direct private-listener routes are checked through the public staging host. The suite cannot create arbitrary private Game traffic or obtain an owner record for a disabled commerce flow.
- The browser registration form opened in all four projects. Each registration submission was rate-limited with 429, including the retry after 65 seconds. Player setup remained blocked.
- An oversized inert form submission to `POST /pl/bazaar/items/{id}/publish` returned one HTTP 502 in `pl-390`. The unchanged Bazaar scenario passed on immediate focused rerun, including the same submission and its localized 503 assertion. The transport response had no browser screenshot.

## Test fixes

- The initial Payments and Bazaar tests assumed the raw handler ran for anonymous account GETs. The corrected tests assert the outer guard's exact 303 and localized login target while preserving the disabled checks for authenticated sessions.
- The first navigation test used the home page with the interior menu selector. The test now opens News, uses the rendered Community menu at both widths and switches locale with the footer link.
