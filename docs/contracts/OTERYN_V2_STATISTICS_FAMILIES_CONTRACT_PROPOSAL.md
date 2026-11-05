Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# Oteryn-v2 statistics projection families — G-8

## 1. Purpose, authority and limits

Proposal ID: `oteryn-v2-statistics-families-proposal-1`. This is a proposed extension of `oteryn-game-public-projections-v1`, not acceptance of new families or permission to implement them. It addresses G-8 in the gate table of [the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md) and its §10 consumers T67 (online-time statistics/rankings) and T69 (daily world monster/boss totals). The game owner, through the coordinator, owns acceptance, scheduling after G-7, authoritative definitions and producer delivery. Platform owns ingestion, disposable read models, privacy, freshness and portal presentation. Neither side may infer G-8 completion from this document.

Scope is two new families: `character_online_sessions` and `world_daily_kills`. No experience/skill ranking inputs, runtime readiness/capacity, loot, damage, healing, private game journal (G-9..G-11), gameplay commands or entitlement effects are added. T67's other statistics and T69's online graph, population and deaths need their own existing sources. Existing bestiary progress and boss-credit receipts are not daily world totals.

These are families of the public-projection pipeline, not a declaration that all fields are public. Raw character intervals are **owner-only**. Anonymous world aggregates may be public. T67 may consume owner-scoped online totals, but public online-time rankings remain disabled pending Q1; owner access is not consent to public ranking. This deliberately leaves that plan/privacy conflict for the owner rather than widening disclosure.

## 2. Sources and compatibility baseline

Platform references read for this proposal:

- [Store wire](OTERYN_V2_STORE_WIRE_CONTRACT.md): compiled internal POST routes, purpose-separated mTLS, bounded canonical JSON, retry identities and digest conventions. Its own status is PROPOSED; it is precedent, not accepted Store activation authority.
- [Public projection architecture](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md): Game facts versus Platform read models, independent privacy floors, rebuilds, and unavailable versus empty.
- [Character commands](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md): operation identity, ambiguous outcomes and reconciliation; it defers exact cryptography/transport.
- [Entitlement delivery](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): fail-closed authority, independent truth domains and finite authority intervals; it also defers cryptographic primitives. No entitlement is involved here.
- Existing `app/GameProjections/{PublicProjectionFamily,PublicProjectionWire,PublicProjectionSnapshot,PublicProjectionWatermark,PublicProjectionSettings,PublicProjectionIngestion}.php` and `app/GameProjections/Http/GuardPublicProjectionPeer.php`: actual consumer vocabulary and validation. The PHP consumer is evidence, not a requirement that the Rust portal use PHP.

