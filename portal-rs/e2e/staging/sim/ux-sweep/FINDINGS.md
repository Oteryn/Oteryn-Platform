# Staging UX sweep

The sweep ran on 23 November 2026 against `https://staging.oteryn.com` with Playwright and system Chromium. It used a 1,440px desktop viewport and a 390px touch viewport in Polish and English. The tests checked readiness through `/readyz` before starting. The crawl followed same-origin links breadth first, deduplicated URLs and checked fragments against their destination documents. The limit was 180 destinations and seven levels per actor and project. The keyboard check sampled up to 32 Tab stops per page. External links received href validation. Localized strings were compared with paired entries in the repository’s Fluent catalogs.

## Product defects

### 1. PRODUCT DEFECT — Analytics script fails to load

- **Routes and steps** — Open the public wiki, news archive, sign-in page or another page marked **1** below. In Chromium, inspect the console and failed requests after the page loads.
- **Expected** — The page loads without browser console errors or failed asset requests. The portal serves an explicit allowed-script policy for these documents. [portal-rs/crates/web/src/surface.rs:278](../../../../crates/web/src/surface.rs#L278) · [portal-rs/crates/web/src/chrome.rs:19](../../../../crates/web/src/chrome.rs#L19)
- **Actual** — The staging response includes a Cloudflare analytics script. Chromium blocks that script and records a console error and a failed request. The page itself renders.
- **Screenshots** — pl-1440 `/tmp/oteryn-ux-confirmed/sweep-anonymous-navigation-and-public-interaction-pl-1440/page-008.png` · pl-390 `/tmp/oteryn-ux-confirmed/sweep-anonymous-navigation-and-public-interaction-pl-390/page-010.png` · en-1440 `/tmp/oteryn-ux-confirmed/sweep-anonymous-navigation-and-public-interaction-en-1440/page-008.png` · en-390 `/tmp/oteryn-ux-confirmed/sweep-anonymous-navigation-and-public-interaction-en-390/page-010.png`

### 2. PRODUCT DEFECT — Valid player registrations return 503

- **Routes and steps** — Open `/pl/register` or `/en/register`, enter a fresh test address and matching random compliant passwords, accept the terms and privacy notice, and submit through the browser while `/readyz` is 200.
- **Expected** — A successful submission redirects with HTTP 303 to `/{lang}/register/check-inbox`, which shows the localized inbox instructions. [portal-rs/crates/web/src/routes/register.rs:151](../../../../crates/web/src/routes/register.rs#L151) · [portal-rs/templates/account/verify.html:3](../../../../templates/account/verify.html#L3)
- **Actual** — The browser stays on the registration route and receives HTTP 503 with the localized “Registration is currently unavailable” message. The response matches the registration service-unavailable state. The user cannot reach the inbox step. [portal-rs/crates/identity/src/abuse/http.rs:45](../../../../crates/identity/src/abuse/http.rs#L45) · [portal-rs/crates/identity/src/abuse/en.ftl:4](../../../../crates/identity/src/abuse/en.ftl#L4)
- **Screenshots** — pl-1440 `/tmp/oteryn-ux-redeploy/sweep-fresh-player-registration-and-account-navigation-pl-1440/page-902.png` · pl-390 `/tmp/oteryn-ux-redeploy/sweep-fresh-player-registration-and-account-navigation-pl-390/page-902.png` · en-1440 `/tmp/oteryn-ux-en-1440-player/sweep-fresh-player-registration-and-account-navigation-en-1440/page-902.png` · en-390 `/tmp/oteryn-ux-en-390-player/sweep-fresh-player-registration-and-account-navigation-en-390/page-902.png`

### 3. PRODUCT DEFECT — Staff password reset returns 503 after valid input

- **Route and steps** — Request a reset for the assigned staff account through the public form. Open the newest addressed Mailpit message’s reset link and submit a new compliant password with a matching confirmation.
- **Expected** — The reset completes and redirects to sign-in. [portal-rs/crates/identity/src/reset_http.rs:331](../../../../crates/identity/src/reset_http.rs#L331)
- **Actual** — `POST /en/reset-password` returns HTTP 503 and shows “Temporarily unavailable” after the reset form successfully opens and rejects mismatched passwords. The reset blocks sign-in with the new password and MFA enrollment. [portal-rs/crates/identity/src/reset_http.rs:339](../../../../crates/identity/src/reset_http.rs#L339)
- **Screenshot** — `/tmp/oteryn-ux-confirmed/sweep-platform-admin-sign-in-and-staff-navigation-pl-1440/page-901.png`

## Test bugs corrected

- The search test now asserts the localized 422 title and explanation from the public error page. The account refusal test uses the mounted MFA sign-in template’s localized title. [portal-rs/crates/web/src/surface.rs:244](../../../../crates/web/src/surface.rs#L244) · [portal-rs/templates/account/mfa/page.html:4](../../../../templates/account/mfa/page.html#L4)
- The runner now waits for the submitted document to load and gives keyboard scrolling time to settle before measuring focus. Each crawled document has its own page so late cancellations from the previous visit cannot enter the next page’s checks.
- The executor’s HTTPS proxy caused a certificate failure for the external analytics asset in the first run. The Chromium context now accommodates the configured proxy during this run. The asset returned 200 on pages where Chromium was allowed to load it, and the independent script-load failure in defect 1 persisted on the affected pages.
- The League page appeared in navigation after staging recovered. The test now requires its documented localized authoritative-results message when the game data is unavailable and the handler returns 503. [portal-rs/crates/minigames/src/league/web.rs:217](../../../../crates/minigames/src/league/web.rs#L217)

## Results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Anonymous page crawl | Fail (1) | Fail (1) | Fail (1) | Fail (1) |
| Language switch, browser history and world navigation | Pass | Pass | Pass | Pass |
| Search success, refresh and invalid input | Pass | Pass | Pass | Pass |
| Signed-out account-access refusal | Pass | Pass | Pass | Pass |
| Player registration password mismatch | Pass | Pass | Pass | Pass |
| Valid player registration | Fail (2) | Fail (2) | Fail (2) | Fail (2) |
| Player verification, sign-in, second tab and account crawl | Blocked | Blocked | Blocked | Blocked |
| Staff reset mail delivery and mismatch refusal | Pass, shared setup | Pass, shared setup | Pass, shared setup | Pass, shared setup |
| Staff password reset with valid input | Fail (3), shared setup | Fail (3), shared setup | Fail (3), shared setup | Fail (3), shared setup |
| Staff MFA, administration crawl and visibility of player verification | Blocked | Blocked | Blocked | Blocked |

The first crawl reached 35 destinations per project. After the redeploy, it reached 43 per project with no queued destinations remaining. Each post-redeploy project logged the analytics defect on 32 visits, including the support-link visit that reached sign-in. The browser exercised the opposite-language home through the language switch.

## Staging limitations

- The Mailpit list returned 404 at 11:37, 11:47 and 11:57 UTC. At 12:07 UTC, readiness, the homepage and Mailpit returned 502. Readiness recovered at 12:09:29, and Mailpit returned 200 after recovery and at the 12:17 scheduled check. The assigned staff account’s reset email subsequently arrived.
- The hosted shop fallback returns 200 and advertises that purchases and top-ups are closed. Bazaar, League and atlas delivery returned documented localized unavailable states with HTTP 503. [portal-rs/static/thais/shop-en.html:38](../../../../static/thais/shop-en.html#L38) · [portal-rs/crates/bazaar/src/characters/http.rs:124](../../../../crates/bazaar/src/characters/http.rs#L124) · [portal-rs/crates/minigames/src/league/web.rs:217](../../../../crates/minigames/src/league/web.rs#L217)
- The signed-out `/admin` probe returned 404 after readiness recovered. The staff password reset stopped at defect 3, so authenticated administration and the staff view of player verification remain unverified. [portal-rs/crates/admin/src/web.rs:294](../../../../crates/admin/src/web.rs#L294)
- Shared egress 429s interrupted some registration attempts. The suite retried after waiting and ran each English project individually. The valid-registration 503 reproduced in all four projects.

## Per-page public report

**1** marks the analytics defect. **Unavailable** marks an asserted localized staging feature state. **Switch** marks the opposite-language home visited by the language-switch flow. An arrow shows the observed document redirect. Puzzles, Collection and ticket creation redirected to the language-matched sign-in page. The other requested page checks passed on the recorded visits. [Puzzles handler](../../../../crates/minigames/src/puzzles/http.rs#L77) · [Collection handler](../../../../crates/minigames/src/collection/http.rs#L178) · [Ticket handler](../../../../crates/support/src/tickets/web.rs#L438)

| Destination | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| `/en/` | Switch | Switch | 200 | 200 |
| `/en/atlas/delivery` | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable |
| `/en/bazaar` | 503 · Unavailable · **1** | 503 · Unavailable · **1** | 503 · Unavailable · **1** | 503 · Unavailable · **1** |
| `/en/collection` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/en/events` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/forgot-password` | 200 | 200 | 200 | 200 |
| `/en/help` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/league` | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable |
| `/en/legal/cookies` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/legal/privacy` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/legal/terms` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/login` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/news` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/puzzles` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/en/register` | 200 | 200 | 200 | 200 |
| `/en/rules` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/server-information` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/support/report-a-bug` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/support/tickets/create` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/en/today` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/en/wiki` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/` | 200 | 200 | Switch | Switch |
| `/pl/atlas/delivery` | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable |
| `/pl/bazaar` | 503 · Unavailable · **1** | 503 · Unavailable · **1** | 503 · Unavailable · **1** | 503 · Unavailable · **1** |
| `/pl/collection` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/pl/events` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/forgot-password` | 200 | 200 | 200 | 200 |
| `/pl/help` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/league` | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable | 503 · Unavailable |
| `/pl/legal/cookies` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/legal/privacy` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/legal/terms` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/login` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/news` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/puzzles` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/pl/register` | 200 | 200 | 200 | 200 |
| `/pl/rules` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/server-information` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/support/report-a-bug` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/support/tickets/create` | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** | 303 → 200 · **1** |
| `/pl/today` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/pl/wiki` | 200 · **1** | 200 · **1** | 200 · **1** | 200 · **1** |
| `/static/thais/shop-en.html` | 200 · Unavailable | 200 · Unavailable | 200 · Unavailable | 200 · Unavailable |
| `/static/thais/shop-pl.html` | 200 · Unavailable | 200 · Unavailable | 200 · Unavailable | 200 · Unavailable |

Raw per-page JSON and browser screenshots are available locally under `/tmp/oteryn-ux-confirmed`. Registration evidence is in the corresponding run directories listed above. The suite writes evidence outside the repository and uses explicit per-request staging access headers and one-hop redirect handling. It runs with two workers, zero Playwright retries, and tracing and video disabled.
