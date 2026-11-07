Status: PROPOSED revision 3 by Platform, pending acceptance by the game owner (coordinator).

# World daily public projection family — G-12

## Revision 3 changes

Owner review verdict on revision 2: **CHANGES-NEEDED**, relayed by the coordinator, only on the points below. Every other revision 2 decision is kept unchanged; acceptance and implementation evidence remain pending.

- G-12 is an additive revision of the PUBPROJ public-projection contract family, not a separate feed: same envelope, same external epoch fence `F_pub` and same operator resync as the public projection. It adds one family and its watermark to that family; nothing else in the envelope changes.
- Sequencing: G-12 lands only after the PUBPROJ package now in flight in Game (branch `claude/happy-davinci-ijz3wy-pubproj` at `16aca0cc1`, not merged) merges. Nothing is added to that package. Game delivers G-12 as a separate lane with a new migration range assigned at acceptance.
- Name the Game sources: daily boost mechanic `0351` and raid schedule mechanic `0434` (partial), pinned at Game main `bbb3e06d04875fc1e63f024b853194f7580cc875`.
- Add an explicit per-element readiness table: mechanic exists without a producer, no mechanic, or ready, with evidence for each.

## Revision 2 changes

Owner review verdict: **CHANGES-NEEDED**. This revision applies the coordinator-relayed answers; acceptance and implementation evidence remain pending.

- Move the new daily snapshot and watermark to the G-4 V2 profile; leave the v1 watermark family enum closed and unchanged.
- Allow reset epoch zero and derive interval ends from the next slot in the same calendar revision.
- Publish raids only from committed running onward; exclude future/skipped slots and random triggers. T94 stays blocked without an outcome source.
- Record the owner’s answers on bounds, operations and operator-only resync. G-8 remains the sole online-time source (owner-only intervals, public world totals); G-4 drops its separate `character_activity` family.

## Purpose, ownership and acceptance boundary

Proposal ID: `oteryn-v2-world-daily-family-v1`. This document proposes the producer/consumer boundary for G-12 in `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md` (gate table and §10). T88 (`crates/community/src/today*`) consumes the boosted creature, boosted boss, their bounded history and raids published from running onward. T94 (`crates/minigames/src/league*`, dependent on T89 and T88) stays blocked: no accepted prediction outcome source exists. This feed supplies neither prediction results nor settlement authority. Acceptance alone does not close G-12: compatible implementation and cross-repository evidence are still required.

Game owns committed boost selections, reset boundaries, raid scheduling, public-release decisions, producer revisions, outbox and recovery. Platform owns Registry `WorldId`, normalized disposable read models, freshness, public allowlisting, presentation and privacy restrictions. The coordinator accepts Game semantics and publication policy; Platform accepts its consumer obligations. Neither side may claim the other's acceptance.

Docs only: no runtime, schema, workflow, transport deployment, gameplay mutation or production activation is authorized. Every new operation, field profile, bound and flag below is proposed, not a claim of an existing Game endpoint.

## Contract family and delivery lane

G-12 is an additive revision of the PUBPROJ public-projection contract family ([public projection](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md) and its [G-4 V2 extension](OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md)), not a separate feed. It reuses that family's envelope (`contract_version`, `operation`, `source_authority`, `projection_epoch`, `projection_revision`, `source_observed_at`), its external durable epoch fence `F_pub`, its outbox acknowledgement and watermark rules and its operator resync path. The only additions are the `world_daily` member of the V2 family enum, the `PublishWorldDailyV2` snapshot body and the `world_daily` watermark family. No new transport, identity, signing, fence, resync mechanism or Platform → Game command is introduced.

Sequencing is fixed by the coordinator:

1. The PUBPROJ package now in flight in Game (branch `claude/happy-davinci-ijz3wy-pubproj` at `16aca0cc1eafe4a2dec574c57fd717a32f5e6aeb`, not merged) merges first, under its own acceptance. G-12 neither waits on nor adds anything to that package; its acceptance does not cover G-12.
2. G-12 is delivered afterwards by Game as a separate lane, with its own task, branch and evidence, on top of the merged V2 family.
3. That lane uses a new Game migration range **assigned at acceptance**. This proposal names no migration numbers and Platform does not allocate them.

