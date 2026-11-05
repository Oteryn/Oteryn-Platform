# Support tickets (T33)

The feature is absent unless `OTERYN_SUPPORT_TICKETS=true`. This flag also gates
retention cleanup. Enabled startup requires `OTERYN_NOTIFY_UNSUBSCRIBE_KEY`
(at least 32 bytes) and `OTERYN_NOTIFY_UNSUBSCRIBE_URL` (HTTPS), passed to the
notification crate's public configuration API. Never commit these values.
Set `OTERYN_PUBLIC_ORIGIN` to the browser origin for CSRF validation.
The configured per-account open limit is 10.

Player routes live under `/{lang}/support/tickets` (`pl` and `en`): list, create,
show, POST reply, and PUT status. Status forms also accept POST. Staff routes
live under `/{lang}/admin/support/tickets` and use the shared admin guard with
`ManageSupportTickets`, admin host checks, session CSRF, MFA and RBAC. The
shared configured staff adapter currently denies authentication. Integrators
can use `staff_router` with trusted implementations of `StaffSession` and
`AccountStateReader`; no request data can choose the actor or staff identity.

Creation request keys are UUIDs. Replaying a key returns the original ticket,
matching the existing application. Ticket and message IDs are UUIDv7 as required
by the Rust plan. Subjects allow 160 Unicode characters and messages allow 8000.

Creation and reopening lock the owner's account before checking the open count.
Every ticket mutation then locks the ticket and checks its version. Reopening
also consumes capacity, closing a limit bypass in the older implementation.
Only `resolved` and `closed` stop counting toward the open limit. Unknown account,
locale or ticket state denies. Player access is restricted to the owner, and
internal messages never appear in player results or notifications.

Public staff replies and staff status changes enqueue localized notifications
through `oteryn-notify`, in the same transaction as the mutation and audit.
Notification refusal rolls back the entire mutation. Internal notes never
notify. Notification delivery and preference handling belong to T36.

Terminal tickets and their messages are deleted 180 days after closure, in
bounded worker batches. Attachments and reports remain separate tasks.

Run `cargo test -p oteryn-support` with `PORTAL_TEST_DATABASE_URL` pointing to a
local PostgreSQL 18.6 test database. Each database test creates and drops its own
schema. Without the variable, database tests print a skip notice.
