# HTTP security headers

This lane qualifies the public portal for [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002) against the local HTTPS browser launcher. Use the disposable PostgreSQL and test prerequisites in the [E2E README](../README.md) and [environment guide](../ENVIRONMENT.md). From `portal-rs/e2e`, run:

```sh
bash sec-headers/launch.sh with-mfa
# In a second shell, use the private environment file printed by the launcher.
node run.mjs /absolute/scratch/env.json test -c sec-headers/playwright.config.mjs
```

Stop that launcher before performing the complementary startup with ordinary login. Both compositions use the real portal binary and the shared launcher and fixtures. For the MFA-disabled run, export `PORTAL_E2E_BINARY` with the absolute path to the binary built by `cargo build --locked -p oteryn-portal`.

```sh
bash sec-headers/launch.sh without-mfa
# In a second shell, use the new environment file from this startup.
node run.mjs /absolute/second/scratch/env.json test -c sec-headers/playwright.config.mjs
```

The dedicated config runs Polish and English at both widths. It selects the ordinary login spec in the MFA-disabled composition. The default config can still run against an already-running portal. The dedicated config disables traces, screenshots, and video because it exercises browser credentials and session material.

## Coverage and policy

- The lane launcher requests a fresh registration report from the testing binary. The suite derives its GET matrix from the composed Axum router and legacy redirect table. It checks Axum's matched pattern on each hop, including login rewritten to the MFA handler. The MFA-disabled startup exercises the ordinary unprefixed and localized GETs and signs in through the ordinary POST. Legacy-only redirects require the exact 301 and destination and the terminal response's exact status and policy. [Router report](../../bin/oteryn-portal/src/main.rs) · [Route matrix](routes.spec.mjs) · [Ordinary login](plain-login.spec.mjs)
- The staff test creates a draft with the authorized form, verifies the existing and missing edit cases, and requires 403 for the same existing draft from an MFA-verified player. The ownership fixture uses the actual mTLS portfolio publisher. The owner opens the character profile editor and another signed-in player gets 404 for that same profile. [Route matrix](routes.spec.mjs) · [Ownership fixture](ownership.mjs)
- Each response contract selects one exact CSP and referrer policy. The parser rejects duplicate CSP fields and directives, weakened or missing source restrictions, and invalid nonce formats. The Atlas viewer test verifies that Chromium runs its nonce-bound script and loads a tile. Chromium also verifies League's stylesheet and the cross-origin framing refusal. [Policy assertions](policy.mjs) · [Route-specific policies](response-contracts.mjs) · [Browser scenarios](browser.spec.mjs)
- Dynamic pages and redirects require `no-store`. The tests assert the exact public cache policy of the favicon and a checked-in stylesheet. The Hunt module's CSS uses its explicit private policy. [Static policy](../../crates/web/src/surface.rs#L310) · [Favicon](../../crates/web/src/favicon.rs#L8) · [Hunts CSS](../../crates/playerdata/src/hunts_web.rs#L58)
- The wire and browser cookie checks cover Secure, HttpOnly, SameSite=Lax, root path and host-only scope. Login rotates the session and its server-side CSRF synchronizer token. Logout expires the session cookie and removes its database record. MFA rotates the pending session on completion and rotates the remembered-device cookie when that device is used again. The opt-in cookie retains its documented 30-day trust across logout. [Session cookie and logout](../../crates/web/src/session.rs#L300) · [CSRF token](../../crates/web/src/csrf.rs#L1) · [Remembered-device cookie](../../crates/identity/src/mfa_http.rs#L422) · [Trust choice](../../templates/account/mfa/en.ftl#L7)
- Bad-input probes send an oversized field, an oversized header section, an oversized cookie, a malformed Host and an unsupported Accept through HTTPS. They require exact 431, 400 or 406 status and a localized HTML refusal with the full policy. The size probes cross the application limits while fitting inside the test proxy's HTTP parser limits. [Request admission](../../crates/web/src/public_http.rs#L14) · [Local HTTPS proxy](../local.mjs#L138)

## Defects reproduced on the base

| Finding | Source |
| --- | --- |
| The shared header and cache middleware was attached before the binary merged browser modules. Bazaar pages lacked CSP and framing denial, some account and public responses lacked `no-store`, and several staff and refusal responses received only a partial policy. | [Initial router](../../bin/oteryn-portal/src/main.rs#L133) · [Bazaar merge](../../bin/oteryn-portal/src/main.rs#L225) · [Original policy attachment](../../crates/web/src/lib.rs#L69) |
| The shell's early anonymous redirect for protected pages returned before the common response policy. The outer minigame deadline could also return 503 after the policy layer had been bypassed. | [Anonymous redirect](../../crates/web/src/chrome.rs#L29) · [Deadline placement](../../bin/oteryn-portal/src/main.rs#L401) |
| Oversized header fields and cookies and a malformed Host received the normal root redirect. An unknown media type received the normal HTML page. The base web policy inspected Accept for JSON error negotiation and did not refuse unsupported HTML media types. | [Base Accept handling](../../crates/web/src/surface.rs#L210) · [Composed request admission](../../crates/web/src/public_http.rs#L67) |
| The favicon's explicit sandbox policy lacked framing denial. Its one-day public cache lifetime is intentional. | [Favicon response](../../crates/web/src/favicon.rs#L8) |

## Review regressions

- `GET /forgot-password` asserts 301 and the exact `Location: /pl/forgot-password`, then verifies the HTML destination. [Legacy registration](../../crates/identity/src/reset_http.rs#L89) · [Redirect contracts](redirects.mjs)
- The reset browser spec submits the real form, requests an unsupported representation in the same session, and verifies that the localized confirmation survives the 406 and appears exactly once. A route-level guard negotiates before the HTML handler can consume it. [HTML negotiation](../../crates/web/src/public_http.rs#L27) · [Reset browser regression](browser.spec.mjs)
- The OAuth browser spec establishes a pending authorization request, receives 406 for a second request with an unsupported Accept, and verifies that sign-in returns to the original request. The consent page also asserts its exact CSP callback origin. [OAuth session guard](../../crates/web/src/lib.rs#L122) · [OAuth browser regression](browser.spec.mjs#L253)

- The mailed password-reset link has the handler's `no-referrer` policy. Unsupported media requests leave the character creation validation message and submitted form intact. Unsupported referrals methods return 405 and the exact Allow header for both JSON and HTML Accept. [Surface policy](../../crates/web/src/surface.rs) · [Referrals probe](routes.spec.mjs) · [Browser regressions](browser.spec.mjs)
