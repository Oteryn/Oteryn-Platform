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

Restore notice/reopening integration belongs to T23. This core does not infer
restoration from a receipt or update LCFA projections from command results. Player
forms, notifications, Bazaar settlement and feature activation are separate tasks.

Fixtures are small canonical vectors from the pinned Game contract and consumer;
each has its own SOURCE note. Database tests use isolated schemas and
`PORTAL_TEST_DATABASE_URL`, printing a skip notice when unset. No SQL query macros
are used, so no generated offline query data is required.
