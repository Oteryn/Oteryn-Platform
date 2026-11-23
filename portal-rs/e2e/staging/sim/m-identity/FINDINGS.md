# Identity and notification staging findings

The run used real staging browser and HTTP requests, two workers, Chromium, and the four Polish and English desktop and phone projects. Browser and API requests supplied Cloudflare Access credentials only to the staging origin. The tests use no database, seed data, or local mail transport.

The final run passed 36 of 84 checks. Four public registration checks and 40 player scenarios failed at fresh account provisioning (I-01). Four staff scenarios failed at password reset submission (I-02). Each project had nine passes and 12 failures.

## Product defects

### I-01 Registration returns a service unavailable screen

For `POST /en/register`, open the English sign-in page, follow its registration link, enter a fresh email address and matching valid passwords, accept both current legal documents, and submit. Registration should redirect to the inbox page with HTTP 303 and display the localized next-step notice. The English phone rerun after readiness recovered returned HTTP 503 and displayed “Registration is currently unavailable. Please try again later.” The public response gives no backend cause. Local screenshot `/tmp/oteryn-m-identity-evidence/registration-503-en-390.png`. [Expected redirect](../../../../crates/web/src/routes/register.rs#L151) · [Unavailable response](../../../../crates/identity/src/abuse/http.rs#L41)

The same result reproduced in every project with a successful challenge response. After Mailpit became available, all fresh-player setup attempts also received HTTP 503. Owner and second-player scenarios depend on that setup.

### I-02 A delivered password reset link fails at submission

For `POST /en/reset-password`, request a reset for a designated staging staff account, open its delivered reset form, and submit a new matching password. The handler should return HTTP 302 to the localized sign-in page. All four exercised staff roles reached the reset form and received HTTP 503 on submission, with “Temporarily unavailable” in the browser. Local screenshots are `/tmp/oteryn-m-identity-evidence/password-reset-503-{pl-1440,pl-390,en-1440,en-390}.png`. [Reset submission handler](../../../../crates/identity/src/reset_http.rs#L330)

## Test issues resolved

- The environment prevented npm from writing to its default cache. Dependency installation succeeded using a cache under `/tmp`.
- The first browser navigation hit the execution proxy's certificate interstitial. The configuration accounts for the proxy when present.
- Context teardown could close a routed request just before `APIResponse.dispose()`. The route handler now tolerates disposal during teardown, so a late asset cannot change a completed test result.
- Chromium could discard the challenge response body during navigation. The browser routing helper now copies the difficulty value from the real response before forwarding it, so the test can assert the value after navigation.
- The password form redirects anonymous users through `/login`, while the localized account pages use a localized sign-in redirect. Assertions were aligned with their respective route contracts.
- Disabled OAuth exits at the web router with an empty HTTP 404 response before the protocol error renderer. The test checks that exact unavailable response. [OAuth wrapper](../../../../crates/web/src/lib.rs#L204)

## Staging limitations

- `/readyz` initially returned HTTP 200 while `/__mailpit/` and `/__mailpit/api/v1/messages` returned HTTP 404. Scheduled checks at 11:46 and 11:56 UTC still returned 404. The API returned HTTP 200 at 12:16 UTC after the gateway outage. Player and staff checks then reached the provisioning failures above.
- The initial platform administrator and game viewer reset requests displayed the generic acknowledgement and had no matching delivery in the 60-second Mailpit window. Their scheduled 12:32 UTC rerun received both messages and exposed the same reset failure as the other two staff roles.
- A valid optional notification delivery is required to observe a successful signed unsubscribe. The public landing page and refusal of missing or oversized input were exercised.
- OAuth responded with the disabled HTTP 404 state on all three method and path combinations. The private identity and closure routes are mounted on a separate mTLS listener. Public listener responses were checked.
- Unprovoked shared-source HTTP 429 responses were retried using `Retry-After`. A test that finishes those waits with a shared registration throttle is recorded as blocked.
- Cloudflare returned HTTP 502 during registration at 12:06 UTC. Mailpit and `/readyz` also returned 502 in the same window. Local screenshot `/tmp/oteryn-m-identity-evidence/registration-502-en-390.png`.
- Registration takes an email address and password. Fresh player addresses supply the unique identifiers. [Registration form](../../../../templates/account/register.html#L5)

The route and actor table lists observed checks and remaining gaps. [COVERAGE.md](COVERAGE.md)
