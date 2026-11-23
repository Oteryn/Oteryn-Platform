# Newcomer staging simulation

Run against `https://staging.oteryn.com` with system Chromium, Polish and English, and 1440 px and 390 px viewports. The suite waits for `/readyz` before each test. Run the following from `portal-rs/e2e`.

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/newcomer/playwright.config.mjs
```

Screenshots are under `/tmp/oteryn-newcomer-*/` on the runner. The Markdown paths below are relative to `portal-rs/` unless they start with `/tmp/`.

## Staging results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
|---|---|---|---|---|
| Homepage, navigation, footer and language | Pass | Fail (N2) | Pass | Fail (N2) |
| Public catalogue, Game availability and wiki errors | Pass | Pass | Pass | Pass |
| Anonymous private-page redirects and login refusal | Pass | Pass | Pass | Pass |
| Registration form validation | Pass | Pass | Fail (N1) | Pass |
| Valid registration and duplicate email | Blocked (429) | Blocked (429) | Fail (N1) | Blocked (429) |
| Malformed link and emailed verification flow | Blocked (429) | Blocked (429) | Blocked (429) | Blocked (429) |
| Verified account, preferences and concurrent sessions | Blocked (Mailpit) | Blocked (Mailpit) | Blocked (Mailpit) | Blocked (Mailpit) |

The full run completed with 12 passes, two mobile failures and 14 blocked cases. The table includes focused reruns. Polish desktop registration validation passed after a quota wait. An English desktop retry encountered Chromium `ERR_FAILED` before an HTTP response. The next retry reached the localized N1 response. The malformed verification check completed before the signup dependency in all four projects.

## Product defects

### N1. Registration returns HTTP 503 on form load or submit

- Routes `GET /en/register` and `POST /en/register`.
- Open registration as a visitor. If the form opens, enter a fresh valid email address and a matching random strong password, accept both current legal editions, and submit.
- Expected HTTP 200 and the form on GET, then HTTP 303 to the inbox instructions on valid POST. See `crates/web/src/routes/register.rs:55` and `:142`.
- Actual HTTP 503 before the inbox page. The first run reached the POST. Focused retries received the same refusal when loading the form. The English desktop browser displayed “Registration is currently unavailable. Please try again later.” The registration protection layer uses this refusal for disabled or unavailable protection. The underlying dependency could not be determined through the browser. See `crates/identity/src/abuse/http.rs:41`, `:112` and `:159`.
- Form-load screenshot `/tmp/oteryn-newcomer-en-validation/registration-registration--60b48-rves-anonymous-consent-flow-en-1440/test-failed-1.png`. A full rerun reset the output directory that held the first-run POST screenshot.

### N2. Tapping outside the expanded homepage group loses the selected navigation action

- Routes `GET /pl/` and `GET /en/` at 390 px.
- Tap Menu, open Game World, then tap Community while Game World is expanded. Open Community with Enter, then tap Getting started in the header.
- Expected Community opens with the mobile menu still visible. Getting started should navigate to `#jak-zaczac`. See `templates/components/thais-header.html:69` and `static/thais/home.js:63`, `:125`.
- Actual the entire menu closes and the selected destination is lost. The Community button receives focus before the click lands outside the header. Opening Community with Enter and following the Getting started footer link work. See `static/thais/home.js:107` and `:112`.
- Example screenshots `/tmp/oteryn-newcomer-playwright/public-anonymous-homepage--cfd21-ooter-and-language-switches-pl-390/mobile-menu-switch.png` and `/tmp/oteryn-newcomer-playwright/public-anonymous-homepage--cfd21-ooter-and-language-switches-pl-390/mobile-getting-started.png`.

## Staging limitations

- The Mailpit listing endpoint returned HTTP 404. Verification with a delivered link and signed-in account settings, profile, device sessions and post-login destinations require a new verified account. A malformed verification link was checked independently. The duplicate-email scenario stopped at the first registration when it returned HTTP 503.
- The game-backed world, rankings and statistics, bestiary, atlas and spawns routes and the league, puzzles, collection and download entry points returned the disabled 404 surface. Bazaar returned the localized HTTP 503 authority-unavailable page. The getting-started guide links to server information for client downloads and the store link opens its localized static preview. Checks for Game and commerce covered public availability.
- The persona does not name any staff account. The suite uses a second tab and a separate browser context to check player-visible state.
- Shared-source HTTP 429 responses are retried after the advertised interval when available. A registration case remains blocked if the quota is still exhausted after the retries.

## Test fixes during iteration

Corrected localized title expectations to match the rendered home, MFA login, events, Today and legal templates. Updated login form selection to include buttons whose submit type is implicit. Adjusted route teardown to tolerate browser requests cancelled during context closure.
