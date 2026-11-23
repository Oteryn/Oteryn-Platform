# Staging findings

Staging was exercised with Playwright Chromium against `https://staging.oteryn.com`. See the route, actor and fixture limits in [COVERAGE.md](COVERAGE.md).

## Run results

The full suite ran with `npx playwright test -c staging/sim/m-community/playwright.config.mjs` from `portal-rs/e2e`. After the staging restart, 24 cases passed and 8 were rate-limited after retries. An earlier run also verified the mismatch response with a 422 and a localized field error.

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
|---|---|---|---|---|
| Community navigation and public projections | Pass | Pass | Pass | Pass |
| Party and staff queue availability | Pass | Pass | Pass | Pass |
| Account entry and invitation availability | Pass | Pass | Pass | Pass |
| Minigame sign-in, league and season pass | Pass | Pass | Pass | Pass |
| Download origin and aliases | Pass | Pass | Pass | Pass |
| Fresh signup and verified player entry | Rate-limited | Rate-limited | Rate-limited | Rate-limited |
| Mismatched passwords | Rate-limited | Rate-limited | Rate-limited | Rate-limited |
| Community and league producer availability | Pass | Pass | Pass | Pass |

## Product defects

### Registration does not reach the inbox confirmation

- Route and steps. From localized home, open sign-in and then registration. Enter a fresh synthetic email and a valid matching password, accept both legal documents, and submit.
- Expected. The valid submission redirects with 303 to `/{lang}/register/check-inbox` and displays the localized inbox instructions. [register.rs](../../../../crates/web/src/routes/register.rs#L142), [verify.html](../../../../templates/account/verify.html)
- Actual. Before the staging restart, a valid Polish registration returned 503 and displayed “Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.” Registration admission uses this status for a disabled or unavailable prerequisite. The specific prerequisite remains undetermined. [abuse/http.rs](../../../../crates/identity/src/abuse/http.rs#L41)
- Evidence. The initial run wrote the localized error page to the ignored `portal-rs/e2e/test-results/community-fresh-player-reg-03186-ough-the-sign-in-navigation-pl-1440/error-context.md`. That run produced a page snapshot only. A later screenshot replay hit the staging gateway outage and subsequent signup attempts were rate-limited.

## Staging limitations

- Mailpit returned 404 on the initial check and both 10-minute rechecks while staging was ready. At about 12:07 UTC both readiness and Mailpit returned Cloudflare 502. At 12:09 UTC both returned 200. The party board and staff queue still returned 404, so no staff account was reset and no MFA enrollment ran.
- The later deployment enabled puzzles, collection, PvP, league, badges and the profile editor. Visitor sign-in and refusal checks ran for them. League results remained unavailable, and the download origin had no selected release documents.
- Shared-egress limits prevented a verified fresh-player session on the later deployment. Ownership, persistence, double submission and cross-actor outcomes remain unverified.
- Shared egress occasionally returned 429 during registration or its challenge. The runner waits and repeats the normal submission. A remaining rate limit is a blocked execution rather than a defect.

## Test bugs corrected during iteration

- The locale switch is in the footer. The sign-in page headings on the deployment are “Sign in” and “Zaloguj się”.
- A form retry previously assumed reopening the page would return 200. The runner now checks readiness and the response status before looking for form fields.
- Anonymous account and pass GET requests redirect in the shell before module dispatch. Mutation expectations now distinguish that shell redirect from a disabled handler or an unmounted route. [chrome.rs](../../../../crates/web/src/chrome.rs#L24)
- npm could not create its default cache under the environment's home directory. Installation succeeded with a cache under `/tmp`.
