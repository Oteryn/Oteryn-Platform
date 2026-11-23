# Staging support simulation findings

The 2026-11-23 UTC run targeted `https://staging.oteryn.com` with Chromium, two workers, and Playwright retries disabled. [Route and actor matrix](COVERAGE.md)

## Product defects

- **REG-01. Browser registration returns 503 before the verification inbox.** Open `/{lang}/register`, provide a fresh test email and matching valid passwords, select both consent checkboxes, and submit the form. The handler should return HTTP 303 to `/{lang}/register/check-inbox` and show the localized inbox prompt. With `/readyz` returning 200, the form loaded, then submission returned HTTP 503 and displayed that registration was currently unavailable. The browser response did not identify the underlying dependency. [register.rs:142](../../../../crates/web/src/routes/register.rs#L142) · [verify.html](../../../../templates/account/verify.html)

Initial evidence is at `/tmp/oteryn-sim-m-support-evidence/first-full-pl-1440.png` and `/tmp/oteryn-sim-m-support-evidence/first-full-pl-390.png`.

## Staging limitations

- Both player support areas returned HTTP 404 from the route availability probes and mutations. Anonymous GETs still went to localized sign-in. Ticket/report success, ownership verification, and player-to-staff persistence were blocked.
- Every staff support path returned 404 on the supplied origin, including the enforcement create page. The shared admin host guard deliberately returns 404 on the wrong host or when the admin panel is disabled. This blocks authenticated staff workflows even if mail becomes available. [web.rs:294](../../../../crates/admin/src/web.rs#L294)
- The Mailpit messages endpoint returned 404 at initial discovery, during the suite, and again at 11:46 UTC. Its root path also returned 404. No staff password reset or MFA enrollment was attempted on these unavailable surfaces.
- Internal sanction export and Game enforcement returned 404 on the public listener. The public run did not claim internal listener execution or Game receipts. No player appeal or sanction-history HTTP CRUD exists in the source baseline. [COVERAGE.md](COVERAGE.md)

## Rerun results

The final run completed with eight passes and sixteen skips across `pl-1440`, `pl-390`, `en-1440`, and `en-390`. Each project passed the anonymous and public-origin boundary scenarios. Each project skipped verified ticket and report workflows because the routes and Mailpit were unavailable. The independent Mailpit availability check also skipped in all four projects. Registration was throttled through two retries in each project on that rerun. The initial full run reproduced REG-01 on both Polish projects.

## Test issues corrected during execution

- The runner's forwarding proxy required separate Chromium certificate handling. The initial browser connection error was a runner issue. The API and browser requests use per-request staging credentials and forward no Cloudflare credentials to external hosts.
- An initial anonymous assertion treated the localized sign-in redirect as proof the support route was mounted. The Chrome layer can issue that redirect before dispatch. The test now checks availability independently and asserts the appropriate refusal for that state. [chrome.rs:32](../../../../crates/web/src/chrome.rs#L32)
- The registration workflow now honors `Retry-After` for throttling at either the challenge or submit stage. A challenge throttle can produce a later verification refusal when portal JavaScript falls back to posting the form. Shared throttling is staging noise. [registration.js:13](../../../../crates/identity/src/abuse/registration.js#L13)