Game evidence is read-only, pinned throughout to **857f15f376907c592bb28d854fccff18a15cdf87**, inspected outside the Platform checkout. Paths below refer to [that Game tree](https://github.com/Oteryn/Oteryn-Game/tree/857f15f376907c592bb28d854fccff18a15cdf87):

| Game path | Evidence used |
|---|---|
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §§2–8, 11–12 | Accepted testing/preproduction families are highscores, character profile, world online and watermark; snapshot/empty-list semantics, TLS, epoch fence, outbox, operator resync and privacy. G-8 families are absent. |
| `crates/gameplay/native-admission-source/src/public_projections.rs` | Existing `Family`, `ProfileWire`, `OnlineWire`, `WatermarkWire`, `PublicStore`, `OutboxFacts`, `EXCHANGE_BOUND`, `DEFINITE_HOLD`, `BACKOFF_MIN/MAX`; compiled operations and exact bytes. New family/operation names below are proposed additions, not existing types. |
| `crates/gameplay/durability/src/durability/public_projections.rs` | `PROFILE`, `CLEAR_PROFILES`, `ONLINE`, reconciled authority and committed online predicate: eligible World admission guard, session state 2, current transport, no control loss. Current scan has no interval history. |
| `apps/game-server/migrations/0091_public_projections.sql` | Public epoch, revision/outbox persistence and operator resync foundation. |
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §§3–4; `crates/gameplay/native-admission-source/src/character_commands.rs` | Existing `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1`, `PublishCharacterCommandReceiptV1`: Game polls Platform, not a new Platform-to-Game listener. |
| `apps/game-server/migrations/0002_fresh_admission_authority.sql` | Admission/reconnect session and guard evidence contains private runtime identity; none is exported here. |
| `apps/game-server/migrations/0019_character_bestiary_progress.sql` | Character-credit receipts/progress saturate at definition thresholds; summing them cannot recover physical World deaths. |
| `apps/game-server/migrations/0532_bosstiary_kill_receipts.sql` | Boss receipts keyed by `(character_id, occurrence_id)` with `boss_key` and categories `bane`, `archfoe`, `nemesis`; multiple credited characters must not multiply world deaths. |

## 3. Flow directions, transport and trust

**Game → Platform ingest:** Game publishes full snapshots on these proposed compiled paths:

| Family | Proposed operation | POST path | Request ceiling |
|---|---|---|---|
| `character_online_sessions` | `PublishCharacterOnlineSessionV1` | `/internal/v1/game-public/character-online-session` | 2048 bytes |
| `world_daily_kills` | `PublishWorldDailyKillsV1` | `/internal/v1/game-public/world-daily-kills` | 2048 bytes |
| Either new family | existing `PublishPublicProjectionWatermarkV1` with extended family enum | `/internal/v1/game-public/watermark` | 512 bytes |

**Platform → Game command:** none for G-8. Platform never starts/stops a session, increments a kill, chooses an epoch or requests resync. HTTP acknowledgements are ingestion receipts, not gameplay commands. Recovery is Game's existing operator resync extended to these families after acceptance. If remote resync is requested later it needs a separate contract; do not add it to Character Authority's closed command set.

Reuse Game public-projections §3 and its LCFA transport by reference: HTTP/1.1 over TLS 1.3 using existing `http1_mtls::exchange`, a new connection per exchange (`Connection: close`), server certificate verification and purpose-bound client certificate verification. Existing public-projection publisher identity/allowlist and configured `source_authority` bind the caller; it must be separate from command, Store, runtime, native-evidence, entitlement and account-projection identities. Only trusted TLS-terminator values establish `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, and the allowlisted certificate subject. Headers supplied by an external caller establish no identity. Wrong peer/authority is `401`; disabled, overlapping identities or invalid configuration is `503` before processing.

No application HMAC, bearer credential or new message-signature envelope is introduced: existing public projections authenticate/integrity-protect bytes with mTLS. SHA-256 is a comparison digest, **not a signature or authentication proof**. An offline captured JSON object alone is never trusted source evidence. Any later signed/offline transport requires an accepted versioned change.

Connect ceiling 1 s, TLS handshake 2 s, whole exchange including setup 3 s (the existing producer bound within the contract's 5 s outer ceiling). One snapshot in flight per family per publisher; response ceiling 256 bytes. Reuse per-peer/per-route rate limiting: proposed default 1200 requests/minute, valid configured range 1..6000. Every response is `Cache-Control: no-store, private`, `Pragma: no-cache`. No public listener, browser call, broker, shared database or runtime query during page rendering.

## 4. Exact encoding and common fields

All objects have exactly the listed members, in listed order for golden serialization; no missing, duplicate, unknown or null member, no floats or coercion. UTF-8 JSON, no BOM, escapes, insignificant whitespace or trailing newline; at most five nested containers. `Content-Type: application/json`. `Transfer-Encoding` absent; `Content-Encoding` absent or `identity`. Content length, if supplied, is canonical nonnegative decimal and within the route ceiling; empty, oversized or malformed input is `400`, following the public-projection vocabulary (Store's `413` is not imported).

| Type | Exact rule |
|---|---|
| `id` | Lowercase canonical non-nil UUIDv7: `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `u64` | JSON string `[1-9][0-9]{0,19}`, value 1..18446744073709551615 |
| `u64z` | `u64` or string `"0"` |
| `time` | `u64z`, Unix seconds UTC, never milliseconds; body start/end/day times must be nonzero |
| `authority` | String `[A-Za-z0-9._:/-]{1,128}`, exactly configured Game namespace |
| `boolean` | JSON `true` or `false` only |

Times use the Game wire's decimal **strings**, matching `ProfileWire`/`WatermarkWire`; do not copy the Platform PHP internal integer representation onto the wire. A missing value is represented by the bounded empty list described below; `null` is always invalid. No field is implicitly optional.

Every snapshot starts with `contract_version` (JSON integer, exactly `1`), `operation` (exact route operation), `source_authority` (`authority`), followed by subject fields, then `projection_epoch` (`u64`), `projection_revision` (`u64`), `source_observed_at` (`time`), then the body. Observation is the revision-assignment time, immutable on retry; it does not order records or renew freshness. All IDs are join keys, not ownership or admission authority.

### 4.1 Character online interval snapshot

Subject fields, in order: `character_id` (`id`), `statistics_id` (`id`). Body member: `sessions` (array, length 0..1). Its single entry, when present, has exactly:

| Member | Type/unit | Meaning and validation |
|---|---|---|
| `world_id` | `id` | Canonical Platform Registry World of this entire interval |
| `started_at` | nonzero `time` | Inclusive start of committed online eligibility |
| `ended_at` | array 0..1 of nonzero `time` | `[]` means end not yet proven; `[t]` is exclusive end, `t >= started_at` and `t <= source_observed_at` |

`started_at <= source_observed_at`. Zero-length closed intervals are permitted and contribute zero seconds. `statistics_id` is a newly minted Game statistics-only UUIDv7 persisted once at interval creation. It is neither `GameSessionId`, an admission receipt ID nor a derivation of either. A reconnect that breaks the committed online predicate closes the old interval and starts another; moving Channel without breaking it creates no new interval. A World transfer splits intervals at the authoritative boundary. Intervals for one character never overlap, including across Worlds. Time precision is whole seconds rounded down from persisted millisecond facts; duration is `end - start` in seconds, never time since receipt. Game must detect unsafe time/negative duration and reconcile rather than clamp or invent an end.

Proposed online predicate is the existing `ONLINE` committed predicate cited above. Start/end must be captured at authoritative transitions, not inferred from the 10-second public scan. A crash, lost transport, control loss or lease expiry requires Game-authoritative closure; Platform never closes an interval from silence, timeout or wall-clock extrapolation. Open intervals contribute no completed duration and prove neither current online status nor total completeness. Reconnect/crash exact boundary policy is Q2.

`sessions: []` is an explicit tombstone/suppression for this subject, including deletion, visibility removal and retention expiry. It is not a zero-length session. The publisher must enumerate all affected retained statistics subjects on a character removal; a profile removal independently fences serving immediately. Higher revisions replace the complete interval record, so an end arriving before its start snapshot is safe. Correction requires a higher revision; a closed interval cannot reopen implicitly. A genuinely unknown end remains unknown until corrected authoritative evidence exists.

### 4.2 World/day aggregate snapshot

Subject fields: `world_id` (`id`), `day_start` (nonzero `time`, divisible by 86400, UTC midnight). Body member: `days` (array 0..1). Entry members, in order:

| Member | Type/unit | Meaning and validation |
|---|---|---|
| `day_end` | nonzero `time` | Exactly `day_start + 86400`, exclusive, no overflow |
| `monsters_killed` | `u64z`, deaths | Physical non-boss monster deaths in this World/day |
| `bosses_killed` | `u64z`, deaths | Physical boss deaths in this World/day |
| `complete` | `boolean` | `true` only after day end and authoritative ingestion of all deaths/corrections through that boundary |

`day_start <= source_observed_at`; if `complete=true`, `day_end <= source_observed_at`. Counts are cumulative replacement totals, never increments to apply. A complete zero day is explicit `"0"`/`"0"`, not a missing row. `days: []` withdraws that World/day as unavailable; absence never means zero. Late authoritative corrections publish higher revisions even after `complete=true`; completion is a proven cut, not immutability.

Proposed counting rule: exactly one count per committed physical creature death occurrence, across every Channel of that World; bosses are excluded from `monsters_killed` so the categories are disjoint. Count by authoritative occurrence time in `[day_start, day_end)`, not reward delivery or export time. No player deaths, despawns, summons without an eligible authoritative creature-death classification, credited-character multiplicity or bestiary saturation effects. Game retains its private death-occurrence dedup key and catalogue classification; no receipt/creature/account/character IDs or content keys cross this aggregate wire. Respawn then another death is another occurrence. Unknown classification or uncertain World/time prevents completeness and stalls the affected family's watermark; do not silently omit it. Q3 must accept these semantics before implementation. Overflow fails the publication, never wraps or saturates.

### 4.3 Watermark and acknowledgement records

Exact existing watermark member order: `contract_version` (integer 1), `operation` (`PublishPublicProjectionWatermarkV1`), `source_authority` (`authority`), `family` (`character_online_sessions` or `world_daily_kills`), `projection_epoch` (`u64`), `complete_through` (`time`), `observed_at` (`time`). Required relation: `complete_through <= observed_at`; existing LCFA future-time/refusal checks apply with 1 s Platform clock uncertainty. No extra fields.

A successful snapshot or watermark response is exactly one of these existing records: `{"contract_version":1,"result":"accepted"}` or `{"contract_version":1,"result":"superseded"}`. No other result, members or nulls. HTTP 200 alone with malformed/truncated body is ambiguous, never an acknowledgement.

## 5. Ordering, outbox, watermark and replay

Deduplication key: `(source_authority, family, subject, projection_epoch, projection_revision)`. Subject is `(character_id, statistics_id)` or `(world_id, day_start)` respectively. Revisions increase per subject, not globally. Do not introduce an operation UUID header or timestamp idempotency key. Game commits authoritative change, subject revision and outbox touch atomically, following `OutboxFacts`/`PublicStore` and migration 0091. Rejected fenced writes produce none. New G-8 transactional source capture is required; current sources do not prove it exists.

Order lexicographically by numeric `(projection_epoch, projection_revision)`. Lower pair is `superseded`; equal pair/equal content is `accepted`; equal pair/different content is `409`, preserving the first record. Content is subject fields plus complete body, recursively sorted object keys with list order preserved, compact UTF-8, SHA-256 lowercase hex internally, as `PublicProjectionSnapshot` does. Envelope/observation/epoch/revision are excluded from the content digest; equal content keeps the first observation time. Wire retries preserve their original exact bytes. A higher pair replaces the subject atomically. Revision gaps are legal for replacement snapshots/outbox coalescing; never treat them as incremental-event continuity or infer missing intervals from them.

Use the existing public feed epoch and independent external high-water fence `F_pub`. A higher epoch invalidates every lower-epoch family immediately. Only Game's reconciled operator restore/resync can raise it, strictly above the fence; no publication while the store trails the fence. Restart/restore cannot reset accepted Platform epoch floors. New families share restore coordination with existing public families, not the Character command or entitlement epoch.

Publisher clears only outbox entries up to the acknowledged revision after valid `accepted`/`superseded`, retaining newer writes. `complete_through` may not pass any unacknowledged committed change, unresolved invalid classification, retention tombstone or correction. Heartbeats alone cannot advance it. An unresolved open interval is faithfully represented as open; watermark completeness is delivery completeness, not proof of logout. A day with `complete=false` remains provisional even with a fresh family watermark.

Watermarks follow the existing maximum 10 s gap (producer target 3 s). Testing/preproduction freshness: stale if `platform_now - complete_through + 1 > 30` seconds, future beyond 1 s, wrong epoch, invalid evidence or incomplete rebuild. Check at every serve/cache access; no stale display window. An old immutable closed interval can remain available while its family is fresh; its observation timestamp is not its freshness clock. Production cadence/skew/S require explicit acceptance (Q5).

Replay is operator-driven authoritative full resync of retained subjects, with outbox replay until acknowledged. Do not replay network captures as authority, advance a watermark across a poison record or infer completeness from row counts. Platform builds a new generation off-path from the enumerated retained set, including tombstones, under a known epoch/cut; reconciles the outbox tail and watermark covering that cut; validates uniqueness, interval overlap, counts, all subject coverage and current privacy floor; switches atomically only after every check passes. A baseline enumerator must prove coverage, including absence for previously stored subjects; a fresh watermark alone does not prove an empty new read model is complete. Lost projection storage requires this full rebuild, not merely resuming the live watermark. Game must qualify the extended existing 60 s stopped-node resync bound or return a contract blocker; do not invent a portal-initiated paging API. Retention beyond the authoritative replay window is unavailable, never reconstructed from saturated bestiary totals.

## 6. Failure, refusal and timeout semantics

| Result | Meaning and required action |
|---|---|
| `200 accepted` | Projection persisted/idempotently accepted; no gameplay effect claimed |
| `200 superseded` | Older source evidence safely superseded; not permission to lower a floor |
| `400` empty | Malformed, unsupported version/enum, null, unknown member, impossible record, over bound; no write |
| `401` empty | Unauthenticated, wrong-purpose peer or authority; no write |
| `409` empty | Equal pair with changed content; first record kept, no overwrite |
| `429` empty | Rate limit, `Retry-After` seconds >=1; no write |
| `503` empty | Disabled/misconfigured/unavailable; no claimed acceptance |
| Timeout, TLS/network failure, unknown status or invalid response | Delivery unknown, including possible commit with lost acknowledgement; retain outbox and retry same identity/bytes |

Reuse public-projection retries: exponential 1..60 s for transient failures, honoring a larger `Retry-After`; reset on valid delivery. Definite `400`/`401`/`409` retains the subject, alarms and holds retries at least 300 s; failed watermark receives the same hold. Local over-bound construction is handled as `400`, never truncation. A poison record blocks family completeness; unrelated families may progress unless the shared epoch invalidates them.

Unknown, stale, ambiguous, rebuilding or invalid source/ownership/privacy state **denies access to affected character data and denies assertions of complete totals/rankings**. Return unavailable/degraded, not zero, offline, no sessions or success. T69 may show explicitly provisional counts only with fresh valid source evidence and `complete=false`; unavailable evidence is not provisional evidence. No synchronous Game fallback or direct SQL. Rollback preserves epoch/privacy floors; disabling publication cannot make old data perpetually fresh. Transport failure never rolls back gameplay or re-executes a death/session transition.

## 7. Privacy, retention and presentation

Current authoritative AccountId↔CharacterId ownership must be proven server-side for every owner interval read, using the accepted account/Character Authority path, independently of projection freshness. Browser IDs, names and these snapshots prove no ownership. No owner identifiers are sent in G-8. Foreign, unknown or ambiguously owned subjects return indistinguishable access denial. Account transfer immediately denies the former owner; proposed policy also withholds pre-transfer history from the new owner pending Q1, requiring an authoritative ownership boundary before access. Uncertain invalidation is denial, not a successful purge.

Raw intervals and derived owner totals never enter public pages, search, CDN, anonymous APIs or public caches. Owner responses are private/no-store. No public cross-character grouping or account linkage. Restrictive visibility/lifecycle changes fence all affected caches and generations at the current independent Platform privacy floor; a fresh source revision never overrides it. Hidden/deleted/unclassifiable profiles deny interval serving regardless of an older interval record. Owner-only classification applies to the whole raw interval, including start/end and `statistics_id`, and to any derived output revealing the schedule. Public T67 rankings need a separate accepted aggregate exposure decision.

Proposed retention, pending Q4: intervals for 30 days after authoritative end; unresolved open intervals have a 30-day hard retention ceiling after start and become unavailable pending Game reconciliation, never auto-closed. World/day aggregates for 400 days after day end. Both systems enforce serve-time expiry even during producer outage; Game publishes higher-revision tombstones and deletes eligible payloads, Platform purges raw payloads/derived caches within 24 hours, backups expire within 30 days of purge. Idempotency/epoch/privacy floors survive payload erasure for the supported replay window; minimal non-payload subject fences remain until their whole retained epoch is fenced out. Replays cannot resurrect expired data, and new-epoch rebuilds still apply retention. No body logging or payload-bearing diagnostic archives; quarantine stores only bounded reason and nonpublic reference for at most 7 days. Exact deletion/backup obligations require owner acceptance; these durations are proposal values, not a legal-policy decision.

Never send AccountId, Identity ID, email, IP, device data, network address, credentials, GameSessionId, connection/reconnect/admission IDs, lease/fence/generation secrets, ChannelId, node/instance/position, private topology, receipt command bindings, names, loot, damage, payment or entitlement data. `statistics_id` is purpose-separated correlation only. Operational logs/metrics contain family, operation, result class and timings, without IDs or bodies, as the existing publisher does. Privacy audit evidence stays access-controlled and payload-free.

## 8. Flags, versioning and rollout

Proposed per-family switches `character_online_sessions.enabled` and `world_daily_kills.enabled` exist on both producer and consumer, **false by default**, subordinate to Game `[platform.public_projections] enabled` and Platform `game-projections.public.enabled` (or equivalent Rust configuration). Separate owner presentation switches default false, including public T67 ranking exposure. Flags here name proposed configuration, not shipped keys. No environment/workflow/config changes are part of this document. Initial acceptance is testing/preproduction only; production needs its own PKI, measured limits, retention/privacy and exact-revision enablement packet.

Existing v1 family and operation enums are closed. These new names are not accepted by old binaries; do not send them until an explicitly accepted extension and exact producer/consumer compatibility matrix exist. `contract_version:1` is proposed for new routes using the existing envelope; extending the watermark family set is a negotiated extension, requiring explicit owner acceptance under the Game contract's new-version rule. If the owner requires `contract_version:2`, revise all new golden payloads and compiled dispatch together before acceptance; never silently change existing accepted family bytes. Any later change of member/path/bound/counting/privacy/ordering meaning needs a new accepted version. Unknown versions fail `400`, never best-effort parsing. No down-conversion to existing `world_online`/profile bodies or mixed source authority.

Rollback disables presentation first, then publishing/ingest as needed; retains outbox and monotonic floors. A previous projection generation may serve only if fresh, retention-eligible and composed under current ownership/privacy evidence. This proposal adds no runtime, schema, deployment, workflow or cross-repository write authority.

## 9. Golden JSON payloads

Each fenced line is the complete compact wire body; the enclosing Markdown newline is not transmitted. UUIDs and times are synthetic.

Valid open interval (revision 1):

```json
{"contract_version":1,"operation":"PublishCharacterOnlineSessionV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","statistics_id":"01934f10-7c08-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"1","source_observed_at":"1790000000","sessions":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","started_at":"1790000000","ended_at":[]}]}
```

Valid closure; exact retry is accepted without adding duration again:

```json
{"contract_version":1,"operation":"PublishCharacterOnlineSessionV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","statistics_id":"01934f10-7c08-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"2","source_observed_at":"1790003600","sessions":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","started_at":"1790000000","ended_at":["1790003600"]}]}
```

Valid explicit suppression:

```json
{"contract_version":1,"operation":"PublishCharacterOnlineSessionV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","statistics_id":"01934f10-7c08-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790003700","sessions":[]}
```

Valid final world/day aggregate (UTC midnight; no double boss counting):

```json
{"contract_version":1,"operation":"PublishWorldDailyKillsV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","day_start":"1789948800","projection_epoch":"1","projection_revision":"2","source_observed_at":"1790035201","days":[{"day_end":"1790035200","monsters_killed":"12345","bosses_killed":"12","complete":true}]}
```

Valid watermark and acknowledgement (each is a separate message):

```json
{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_daily_kills","projection_epoch":"1","complete_through":"1790035201","observed_at":"1790035203"}
```

```json
{"contract_version":1,"result":"accepted"}
```

Invalid `400`: explicit null is not an unknown end list:

```json
{"contract_version":1,"operation":"PublishCharacterOnlineSessionV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","statistics_id":"01934f10-7c08-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"1","source_observed_at":"1790000000","sessions":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","started_at":"1790000000","ended_at":null}]}
```

Invalid `400`: numeric count instead of canonical decimal string (all other members valid):

```json
{"contract_version":1,"operation":"PublishWorldDailyKillsV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","day_start":"1789948800","projection_epoch":"1","projection_revision":"2","source_observed_at":"1790035201","days":[{"day_end":"1790035200","monsters_killed":12345,"bosses_killed":"12","complete":true}]}
```

Invalid `409` **after** the valid final aggregate above was stored: same pair, changed count. Without that predecessor it is a structurally valid snapshot, demonstrating a stateful refusal:

```json
{"contract_version":1,"operation":"PublishWorldDailyKillsV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","day_start":"1789948800","projection_epoch":"1","projection_revision":"2","source_observed_at":"1790035201","days":[{"day_end":"1790035200","monsters_killed":"12346","bosses_killed":"12","complete":true}]}
```

Additional mandatory negatives: end before start, overlapping intervals, future end, unaligned day, day overflow, unknown family/version/member, duplicate JSON key, zero/numeric/malformed ID or revision, oversized request, forged peer, future watermark, stale epoch, premature `complete=true`, owner transfer/privacy outage, timeout after consumer commit and replay after retention expiry. These must be shared fixtures in the later implementation; this docs-only task introduces no tests or runtime code.

## 10. Acceptance checklists

Platform:

- [ ] Coordinator accepts privacy scope/Q1 and exact wire version; preserve G-8 as unaccepted until then.
- [ ] Pin accepted Game producer revision and share byte-identical valid/invalid fixtures and compatibility matrix.
- [ ] Validate bounds/types/exact keys, purpose mTLS, authority, empty refusal responses and rate limits.
- [ ] Prove equal-content retry, conflict, superseded replacement, timeout-after-commit, epoch fencing and rebuild coverage.
- [ ] Prove T67 never double-counts intervals or extrapolates unknown ends; T69 preserves provisional/complete/zero/unavailable distinctions.
- [ ] Prove owner/foreign-object denial, ownership transfer cutoff, privacy-floor fencing, hidden/deleted profiles and every cache path.
- [ ] Prove stale watermarks stop serving, retention erasure blocks replay resurrection, rollback preserves floors, flags default off.
- [ ] Qualify exact-head Rust consumer integration before enablement; documentation acceptance alone does not close T67/T69 or G-8.

Game:

- [ ] Accept/revise Q1–Q6 through the coordinator and explicitly extend the closed family/watermark vocabulary/version.
- [ ] Supply durable interval history and physical death dedup/count sources; existing online scans/bestiary/boss credits alone do not satisfy G-8.
- [ ] Prove atomic source/revision/outbox capture under session/recovery fences; reconnect, crash, expiry and World boundaries produce nonoverlapping intervals.
- [ ] Prove physical deaths count once across parties/Channels/retries, boss categories are disjoint, UTC boundaries and late corrections are deterministic.
- [ ] Prove stable statistics-only IDs, no prohibited fields, tombstone enumeration, bounded payloads and no silent truncation.
- [ ] Prove outbox acknowledgement races, retry holds/backoff, watermark stalls, external `F_pub`, restore and full retained-set resync within qualified bounds.
- [ ] Supply measured retention/replay/backup coverage, exact producer revision, shared fixtures and default-off testing/preproduction rollout evidence.

## 11. Open questions for the game owner (coordinator)

1. **Privacy/product:** accept owner-only raw intervals and anonymous public world totals? Should T67 public online-time rankings be deferred, or supplied by a separately approved aggregate/consent profile? Confirm whether a new owner may see any pre-transfer history; proposed answer is no.
2. **Session definition:** accept the existing committed `ONLINE` predicate as the interval boundary, including reconnect/control-loss gaps? What authoritative timestamp closes crash/expiry intervals, and how are unsafe clocks and unclosed intervals reconciled without invented duration?
3. **Counting:** accept physical deaths once per occurrence, disjoint non-boss/boss totals, UTC days and late correction semantics? Which authoritative source classifies bosses/summons and guarantees no multiplication by character credits or threshold saturation?
4. **Retention/replay:** accept 30-day intervals, 400-day aggregates, open-interval ceiling, purge/backup windows and retained non-payload fences? Can full retained-set resync prove coverage within the current stopped-node 60 s bound, or is a separately versioned bounded resync needed?
5. **Wire/operations:** accept new-route v1 plus negotiated watermark enum extension, or require v2 under the existing change rule? Confirm purpose identity, proposed bounds/rate, test freshness values and production PKI/SLA separately.
6. **Delivery/acceptance:** confirm Game ownership and queue after G-7, exact implementation paths, cross-repository fixture/compatibility evidence and the coordinator decision that will close G-8. No new Platform → Game command is proposed; is operator-driven resync sufficient?
