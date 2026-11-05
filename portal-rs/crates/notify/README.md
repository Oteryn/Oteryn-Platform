# Notifications core

T36 implements notification storage and typed Polish/English email templates. It uses
UUID account identifiers without an accounts foreign key. No HTTP routes or pages
are supplied by this core task.

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

`Unsubscribe` requires an application-managed random key of at least 32 bytes and an
HTTPS endpoint URL. Keep that key stable for the 30-day link lifetime. The HTTP adapter
must pass decoded tokens to `Notifications::unsubscribe` only on a one-click POST;
GET must never mutate preferences. Repeated valid unsubscribe requests are idempotent.
The future authenticated HTTP adapter must establish account ownership before calling
list, read/unread or preference APIs; these Rust APIs accept trusted account IDs.

Verification: PostgreSQL tests create isolated schemas, migrate them and drop them.
They read `PORTAL_TEST_DATABASE_URL`, and print a skip notice if it is absent. They
cover rollback, default and explicit preferences, security mail, optional delivery,
read/unread ownership, inactive/unknown state, malformed payloads, token tampering and
expiry, and shared retry/dead transitions. No game or money features are introduced.
