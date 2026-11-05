# Game restore reconciliation

The domain entry point is `Restore::notice`; configuration defaults off. Network
callers use `restore::router(restore, command_settings, identities)` on the private
verified TLS listener. This composite **replaces** the bare `commands::router`:
mounting the bare router separately would bypass receipt serialization. The
command implementation remains unchanged and is called through its public router.
Use a pool with at least two connections. Receipt concurrency reserves half the
pool for the command API's own transaction.

`exchange_available` is a deployment qualification, including the environment
restriction on the command exchange. The router also checks that its command
configuration and dedicated identities can serve requests. Recovery may enable
list/read/receipt while issuance stays disabled; it never modifies an issuing
`Commands` instance or enables create/transfer flags. Supply the complete shared
`IdentityMap` for all internal purposes so overlapping identities fail closed.
The listener and deployment flags remain integration responsibilities; there is
no public route or automatic enablement.

A notice holds the singleton row in `character_restore_locks` exclusively.
Receipt middleware holds it shared before calling the command router and until
its response, including commit. The receipt trigger retains immutable JSON history,
checks the latest generation, and answers all reopen rows in the same transaction
as T22's terminal transition. No timestamp selects a receipt. Replays report the
current reconciliation status; they never reopen additional operations.

`Restore::queue` exposes up to 1000 unresolved alarms to a trusted staff service.
It is not a player or HTTP API. Conflicting/stale notices, uncertain history,
unavailable exchange, lost receipts and reversed outcomes are durable queue items.
A reversal does not infer compensation, settle money, or alter account authority.
Account identifiers are UUIDs without account foreign keys. T22 remains responsible
for account authorization before issuing commands; restore never issues commands.

This is character reconciliation only. Store delivery must remain disabled on a
stack using this surface until the Store restore amendment and its reconciliation
are implemented, as required by the Store wire contract section 7.

Tests read `PORTAL_TEST_DATABASE_URL`, print a skip notice when it is absent, and
otherwise isolate each case in a temporary PostgreSQL schema. Golden vectors are
small exact extracts from the pinned Game producer; each has a `.SOURCE` note.
