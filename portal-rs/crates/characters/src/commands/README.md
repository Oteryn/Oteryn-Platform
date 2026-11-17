# Character command core

`Commands::enqueue` stores one immutable intent per submission key and authorization
decision. The caller must authorize the actor or Bazaar saga before calling it.
`AccountStateReader` checks both affected accounts; only `Eligible` permits issuance.
Unknown, ambiguous and denied accounts fail closed. Account UUIDs have no foreign key
until account storage exists. Game owns character ownership, lifecycle, names and
execution eligibility; this module never reads or writes game persistence.

All three switches in `Settings` default off. A valid source authority, TTL of
1..300 seconds, backlog of 1..40 and positive request budget are required. Issuance
serializes under a PostgreSQL transaction advisory lock before drawing the cursor
revision and holds it through commit. Expiry never removes an unreceipted intent.
An exact submission retry returns its original operation, including after receipt;
a changed binding, command or authorization decision conflicts.

`router` exposes the three compiled POST routes using the internal-api verified TLS
identity guard, with the dedicated `OTERYN_GAME_CHARACTER_COMMANDS` group, canonical
JSON and body limits. Mount it on the private TLS listener when that listener is
composed; do not mount it on the public router. HTTP headers cannot establish peer
identity. The group has a shared bounded request budget and all responses use
`Cache-Control: no-store, private`. Failures have empty bodies.

Receipts lock the command row, check the command and committed subject bindings,
and atomically make it terminal. Exact byte replay succeeds without a write;
changed bytes conflict. `status` is an internal saga API: callers must authorize
access before displaying its private intent or receipt.

Create is produced only as create intent v2 (#1937): the v1 members plus
`requested_sex` (`Female`, `Male`) and `requested_vocation` (`Knight`, `Paladin`,
`Sorcerer`, `Druid`, `Monk`), inserted right after `requested_name`. Values are
case-sensitive with no default; `Intent::Create` cannot be built without both. Both
are part of the binding, so a retry that changes either conflicts. Transfer intents
and the pending, read and receipt envelopes of this route stay v1, the create
receipt does not echo sex or vocation, and the v2 service commands are unchanged. A create intent v1 that is still stored as pending is
retired and fails closed: `pending` lists only create rows whose stored intent starts
with `contract_version` 2, and `read` answers 404 for a create whose stored intent is
not a valid v2. Such a row stays `PENDING` (it still counts toward the backlog and
its owner sees a pending status); a receipt for it is still accepted, because Game
may have read it before this change. Restore reopening a v1 create does not serve it
again either. No migration: sex and vocation live only in `binding_json` and
`intent_json`. Vectors and their digest test: `docs/contracts/fixtures/character-create-v2/`
and `create_v2_tests.rs`.

Restore notice/reopening integration belongs to T23. This core does not infer
restoration from a receipt or update LCFA projections from command results. Player
forms, notifications, Bazaar settlement and feature activation are separate tasks.

Fixtures are small canonical vectors from the pinned Game contract and consumer;
each has its own SOURCE note. Database tests use isolated schemas and
`PORTAL_TEST_DATABASE_URL`, printing a skip notice when unset. No SQL query macros
are used, so no generated offline query data is required.
