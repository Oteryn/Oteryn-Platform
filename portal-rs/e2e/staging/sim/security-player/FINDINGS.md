# Staging security-player simulation

Target `https://staging.oteryn.com`. Chromium at `/usr/bin/chromium`, two workers and zero Playwright retries. The suite uses fresh player addresses, browser forms and same-origin HTTP. Mailpit reads use GET.

```sh
cd portal-rs/e2e
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/security-player/playwright.config.mjs
```

The table combines the confirmed failures from the first complete run with the final full run. The final full run had eight passes and 48 skips, including four signup retries blocked by the shared ingress quota.

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Registration input validation | Pass | Pass | Pass | Pass |
| Valid registration | Fail | Fail | Fail | Blocked by quota |
| Email verification | Blocked | Blocked | Blocked | Blocked |
| Signed-out access to account settings | Pass | Pass | Pass | Pass |
| Password change and notification | Blocked | Blocked | Blocked | Blocked |
| Email change and both notifications | Blocked | Blocked | Blocked | Blocked |
| TOTP, recovery codes and disable availability | Blocked | Blocked | Blocked | Blocked |
| One-time account recovery key | Blocked | Blocked | Blocked | Blocked |
| Passkey registration and sign-in | Blocked | Blocked | Blocked | Blocked |
| Session list, revocation and new-device notification | Blocked | Blocked | Blocked | Blocked |
| Security wizard | Blocked | Blocked | Blocked | Blocked |
| Privacy and language persistence | Blocked | Blocked | Blocked | Blocked |
| Personal data export, re-authentication and repeat limit | Blocked | Blocked | Blocked | Blocked |
| Account closure | Blocked | Blocked | Blocked | Blocked |

## PRODUCT DEFECT

- **Valid signup returns HTTP 503.** Open `/{pl,en}/register`, enter a fresh email and matching valid password, accept both legal documents, then submit. Expected HTTP 303 to `/{lang}/register/check-inbox` and the localized inbox instruction. Actual HTTP 503 with “Registration is currently unavailable. Please try again later.” or the Polish equivalent. The public response does not identify which dependency failed. [Registration success handler](../../../../crates/web/src/routes/register.rs#L151) · [Unavailable response](../../../../crates/identity/src/abuse/http.rs#L41) · [Inbox template](../../../../templates/account/verify.html#L3)

  The first complete run captured screenshots at the paths below. Playwright cleared that output directory on the rerun.

  - `/tmp/oteryn-staging-security-player/registration-valid-registration-reaches-the-check-inbox-page-pl-1440/failure.png`
  - `/tmp/oteryn-staging-security-player/registration-valid-registration-reaches-the-check-inbox-page-pl-390/failure.png`
  - `/tmp/oteryn-staging-security-player/registration-valid-registration-reaches-the-check-inbox-page-en-1440/failure.png`

## Staging limitations and coverage

- The shared ingress quota still returned HTTP 429 during all four valid-registration cases on the final full run. Each case retried before being marked blocked. A separate `en-390` run hit the same quota.
- The supplied `GET /__mailpit/api/v1/messages?limit=100` endpoint returned HTTP 404 while `/readyz` returned 200. Email verification and every fresh-player scenario that needs it are skipped explicitly. The ingress configuration forwards requests to the portal. [Staging ingress](../../../../../deploy/oracle/staging.compose.yml#L78)
- The persona names no staff role. The session scenario uses a second browser context for the same player and the email change examines both test-owned inboxes.
- Account closure scheduling, sign-out and the pending state are covered when the Game-backed route is available. Finalization requires the grace period and a verified Game receipt. Post-finalization sign-in refusal could not be reached. [Closure UI and acceptance gate](../../../../crates/identity/src/account_lifecycle_http.rs#L470) · [Default grace period](../../../../crates/identity/src/termination.rs#L54)
- The TOTP case checks for a player-facing disable action after re-authentication. The current source exposes only enrollment, challenge and trusted-device management to players. The disable interaction could not be completed or verified on staging. [MFA routes](../../../../crates/identity/src/mfa_http.rs#L65)
- Mail assertions cover the messages defined for password changes, email confirmation and cancellation, and a new browser device. [Notification templates](../../../../crates/notify/src/service.rs#L111) · [Email-change deliveries](../../../../crates/identity/src/email_change.rs#L234)

## Test and environment fixes

- Used a writable temporary npm cache after the default cache failed. Configured Chromium and Playwright's HTTP transport to accept the execution environment's proxy CA.
- The registration JavaScript can turn a shared challenge refusal into a verification-error page. The suite now waits out an observed HTTP 429 from either the challenge endpoint or the form POST and retries the original browser action. The valid-registration assertion still requires HTTP 303.
