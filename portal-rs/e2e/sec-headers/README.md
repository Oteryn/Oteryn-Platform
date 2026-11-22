# HTTP security headers

This suite qualifies the public portal for [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002) against the local HTTPS browser launcher. Start the disposable PostgreSQL and portal services as described in the [E2E README](../README.md) and [environment guide](../ENVIRONMENT.md). The launcher also starts the shared harness and creates test-only mTLS identities. From `portal-rs/e2e`, use the private scratch environment file printed by the launcher.

```sh
node run.mjs /absolute/scratch/env.json test -c sec-headers/playwright.config.mjs
```

The command runs `pl-1440`, `pl-390`, `en-1440` and `en-390` with Chromium. Traces, screenshots, and video are disabled because the scenarios exercise browser credentials and session material.

## Coverage and policy

- The GET matrix reconciles `inventory.json` with `route-expectations.json`, uses the exact terminal status in each row, and checks every intermediate redirect before following it. It uses the seed CLI for the player and MFA-enrolled staff accounts. Known and missing path references match the existing route E2E suite. The profile probes publish a fresh watermark through the real local mTLS listener so the expected 404 proves a missing subject under healthy ownership authority. Cold Game-dependent routes keep the specified 503. Additional concrete routes missing from the checked-in inventory are listed with their exact expected outcomes. [Route matrix](routes.spec.mjs) · [Additional GET routes](additional-routes.mjs)
- The response assertions use the portal's two-year HSTS policy, nosniff, referrer policies and five denied browser capabilities. CSP must deny framing with `frame-ancestors 'none'` or `X-Frame-Options: DENY` and disallow `'unsafe-inline'` in effective script directives. Chromium also opens Bazaar directly, then verifies its navigation is blocked in an iframe served from a separate loopback origin. [Common policy](../../crates/web/src/surface.rs#L282) · [Composed policy](../../crates/web/src/public_http.rs#L58) · [Browser scenarios](browser.spec.mjs)
- Dynamic pages and redirects require `no-store`. The tests assert the exact public cache policy of the favicon and a checked-in stylesheet. The Hunt module's CSS uses its explicit private policy. [Static policy](../../crates/web/src/surface.rs#L316) · [Favicon](../../crates/web/src/favicon.rs#L8) · [Hunts CSS](../../crates/playerdata/src/hunts_web.rs#L58)
- The wire and browser cookie checks cover Secure, HttpOnly, SameSite=Lax, root path and host-only scope. Login rotates the session and its server-side CSRF synchronizer token. Logout expires the session cookie and removes its database record. MFA rotates the pending session on completion and rotates the remembered-device cookie when that device is used again. The opt-in cookie retains its documented 30-day trust across logout. [Session cookie and logout](../../crates/web/src/session.rs#L300) · [CSRF token](../../crates/web/src/csrf.rs#L1) · [Remembered-device cookie](../../crates/identity/src/mfa_http.rs#L422) · [Trust choice](../../templates/account/mfa/en.ftl#L7)
- Bad-input probes send an oversized field, an oversized header section, an oversized cookie, a malformed Host and an unsupported Accept through HTTPS. They require exact 431, 400 or 406 status and a localized HTML refusal with the full policy. The size probes cross the application limits while fitting inside the test proxy's HTTP parser limits. [Request admission](../../crates/web/src/public_http.rs#L14) · [Local HTTPS proxy](../local.mjs#L138)

## Defects reproduced on the base

| Finding | Source |
| --- | --- |
| The shared header and cache middleware was attached before the binary merged browser modules. Bazaar pages lacked CSP and framing denial, some account and public responses lacked `no-store`, and several staff and refusal responses received only a partial policy. | [Initial router](../../bin/oteryn-portal/src/main.rs#L133) · [Bazaar merge](../../bin/oteryn-portal/src/main.rs#L225) · [Original policy attachment](../../crates/web/src/lib.rs#L69) |
| The shell's early anonymous redirect for protected pages returned before the common response policy. The outer minigame deadline could also return 503 after the policy layer had been bypassed. | [Anonymous redirect](../../crates/web/src/chrome.rs#L29) · [Deadline placement](../../bin/oteryn-portal/src/main.rs#L397) |
| Oversized header fields and cookies and a malformed Host received the normal root redirect. An unknown media type received the normal HTML page. The base web policy inspected Accept for JSON error negotiation and did not refuse unsupported HTML media types. | [Base Accept handling](../../crates/web/src/surface.rs#L210) · [Composed request admission](../../crates/web/src/public_http.rs#L31) |
| The favicon's explicit sandbox policy lacked framing denial. Its one-day public cache lifetime is intentional. | [Favicon response](../../crates/web/src/favicon.rs#L8) |
