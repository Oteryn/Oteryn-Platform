# House helpers: command boundary (T92B)

`commands::Commands` issues only `StartHirelingExpedition` and
`CollectHirelingExpedition` under the accepted-with-notes G-14 command boundary
in `docs/contracts/OTERYN_V2_HOME_HIRELING_COMMAND_CONTRACT_PROPOSAL.md` (§§6–8,
10). Contract revision 2 is integrated through #1737. The Game remains authoritative.
This module does not implement expeditions, professions, chests or rewards.

Every new command needs all applicable `CommandFlags`, which default false,
exact source configuration, authenticated AccountId supplied server-side and a
trusted `CommandAuthority`. The sole shipped adapter, `DenyCommands`, refuses.
There is no command route, runtime flag mapping, worker registration or Game client.
A future browser adapter must authenticate the session, enforce CSRF and supply
AccountId independently of the request. Page rendering never contacts Game.

Preflight requires independent current ownership/privacy and active account/security,
a complete fresh baseline across causal dependencies, recovery fences, a placed
helper in its character's owned house, character revision and offline evidence.
Start additionally requires an exact approved non-currency table/row quote, weekly
budget and fewer than three unfinished expeditions across the account. These are
advisory checks; Game must repeat them atomically with session/admission authority
and its eventual effect. Unknown, stale or ambiguous evidence refuses. Existing ten
identities per character are not limited by this boundary. Collect requires an
explicit Game-published READY_TO_COLLECT expedition/revision, retained approved
non-currency reward binding and known chest capacity, never a countdown.
No client reward amounts, currency, price, duration, clock or chest selector are
accepted. No reward/XP/counter is calculated or changed locally.

The additive migration `20261105231304_t92b_command_boundary.sql` creates a separate
immutable intent ledger; existing T92 rows are not reinterpreted as Game evidence.
Account-scoped UUIDv7 browser keys bind complete typed request bytes. Same-key
replay returns the original operation, revision, exact intent bytes and SHA-256
binding without another outbox job, including after mutable preconditions change.
Current owner/security/freshness checks still apply; changed actions conflict.
The browser key is never sent to Game. All wire identifiers, authorization decision
IDs, sequence and 300-second technical expiry are supplied server-side. Exact field
order and string integer representations match the contract golden vectors.

One issuer-wide allocation lock is held through commit, serializing idempotency,
rate/backlog limits and revision allocation. Intent and existing shared outbox job
are committed together. Limits are ten new commands per account/minute and forty
unresolved commands per source. An unresolved helper cannot receive another
operation as a blind retry. SQL errors roll back allocation, intent and job together.
Only an operation reference is enqueued as `hirelings.command.v1`; canonical bytes
remain in the immutable ledger. Nothing consumes this kind in the application.
A queued result always means pending, including after expiry, response loss or
worker failure. No local timeout can fabricate a terminal reject or replacement.

Purchase always returns typed `PurchaseBlocked`, even with old flags/entitlement
adapters enabled, pending the Store `unit_kind` amendment (Q5). The older T92
start/claim/return mutation APIs now return `LegacyBoundaryUnavailable`; their
local identity/reward model and boolean receipt seam cannot bypass this boundary.
The existing optional read-only PL/EN page remains available behind its old
`PORTAL_HIRELINGS_ENABLED` flag, default off. This task adds no browser mutation.

Still blocked before activation: Game mechanics and approved policy/content (Q1/Q2),
remaining lifecycle decisions (Q4), Store mapping (Q5), authenticated projection and
receipt transport, restore-barrier registration and reconciliation. Receipt-driven
resolution/notifications and issuance-off draining must be integrated before any
consumer can be registered. Pending intents must be retained, not pruned, replaced
or treated as failed. No production enablement is provided by this task.

Validation uses PostgreSQL 18.6 with isolated schemas. Five DB tests (four command
boundary tests plus the existing read-only page regression) run when
`PORTAL_TEST_DATABASE_URL` is set; otherwise each prints a skip notice. They cover
concurrent replay, conflict, exact binding persistence, immutable intents, authority
and rate limits, atomic outbox failure, owner/stale/ambiguous refusals, disabled
flags, independent key scopes and page authentication/localization. Pure tests
cover exact contract bytes/digests, strict bad input and preflight refusals. No
SQL query macros were added, so no new offline query metadata is required.
