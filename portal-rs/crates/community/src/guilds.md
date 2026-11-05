# Guild projections (T24E)

G-4 v2 guild publication uses `/internal/v2/game-public/guild` and the v2
watermark endpoint. This module currently accepts only `family: guild` on that
endpoint; unsupported v2 families refuse with 400. The closed v1 enum is unchanged.
It shares T24's storage and global epoch fence and uses `ingest::latest` for
subject reads. No migration or SQL query macros are needed.

`PORTAL_GUILDS_INGEST_ENABLED=true` enables ingest only in testing/preproduction,
with the existing `PORTAL_PUBLIC_PROJECTIONS_AUTHORITY` and
`PORTAL_PUBLIC_PROJECTIONS_SAN` configuration. All requests require the trusted
mTLS connection extension and the public-projection identity group. The transport
adapter must enforce identity separation across other internal purposes.

`PORTAL_GUILDS_ENABLED=true` enables presentation only in testing/preproduction.
Both flags default off. Missing or stale same-epoch v2 guild completeness evidence
suppresses presentation; no stale-serving interval is allowed. Suppression replaces
the current roster. Internal subject/world IDs never appear in rendered markup.
The list uses name order, literal search (80 characters), and pages of 50. Roster
pages preserve `(rank_level, character_id)` ordering and contain visible members
only. Counts describe that visible cohort. No private guild fields or inferred
progression aggregates are exposed.

`PublicVisibility` is the integration boundary for current eligible profiles and
Platform profile/guild preferences. The module independently checks eligible same-world
profile/guild membership using the public T24 read API, and also requires a fresh
same-epoch character-profile watermark. It receives stable IDs, world, epoch and a
name consistency check; implementations must never join privacy by name. Missing
profile/ownership/privacy evidence denies. The base branch exposes no public
character-privacy API, so the configured router deliberately uses `FailClosed`.
An integrator must supply that adapter through `router` before public activation.
This task does not infer permission from absent preferences or fabricate profiles.

Reference: Game commit 857f15f376907c592bb28d854fccff18a15cdf87,
`crates/gameplay/durability/src/durability/public_projections.rs` (profiles have
no native guild yet) and Platform's G-4 proposal guild vector. No Game source was
copied. The task's accepted G-4 notes govern v2 implementation.

Validation: guild tests cover authenticated ingestion, disabled/wrong-identity
refusal, malformed wire/oversize input, replay equality and conflict, hidden-member
filtering and counts, named detail lookup, missing names, suppression, stale
watermarks, and cross-family epoch fencing on real PostgreSQL. Set
`PORTAL_TEST_DATABASE_URL`; otherwise the database test prints a skip notice.
