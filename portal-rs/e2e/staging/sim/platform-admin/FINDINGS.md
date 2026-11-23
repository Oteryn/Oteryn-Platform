# Platform administrator staging simulation

Run from `portal-rs/e2e` with `npx playwright test -c staging/sim/platform-admin/playwright.config.mjs`. The suite uses staging browser sessions, the public forms, and the Mailpit GET API. Each test checks `/readyz` before opening the portal. Browser views cover Polish and English at 1440 px and 390 px.

## Staging access

- `/readyz` returned HTTP 200. `/__mailpit/api/v1/messages` returned HTTP 404. Mail was required to finish the founder's public password reset and confirm the fresh colleagues' email addresses.
- A signed-out request to `/admin` returned HTTP 404. The application can return this status before resolving the session, as defined in [the admin host guard](../../../../crates/admin/src/web.rs#L294-L310). The deployed settings could not be confirmed from the browser.
- The public registration test covers the form through the inbox page. The staff journey covers mailbox confirmation, founder sign-in, role changes, permission views, staff MFA, and audit from the browser when the necessary staging routes are available.

## Test adjustments

- Fixed Chromium's handling of the execution environment's HTTPS proxy on browser-managed redirects. Requests with Cloudflare Access headers use Playwright's scoped routing and disable automatic redirect following.
- Matched registration and verification responses to the redirects and localized messages defined in the corresponding handlers and locale files.
- Retried HTTP 429 responses using the server's retry window and treated repeated shared-egress refusals as staging interference.

## Staging run on 2026-11-23 UTC

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Colleague registration through inbox page | Blocked | Blocked | Blocked | Blocked |
| Password recovery request and form validation | Pass | Pass | Pass | Pass |
| Sign-in refusal and language switch | Pass | Pass | Pass | Pass |
| Team onboarding, permission matrix, audit and admin navigation | Blocked | Blocked | Blocked | Blocked |

Playwright finished with eight passes and eight skips. Registration received shared HTTP 429 responses after two waits. The team test stopped at the Mailpit check. No product defects were confirmed in the exercised flows.
