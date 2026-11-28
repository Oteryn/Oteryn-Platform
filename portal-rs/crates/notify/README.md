# Notifications core

T36 implements notification storage and typed Polish/English email templates. It uses
UUID account identifiers without an accounts foreign key. The account lifecycle and
player API routers provide the HTTP integration described below.

`Notifications::enqueue` and preference/unsubscribe mutations use the caller's
transaction. Optional email defaults to disabled; portal delivery defaults to enabled.
Security messages always use both channels and cannot be unsubscribed. The account
adapter must return only an unambiguous, active, verified account. Unknown, deleted,
inactive or ambiguous accounts must return `None`; `DenyAccounts` is the safe default.
Recipient state is checked again immediately before delivery.

Other domains construct `EmailTemplate` values, enqueue once within their transaction,
and retain the returned email ID. Rendered content is stored in `email_outbox`; the
shared outbox contains only the email ID. Shared leases, bounded retries and dead
letters belong to `oteryn-db`. The worker's `Registry::notifications` registers a
`Delivery` with an explicit account adapter and sender. Identity and SMTP deployment
configuration must supply those dependencies before enabling delivery. No SMTP
credentials or permissive placeholder account adapter are built into the worker.

`SmtpSender` requires STARTTLS and certificate validation. Messages use multipart
text/HTML, a stable Message-ID, and signed List-Unsubscribe / List-Unsubscribe-Post
headers for optional categories. SMTP delivery is at least once across a crash after
SMTP acceptance; stable Message-ID is not a guarantee of recipient deduplication.
Delivered rows suppress subsequent ordinary retries. Provider errors are reduced to
a generic worker error to avoid retaining credentials or recipient details in jobs.

The account lifecycle router always mounts `GET /{lang}/account/notifications`
and `POST /{lang}/account/notifications`, even without unsubscribe configuration.
GET displays the signed-in account's preferences. POST saves preferences and
requires a signed-in browser session and CSRF validation.

The router mounts unsubscribe routes only when `Unsubscribe::configured` accepts
`OTERYN_NOTIFY_UNSUBSCRIBE_KEY` and `OTERYN_NOTIFY_UNSUBSCRIBE_URL`. Use an
application-managed random key of at least 32 bytes and an HTTPS endpoint URL.
Keep the key stable for the 30-day link lifetime. `GET /{lang}/unsubscribe` shows
a confirmation page for a valid link. The browser form submits with CSRF protection
to `POST /{lang}/unsubscribe/confirm`. Mail clients use `POST /{lang}/unsubscribe`
for one-click unsubscribe. Both POST routes use the signed token to disable optional email
for its category.
Repeated valid unsubscribe requests are idempotent.

`Notifications::list` and `Notifications::list_for` are inbox list APIs for
trusted account IDs. The configured player API enables `GET /api/v1/feed` when
it has a registered OAuth client. It calls `Notifications::list_for` for the
active, verified account identified by a valid `account:read` bearer token and
returns each item's read state. `Notifications::mark_read` sets an item to read or
unread for the trusted account and awaits HTTP integration.

Verification: PostgreSQL tests create isolated schemas, migrate them and drop them.
They read `PORTAL_TEST_DATABASE_URL`, and print a skip notice if it is absent. They
cover rollback, default and explicit preferences, security mail, optional delivery,
read/unread ownership, inactive/unknown state, malformed payloads, token tampering and
expiry, and shared retry/dead transitions. No game or money features are introduced.
