# Hunting-place daily layer (T81)

Governing proposal: `docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md`, especially sections 6, 8 and 9. The task's revision-2 clarification supersedes the old ownership field and session-boundary wording in that proposal. Laravel's private session analysis controller supplies the owner-denial and private/no-store precedent; Laravel has no equivalent automatic hunting-place aggregate route.

The base has no T73 branch or playerdata crate. The T41 dependency is merged. This module supplies only T81's normalized contribution storage, privacy preferences, fixed daily cohorts and Atlas presentation. It does not implement T72's raw journal transport, replay commands, G-10 or G-11 families. No Game command is emitted.

## Activation boundary

`OTERYN_PLAYERDATA_PLACES_AGGREGATE` defaults off. The binary registers an unresolved source and keeps normalized ingest off. Even setting the environment flag cannot publish through that source: `daily` returns unavailable until a qualified `JournalSource` is injected. No new Game-facing HTTP endpoint is exposed. The proposed G-9 ingest remains unavailable until T72 supplies its purpose-bound internal listener and contract-qualified adapter.

The adapter is trusted server code, not request input. It must use the supplied transaction connection to verify and lock authoritative T73 session evidence and current privacy/ownership/generation fences. See the trait's complete requirements. In particular it must prove a durable logout, contiguous applied account revisions, every segment and sealed drop, catalogue membership, homogeneous solo attribution, common NPC valuation and the authoritative transfer-receipt cut. A bare final segment, cached profile or an accepted ingest response proves none of these. Unknown state refuses publication. The adapter must not claim `ready` before producer and cohort privacy qualification.

## Conservative choices awaiting acceptance

- Participation requires explicit opt-in. Preference changes erase prior contributions and retain payload-free replay fences; only sessions starting after the latest opt-in can enter. Re-enabling never restores historical contributions.
- One complete solo session enters one cell. Mixed places, party data, mixed level bands/vocations/catalogues/prices and unproven valuation are excluded by the adapter. Session IDs are distinct from game admission sessions and leases.
- Cells are fixed UTC logout day, public world, catalogue, common NPC price revision, place, vocation and 50-level band (1–50, 51–100, ...). There are no arbitrary filters, marginal totals or overlapping windows. Each cell independently needs 10 distinct sessions and 5 accounts.
- A day is withheld until its end plus 24 hours. Rates are arithmetic means of per-session integer rates, rounded toward zero; negative profit is retained. No price or exchange-rate guesses are made.
- Contribution evidence expires after 90 days, the stricter proposal retention rather than the plan's older 365-day figure. Replay fences expire after 120 days; old captures still fail the 90-day check. Exclusion removes contributions synchronously. An hourly worker cleanup purges expired evidence even while flags are off and fences closed accounts. Reads derive cohorts again and revalidate source evidence; no aggregate cache can retain an obsolete allow.
- Cohort definitions and before/after differencing protection still need the separate privacy acceptance required by G-9. Publication remains off pending that acceptance and invalidation qualification.
- Atlas includes an accessible collapsible daily layer listing and links to its owner page by catalogue place key. No position is invented: a geographical overlay needs an accepted place-to-map mapping from the world catalogue. The layer does not disclose individual tracks, coordinates, identities or party data.

## HTTP and validation

Owner-only `/{pl|en}/atlas/places`; participation is read and updated at the `/participation` suffix. Browser identity comes exclusively from `BrowserSession`; mutations use the existing synchronizer-token/origin middleware, with the exact `OTERYN_PUBLIC_ORIGIN` configured as the trusted origin. Missing origin configuration fails closed for browser POSTs. Cross-account targets and arbitrary filters are unsupported. Disabled routes return 503; anonymous/unauthorized reads return 403; invalid input returns 400 (framework form-shape rejection may return 422); all responses carry private/no-store.

The four `postgres_*` tests require `PORTAL_TEST_DATABASE_URL`; without it they print a skip notice. They use separate generated schemas, migrate the actual PostgreSQL schema, and cover thresholds, delay, immutable duplicates, source refusal, ownership, exclusion/replay, flags, CSRF and localized browser forms. One additional unit test covers the exact UTC delay boundary and repeated sessions. No SQL query macros are added, so no offline query metadata is required.
