# V2 statistics consumer

Contract: `docs/contracts/OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md`,
revision 2 (PR #1737, merged as 738278a015c47c9f67140c7f5997192f4d2a1944).
The existing growth ranking implementation matches the prerequisite task branch
at c6203350bc018696e346ca759e0406ec24a58e3e and is already integrated on main.

## Scope and privacy

The G-8 contract explicitly defers **public online-time rankings**. Its only
public aggregates are world monster/boss kills, which contain no duration.
`rankings::online::public_online_ranking` therefore always returns unavailable.
No flag grants permission to publish intervals or derived character durations.
No templates or public duration route are added.

`rankings::online::owner_online_ranking` computes an ordering among a currently
proven owner's characters over an inclusive-start/exclusive-end UTC window.
It clips authoritative closed intervals to the requested window and sums seconds.
Open intervals never contribute an invented duration; an intersecting unresolved
interval makes the requested totals unavailable. Zero-length intervals contribute
zero. Results require private/no-store responses and never enter public caches.

The owner adapter must prove current ownership, the ownership-history boundary,
active profiles and current independent privacy floors on every read. It must
reject hidden/deleted/unclassifiable characters and foreign accounts. Authority
is revalidated after reading. A production adapter must coordinate these checks
with privacy/ownership invalidation; this task does not fabricate that evidence.
Pre-transfer interval history is refused. No configured owner adapter or owner
HTTP presentation is shipped while its authoritative integration is unavailable.

## Ingestion and storage

| POST route under `/internal/v2/game-public/` | Family | Ceiling |
| --- | --- | --- |
| `character-online-session` | `character_online_sessions` | 2048 bytes |
| `world-daily-kills` | `world_daily_kills` | 2048 bytes |
| `watermark` | These two G-8 V2 families only | 512 bytes |

The existing trusted TLS adapter must supply a verified TLS 1.3/HTTP 1.1 peer in
purpose `game-public`; HTTP headers cannot establish identity. Configuration
must use the same purpose-separated identity registry as the private listener.
The route guard checks bounded canonical JSON, exact ordered members and types,
UUIDv7 and decimal strings. Oversized input returns an empty 400. Unknown family,
version, operation or member fails 400. Wrong peer/authority fails 401. Disabled
or invalid configuration fails 503. The per-route publisher limit is 1200/minute;
429 returns Retry-After. All responses are private/no-store with Pragma no-cache.

`stats_v2_subjects` belongs to the existing stats module, separate from public
snapshots and daily tables. Stable subject keys, numeric epoch/revision ordering,
SHA-256 content comparisons and a transaction produce exact accepted/superseded
receipts and conflicts. The content excludes the observation/envelope fields.
Equal content never renews observation time. Replacement totals are not increments.
Writes, overlap checks and retention serialize with the existing public feed's
persisted epoch floor; an epoch raise immediately fences every lower-epoch family.
The v1 family enum and routes remain unchanged. No producer or Game calls exist.

Every read requires fresh family watermarks, the current shared epoch, and a
trusted `CoveragePolicy` confirming a qualified retained-set rebuild and external
public-feed fence. A heartbeat cannot certify an empty database or a new generation.
Source rows newer than the proven cut are unavailable. Production integration must
keep that coverage evidence valid across storage loss, restore and privacy changes.
`world_total` exposes only anonymous kill totals, retaining explicit complete,
provisional, zero and unavailable distinctions and UTC day boundaries.

## Flags and retention

All ingestion defaults off. Testing/preproduction activation requires:

- `PORTAL_PUBLIC_PROJECTIONS_ENABLED=true`, its existing authority and SAN settings;
- `PORTAL_STATS_ONLINE_INTERVALS_ENABLED=true` for owner-only interval ingestion;
- `PORTAL_STATS_WORLD_TOTALS_ENABLED=true` for world/day ingestion.

The separate `Config::owner_rankings_enabled` switch defaults false and requires
trusted owner/coverage adapters; no environment flag bypasses those integrations.
`configured_router` does not mount owner presentation. Never mount this consumer
on a public listener; missing verified transport provenance always denies.

Serve-time expiry is 30 days after a proven interval end, a 30-day hard ceiling
for an open interval, and 400 days after a world/day end. The registered hourly
cleanup erases payloads even after ingest/presentation are disabled, retaining
minimal subject, content-digest and revision/epoch fences. Obsolete observation
and expiry timestamps are cleared as part of erasure. Expired payloads cannot
be restored by retry, later correction or epoch replay. An expired unresolved open
interval continues to deny complete owner totals pending authoritative reconciliation.
Tombstones suppress affected totals. Payloads are not logged or quarantined.
Backup expiry and physical erasure guarantees require operational qualification.

## Qualification and remaining gates

Fixtures in `tests/fixtures` are synthetic exact lines authored from the contract;
files have a storage newline which the wire harness removes. No Game producer exists,
so these fixtures prove consumer behavior only, not producer compatibility.

Before enablement, the coordinator must supply the Game producer revision,
byte-identical producer/consumer fixture qualification, retained-set resync within
the accepted bound, a durable rebuild/privacy/ownership adapter, external fence
integration and private-listener PKI configuration. Public online-time ranking
requires a separately accepted duration aggregate/exposure contract. None of these
activation gates is inferred from a successful consumer test run.
