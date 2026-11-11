# Web sessions

T08 is enabled only with `OTERYN_ACCOUNT_SESSIONS=true`. T04 login still requires
its own `OTERYN_ACCOUNT_LOGIN=true` flag. Set `OTERYN_PUBLIC_ORIGIN` to the public
origin for browser CSRF checks. Neither flag is enabled by default.

Routes exist without a language prefix and under `/pl` and `/en`:

- `GET /account/sessions`: at most 50 active sessions and 50 recent security events.
- `POST /account/sessions/{uuid}` (also DELETE): revoke an owned session.
- `POST /account/sessions/others` (also DELETE on the collection): preserve current.
- `POST /account/sessions/all`: revoke every web session, including current.

Mutations require T02 CSRF verification, a fresh active verified account/session,
and the existing account/source revocation rate limit. Successful mutations use
302 redirects; current/all revocation clears the cookie and redirects to login.
An unavailable or foreign target uses the same generic flash message and redirect.
Malformed UUIDs return 400; unauthenticated page access redirects to login.

The source of the owner-scoping, redirect and preserve-current behavior is
`legacy/app/Http/Controllers/Identity/AccountWebSessionController.php` and
`legacy/app/Identity/Sessions/IdentityWebSessionRegistry.php`. T08 uses the plan's new
session page URL. No game protocol or remembered client device contract changes.

The binary composition middleware observes T04's successful login response before
releasing its rotated cookie. It records successful logins and revocation events;
credential failures are not added to account history. Session cleanup cannot delete
this history. Device recognition compares the bounded browser user-agent summary,
not IP addresses. It is only an alert heuristic, never authentication or trusted-device
proof. A missing summary is always new. Spoofed or identical user agents can defeat
this heuristic; a changed browser version can produce another alert.

An account lock serializes observation and revocation. A unique login event and
account/device key deduplicate retries and concurrent first observations. The history,
known device and T36 security email/outbox job commit together. A failed enqueue
refuses the login response and removes its session. A crash before observation does
not release a successful cookie. Security messages bypass optional preferences.

The worker registers T36 delivery with the active verified account adapter when the
flag is enabled. It then requires `OTERYN_SMTP_HOST`, `OTERYN_SMTP_USERNAME`,
`OTERYN_SMTP_PASSWORD`, and `OTERYN_SMTP_FROM`; STARTTLS validates certificates.
Deploy the worker configuration alongside the flag before enabling logins. No SMTP
service is contacted by tests: they use T36's capture sender. The private security-only
notification factory generates an ephemeral unused unsubscribe signer because T36
requires one; it never issues optional-category links. Optional mail needs a separate
stable signer owned by its caller.

Database tests use `PORTAL_TEST_DATABASE_URL`, isolated schemas and PostgreSQL 18.6.
They print a skip notice when that variable is unset. No query macros are introduced.
