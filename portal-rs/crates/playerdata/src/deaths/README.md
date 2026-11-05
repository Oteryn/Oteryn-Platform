# T78 death and PvP projection

Governing contract: `docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md`, revision 2, sections 3–9, families death_recap, pvp and skull. G-9 and G-10 do not authorize additional death actions. No game commands are issued.

The T72 dependency branch was absent at implementation time. This module therefore supplies a narrow, default-denying `Authority` seam within T78 paths. It does not invent an ownership table or enable an HTTP listener. The enclosing application must supply its trusted Identity/session and mTLS context, purpose/partition binding, limiter, accepted source/epoch/recovery configuration, ownership locking, transfer receipt visibility floor and catalog resolver. The adapter must use the supplied transaction and retain authority locks until commit. Skull's projection revision is a table revision and cannot substitute for independent current ownership evidence.

`internal_router` is only for the trusted internal listener, and only handles the three T78 families. T72 must dispatch these families from its shared progress endpoint when other families are implemented; do not mount overlapping routes. `owner_router` needs a server-created `ReadContext` and rechecks both account and character scope on each read. Neither router is mounted automatically. With `Flags::default()` or `DenyAll`, ingest cannot succeed. Both switches and an explicit family allowlist are required.

The retention worker calls `purge_expired`; the account erasure workflow calls `erase_account`. Both are internal maintenance interfaces, never browser actions. Parsed history payloads expire after 90 days. Superseded skull payloads expire after seven days. Minimal digests and ordering metadata remain for replay prevention; T72 must close/delete streams under its accepted lifecycle, including the 90-day closed-stream fence window. No raw wire bytes or action logs are persisted. Database encryption and backup expiry remain deployment requirements before activation.

Fail-closed choices:

- V1 actions are always unavailable; populated actions, a nonzero window and invented damage/healing are rejected.
- G-11's 90-day history and transfer suppression override the older plan's unlimited retention and seller-history wording.
- Missing or stale skull evidence is unavailable; an elapsed deadline never clears a skull. Death/PvP history may be explicitly stale for at most 24 hours.
- Historical UPSERT replacement is refused because v1 has no explicit correction proof. REMOVE is supported; reinstatement needs an accepted correction mechanism.
- Gapped delivery returns 409 instead of staging; the producer must retry the missing contiguous sequence. A watermark cannot promote missing data or an incomplete baseline.
- A presentation result exceeding 200 entities is unavailable rather than silently truncated. Pagination requires an accepted T72 read interface.

The synthetic fixtures are derived from the G-11 schema, not copied from Game. Read-only reference checked at Game commit `857f15f376907c592bb28d854fccff18a15cdf87`: death receipt migration 0016 and durable PvP skull storage. There is no existing Laravel private death recap endpoint; its public deaths feed is not reused for private authorization or data.

Tests use `PORTAL_TEST_DATABASE_URL`, print a skip notice when absent, and create isolated generated PostgreSQL schemas. Five PostgreSQL tests cover storage, owner denial, transfer/recovery fencing, retention/removal, freshness and the HTTP owner-page path. Other tests cover closed payloads, defaults and Polish/English semantic rendering. SQL uses runtime bound queries, so no sqlx offline query artifacts are required.
