# Locked-out player staging simulation

Ran against `https://staging.oteryn.com` using Chromium, two workers, and zero Playwright retries. Browser and Mailpit requests used Cloudflare Access on staging. Each scenario used its own synthetic player address. The public registration form has no player-name field. The persona did not name a staff account.

Reset request throttling passed in all four locale and viewport projects. Submitting an unregistered address displayed the expected neutral receipt in both locales. The reset form showed a 503 for a well-formed invalid link in every project. The final Chromium run recorded 4 passes, 4 failures and 32 staging-blocked cases across 40 tests. The registration 503 was recorded in earlier runs.

## Product defects

- **PRODUCT DEFECT — registration unavailable after form submission.** Open `/{lang}/register`, enter a fresh test email and matching strong passwords, accept both consent checkboxes and submit. The handler expects HTTP 303 to the localized check-inbox screen for valid registration, with localized inbox instructions. Staging returned HTTP 503 with the localized registration-unavailable message. This prevented verifying the known-address reset receipt and attempting logins on a registered player account. [Expected redirect and follow-up screen](../../../../crates/web/src/routes/register.rs#L148-L157) · Local screenshot `/tmp/oteryn-lockedout-3J1G2C/lockedout-reset-request-ke-a3846-known-and-unknown-addresses-pl-1440/portal-error.png`
- **PRODUCT DEFECT — reset form returns an availability error for a well-formed invalid link.** Open an invalid reset link, enter the fresh test email and a matching strong password, and submit. The refusal branch redirects with HTTP 302 and the reset form renders a localized invalid-link message. Staging returned HTTP 503 and displayed its localized temporarily-unavailable page. [Refusal handler](../../../../crates/identity/src/reset_http.rs#L325-L342) · [Expected alert](../../../../templates/account/reset/page.html#L5-L6) · Local screenshot `/tmp/oteryn-lockedout-lP2PBp/lockedout-reset-submission-7b78f--link-with-an-invalid-token-en-390/portal-error.png`

## Staging limitations

- `GET /__mailpit/api/v1/messages` returned HTTP 404 while `GET /readyz` returned HTTP 200. The suite observed the neutral receipt for an unregistered address in each locale, but could not observe the absence of delivery. Player verification, emailed reset completion and expiry, session revocation, successful normalized login, and MFA recovery and re-enrolment were blocked. A real unused reset token would require the advertised 60-minute server window. [Reset mail lifetime](../../../../crates/identity/src/reset.rs#L108-L114) · [Verification handler](../../../../crates/web/src/routes/verify.rs#L106-L154)

- Shared-exit HTTP 429 responses interrupted registration on the follow-up run. The suite waited for the server's retry window and made three attempts per scenario. These responses were treated as shared staging capacity.

## Runner issues resolved

- The default npm cache was unavailable in this environment. A temporary writable cache allowed `npm ci --ignore-scripts` to complete.
- Chromium did not trust the environment's HTTPS proxy. The run used the SHA-256 public-key pin of the proxy certificate served for staging, obtained over a CA-validated connection. Playwright kept traces, video and automatic page snapshots off. The only screenshots are redacted and stored outside the repository.
- An early run retried the registration POST alone. The browser also requests a challenge before posting the form, so the retry now observes either response and reopens the registration form after a shared limit.
