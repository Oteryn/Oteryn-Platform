# Staging admin and community staff findings

The Playwright configuration runs Chromium in Polish and English at widths 1440 and 390. The route scenarios assert the deployment boundary. The actor matrix records which application workflows remain unverified.

The final staging run reported 29 passes, four failures from the login-heading defect, and three skips from persistent registration throttling. The registration scenario passed in `pl-390`. All seven route scenarios passed in every project.

## Product defect

**Login heading differs from the source locale in both languages.** Open `/pl/login` or `/en/login` in a signed-out browser. The template renders `login-title` as the level-one heading. Polish should show “Logowanie” and actually shows “Zaloguj się.” English should show “Log in” and actually shows “Sign in.” The mismatch persists after returning to login with Back. It occurs at both viewport widths. The staging build revision could not be established from this browser run. [Login template](../../../../templates/account/login/page.html#L4) · [Polish locale](../../../../templates/account/login/pl.ftl#L1) · [English locale](../../../../templates/account/login/en.ftl#L1)

Screenshots are local at `/tmp/oteryn-m-admin-results/browser-signed-out-browser-64daa-see-the-staff-host-boundary-<project>/login-heading.png`, where `<project>` is `pl-1440`, `pl-390`, `en-1440` or `en-390`.

## Staging limitations

- The specified staging host returned 404 for all 105 inventoried method/path combinations in each project. The admin guard can refuse a request before session resolution for a disabled panel or a different configured hostname. Individual feature gates can also omit routes. This blocks staff navigation, permissions by role, form validation and persisted changes. [Host guard](../../../../crates/admin/src/web.rs#L296) · [Route and actor coverage](COVERAGE.md)
- The public registration form opened, but a valid submission returned the Polish temporary-unavailability page with HTTP 503. The code includes a registration delivery-readiness gate that can return this status. The Mailpit API returned 404 with Access headers initially and at the 10-, 20- and 30-minute checks. Staff reset and MFA enrollment, new player verification, and owner-versus-other-player outcomes were blocked. Polling ended after the non-staff checks finished and the admin routes remained independently unavailable. [Registration delivery gate](../../../../bin/oteryn-portal/src/registration_delivery.rs#L321)
- Shared-source 429 responses during registration were treated as retryable staging noise. The scenario retries after a cooldown and marks continuing rate limitation as blocked.

## Test and runner fixes

- The first dependency install tried to use an unwritable npm cache. The locked install completed using a cache under `/tmp`.
- The first browser run failed before reaching the portal. The fixture now handles each staging request independently, and Chromium reached the localized pages on the following run.
- The first rate-limit retry only checked the final form response. The test now observes the browser's verification request, whose 429 can be followed by a form refusal. The final run reported no unexpected registration responses.
