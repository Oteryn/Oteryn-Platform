Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# World daily public projection family — G-12

## Purpose, ownership and acceptance boundary

Proposal ID: `oteryn-v2-world-daily-family-v1`. This document proposes the producer/consumer boundary for G-12 in `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md` (gate table and §10). T88 (`crates/community/src/today*`) consumes the boosted creature, boosted boss, their bounded history and scheduled raids. T94 (`crates/minigames/src/league*`, dependent on T89 and T88) may display those same facts for world-event predictions without stakes. This feed supplies neither prediction results nor settlement authority. Acceptance alone does not close G-12: compatible implementation and cross-repository evidence are still required.

Game owns committed boost selections, reset boundaries, raid scheduling, public-release decisions, producer revisions, outbox and recovery. Platform owns Registry `WorldId`, normalized disposable read models, freshness, public allowlisting, presentation and privacy restrictions. The coordinator accepts Game semantics and publication policy; Platform accepts its consumer obligations. Neither side may claim the other's acceptance.

Docs only: no runtime, schema, workflow, transport deployment, gameplay mutation or production activation is authorized. Every new operation, field profile, bound and flag below is proposed, not a claim of an existing Game endpoint.

## Existing conventions and pinned evidence

Platform references, read together:

- `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`: source authority, rebuildable public families, independent privacy floor, evidence versus content state, no synchronous Game call during page rendering.
- `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`: stable operation identity, conflicting reuse refusal, durable outcomes and same-identity reconciliation after ambiguity. Its architecture contract deliberately defers transport/signing.
- `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`: authority separation, bounded authority and unknown-state denial; it also defers exact transport. This feed conveys no entitlement or benefit authorization.
- `OTERYN_V2_STORE_WIRE_CONTRACT.md` §§2–4, 9: bounded canonical JSON, source namespace, dedicated mTLS identities, internal listener, digest/idempotency conventions. That document is itself **PROPOSED**; it is not evidence of accepted Store activation. Where its Store-specific rules differ from the accepted public publisher (depth, response vocabulary, oversize status), the public publisher governs this proposal.

