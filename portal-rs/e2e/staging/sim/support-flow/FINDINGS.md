# Support-flow staging simulation

On 2026-11-23, system Chromium ran the player and staff scenarios against `https://staging.oteryn.com` with two workers and zero Playwright retries. Each scenario checked readiness first. The player entry points redirected signed-out visitors to localized sign-in. Both versions of the public password-reset form enforced the required email field.

From `portal-rs/e2e`:

```sh
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts
npx playwright test -c staging/sim/support-flow/playwright.config.mjs
```

## Staging results

| Scenario | pl-1440 | pl-390 | en-1440 | en-390 |
| --- | --- | --- | --- | --- |
| Player tickets and reports | Fail, signup 503 | Fail, signup 503 | Fail, signup 503 | Fail, signup 503 |
| Staff support and moderation | Fail, reset 503 | Fail, reset 503 | Fail, reset 503 | Fail, reset 503 |

The complete run reproduced seven failures. The English phone staff scenario initially timed out waiting for a reset message. Its focused rerun reproduced the reset 503 after accounting for the reset delivery window.

## Product defects

- **Player signup returns 503 in all four projects.** Open `/{lang}/register`, enter a fresh email and a valid matching password, accept the terms and privacy policy, and submit in the browser. Expected POST 303 to `/{lang}/register/check-inbox`. Actual POST 503 with the registration-unavailable message in the selected language. One English phone attempt also received 503 opening the form, and an earlier submission received a transient edge 502. The local evidence directory `/tmp/oteryn-support-flow-evidence/` has `<project>-registration-unavailable.png` for all four projects. [Expected form](../../../../crates/web/src/routes/register.rs#L58-L80) · [Expected redirect](../../../../crates/web/src/routes/register.rs#L148-L151) · [Localized 503](../../../../crates/identity/src/abuse/http.rs#L41-L57)
- **Completing a public password reset returns 503.** Request a reset for either the support or moderator staging account, open the newest delivered reset link, and submit a valid matching password with the account email. Expected POST 302 to the login page. Actual POST 503 before sign-in. Local screenshots at `/tmp/oteryn-support-flow-evidence/pl-1440-support-password-reset-503.png` and `/tmp/oteryn-support-flow-evidence/pl-1440-moderator-password-reset-503.png`. [Expected redirect and unavailable response](../../../../crates/identity/src/reset_http.rs#L330-L339)

## Staging and coverage limits

- Mailpit GET returned 404 at the initial check and each ten-minute check through 30 minutes. It recovered to 200 at the 40-minute check. The reset requests and mail retrieval then reached the password-confirmation form, where the reset defect blocked staff work.
- Player signup and staff reset failures blocked authenticated ticket, report, and sanction workflows.
- The ticket form has no attachment field and the attachment module exposes no HTTP route. The staff ticket view offers reply and status controls with no assignment action. The player can acknowledge a staff reply with its read control. [Ticket template](../../../../crates/support/src/tickets/page.html#L8-L14) · [Attachments module](../../../../crates/support/src/attachments.rs#L1-L7) · [Staff view](../../../../crates/admin/src/support_web.rs#L325-L327)
- The player appeal workflow is a domain interface with no mounted browser form. The staff panel shows a decision form and accepts decisions for requested or reviewing appeals. Player appeal submission and both terminal decisions need that owner flow. [Appeals domain](../../../../crates/support/src/appeals.rs#L164-L189) · [Staff transitions](../../../../crates/admin/src/support.rs#L135-L153)

## Test fixes during qualification

- Scoped Access forwarding to HTTPS requests for staging and configured Chromium to recognize the environment's proxy certificate for browser redirect hops.
- Corrected the sign-in heading to use the MFA template and handled browser cancellation during context shutdown.
- Matched reset messages in either seeded account language and accounted for the reset delivery window between projects.
- Retried ordinary submissions when the shared ingress rate budget intervened. The report limit assertion counts a fresh player's own submission attempts.
