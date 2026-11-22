# HTTP security headers

This suite qualifies the public portal for [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002) against the local HTTPS browser launcher. Start the disposable PostgreSQL and portal services as described in the [E2E README](../README.md) and [environment guide](../ENVIRONMENT.md). The launcher also starts the shared harness and creates test-only mTLS identities. From `portal-rs/e2e`, use the private scratch environment file printed by the launcher.

```sh
node run.mjs /absolute/scratch/env.json test -c sec-headers/playwright.config.mjs
```

The default Playwright config discovers this lane, including when the existing E2E job passes a project and shard. The command above runs this lane alone in `pl-1440`, `pl-390`, `en-1440`, and `en-390` with Chromium. Traces, screenshots, and video are disabled for this lane because it exercises browser credentials and session material.

## Coverage and policy

- The testing binary records the composed Axum public router, the routes behind the MFA fallback, and the legacy redirect table when launched through `local.mjs`. The suite derives its GET matrix from that report and fails when any registration lacks a scenario or the sample selects a different route. `route-expectations.json` provides response contracts and fixture inputs. Its existing refusal probes also run when they use unregistered paths. Legacy-only redirects assert 301 and their destination from the table. Shadowing registrations and method fallbacks are probed using their own response contracts. [Router registration report](../../bin/oteryn-portal/src/main.rs) · [Route matrix](routes.spec.mjs)
- Player and MFA-enrolled staff identities come from the seed CLI. Profile probes publish a live ownership watermark through the local mTLS listener to distinguish the expected missing-subject 404 from a cold-feed 503. The announcement test creates a draft through the authorized staff form, checks its exact redirect against the stored ID, and opens the editor to verify the draft alongside the missing-ID refusal. [Route matrix](routes.spec.mjs)
- Each response must carry a single enforced CSP that matches one of the complete directive and source sets defined by the common middleware or route-specific policy. Duplicate directives, missing script restrictions, and unsafe script sources fail. Chromium opens Bazaar directly before verifying it cannot render in a cross-origin iframe. The League test checks the same-origin stylesheet response and its computed layout under CSP. [Policy assertions](policy.mjs) · [Browser scenarios](browser.spec.mjs)
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
- The OAuth browser spec establishes a pending authorization request, receives 406 for a second request with an unsupported Accept, and verifies that sign-in returns to the original request. The consent page also asserts its exact CSP callback origin. [OAuth session guard](../../crates/web/src/lib.rs#L122) · [OAuth browser regression](browser.spec.mjs#L241)