Read-only Game evidence is pinned to [857f15f376907c592bb28d854fccff18a15cdf87](https://github.com/Oteryn/Oteryn-Game/tree/857f15f376907c592bb28d854fccff18a15cdf87). All following paths are relative to that revision:

| Game path | Evidence used |
|---|---|
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §§3–6, 11 | Accepted public push transport, envelope, epoch fence `F_pub`, outbox acknowledgements, watermark, freshness and operator resync; accepted families do not yet include `world_daily` |
| `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §§3–5 | Underlying HTTP/1.1 mTLS convention, decimal encodings, external epoch fence and late-transaction watermark protection |
| `crates/gameplay/native-admission-source/src/public_projections.rs` | `Family`, `HighscoresWire`, `ProfileWire`, `OnlineWire`, `WatermarkWire`, `PURPOSE = OTERYN_GAME_PUBLIC_PROJECTIONS`, compiled paths and bounded retries; no daily family yet |
| `crates/gameplay/durability/src/durability/public_projections.rs` and `apps/game-server/migrations/0091_public_projections.sql` | Reconciled-authority reads, same-transaction revisions/outboxes, clear only through acknowledged revisions |
| `crates/gameplay/durability/src/durability/daily_boost.rs` | `DailyBoostBinding`, `prepare_daily_boost`, `game_world_daily_boosts`; durable selection per World/reset, restart/concurrent-Channel reuse; reset epoch is not a calendar day |
| `crates/protocol-oteryn/src/daily_boost.rs` | `DailyBoostView`, `reset_epoch`, `creature_key`, `boss_key`; query command 177, capability 151; keys bounded to 255 bytes by codec/durable preparation |
| `crates/gameplay/durability/src/durability/raid_schedule.rs` | Metadata CAS with `RaidWrite::{Applied,Replayed,CompareFailed}`; metadata disposition is not a commit token |
| `crates/gameplay/encounter-rules/src/raid/schedule.rs` and `raid/calendar.rs` | `RaidSchedule`, `FiringState::{Running,Done,Cancelled}`, fixed calendar slots, random/source scheduling; `next_check_at_ms` is not a promised raid time |
| `crates/protocol-oteryn/src/raid_schedule.rs` | `RaidScheduleState`; query 171, subscribe 172, notice ACK 173, capability 149; read-only gameplay protocol, no command to start/advance/cancel raids |
| `crates/gameplay/content/src/content/production.rs` | Production key ASCII alphabet; public profile below narrows lengths explicitly |

These are source citations, not instructions to modify Game. Existing client binary query payloads are not service-to-service publication payloads. This proposal adds no gameplay opcode or reused Character/Store command with unrelated meaning.

## Flows and transport

| Semantic flow | Physical direction | Scope |
|---|---|---|
| Daily snapshot ingest | Game → Platform, proposed `POST /internal/v1/game-public/world-daily`, `PublishWorldDailyV1` | Complete World snapshot, not delta |
| Liveness evidence | Game → Platform, existing `POST /internal/v1/game-public/watermark`, `PublishPublicProjectionWatermarkV1` | Proposed new closed enum value `family: world_daily` |
| Ingest acknowledgement | Platform → Game HTTP response on the above exchange | Receipt of projection only; never gameplay success |
| Platform → Game command | **None in v1** | No boost selection, raid start/cancel/reschedule, resync request, subscription or prediction command |

Rebuild requests go through the existing authorized Game operator process, not a new inbound Game service. Extending `oteryn-game-ops public-projection resync --family` to `world_daily` requires Game acceptance/implementation; it does not exist at the pinned revision.

Reuse the public-projection publisher's HTTP/1.1 over TLS 1.3, existing trusted internal listener and purpose-bound public-projection client certificate; do not create a Store identity or give browsers access to ingest. The listener verifies the certificate chain and configured exact subject, with `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3` and trusted terminator-supplied identity only. Strip spoofable client headers. Namespace and allowed World scope must match configured authority. Missing/overlapping identity, missing trust or ambiguous configuration denies the surface. The certificate cannot authorize account/character/Store routes. Verify Platform server identity through the existing trusted CA/hostname policy; no redirects or plaintext fallback.

Signing: the reused publisher uses mTLS, not application-level JWS/HMAC. Add no signing header, bearer secret, nonce scheme or private key to JSON. SHA-256 content fingerprints are comparison evidence, never signatures or authorization. Preserve existing certificate rotation/revocation policy; inability to authenticate denies publication.

Connect ≤1 s, TLS handshake ≤2 s, whole cold exchange ≤3 s (inside the public contract's 5 s ceiling), `Connection: close`. One daily publication in flight per publisher/family. Snapshot ≤131072 UTF-8 bytes; watermark ≤512 bytes; response ≤256 bytes. `Content-Type: application/json`, no BOM, no trailing newline, no insignificant whitespace or string escapes, exact table member order, maximum JSON nesting depth 5. Reject duplicate, unknown, missing or null members. No transfer encoding; content encoding absent or `identity`; if present Content-Length is canonical nonnegative decimal and within bound. Refuse oversized actual bodies too, without unbounded reads. Public-family malformed/oversized requests return empty `400`, rather than importing Store's `413` response.

All responses use `Cache-Control: no-store`. Rate limit by authenticated publisher under the existing public-projection guard; proposed daily allowance is 120 requests/minute, watermark allowance 60/minute, burst no larger than those limits. Return `429`, with positive integral `Retry-After` seconds when supplied; never spin on retry.

## Exact proposed wire schema

Every table is ordered; all fields are required and non-null. Empty arrays represent explicit absence only as defined below. Bounds are inclusive. No free-form text or local numeric database identifiers cross this boundary.

| Alias | Exact type and bounds |
|---|---|
| `u64+` | JSON string matching `[1-9][0-9]{0,19}`, numeric value ≤18446744073709551615; compare as integers, never floating point |
| `seconds` | JSON string `0` or `u64+`, Unix UTC seconds ≤9223372036854775, safely convertible to Game signed milliseconds |
| `uuid7` | Lowercase non-nil UUIDv7, `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `authority` | ASCII `[A-Za-z0-9._:/-]{1,128}`, exact configured namespace |
| `content_key(n)` | ASCII `[A-Za-z0-9:._/-]{1,n}`, exact Game content key, case preserved; no URL, display-name inference or truncation |

Snapshot members:

| Field | Type | Meaning |
|---|---|---|
| `contract_version` | JSON integer `1` | Daily wire version |
| `operation` | literal string `PublishWorldDailyV1` | Route-bound operation |
| `source_authority` | `authority` | Game publisher namespace |
| `world_id` | `uuid7` | Registered World, shared across its Channels |
| `projection_epoch` | `u64+` | Existing public-family recovery epoch, not reset epoch |
| `projection_revision` | `u64+` | Monotonic per World daily snapshot revision |
| `source_observed_at` | `seconds` | Time revision assigned; stable on retry |
| `daily` | array of 0 or 1 daily record | `[]` explicitly withdraws the whole World daily public view |

Daily record members:

| Field | Type | Meaning |
|---|---|---|
| `reset_epoch` | `u64+` | Current committed World server-save epoch |
| `effective_from` | `seconds` | Inclusive current reset interval start |
| `effective_until` | `seconds` | Exclusive authoritative interval end; strictly after start |
| `creature_key` | `content_key(255)` | Boosted monster; retains Game's creature terminology |
| `boss_key` | `content_key(255)` | Boosted boss |
| `history_complete_from` | `seconds` | Inclusive source coverage floor for history |
| `history` | array 0..32 of history records | All completed boost intervals within coverage; descending `reset_epoch`, unique |
| `raids` | array 0..256 of raid records | Complete public fixed-slot schedule within current interval; sorted by (`scheduled_at`, `raid_key`), unique pair |

History record members, in order: `reset_epoch` (`u64+`), `effective_from` (`seconds`), `effective_until` (`seconds`), `creature_key` (`content_key(255)`), `boss_key` (`content_key(255)`). Each epoch is less than current; each interval has start < end ≤ current start; intervals do not overlap and starts are within `[history_complete_from,effective_from)`. The coverage floor is ≤ current start and no earlier than current start minus 2592000 seconds (30 days, clamped to zero). If there are more than 32 intervals in 30 days, raise the floor to the earliest fully retained interval; never claim complete history after silently truncating it. Empty history with floor equal to current start explicitly claims no older coverage. Epoch gaps are permitted; epochs are not days. Coverage contains every selection interval beginning within it, including multiple resets on one date. Missing older history is outside coverage, not proof of no boost.

Raid record members:

| Field | Type | Meaning |
|---|---|---|
| `raid_key` | `content_key(128)` | Game raid definition key; narrowed to existing client raid key bound |
| `scheduled_at` | `seconds` | Fixed calendar slot in `[effective_from,effective_until)` |
| `state` | enum string `scheduled`, `running`, `done`, `cancelled` | Public scheduling state only |

Only Game-approved fixed slots appear. `scheduled` means a published opportunity, not guaranteed execution: eligibility can consume/skip a slot. `running`, `done`, `cancelled` require committed corresponding Game evidence; metadata `Applied` or time passing alone never proves runtime effects or completion. No undisclosed random draw, source-scheduled future trigger or `next_check_at_ms` is converted to a predicted firing time. A consumed slot that did not run maps to `cancelled` only when Game proves it cannot later fire for that slot. Unsupported/ambiguous source classification withholds the family watermark rather than inventing state. Subsecond fixed slots cannot be rounded: refuse until an accepted representation exists. `raids: []` means no publishable fixed slots in this interval, not no possible random raids. There is no running-creature/player count, participant list, reward, score or prediction outcome.

A daily record requires both committed boost keys and a fully classified public schedule. Unknown boosts, uncertain interval end or an over-bound schedule must not become `daily: []` or `raids: []`. Hold publication and freshness instead. `daily: []` is reserved for an explicit committed withdrawal/no-public-view decision and removes current facts and history from public serving. Static catalogue names/artwork resolve through separately accepted content snapshots; an unresolved key is shown as unavailable, never guessed.

Watermark members in order: `contract_version` (integer `1`), `operation` (literal `PublishPublicProjectionWatermarkV1`), `source_authority` (`authority`), `family` (literal `world_daily`), `projection_epoch` (`u64+`), `complete_through` (`seconds`), `observed_at` (`seconds`). Require `complete_through ≤ observed_at`; neither may exceed Platform now +1 second. No other member or null is valid.

Success responses are exactly one of:

```json
{"contract_version":1,"result":"accepted"}
```

```json
{"contract_version":1,"result":"superseded"}
```

## Identifiers, ordering, outbox and replay

Subject key is (`source_authority`, `world_daily`, `world_id`). Idempotency key is that subject plus (`projection_epoch`, `projection_revision`); no independent HTTP idempotency header or random event ID. Content is canonical `world_id` plus `daily` in that order. Internally a SHA-256 fingerprint of those exact canonical object bytes may accelerate comparisons; retain exact content for conflict verification. Envelope and `source_observed_at` are not content, following the public contract. Producers retry identical bytes; an equal pair with equal content is `accepted` and retains the first timestamp, even if a defective retry changes only the timestamp. An equal pair with changed content returns `409` and preserves the first snapshot.

Game transactionally advances the World revision and records its outbox touch with every projected change: reset activation, boost selection, history coverage change, public fixed-slot/status change or withdrawal. A publisher reads one committed consistent snapshot under reconciled authority. Acknowledgement follows Platform's durable atomic content+revision commit. Only `accepted` or `superseded` clears Game outbox rows through the sent pair; concurrent newer rows remain. Crash before/after acknowledgement therefore safely repeats delivery. No projection failure rolls back a gameplay change.

Compare epoch/revision pairs lexicographically. Lower pairs are `superseded`; higher pairs replace the complete subject. Because these are complete snapshots, skipped revision numbers do not imply missing deltas. Completeness comes from the outbox and watermark, not sequence contiguity or arrival time. Persist the highest public epoch across all participating public families; first sight of a higher epoch invalidates all entries below it. A snapshot/watermark from an older epoch cannot restore freshness.

Reuse the existing external durable `F_pub` fence: before publishing an epoch persist its high-water mark outside Game backups; missing/unreadable fence refuses publication. Restore stops the publisher, raises epoch strictly above every previously published epoch, resyncs every public subject and restarts only under reconciled authority. Never reset the fence with a process restart or Platform projection rebuild. Platform cannot assign Game epochs or reset epochs.

Watermarks attempt every 3 seconds and must progress within 10 seconds while healthy. `complete_through` certifies that every committed projected change at/before it has been acknowledged. Use the existing outbox computation with its maximum transaction-duration protection; never advance past an undelivered row or an unfinished resync. It covers all Worlds in the configured authority, not just the last successful World. No source tables/source classification means no advancing watermark.

Initial fill/read-model loss uses a Game operator full current snapshot resync, including explicit withdrawals for previously exported Worlds with no public view. Retain a bounded exported-subject inventory so deletion cannot leave an orphan. Build a candidate Platform generation off-path, consume the resync and later outbox tail, then verify all configured Worlds, current epoch, fresh completeness watermark and privacy floor before atomic switch. No event-log retention or historical delta replay is promised: complete snapshots plus operator resync are the authoritative rebuild path. Lost pre-coverage history is not reconstructed from guesses. During resync keep completeness below resync start until all publications are acknowledged.

## Failure, refusal, timeout and serving

| Outcome | Meaning and required action |
|---|---|
| `200 accepted` | Durable ingest or equal-content retry; not gameplay execution proof |
| `200 superseded` | Newer evidence already fenced this input; safe outbox acknowledgement |
| Empty `400` | Malformed, unsupported version/member/value, scope/bound/time error; no ingest |
| Empty `401` | Wrong service identity/namespace/unauthorized World; no ingest |
| Empty `409` | Same pair, conflicting content; retain first, quarantine conflict and block affected serving pending reconciliation |
| Empty `429` / `503` | Rate limit / disabled, misconfigured or unavailable dependency; no success inference |
| Timeout, bad/truncated response, unrecognized result, TLS/connection loss | Ambiguous receipt; retain outbox and retry same identity/bytes; never manufacture acceptance or a new revision merely to retry |

Transient failures back off 1, 2, 4…60 seconds, reset on acknowledged delivery; watermark retries also respect their own cadence/backoff. `400`, `401`, `409` and local oversize/schema defects alarm and retain evidence/outbox, at most one retry per subject per 300 seconds. Do not drop poison evidence or advance completeness past it. Logs contain family, revision references, result class, counts and timings only. An invalid request never supplies new public truth; a known conflict/gap makes affected output invalid/reconciling.

Proposed testing/preproduction freshness reuses public-family `S=30` seconds and clock uncertainty 1 second. At every serve/cache hit require `Platform_now - complete_through + 1 ≤ 30`, no future watermark beyond uncertainty, highest epoch, valid subject, current privacy evidence and current time provably inside the daily half-open interval. Unsafe clocks deny current-day claims. No stale serving window: family stale/unavailable/invalid/rebuilding means no authoritative daily entries; T88 answers unavailable (503 for its data endpoint), never zero/no raids, and T94 disables dependent predictions. A cached object cannot outlive any of these gates. Unknown or ambiguous state denies disclosure/actions that require current evidence. Production freshness values require owner acceptance and measured evidence; no automatic production enablement.

History is presentation evidence within its stated coverage, not proof that any current boost remains active. Public projections never authorize admission, ownership, rewards or gameplay benefits. Prediction locking/outcome rules require a separate accepted contract before T94 can do more than display this schedule.

## Privacy, retention and rollout

The allowlisted World facts are public only after both Game release policy and Platform's current privacy/presentation floor permit them. Public does not mean internal ingest/provenance is publicly readable. Raw payloads, quarantines, certificates, operational logs, resync/fence controls and configuration are owner-only operational access with audited least privilege; an ordinary account owner cannot inspect another account or service diagnostics. If a future record needs player/account data, this family must not carry it: use a separately accepted owner-only contract with server-side current-owner checks. No personal fields are required here.

Never send AccountId, CharacterId, identity/account linkage, IP/device/login data, tokens, passwords, payment/entitlement details, GameSessionId, lease/session fencing values, ChannelId, node/instance/management endpoints, map coordinates, hidden raid triggers, seeds/draws, spawn rules or private content. Projection/reset epochs are provenance, not gameplay fencing tokens, and are not exposed in public responses. Platform must not join the family to private account projections or infer participants.

Proposed retention: public boost history at most 30 days and 32 completed intervals; public raid slots only for the current reset interval. Keep only the latest accepted raw World snapshot plus one prior candidate for rollback, each at most 30 days; quarantined payloads/diagnostics at most 7 days and minimal operational audit at most 30 days. Purge withdrawn facts from public caches/search immediately under the privacy floor. Keep minimal epoch/revision/content-identity fences and exported-subject inventory for the read model lifetime so expiry cannot admit replay; these are owner-only, without personal data. No unbounded raw event archive. Game outbox persists unacknowledged work until reconciliation, then clears only through the acknowledged pair. Backup retention follows the owning backup policy and restore must reapply current withdrawal/privacy fences before serving.

Proposed switches `world_daily.publish_enabled`, `world_daily.ingest_enabled`, `world_daily.public_enabled`, `world_daily.t94_enabled` are all **false by default**, in addition to existing public-publisher controls. These are proposed names, not existing config claims. Enable only for explicit allowlisted environments/authorities/Worlds after acceptance and fixtures/E2E. Rollback disables serving/ingest or switches to a verified generation recomposed at the current privacy floor; it never lowers epoch/revision fences, resurrects withdrawn history or silently falls back to shared SQL. No command switch exists.

## Versioning and compatibility

This is a proposed additive family of the existing public-projection protocol, with daily `contract_version: 1`. Existing family payloads and responses remain unchanged. Both peers must explicitly support the new operation and watermark enum before activation; an older consumer's `400` must stall this family, not suppress it or enable a fallback. Adding a field, changing its order/type/nullability/bound, changing identity, public release, coverage or response semantics requires a new accepted wire version and coordinated rollout. No ignore-unknown-fields rule, version negotiation downgrade or mixed-version read model. Source revisions, reset epochs, recovery epochs and wire versions are different dimensions.

## Golden proposed payloads

Synthetic keys below illustrate wire syntax, not catalogue membership. In a deployed fixture they must resolve to the pinned Game content. Fenced bodies are exact single-line JSON; the newline before the closing fence is documentation only.

Valid complete snapshot with a current boost, one historical interval and one fixed raid slot:

```json
{"contract_version":1,"operation":"PublishWorldDailyV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"17","source_observed_at":"1790000010","daily":[{"reset_epoch":"42","effective_from":"1790000000","effective_until":"1790086400","creature_key":"rat","boss_key":"ferumbras","history_complete_from":"1789913600","history":[{"reset_epoch":"41","effective_from":"1789913600","effective_until":"1790000000","creature_key":"dragon","boss_key":"orshabaal"}],"raids":[{"raid_key":"raid:badger","scheduled_at":"1790003600","state":"scheduled"}]}]}
```

Valid explicit withdrawal (not a producer outage):

```json
{"contract_version":1,"operation":"PublishWorldDailyV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"18","source_observed_at":"1790000020","daily":[]}
```

Valid watermark, assuming Platform now is at least 1790000022 and every earlier change was acknowledged:

```json
{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_daily","projection_epoch":"2","complete_through":"1790000020","observed_at":"1790000023"}
```

Invalid `400`: numeric revision instead of decimal string and nullable body:

```json
{"contract_version":1,"operation":"PublishWorldDailyV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":19,"source_observed_at":"1790000030","daily":null}
```

Invalid `400`: completeness later than observation; no stored watermark change:

```json
{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_daily","projection_epoch":"2","complete_through":"1790000040","observed_at":"1790000030"}
```

Invalid `409` after accepting the first complete example: exact same pair now withdraws different content. First snapshot remains stored but affected serving is blocked for reconciliation:

```json
{"contract_version":1,"operation":"PublishWorldDailyV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"17","source_observed_at":"1790000010","daily":[]}
```

## Acceptance checklist — Game owner/producer

- [ ] Coordinator accepts this exact proposal revision, public fixed-slot disclosure and the answers below; confirms Game source mapping and content-key bounds.
- [ ] Add daily family/watermark enum under the existing purpose-bound publisher, without changing gameplay query opcodes or granting Platform mutations.
- [ ] Prove committed World/reset interval and durable boost selection mapping; same-day restarts and concurrent Channels do not redraw or duplicate.
- [ ] Prove fixed-slot state mappings against runtime commit evidence; skipped, random and source-scheduled raids cannot become fabricated promises/results.
- [ ] Transactional revision/outbox covers every projected field, withdrawal and coverage change; exact retry, changed-content conflict and ACK races are exercised.
- [ ] Restore external fence, higher-epoch full resync, deleted-World inventory, late transaction watermark and poison/oversize withholding are tested.
- [ ] Operator resync includes the new family and bounded history; retry/timeout/resource limits are measured under maximum World/schedule size.
- [ ] Confirm privacy allowlist, retention, disabled flags and exact producer revision; deliver shared positive/negative golden fixtures.

## Acceptance checklist — Platform consumer

- [ ] Accept exact Game producer revision and this wire profile; retain PROPOSED until coordinator acceptance, without marking G-12 complete from prose alone.
- [ ] Guard identity/authority/World scope, TLS, closed schema, canonical encoding, bounds, unsupported versions and disabled/misconfigured surfaces.
- [ ] Atomic snapshot+watermark/fence storage; duplicate, conflicting pair, superseded pair, cross-family epoch raise and post-restore replay tests pass.
- [ ] Verify completeness, interval edges, history coverage, empty versus unknown, withheld poison row, timeout-after-commit and lost ACK recovery.
- [ ] T88 current/history/cache/API paths and T94 dependencies fail closed on stale or ambiguous evidence; no page-render call to Game.
- [ ] Current privacy floor fences every public cache/search/SSR generation, including rollback and withdrawal; owner-only diagnostics and retention are verified.
- [ ] Baseline/resync/tail rebuild proves all configured Worlds and watermark before switching; flags remain off until exact-revision cross-repository E2E succeeds.
- [ ] Game and Platform independently record acceptance; production activation remains a separate authorized decision. Runtime E2E is not applicable to this docs-only draft and is mandatory for later implementation.

## Open questions for the game owner (coordinator)

1. Accept `world_daily` as one World snapshot family with the proposed route/operation and existing public-projection identity, epoch and watermark? Which exact producer child will own it?
2. What authoritative reset source provides a finite interval end, including extra saves, postponed saves and early resets? If the next end can change, how will the update reach Platform before the old interval becomes false?
3. May fixed raid slots be publicly disclosed before execution? Which content qualifies, and what durable evidence maps skipped slots and actual runtime completion to the proposed public states? Should raids instead be a separately gated family if their source cannot be complete with boosts?
4. Accept excluding random/source-scheduled future triggers, internal firing IDs and detailed gameplay progress? Which separately accepted outcome source, if any, should eventually support T94 prediction closure?
5. Accept 30-day/32-interval boost history with explicit coverage, 256 slots and 131072-byte snapshot bound, including multiple resets per day? Confirm exact content-key grammar and the narrowed 128-byte raid key bound against actual content.
6. Accept the retention and owner-only diagnostic policy, 30-second freshness/1-second uncertainty in testing/preproduction, rate limits and no stale display? Production cadence, maximum transaction bound, durable external fence location and load evidence remain to be named.
7. Accept extending operator-only resync and exported-World withdrawal inventory under the existing public epoch fence, with no Platform → Game command in v1? Identify rollout environments, exact peer revisions and acceptance evidence before G-12 closure.