Platform consumer work for G-12 likewise starts only after the merged V2 family exists and this revision is accepted.

## Existing conventions and pinned evidence

Platform references, read together:

- `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`: source authority, rebuildable public families, independent privacy floor, evidence versus content state, no synchronous Game call during page rendering.
- `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`: stable operation identity, conflicting reuse refusal, durable outcomes and same-identity reconciliation after ambiguity. Its architecture contract deliberately defers transport/signing.
- `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`: authority separation, bounded authority and unknown-state denial; it also defers exact transport. This feed conveys no entitlement or benefit authorization.
- `OTERYN_V2_STORE_WIRE_CONTRACT.md` §§2–4, 9: bounded canonical JSON, source namespace, dedicated mTLS identities, internal listener, digest/idempotency conventions. That document is itself **PROPOSED**; it is not evidence of accepted Store activation. Where its Store-specific rules differ from the accepted public publisher (depth, response vocabulary, oversize status), the public publisher governs this proposal.

Read-only Game evidence is pinned to Game `main` at [bbb3e06d04875fc1e63f024b853194f7580cc875](https://github.com/Oteryn/Oteryn-Game/tree/bbb3e06d04875fc1e63f024b853194f7580cc875), resolved for revision 3 (revision 2 pinned `857f15f376907c592bb28d854fccff18a15cdf87`; every path it cited still exists at the new pin). All following paths are relative to that revision:

| Game path | Evidence used |
|---|---|
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §§3–6, 11 | Accepted public push transport, envelope, epoch fence `F_pub`, outbox acknowledgements, watermark, freshness and operator resync; accepted families do not yet include `world_daily` |
| `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §§3–5 | Underlying HTTP/1.1 mTLS convention, decimal encodings, external epoch fence and late-transaction watermark protection |
| `crates/gameplay/native-admission-source/src/public_projections.rs` | `Family`, `HighscoresWire`, `ProfileWire`, `OnlineWire`, `WatermarkWire`, `PURPOSE = OTERYN_GAME_PUBLIC_PROJECTIONS`, compiled paths and bounded retries; no daily family yet |
| `crates/gameplay/durability/src/durability/public_projections.rs` and `apps/game-server/migrations/0091_public_projections.sql` | Reconciled-authority reads, same-transaction revisions/outboxes, clear only through acknowledged revisions |
| `apps/game-server/migrations/0148_world_reset.sql` and `apps/game-server/migrations/0333_daily_boost.sql` | Reset epochs and daily boost keys admit epoch zero (`CHECK >= 0`); reset history stores `calendar_revision` and activation, not `effective_until` |
| `apps/game-server/migrations/0562_server_save_berlin_06.sql` and `crates/gameplay/durability/src/durability/world_reset/active_epoch.rs` | Revision-bound reset calendar and next-slot calculation at 06:00 Europe/Berlin, including DST; no stored interval-end field |
| `crates/gameplay/dispatch/mechanics/0351-daily_boost.mechanic` | Daily boost mechanic `0351`: registers command 177 with transport, composed/authority methods and capability offer `daily_boost_available`; player-facing read path only |
| `crates/gameplay/dispatch/mechanics/0434-raid_schedule.mechanic` | Raid schedule mechanic `0434`, **partial**: registers only query command 171, with no transport/composed/authority/offer entries; subscribe 172 and notice ACK 173 are declared in the protocol crate but not registered by this fragment |
| `apps/game-server/migrations/0153_raid_schedule.sql` | `game_world_raid_schedules` (metadata, `next_check_at_ms`, `last_fixed_slot_ms`) and immutable `game_world_raid_firings` (`fired_at_ms`, state 1/2/3 = Running/Done/Cancelled, one running per World/raid); raid `reset_epoch` is ≥1; no per-firing fixed-slot or source classification |
| `crates/gameplay/durability/src/durability/daily_boost.rs` | `DailyBoostBinding`, `prepare_daily_boost`, `game_world_daily_boosts`; durable selection per World/reset, restart/concurrent-Channel reuse; reset epoch is not a calendar day |
| `crates/protocol-oteryn/src/daily_boost.rs` | `DailyBoostView`, `reset_epoch`, `creature_key`, `boss_key`; query command 177, capability 151; keys bounded to 255 bytes by codec/durable preparation |
| `crates/gameplay/durability/src/durability/raid_schedule.rs` | Metadata CAS with `RaidWrite::{Applied,Replayed,CompareFailed}`; metadata disposition is not a commit token |
| `crates/gameplay/encounter-rules/src/raid/schedule.rs` and `raid/calendar.rs` | `RaidSchedule`, `FiringState::{Running,Done,Cancelled}`, fixed calendar slots, random/source scheduling; `next_check_at_ms` is not a promised raid time |
| `crates/protocol-oteryn/src/raid_schedule.rs` | `RaidScheduleState` (`raid_key`, `running`, `committed_steps`, `active_creatures`): current per-Channel progress, not a World schedule; query 171, subscribe 172, notice ACK 173 constants, capability 149; read-only gameplay protocol, no command to start/advance/cancel raids |
| `crates/gameplay/content/src/content/production.rs` | Production key ASCII alphabet; public profile below narrows lengths explicitly |

These are source citations, not instructions to modify Game. Existing client binary query payloads are not service-to-service publication payloads. This proposal adds no gameplay opcode or reused Character/Store command with unrelated meaning.

## Readiness per element

Each element of the family is classified at the pinned Game `main` as **ready**, **mechanic exists, no producer** (the gameplay source is committed but nothing projects it into this family) or **no mechanic** (no committed Game source; the element must be withheld or is excluded). "No producer" holds for every family element because the public publisher's `Family` enum at the pin has only `Highscores`, `CharacterProfile`, `WorldOnline` and `WorldTotals` (`crates/gameplay/native-admission-source/src/public_projections.rs`).

| Element (wire member) | Classification | Evidence at `bbb3e06d0487` |
|---|---|---|
| Boosted creature (`creature_key`) | Mechanic exists, no producer | Mechanic `0351`; `game_world_daily_boosts.creature_key` (`0333_daily_boost.sql`, 1..255 bytes); durable per World/reset selection in `durability/daily_boost.rs` |
| Boosted boss (`boss_key`) | Mechanic exists, no producer | Same row, `boss_key` (1..255 bytes); same mechanic `0351` |
| Current interval start (`reset_epoch`, `effective_from`) | Mechanic exists, no producer | `game_world_reset_epochs` with `activated_at` and `calendar_revision` (`0148_world_reset.sql`); boost epoch admits zero (`0333`) |
| Current interval end (`effective_until`) | Mechanic exists, no producer | Not stored; derivable from the revision-bound calendar (`0562_server_save_berlin_06.sql`) and next-slot calculation (`world_reset/active_epoch.rs`); the producer derivation does not exist |
| Boost history (`history`, `history_complete_from`) | Mechanic exists, no producer | One immutable row per (World, reset epoch): primary key `(world_id, reset_epoch)`, runtime granted only `SELECT, INSERT` (`0333`); the coverage floor and the 30-day/32-interval bound are producer-derived, not stored |
| Raid occurrence (`raid_key`, `state`) | Mechanic exists, no producer | `game_world_raid_firings` with state Running/Done/Cancelled, immutable facts and terminal state (`0153_raid_schedule.sql`); mechanic `0434` exposes only per-Channel current progress (query 171) |
| Raid fixed slot of an occurrence (`scheduled_at`) | No mechanic | Firing rows store `fired_at_ms` only; schedule metadata keeps a single `last_fixed_slot_ms` per raid; nothing records per firing whether it came from a fixed slot, a random trigger or a source callback, or which slot. Withhold raid records (and so the daily record) until Game commits that classification |
| Raid schedule of future slots | Mechanic exists (partial), excluded by design | Mechanic `0434` is partial (query 171 only); fixed calendar slots exist in `encounter-rules/src/raid/calendar.rs`, but `next_check_at_ms` is not a promised time; revision 2 already excludes future and skipped slots |
| Raid outcome, participants, rewards, prediction result (T94) | No mechanic, excluded | No accepted outcome source; T94 stays blocked |
| Explicit World withdrawal (`daily: []`) | No mechanic | No committed Game release/withdrawal decision for a World daily view at the pin |
| Content key grammar (`content_key(n)`) | Ready | Production key alphabet in `content/production.rs`; bounds 255 (boost) and 128 (`MAX_RAID_KEY_BYTES`) |
| Envelope, `F_pub` fence, outbox ACK, watermark rules, operator resync | Ready (v1), V2 pending the in-flight merge | Accepted `OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §§3–6, 11, `0091_public_projections.sql`, `durability/public_projections.rs`; the V2 profile G-12 extends is in the unmerged in-flight package |
| `world_daily` watermark family and operator resync coverage | Mechanic exists, no producer | Watermark and resync machinery exist for v1 families only; `world_daily` is in neither |

Not counted as evidence: the unmerged in-flight branch at `16aca0cc1eafe4a2dec574c57fd717a32f5e6aeb` contains a draft world daily capture (`apps/game-server/migrations/1353_public_world_daily.sql`, `crates/gameplay/native-admission-source/src/public_projections/v2/world_daily.rs`), including a per-firing public fixed-slot classification. Under the sequencing above this proposal relies on none of it; whether it leaves the in-flight package or is redelivered in the G-12 lane is the coordinator's decision.

## Flows and transport

| Semantic flow | Physical direction | Scope |
|---|---|---|
| Daily snapshot ingest | Game → Platform, proposed `POST /internal/v2/game-public/world-daily`, `PublishWorldDailyV2` | Complete World snapshot, not delta |
| Liveness evidence | Game → Platform, proposed `POST /internal/v2/game-public/watermark`, `PublishPublicProjectionWatermarkV2` | `family: world_daily` in the V2 family enum defined by [G-4](OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md); no v1 enum extension |
| Ingest acknowledgement | Platform → Game HTTP response on the above exchange | Receipt of projection only; never gameplay success |
| Platform → Game command | **None in this revision** | No boost selection, raid start/cancel/reschedule, resync request, subscription or prediction command |

Rebuild requests go through the existing authorized Game operator process, not a new inbound Game service. Extending `oteryn-game-ops public-projection resync --family` to `world_daily` requires Game acceptance/implementation; it does not exist at the pinned revision.

Reuse the public-projection publisher's HTTP/1.1 over TLS 1.3, existing trusted internal listener and purpose-bound public-projection client certificate; do not create a Store identity or give browsers access to ingest. The listener verifies the certificate chain and configured exact subject, with `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3` and trusted terminator-supplied identity only. Strip spoofable client headers. Namespace and allowed World scope must match configured authority. Missing/overlapping identity, missing trust or ambiguous configuration denies the surface. The certificate cannot authorize account/character/Store routes. Verify Platform server identity through the existing trusted CA/hostname policy; no redirects or plaintext fallback.

Signing: the reused publisher uses mTLS, not application-level JWS/HMAC. Add no signing header, bearer secret, nonce scheme or private key to JSON. SHA-256 content fingerprints are comparison evidence, never signatures or authorization. Preserve existing certificate rotation/revocation policy; inability to authenticate denies publication.

Connect ≤1 s, TLS handshake ≤2 s, whole cold exchange ≤3 s (inside the public contract's 5 s ceiling), `Connection: close`. One daily publication in flight per publisher/family. Snapshot ≤131072 UTF-8 bytes; watermark ≤512 bytes; response ≤256 bytes. `Content-Type: application/json`, no BOM, no trailing newline, no insignificant whitespace or string escapes, exact table member order, maximum JSON nesting depth 5. Reject duplicate, unknown, missing or null members. No transfer encoding; content encoding absent or `identity`; if present Content-Length is canonical nonnegative decimal and within bound. Refuse oversized actual bodies too, without unbounded reads. Malformed and oversized requests return empty `400` everywhere, including over-limit Content-Length and actual bodies.

All responses use `Cache-Control: no-store`. Rate limit by authenticated publisher under the existing public-projection guard; proposed daily allowance is 120 requests/minute, watermark allowance 60/minute, burst no larger than those limits. Return `429`, with positive integral `Retry-After` seconds when supplied; never spin on retry.

## Exact proposed wire schema

Every table is ordered; all fields are required and non-null. Empty arrays represent explicit absence only as defined below. Bounds are inclusive. No free-form text or local numeric database identifiers cross this boundary.

| Alias | Exact type and bounds |
|---|---|
| `u64+` | JSON string matching `[1-9][0-9]{0,19}`, numeric value ≤18446744073709551615; compare as integers, never floating point |
| `u64` | JSON string `0` or `u64+`, unsigned integer ≥0 and ≤18446744073709551615 |
| `seconds` | JSON string `0` or `u64+`, Unix UTC seconds ≤9223372036854775, safely convertible to Game signed milliseconds |
| `uuid7` | Lowercase non-nil UUIDv7, `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `authority` | ASCII `[A-Za-z0-9._:/-]{1,128}`, exact configured namespace |
| `content_key(n)` | ASCII `[A-Za-z0-9:._/-]{1,n}`, exact Game content key, case preserved; no URL, display-name inference or truncation |

Snapshot members:

| Field | Type | Meaning |
|---|---|---|
| `contract_version` | JSON integer `2` | G-4 V2 public-projection wire version |
| `operation` | literal string `PublishWorldDailyV2` | Route-bound operation |
| `source_authority` | `authority` | Game publisher namespace |
| `world_id` | `uuid7` | Registered World, shared across its Channels |
| `projection_epoch` | `u64+` | Existing public-family recovery epoch, not reset epoch |
| `projection_revision` | `u64+` | Monotonic per World daily snapshot revision |
| `source_observed_at` | `seconds` | Time revision assigned; stable on retry |
| `daily` | array of 0 or 1 daily record | `[]` explicitly withdraws the whole World daily public view |

Daily record members:

| Field | Type | Meaning |
|---|---|---|
| `reset_epoch` | `u64` | Current committed World server-save epoch |
| `effective_from` | `seconds` | Inclusive current reset interval start |
| `effective_until` | `seconds` | Derived exclusive end: next reset slot within the same `calendar_revision`; strictly after start |
| `creature_key` | `content_key(255)` | Boosted monster; retains Game's creature terminology |
| `boss_key` | `content_key(255)` | Boosted boss |
| `history_complete_from` | `seconds` | Inclusive source coverage floor for history |
| `history` | array 0..32 of history records | All completed boost intervals within coverage; descending `reset_epoch`, unique |
| `raids` | array 0..256 of raid records | Complete publishable fixed-slot raid records from running onward within current interval; sorted by (`scheduled_at`, `raid_key`), unique pair |

`effective_until` is a producer-derived value, not a Game column: calculate the next reset slot after the interval start using that reset's authoritative calendar and the same `calendar_revision`. Apply the same rule to historical intervals. Do not assume a fixed 86400-second day, infer the end from a different calendar revision or substitute the next observed boost selection. The pinned calendar is 06:00 Europe/Berlin and must account for DST. The calendar revision remains producer-side derivation evidence, not an extra wire member. A missing/ambiguous next slot withholds publication and completeness. Calendar changes must transactionally revise the projected interval before affected claims are served.

History record members, in order: `reset_epoch` (`u64`), `effective_from` (`seconds`), `effective_until` (`seconds`), `creature_key` (`content_key(255)`), `boss_key` (`content_key(255)`). Each epoch is less than current; each interval has start < end ≤ current start; intervals do not overlap and starts are within `[history_complete_from,effective_from)`. The coverage floor is ≤ current start and no earlier than current start minus 2592000 seconds (30 days, clamped to zero). If there are more than 32 intervals in 30 days, raise the floor to the earliest fully retained interval; never claim complete history after silently truncating it. Empty history with floor equal to current start explicitly claims no older coverage. Epoch gaps are permitted; epochs are not days. Coverage contains every selection interval beginning within it, including multiple resets on one date. Missing older history is outside coverage, not proof of no boost.

Raid record members:

| Field | Type | Meaning |
|---|---|---|
| `raid_key` | `content_key(128)` | Game raid definition key; narrowed to existing client raid key bound |
| `scheduled_at` | `seconds` | Fixed calendar slot in `[effective_from,effective_until)`, no later than `source_observed_at` |
| `state` | enum string `running`, `done`, `cancelled` | Public scheduling state only |

Only Game-approved fixed-slot raids with committed runtime evidence appear, starting at `Running`, mapped to `running`; `Done` maps to `done` and `Cancelled` to `cancelled`. Game has no `scheduled` state or source for skipped slots. This first daily profile exposes no future slots; time passing, metadata `Applied` or slot consumption never fabricates a raid or cancellation. No random trigger, source-scheduled future trigger or `next_check_at_ms` becomes a predicted firing time. Unsupported/ambiguous source classification withholds the family watermark rather than inventing state. Subsecond fixed slots cannot be rounded: refuse until an accepted representation exists. `raids: []` means no publishable raids in this interval, not no possible random raids. There is no running-creature/player count, participant list, reward, score or prediction outcome.

A daily record requires both committed boost keys and a fully classified set of publishable raid records. Unknown boosts, uncertain interval end or an over-bound schedule must not become `daily: []` or `raids: []`. Hold publication and freshness instead. `daily: []` is reserved for an explicit committed withdrawal/no-public-view decision and removes current facts and history from public serving. Static catalogue names/artwork resolve through separately accepted content snapshots; an unresolved key is shown as unavailable, never guessed.

Watermark members in order: `contract_version` (integer `2`), `operation` (literal `PublishPublicProjectionWatermarkV2`), `source_authority` (`authority`), `family` (literal `world_daily`), `projection_epoch` (`u64+`), `complete_through` (`seconds`), `observed_at` (`seconds`). Require `complete_through ≤ observed_at`; neither may exceed Platform now +1 second. No other member or null is valid.

Success responses are exactly one of:

```json
{"contract_version":2,"result":"accepted"}
```

```json
{"contract_version":2,"result":"superseded"}
```

## Identifiers, ordering, outbox and replay

Subject key is (`source_authority`, `world_daily`, `world_id`). Idempotency key is that subject plus (`projection_epoch`, `projection_revision`); no independent HTTP idempotency header or random event ID. Content is canonical `world_id` plus `daily` in that order. Internally a SHA-256 fingerprint of those exact canonical object bytes may accelerate comparisons; retain exact content for conflict verification. Envelope and `source_observed_at` are not content, following the public contract. Producers retry identical bytes; an equal pair with equal content is `accepted` and retains the first timestamp, even if a defective retry changes only the timestamp. An equal pair with changed content returns `409` and preserves the first snapshot.

Game transactionally advances the World revision and records its outbox touch with every projected change: reset activation, calendar/derived interval change, boost selection, history coverage change, publishable raid/status change or withdrawal. A publisher reads one committed consistent snapshot under reconciled authority. Acknowledgement follows Platform's durable atomic content+revision commit. Only `accepted` or `superseded` clears Game outbox rows through the sent pair; concurrent newer rows remain. Crash before/after acknowledgement therefore safely repeats delivery. No projection failure rolls back a gameplay change.

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

History is presentation evidence within its stated coverage, not proof that any current boost remains active. Public projections never authorize admission, ownership, rewards or gameplay benefits. T94 stays blocked: this family provides no prediction outcome source. Prediction locking and closure require a separately accepted authoritative outcome contract.

## Privacy, retention and rollout

The allowlisted World facts are public only after both Game release policy and Platform's current privacy/presentation floor permit them. Public does not mean internal ingest/provenance is publicly readable. Raw payloads, quarantines, certificates, operational logs, resync/fence controls and configuration are owner-only operational access with audited least privilege; an ordinary account owner cannot inspect another account or service diagnostics. If a future record needs player/account data, this family must not carry it: use a separately accepted owner-only contract with server-side current-owner checks. No personal fields are required here.

Never send AccountId, CharacterId, identity/account linkage, IP/device/login data, tokens, passwords, payment/entitlement details, GameSessionId, lease/session fencing values, ChannelId, node/instance/management endpoints, map coordinates, hidden raid triggers, seeds/draws, spawn rules or private content. Projection/reset epochs are provenance, not gameplay fencing tokens, and are not exposed in public responses. Platform must not join the family to private account projections or infer participants.

Proposed retention: public boost history at most 30 days and 32 completed intervals; public raid slots only for the current reset interval. Keep only the latest accepted raw World snapshot plus one prior candidate for rollback, each at most 30 days; quarantined payloads/diagnostics at most 7 days and minimal operational audit at most 30 days. Purge withdrawn facts from public caches/search immediately under the privacy floor. Keep minimal epoch/revision/content-identity fences and exported-subject inventory for the read model lifetime so expiry cannot admit replay; these are owner-only, without personal data. No unbounded raw event archive. Game outbox persists unacknowledged work until reconciliation, then clears only through the acknowledged pair. Backup retention follows the owning backup policy and restore must reapply current withdrawal/privacy fences before serving.

Proposed switches `world_daily.publish_enabled`, `world_daily.ingest_enabled`, `world_daily.public_enabled`, `world_daily.t94_enabled` are all **false by default**, in addition to existing public-publisher controls. These are proposed names, not existing config claims. Enable only for explicit allowlisted environments/authorities/Worlds after acceptance and fixtures/E2E. Rollback disables serving/ingest or switches to a verified generation recomposed at the current privacy floor; it never lowers epoch/revision fences, resurrects withdrawn history or silently falls back to shared SQL. No command switch exists.

## Versioning and compatibility

This is a proposed additive revision of the PUBPROJ family `oteryn-game-public-projections-v2` (same envelope, `F_pub` fence and operator resync), with `contract_version: 2`, `PublishWorldDailyV2` at `/internal/v2/game-public/world-daily` and `PublishPublicProjectionWatermarkV2` at `/internal/v2/game-public/watermark`. The V2 family enum is defined in [G-4](OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md) and includes `world_daily`; the v1 watermark enum stays closed and unchanged. Both peers must explicitly support the new V2 operation and family before activation; an older consumer's `400` stalls this family. Existing v1 payloads and responses remain unchanged, with no downgrade or redirect. G-8 supplies the single source of online time: owner-only online intervals and public world totals. G-4 drops its own `character_activity` family; this daily family supplies no online time. Adding a field, changing its order/type/nullability/bound, changing identity, public release, coverage or response semantics requires a new accepted wire version and coordinated rollout. No ignore-unknown-fields rule or mixed-version read model. Source revisions, reset epochs, recovery epochs and wire versions are different dimensions.

## Golden proposed payloads

Synthetic keys below illustrate wire syntax, not catalogue membership. In a deployed fixture they must resolve to the pinned Game content. Fenced bodies are exact single-line JSON; the newline before the closing fence is documentation only.

Valid complete snapshot with a current boost, one historical interval and one committed running fixed-slot raid (observation is after its slot):

```json
{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"17","source_observed_at":"1790003610","daily":[{"reset_epoch":"42","effective_from":"1790000000","effective_until":"1790086400","creature_key":"rat","boss_key":"ferumbras","history_complete_from":"1789913600","history":[{"reset_epoch":"41","effective_from":"1789913600","effective_until":"1790000000","creature_key":"dragon","boss_key":"orshabaal"}],"raids":[{"raid_key":"raid:badger","scheduled_at":"1790003600","state":"running"}]}]}
```

Valid explicit withdrawal (not a producer outage):

```json
{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"18","source_observed_at":"1790003620","daily":[]}
```

Valid watermark, assuming Platform now is at least 1790003622 and every earlier change was acknowledged:

```json
{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"world_daily","projection_epoch":"2","complete_through":"1790003620","observed_at":"1790003623"}
```

Invalid `400`: numeric revision instead of decimal string and nullable body:

```json
{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":19,"source_observed_at":"1790003630","daily":null}
```

Invalid `400`: completeness later than observation; no stored watermark change:

```json
{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"world_daily","projection_epoch":"2","complete_through":"1790003640","observed_at":"1790003630"}
```

Invalid `409` after accepting the first complete example: exact same pair now withdraws different content. First snapshot remains stored but affected serving is blocked for reconciliation:

```json
{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"2","projection_revision":"17","source_observed_at":"1790003610","daily":[]}
```

## Acceptance checklist — Game owner/producer

- [ ] Coordinator accepts this exact proposal revision, disclosure only from committed running onward and the answers below; confirms Game source mapping and content-key bounds.
- [ ] Deliver G-12 as a separate lane after the in-flight PUBPROJ package merges, adding nothing to that package; assign the lane's migration range at acceptance.
- [ ] Close every **no mechanic** row of the readiness table that the accepted profile needs (at minimum per-firing fixed-slot classification and the World withdrawal decision), or keep the affected records withheld.
- [ ] Add V2 daily family and watermark using the G-4 V2 family enum under the existing purpose-bound publisher; preserve the closed v1 enum, without changing gameplay query opcodes or granting Platform mutations.
- [ ] Prove next-slot derivation within the same calendar revision, committed World/reset interval and durable boost selection mapping (including epoch zero); same-day restarts and concurrent Channels do not redraw or duplicate.
- [ ] Prove fixed-slot state mappings against runtime commit evidence; skipped, random and source-scheduled raids cannot become fabricated promises/results.
- [ ] Transactional revision/outbox covers every projected field, withdrawal and coverage change; exact retry, changed-content conflict and ACK races are exercised.
- [ ] Restore external fence, higher-epoch full resync, deleted-World inventory, late transaction watermark and poison/oversize withholding are tested.
- [ ] Operator resync includes the new family and bounded history; retry/timeout/resource limits are measured under maximum World/schedule size.
- [ ] Confirm privacy allowlist, retention, disabled flags and exact producer revision; deliver shared positive/negative golden fixtures.

## Acceptance checklist — Platform consumer

- [ ] Accept exact Game producer revision and this wire profile; retain PROPOSED until coordinator acceptance, without marking G-12 complete from prose alone.
- [ ] Start consumer work only after the V2 PUBPROJ family has merged in Game and this revision is accepted; reuse the PUBPROJ envelope, fence and resync handling rather than a separate feed.
- [ ] Guard identity/authority/World scope, TLS, closed schema, canonical encoding, bounds, unsupported versions and disabled/misconfigured surfaces.
- [ ] Atomic snapshot+watermark/fence storage; duplicate, conflicting pair, superseded pair, cross-family epoch raise and post-restore replay tests pass.
- [ ] Verify completeness, interval edges, history coverage, empty versus unknown, withheld poison row, timeout-after-commit and lost ACK recovery.
- [ ] T88 current/history/cache/API paths fail closed and T94 stays blocked without an outcome source; dependent paths fail closed on stale or ambiguous evidence; no page-render call to Game.
- [ ] Current privacy floor fences every public cache/search/SSR generation, including rollback and withdrawal; owner-only diagnostics and retention are verified.
- [ ] Baseline/resync/tail rebuild proves all configured Worlds and watermark before switching; flags remain off until exact-revision cross-repository E2E succeeds.
- [ ] Game and Platform independently record acceptance; production activation remains a separate authorized decision. Runtime E2E is not applicable to this docs-only draft and is mandatory for later implementation.

## Open questions for the game owner (coordinator)

Closed questions are recorded below as answered; they no longer request an owner decision. Implementation qualifications in the acceptance checklists remain open and do not imply acceptance of this revision.

1. **Answered — V2 compatibility:** use `/internal/v2/game-public/*` and the V2 family enum defined in G-4, as for G-8. The v1 watermark enum remains closed and unchanged. Exact producer allocation/revision remains implementation evidence for acceptance.
2. **Answered — interval end and epoch:** `reset_epoch` is an unsigned integer ≥0. Game does not store `effective_until`; derive it from the next reset slot within the same `calendar_revision`, never across revisions. If that next slot cannot be established, withhold publication/freshness. Calendar changes require a new consistent snapshot/revision before affected interval claims are served.
3. **Answered — raid disclosure:** Game has only `Running`, `Done` and `Cancelled`. Publish fixed-slot raids from running onward; expose no future or skipped slots and invent no cancellation for a skipped slot.
4. **Answered — exclusions and T94:** no random triggers. Internal firing IDs and detailed gameplay progress remain excluded. There is no T94 outcome source; T94 stays blocked pending a separately accepted authoritative source.
5. **Answered — yes:** accept 30-day/32-interval boost history with explicit coverage, 256 raid records and the 131072-byte snapshot bound, including multiple resets per day, and the proposed content-key grammar/128-byte raid key bound.
6. **Answered — yes:** accept retention, owner-only diagnostics, testing/preproduction freshness of 30 seconds with 1-second uncertainty, rate limits and no stale display. Production cadence, transaction bound, external fence location and measured load evidence remain implementation/activation qualifications.
7. **Answered — operator resync only:** rebuild through the authorized Game operator process, with exported-World withdrawal inventory under the existing public epoch fence. No Platform → Game resync command. Rollout environments, exact peer revisions and acceptance evidence are still required before G-12 closure.
8. **Answered — family and sequencing:** G-12 is an additive revision of the PUBPROJ family (same envelope, fence and resync), lands only after the in-flight PUBPROJ package merges, adds nothing to that package, and is delivered by Game as a separate lane with a migration range assigned at acceptance.
