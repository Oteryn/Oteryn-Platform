# Registration retest

The tests use the shared staging portal identified as `aa1316fd` for this retest. The branch starts at Platform `origin/main` commit `7847cf7d`.

## Results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Registration field validation | Pass | Pass | Pass | Pass |
| Breached-password samples | Pass | Pass | Pass | Pass |
| New-player lifecycle | Fail | Fail | Fail | Fail |
| Password-recovery rate limit | Pass | Pass | Pass | Pass |

The lifecycle test continues past the overview link assertion to verify password reset, rejection of the old password, and revocation of both existing sessions.

## PRODUCT DEFECT

- **Account overview omits the active language from its security shortcut.** Sign in, open `/account` in Polish or English, and inspect the Security shortcut. The overview should use `/{lang}/account/security`, consistent with the neighboring links. The rendered href is `/account/security`. The template emits that literal at `portal-rs/templates/account/overview.html:34`. Evidence is saved locally under `test-results/registration-new-player-ve-33e3d-unt-and-resets-the-password-<project>/overview-links.png`.

## Staging and coverage limits

- The journal report and account Bazaar pages return a localized HTTP 503 because the Game projection and custody authority are unavailable in this staging profile. The tests assert the localized unavailable page and exact status. See `portal-rs/crates/playerdata/src/reports/http.rs:42` and `portal-rs/crates/bazaar/src/characters/http.rs:120`.
- Premium, payment history, and shop history are unmounted in this staging profile. The tests assert their localized HTTP 404 pages. See `deploy/oracle/staging-deploy.py:41`.
- Game character creation and the live character projection are disabled in the staging profile. Registration has no name input. Long, Unicode, and whitespace-only names are covered in the character form when that form is available. A successful Game creation, populated overview, and per-character profile require an available world and character projection. See `deploy/oracle/staging-deploy.py:37` and `portal-rs/crates/characters/src/create_web/http.rs:56`.
- The verification scenario checks the invalid-or-expired message with an altered token, an unknown token, and a consumed token. Actual time-based expiry requires a 24-hour-old player token. See `portal-rs/crates/identity/src/verify.rs:19`.
- The reg2 persona has no assigned staff account. Mail delivery and session revocation are checked through separate browser contexts and GET-only Mailpit requests.

## Iteration

- Test bugs fixed during staging runs included the proxy certificate handling needed for Chromium redirects, the MFA-enabled login refusal text, the canonical `/account` route, and the notifications form's empty action attribute.
- Staging had source-budget refusals during concurrent tests. The suite waits and repeats affected user actions. The password-recovery scenario uses a fresh address for each test and asserts the localized refusal and `Retry-After` after three requests in the same server minute.
