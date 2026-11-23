# Platform administrator staging simulation

Run from `portal-rs/e2e` with `npx playwright test -c staging/sim/platform-admin/playwright.config.mjs`. The suite uses staging browser sessions, the public forms, and the Mailpit GET API. Each test checks `/readyz` before opening the portal. Browser views cover Polish and English at 1440 px and 390 px.

## Staging access

- `/readyz` returned HTTP 200. Mailpit initially returned HTTP 404. Ten-minute checks continued until it returned HTTP 200 at 12:10 UTC, and the founder's reset email arrived.
- Signed-out requests to `/admin`, `/admin/roles`, and `/admin/console/access` continued to return HTTP 404 after Mailpit recovered. The [admin host guard](../../../../crates/admin/src/web.rs#L294-L310) can emit 404 before resolving a browser session. The deployed admin settings could not be confirmed from the browser.
- The public registration test covers the form through the inbox page. The staff journey covers mailbox confirmation, founder sign-in, role changes, permission views, staff MFA, and audit from the browser when the necessary staging routes are available.

## Test adjustments

- Fixed Chromium's handling of the execution environment's HTTPS proxy on browser-managed redirects. Requests with Cloudflare Access headers use Playwright's scoped routing and disable automatic redirect following.
- Handled Chromium requests canceled by reload or teardown without attempting to close an already-completed route a second time.
- Matched registration and verification responses to their handlers. Used the staging login handler's MFA template for the localized credential refusal.
- Reopened the original GET form before retrying a rate-limited POST. Going back through Chromium's submission history could leave the rate-limit page in place.
- Retried HTTP 429 responses using the server's retry window and treated repeated shared-egress refusals as staging interference.

## Staging run on 2026-11-23 UTC

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Colleague registration through inbox page | Blocked | Blocked | Blocked | Blocked |
| Password recovery request and form validation | Pass | Pass | Pass | Pass |
| Sign-in refusal and language switch | Pass | Pass | Pass | Pass |
| Team onboarding, permission matrix, audit and admin navigation | Blocked | Blocked | Blocked | Blocked |
| Founder password reset | — | — | Fail | — |

The last full run recorded six passes and fourteen skips. Both English password recovery request checks passed in earlier runs and encountered shared HTTP 429 responses in the last full run. Registration remained subject to the same interference. The team journey stopped at the admin entry point. The founder defect was reproduced in two focused English desktop checks. The new dedicated case retains the expected redirect assertion and encountered shared throttling on rerun.

## Product defect

**Founder password reset ends at HTTP 503.** Submit the public forgot-password form for the staging founder, open the fresh link delivered to the account, and submit matching values for a new password. `POST /en/reset-password` should redirect to sign-in with HTTP 302 after completion. Two independent attempts showed the portal's “Temporarily unavailable” page with HTTP 503. The underlying service failure was unconfirmed. [Expected redirect](../../../../crates/identity/src/reset_http.rs#L330-L339) · Screenshot `test-results/evidence/founder-reset-503.png` (local artifact).
