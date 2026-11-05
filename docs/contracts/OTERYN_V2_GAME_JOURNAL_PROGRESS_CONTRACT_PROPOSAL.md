Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# G-11 private game journal: progress and events

## Revision 2 (review of 2026-11-05)

- Re-pinned all Game references to main `2253b997`; retained paths are unverified at this pin.
- Clarified reuse as aligned with the proposed Store wire, pending acceptance.
- Identified the private journal outbox/emitter as NEW game-side deliverables; existing `store/outbox.rs` is Store-only.
- Replaced the ownership revision field with receipt `projection_revision` from §4.3, with verified skull/premium source-revision exceptions.
- Made death recap actions UNAVAILABLE in v1; deferred capture to Future versions.
- Applied owner verification at `2253b997`: skull is producible; stamina and premium are partial, with corrected revisions, ordering and units.
- Kept house_rent pending game-side work and reduced game_login to four fields for a NEW emitter; removed country/device data.

## 1. Purpose, owners and acceptance boundary

Proposal identifier: `oteryn-v2-game-journal-progress-v1`, wire `contract_version: 1`.
This is the Platform proposal for G-11 in the gate table of
[`OTERYN_PORTAL_RUST_PLAN.md`](../architecture/OTERYN_PORTAL_RUST_PLAN.md),
including its §10 consumers. It neither closes G-11 nor authorizes implementation,
game schema changes, deployment or activation. The game owner/coordinator must
accept the producer mapping, wire and unresolved questions before either side enables it.

Game owns gameplay facts, committed occurrence identities, ordering and recovery
fences. Platform owns private `playerdata` projections, authenticated access,
retention, goals, presentation and reminders. ProductsEntitlements remains the
commercial premium authority. Identity remains the account/security authority.
The coordinator owns cross-repository acceptance and schedules missing producers.

| Consumer | Required facts / responsibility |
|---|---|
| T72 | Private ingest, deduplication, watermarks, replay, ownership boundary, retention and flags for every family below |
| T77 | Bestiary, Bosstiary/cooldowns, quest progress, achievement grants/progress, skill history; goals/forecasts are Platform-derived |
| T78 | Death recap, losses/blessings, PvP kills/assists and durable skull state |
| T79 | Dashboard and reminders: stamina, premium end, boss availability and house rent |
| T80 | Game login history; security action orchestration remains in Identity/session contracts |

G-9 session telemetry and G-10 inventory/trade history are separate gates. A death's
bounded loss list is included here; a general inventory feed is not. Bazaar deadlines
in T79 come from Bazaar. This proposal grants no public progress API or sharing link.

## 2. Flows and existing contract conventions

| Logical direction | Flow | Authority / physical direction |
|---|---|---|
| Game → Platform ingest | All twelve families in §5, baseline, removals, liveness watermark | Game calls Platform; durable private projection only |
| Platform → Game command | **None newly introduced by G-11**: no quest completion, achievement grant, skull clearing, stamina refill, rent payment or cooldown reset | Any future mutation requires its owning accepted command contract |
| Platform → Game existing command | Ownership changes use `TransferCharacterOwnership` / `PLATFORM_BAZAAR_TRANSFER` through Character Authority | Game polls existing pending/read routes, then publishes its durable receipt; journal is never the command bus |
| Platform → Game security intent | T80 “not me” invokes the accepted Identity/session revocation flow only when supported | Password change is Platform-only; G-11 login records cannot revoke a session. Without proven game-wide revocation support, report action unavailable/pending, never “logged out everywhere” |
| Platform entitlement → Game | Existing premium snapshot/Profile-B exchange | Journal `premium_end` is an observation, never a grant or authority lease |

Read and reused Platform contracts:

- [`OTERYN_V2_STORE_WIRE_CONTRACT.md`](OTERYN_V2_STORE_WIRE_CONTRACT.md): internal
  mTLS guard, closed canonical JSON, decimal counters, digest/conflicting-retry rules,
  empty failures and default-off rollout. **That wire is itself PROPOSED**, not an
  accepted transport mandate for this gate.
- [`OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md):
  rebuildable projections, stable source identity, source ordering, tombstones,
  truthful evidence states and independent privacy freshness. Its accepted semantic
  boundary explicitly defers producer transport; this private feed is separate.
- [`OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md):
  current ownership revalidation, one semantic operation identity, terminal refusal
  versus ambiguous outcome, and reconciliation without blind replacement commands.
- [`OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md):
  commercial truth versus game observation, at-least-once effects, and finite
  Profile-B authority. It explicitly defers cryptographic transport selection.

The concrete proposal reuses Game's existing `http1_mtls::exchange`,
`ProducerDescriptor` purpose separation and public projection patterns (unverified
references in §12). The private journal outbox/emitter is NEW game-side work (§6.1).
This proposal avoids introducing a broker, shared SQL, bearer token or signing scheme.
TLS 1.3 authenticates both peers and signs the handshake through existing trust roots.
There is **no new detached payload signature/HMAC**, key distribution or browser token.
A SHA-256 binding is an integrity/idempotency digest, not a signature. Existing
signed security/entitlement evidence stays on its existing surface unchanged.
Acceptance must confirm the new purpose/descriptor binding; no existing producer
certificate may be repurposed as a private-journal identity.

## 3. Transport, authentication and closed messages

Proposed POST routes on the existing Platform internal listener:

| Path | `operation` | Maximum request bytes |
|---|---|---:|
| `/internal/v1/playerdata/progress/record` | `PublishGameJournalProgressV1` | 262144 |
| `/internal/v1/playerdata/progress/watermark` | `PublishGameJournalProgressWatermarkV1` | 1024 |

Every response is `Cache-Control: no-store`; successful bodies are at most 256 bytes.
Aligned with the proposed Store wire, the proposed peer guard order is: disabled/misconfigured → `503`;
`SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3` and exact constant-time subject
comparison → otherwise `401`; per-peer limiter → `429` with `Retry-After` ≥1 second;
then transport, source binding and message checks. TLS variables are supplied only
by the trusted terminator, never request headers. Server certificate chain and
configured server name must validate in Game's existing descriptor verifier.

Proposed dedicated purpose `OTERYN_GAME_JOURNAL_PROGRESS` binds precisely these two
routes, configured `source_authority`, peer identity and permitted account partition.
Identity is printable ASCII 1–128 bytes and distinct from Store, commands, public
projection, premium, restore and native evidence identities. Missing/overlapping
identity, authority or route binding disables the surface. Limits: 600 requests per
minute per peer, 60 per minute per account, 32 in-flight exchanges per peer; excess
is `429`, no watermark change. These numbers require coordinator acceptance.

Canonical encoding is aligned with the proposed Store wire §2: one UTF-8 JSON object, exact listed member
order, no BOM/newline/whitespace/backslash escapes, no extra/missing/duplicate members,
no floats, `Content-Type: application/json`. All strings below are restricted ASCII.
Maximum nesting depth is **8** for the bounded payload arrays (an explicit proposed
extension of the proposed Store wire's depth 4). `Transfer-Encoding` absent;
`Content-Encoding` absent or `identity`; `Content-Length`, if present, is canonical
unsigned decimal with at most 10 digits and within the route cap. Oversize → `413`;
other encoding violations → `400`. Reject an entire record; never truncate arrays.

All fields are required and non-null unless explicitly `?` below; `?` means the
member must be present with either its type or JSON `null`. Tables/list order fixes
canonical member order, including nested objects. Bounds are inclusive.

| Type | Exact wire representation |
|---|---|
| `id` | Lowercase non-nil UUIDv7, `xxxxxxxx-xxxx-7xxx-[89ab]xxx-xxxxxxxxxxxx`, each x hex; canonical AccountId, CharacterId, WorldId or occurrence according to field |
| `u64` | Decimal string `[1-9][0-9]{0,19}`, numeric value ≤18446744073709551615 |
| `n64` | `"0"` or `u64`; counts/revisions only as specified |
| `amount` | Decimal string `0` or non-leading-zero digits, 0–9223372036854775807 |
| `i64` | Decimal string `0` or optional minus and non-leading-zero digits, −9223372036854775808…9223372036854775807; no `-0` |
| `u32` | JSON integer 0–4294967295 |
| `key` | ASCII `[A-Za-z0-9][A-Za-z0-9._:/-]{0,511}`, ≤512 bytes; exact Game catalog/track key, never a display name |
| `revision` | ASCII `[A-Za-z0-9][A-Za-z0-9._:/+-]{0,127}`, ≤128 bytes |
| `authority` | ASCII `[A-Za-z0-9._:/-]{1,128}`, equals configured source |
| `hex64` | 64 lowercase hexadecimal characters (SHA-256) |
| `time_us` | `n64` Unix **microseconds**, range 0–253402300799999999; premium payload only, not ordering authority |
| `time` | `n64` Unix **milliseconds**, range 0–253402300799999 (through year 9999); not ordering authority |
| `bool` | JSON `true` or `false` |

Except for the explicitly microsecond premium payload, milliseconds preserve existing boss `available_at_unix_ms` naming. Fields with
`_unix_ms` explicitly differ from command/Store Unix-second fields. Convert source
microseconds conservatively: floor occurrence times, ceil future expiry deadlines;
never reinterpret units. A timer observation never grants eligibility.

## 4. Record envelope, watermark and acknowledgement

One record is one logical entity replacement/removal or one immutable historical
event. The envelope members, in order:

| Field | Type / invariant |
|---|---|
| `contract_version` | JSON integer `1` |
| `operation` | literal `PublishGameJournalProgressV1` |
| `source_authority` | `authority` |
| `account_id` | `id`, private recipient account fixed when the fact is committed |
| `character_id` | `id?`; null only for account achievement grants, premium end or account login records as specified below |
| `world_id` | `id?`; required for character-scoped facts, null for account-only facts |
| `projection_revision` | `u64?`; ownership publication `projection_revision` taken from the §4.3 receipts of `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` (unverified reference in §12), required for character records except the family-specific source mapping in §5.8; §5.10 instead requires account authority_revision; not CharacterRevision or journal source_revision |
| `projection_epoch` | `u64`, producer-fenced generation shared with the accepted account ownership projection |
| `recovery_generation` | `u64`, current Game restore fence |
| `family` | exact enum from §5 |
| `source_revision` | `u64`, contiguous private stream sequence per (authority, account, epoch, family), **not** CharacterRevision |
| `record_id` | `id`, minted once in the source transaction/outbox; never regenerated on retry |
| `entity_key` | `key`, exact entity identity rule in §5 |
| `character_revision` | `u64?`, Game CharacterRevision of observation/commit; null for account-only records |
| `occurred_at_unix_ms` | `time`, Game source observation/commit time |
| `record_kind` | `UPSERT` or `REMOVE` |
| `payload` | exact family object for UPSERT; null for REMOVE |

REMOVE targets the identical family/entity/subject and removes its live presentation,
not source audit truth. It may remove a historical event for accepted correction or
privacy erasure. Reinstating an event requires a later source revision, explicit
Game correction and the same event identity. It never repeats gameplay effects.
`character_id`, `world_id` and `character_revision` are null for account-only
records and non-null for character records. `projection_revision` is null for
account-only records except premium, which requires authority_revision. Skull uses
the table revision instead of an ownership receipt revision; neither it nor premium
proves ownership. Independent current ownership revalidation remains mandatory. Login may use either form.
A removal obeys the same scope as the removed record.

Watermark members, in order: `contract_version` (1), `operation`
(`PublishGameJournalProgressWatermarkV1`), `source_authority` (`authority`),
`account_id` (`id`), `projection_epoch` (`u64`), `recovery_generation` (`u64`),
`family` (§5 enum), `complete_through` (`n64`), `observed_at_unix_ms` (`time`),
`baseline_complete` (`bool`). It asserts a complete source scan through that
revision, including all pending source writes. `"0"` means a proven empty baseline
only when `baseline_complete=true`; missing/failed scans never imply empty.

Record 200 answer has exactly `contract_version` (1), `result` (`accepted`),
`record_id` (echo id), `accepted_source_revision` (echo u64). Watermark 200 answer has
exactly `contract_version` (1), `result` (`accepted`), `complete_through` (echo n64).
A durable acceptance does not promise the stream is complete or user-visible yet.
Exact duplicate record bytes return identical answer bytes and write nothing.

## 5. Closed family payload schemas

Notation: `array[0..N]<T>` is a JSON array with that count bound; ordered sets use
ascending ASCII byte order with no duplicates. Object fields below appear in the
listed order. No free-form text, display names or arbitrary metadata are accepted.
All proposed export types are adapters over Game owners, **not claims that these
journal wire types already exist**. Catalog keys/revisions must resolve against the
corresponding pinned Game catalog; unknown mappings remain unavailable.

### 5.1 `bestiary` (T77)

Character scope. Entity `race_key`.
Payload: `race_key` (`key`), `race_definition_revision` (`revision`), `kill_count`
(`u32`), `final_kill_threshold` (`u32`, ≥1), `charm_points_awarded` (`u32`).
Use committed absolute counts, not increments. A lowered catalog threshold may leave
kill_count above threshold; retain Game's value. Do not derive an extra charm grant.
Definition thresholds, class/difficulty/stars come from Catalog, outside this feed.

### 5.2 `boss_cooldown` (T77, T79)

Character scope. Entity `boss_key`.
Payload: `boss_key` (`key`, ≤512 bytes), `definition_revision` (`revision`),
`kill_count` (`n64`, Bosstiary absolute count), `available_at_unix_ms` (`time?`),
`last_death_occurrence_id` (`id?`). Null availability means unknown, never ready;
non-null is Game's current cooldown deadline (past is an observation of elapsed time).
Null death reference means no linked retained kill receipt. Bosstiary points/tiers
are catalog/ruleset interpretations, not invented fields. Export all supported bosses,
including an explicit zero count; an omitted boss means unknown until baseline proof.
No spawn-today, room occupancy or encounter secrets are included.

### 5.3 `quest` (T77)

Character scope. Entity `quest_key`.
Payload: `quest_key` (`key`), `pinned_content_revision` (`revision`), `definition_hash`
(`hex64`), `completed` (`bool`), `tracks` (`array[0..256]<Track>`).
Track: `track_key` (`key`), `value` (`i64`); sorted unique by track_key.
This exports Game's named tracks and completion, not a fabricated numeric quest stage.
Only owner-visible journal tracks are allowed; internal callback flags, future hidden
stages, reward obligations and scripts are never exported. A quest exceeding the
bound is unavailable until a mutually accepted larger/paged version exists.

### 5.4 `achievement` (T77)

Account scope for a granted account achievement; character scope for counted
progress. Entity `achievement_key` within that subject scope.
Payload: `achievement_key` (`key`, additionally
`^oteryn:achievement/[a-z0-9]+(_[a-z0-9]+)*$`, ≤160 bytes),
`definition_revision` (`revision`), `progress` (JSON integer 0–100000, nullable),
`target` (JSON integer 1–100000, nullable), `granted_at_unix_ms` (`time?`).
Account form: progress/target null, granted_at non-null. Character form:
progress and target non-null, progress ≤target, granted_at null. An account grant
must not be presented as originating from a sold character. Counted character
progress must not be used to infer a missing account grant. Hidden/unearned secret
achievement definitions are not sent.

### 5.5 `skill_history` (T77)

Character scope. Entity `build_occurrence_id` (UUID text is also a key).
Payload: `build_occurrence_id` (`id`, source build/death occurrence), `cause`
(`training|vocation_choice|promotion|offline_settlement|logout|death`),
`policy_revision` (`revision`), `skills` (`array[1..8]<SkillChange>`).
SkillChange: `skill` (`magic|fist|club|sword|axe|distance|shielding|fishing`),
`level_before` (JSON integer: magic 0–1000, others 10–1000), `progress_before`
(`amount`), `level_after` (same bounds), `progress_after` (`amount`).
Sorted unique by skill. Progress is mana_spent for magic, tries for other skills;
these are Game counters, not percentages. Include changed skills only. `death` uses
the death receipt's build fields. No inferred history before retained receipts;
baseline may contain zero historical events even when current skills are nonzero.

### 5.6 `death_recap` (T78)

Character scope. Entity `death_occurrence_id`.
Payload fields in order:

| Field | Type / rule |
|---|---|
| `death_occurrence_id` | `id`, exact PlayerDeathOccurrence |
| `level_before`, `level_after` | JSON integers 1–4294967295; after ≤before |
| `experience_lost` | `amount`, committed loss, never estimated |
| `blessings_before`, `blessings_after` | each ordered set array[0..32] of key restricted to ≤128 bytes; after subset of before |
| `lost_items` | array[0..256] of Loss, sorted unique by loss_index |
| `recap_state` | literal `UNAVAILABLE` in v1 |
| `window_ms` | JSON integer `0` in v1 |
| `actions` | empty array `[]` in v1; populated actions are rejected |

Loss fields: `loss_index` (JSON integer 0–255), `item_key` (`key`), `quantity`
(JSON integer 1–65535). Export item type/count only; no item-instance identifiers,
container tree, inscriptions or trading provenance. Losses must be complete; over
bound → no record accepted. Blessing effect is the committed before/after set;
unknown calculation details cannot be presented as a verified percentage reduction.

Death action capture is UNAVAILABLE in v1. Proven receipt losses/blessings remain
available; never invent last-seconds actions from a final killer or death receipt.
See Future versions for separately accepted action capture.

### 5.7 `pvp` (T78)

Character scope. Entity `death_occurrence_id`.
Payload: `death_occurrence_id` (`id`), `role` (`KILL|ASSIST|VICTIM`),
`counterpart_character_id` (`id?`, null if redacted/unknown), `justification`
(`JUSTIFIED|UNJUSTIFIED|UNKNOWN`), `points_milli` (JSON integer 0–65535, nullable).
Points are Game's unjustified-point units (1000 = one point), not inferred kill
counts. UNKNOWN requires null points; other values require non-null points, with
JUSTIFIED =0. One subject/occurrence has one role, assigned by Game, never by Platform
ranking of recap damage. Counterpart is victim for KILL/ASSIST and final credited
killer for VICTIM, null if that singular identity cannot safely be established.

### 5.8 `skull` (T78)

**Producible**, verified by the game owner at `2253b997`.
Source: `durability/pvp_state.rs`, table `game_character_pvp_state`, and
`pvp_skull_storage.rs`. Table skull values 0/1/2 map to NONE/RED/BLACK;
`skull_until` and `kill_block_until` supply the deadlines, converted to wire
milliseconds. Envelope `projection_revision` is the table's `revision` column.

Character scope. Entity literal `skull`.
Payload: `skull` (`NONE|RED|BLACK`), `skull_until_unix_ms` (`time?`),
`kill_block_until_unix_ms` (`time?`). NONE requires null skull_until; RED/BLACK
require a non-null deadline. Game owns expiry/normalization; an elapsed deadline
must not fabricate a cleared source state. The private emitter remains NEW work.

### 5.9 `stamina` (T79)

**Partial**, verified by the game owner at `2253b997`.
Source: `hunting_stamina.rs`, `commit_hunting_stamina` / `commit_resting_stamina`,
with stamina before/after in `game_character_build_receipts`.

Character scope. Entity literal `stamina`.
Payload, in order: `stamina_minutes` (JSON integer 0–2520, committed after value),
`content_revision` (`u64`), `build_occurrence_id` (`id`, receipt occurrence).
Record ordering is `content_revision` then `build_occurrence_id`; the NEW emitter
must preserve that order when allocating the envelope's contiguous source_revision.
The maximum is the constant **2520 minutes**, not a source or payload field.
The game does not store a full-time prediction; no such field is exported in v1.
No Platform tick may replenish authoritative stamina. Any higher supported maximum
requires a version change, not silent clamping.

### 5.10 `premium_end` (T79)

**Partial**, verified by the game owner at `2253b997`.
Source: `premium_fence.rs`, account-scoped `game_premium_evidence`, with monotonic
`authority_revision`. Envelope `projection_revision` equals `authority_revision`.

Account scope. Entity literal `premium`.
Payload, in order: `observed_state`
(`ACTIVE|NOT_YET_EFFECTIVE|EXPIRED|REVOKED|NONE`), `effective_until_unix_us`
(`time_us?`), `authority_revision` (`u64`). No UNKNOWN state is accepted.
Times in this payload are Unix **microseconds**, preserved from the source;
null end means no evidenced deadline, never an inferred entitlement.
The exact state/deadline mapping requires acceptance when implementing the NEW emitter.
This is Game's observation of Platform premium evidence. Platform reminders must
recheck its own ProductsEntitlements source, including commercial start/end and
revocation; disagreement suppresses the reminder and raises reconciliation.
This record carries no usable entitlement lease and cannot activate a benefit.

### 5.11 `house_rent` (T79)

**Pending, game-side work; unverified.** The game owner confirms
`advance_house_rent` in `house_rent.rs`, SQL `game_house_rent_tick` and a disposition
audit. The fields `rent_period`, `rent_gold`, `next_rent_at`, `warnings` and
`last_payment_id` are not confirmed. The owner will verify them when implementing
the NEW emitter; no closed payload or golden is accepted for this family yet.
Character scope and `house_key`/world mapping also require emitter acceptance.
Keep the family disabled and unavailable until its schema is accepted.
Portal cannot pay rent through this feed.

### 5.12 `game_login` (T80)

**NEW game emitter required; no game-side login history exists.**
`admission_journal.rs` handles reconnect/recovery, not login history.
Account scope, optionally character scope once entry binds a character. Entity
`login_event_id`.
Payload, in order, contains only `login_event_id` (`id`, non-secret occurrence),
`entered_at` (`time`, Unix milliseconds), `left_at` (`time?`, Unix milliseconds),
`end_reason` (`LOGOUT|DISCONNECT|REVOKED|UNKNOWN`, nullable).
An open entry has null left_at and end_reason; a closed entry has both non-null,
with left_at ≥entered_at. Updates retain the same login_event_id and use a later
source_revision. A missing exit is unknown continuity, not proof of an active session.
Failed login attempts remain outside this history. Do not send GameSessionId or a
reconnect capability. Country and device data are outside this contract; the portal
may derive them only from its own web session, never from this game feed.

## 6. Ordering, outbox, watermarks and replay

The NEW game-side producer must write each journal export and its immutable bytes in the same committed
semantic transaction as its authoritative receipt/state change, or use a consistent
source scan with a fenced outbox cut. It never calls Platform while holding gameplay
locks. Export failure cannot roll back gameplay or admission. The existing `store/outbox.rs` is Store-only, not a journal/ingest outbox.
A separate private journal outbox is NEW game-side work; do not
append private data to public projection rows or repurpose Store receipt tables.

### 6.1 Game-side deliverables

All items below are **NEW game-side work**, requiring separate authorization and
owner acceptance; none is supplied by the existing Store-only `store/outbox.rs`:

- Private journal/ingest outbox and emitter with transactional capture or a fenced
  consistent scan, stable IDs, immutable bytes and contiguous family sequences.
- Baseline enumeration, tombstones, acknowledgement validation, retries, watermarks,
  replay, retention and restore/ownership fencing for this private feed.
- Dedicated descriptor purpose/route binding, default-off flags and family adapters;
  house_rent requires owner verification during emitter implementation; skull,
  stamina and premium use the owner-verified sources above, and login needs new capture.

### 6.2 Proposed delivery behavior

Business keys are (authority, account, epoch, family, character_id-or-null,
world_id-or-null, entity_key). Delivery key is (authority, record_id); sequence
uniqueness is (authority, account, epoch, family, source_revision). Keep exact
canonical bytes and SHA-256 digest. Same key/bytes is an exact retry; changed bytes
or reused sequence with another record is `409`, quarantined with an operator alarm.
Source revisions may have gaps in transport but **not in the defined export stream**;
CharacterRevision can have legitimate unrelated gaps. Neither time, arrival order
nor UUID ordering overrides a source revision.

Platform may durably stage out-of-order records but only exposes a contiguous
prefix after an accepted complete baseline. An older revision cannot overwrite a
newer entity; equal revision/different bytes conflicts. Snapshot current-state
families replace entities; history families deduplicate immutable occurrences.
Login entries are the explicit exception: later revisions may close the same entry
with left_at/end_reason, without creating a second login occurrence.
A watermark advances freshness only after every sequence through complete_through
is durably accepted and applied and its baseline is proven. Ahead-of-data watermark
→ `409`, no advancement; Game retries after filling gaps. A regressing watermark,
older observation time, lower epoch or recovery fence is `409`. Exact duplicate
watermarks return 200 without extending observation time/freshness.

Bootstrap/rebuild: coordinator establishes a new fenced projection_epoch through
the existing account projection/recovery authority, never from an arbitrary request.
Game stages a complete consistent enumeration of current entities and retained
history, numbered from 1 per family, then sends baseline_complete=true. Platform
atomically promotes only after the baseline cut is complete; omissions in that
baseline mean absent **within its declared retained history window**, not “never
happened.” Subsequent watermarks keep baseline_complete=true. false can acknowledge
staging but cannot expose data. Empty baseline has complete_through="0". New changes
serialize after the baseline cut; no mixed-epoch rendering. Each family is independent;
a completed Bestiary baseline cannot imply a complete login feed.

Replay republishes identical source bytes and IDs from the last acknowledged cut;
acknowledge an outbox row only after the bound 200 answer. A response lost after
Platform commit is resolved by exact retry. Retain pending exports until acknowledged
or explicitly quarantined; operator action cannot skip a revision and claim a complete
watermark. If source retention no longer covers a gap, perform a fenced baseline;
never guess missing events. Exact replay after payload expiry is acknowledged from the retained minimal
digest/fence (§8), never reinserted as new history. Unverifiable replay is denied.
For a first delivery of already-expired historical content, accept only its digest,
sequence and acknowledgement, discard the payload and advance completeness without
resurrecting the expired history.

Game restore uses the existing `PublishGameRestoreNoticeV1` reconciliation mechanism,
not a second journal restore endpoint. Before activation its accepted scope must
explicitly include this feed. On a newer recovery generation, suspend serving and
reminders for affected streams, retain prior evidence for reconciliation, fence old
messages, and rebuild under an authorized new epoch. Platform PITR requires an
externally preserved epoch/replay fence before ingest resumes; if that cannot be
proven, remain unavailable. A restore never lowers ownership or privacy fences.

Proposed liveness: watermark attempt every 3 seconds, source scan gap ≤10 seconds,
fresh through 10 seconds from its source observed time; history/current progress
may be served as explicitly stale up to 24 hours if ownership is independently
current. After 24 hours → unavailable. Timers, login continuity and reminders require
fresh evidence; stale/invalid/rebuilding/unknown denies action. Trusted-time uncertainty
>5 seconds or future observed time >5 seconds invalidates freshness until reconciled.
UI time is informational; this is not a gameplay or entitlement authority lease.

## 7. Failure, refusal and timeout semantics

| Response / condition | Required behavior |
|---|---|
| `200` exact acknowledgement | Durable ingest only; acknowledge matching outbox bytes |
| `400` | Closed-schema/version/unit/cross-field/canonical violation; no write, quarantine and fix producer |
| `401` | Wrong TLS/source/purpose/account-partition binding; no write, stop peer and alarm |
| `409` | Binding conflict, unsupported epoch, stale fence, ownership mismatch or incomplete watermark; no write, reconcile source/cut; never overwrite |
| `413` | Over route/array bound; no truncation, no write; family unavailable until compatible export |
| `429` | No write; bounded retry honoring Retry-After, retain bytes |
| `503` | Off, misconfigured, backing store unavailable or current ownership/recovery proof unavailable; no acknowledgement |
| Timeout, connection loss, malformed success, unexpected status | Not delivered/ambiguous acceptance; exact retry, never advance watermark or mint a replacement record |

Every non-200 has an empty body. No error echoes private content. Use Game's existing
projection exchange bound: 3 seconds including connection setup (inside transport's
5-second ceiling), backoff 1–60 seconds for transient failures; definite 400/401/409
holds the affected stream for 300 seconds and alarms, without advancing completeness.
A gap-caused 409 watermark is retried after the gap is repaired. Retries have no new
business identity. Persistent failure makes presentation stale/unavailable, never
empty/zero/not-found. A valid source empty baseline is distinct from unavailable.
No unknown family/result/command can default to success.

Unknown or ambiguous ownership **denies access**; unknown premium/stamina/cooldown
state **denies reminders or claims of readiness**; unknown revocation outcome
**denies a claim of completed security action**. Journal facts never authorize a
mutation. Existing command outcomes remain PENDING, terminal COMMITTED/REJECTED or
Platform AMBIGUOUS/RECONCILING under the owning exchange. No new command ID after
a timeout and no fallback to a direct game write.

## 8. Privacy, sale boundary and retention

Owner-only: authenticate Identity, resolve canonical AccountId server-side, then
prove current accepted ownership and the current visibility floor on **every**
HTTP/API/export/background reminder. Browser identifiers and cached journal ownership
are not authority. Other accounts, anonymous actors, moderators and support without
an explicitly accepted separate access contract are denied. No public search,
rankings, CDN cache, generic analytics event body or full-payload logs. Account pages
and APIs use private no-store responses; queues carry references rather than bodies.

On `TransferCharacterOwnership`, use the accepted Character Authority receipt and
account projection epoch/revisions to establish the cut. Revoke old-owner access to
character-scoped projections, clear browser/server caches and cancel queued reminders
before allowing post-transfer access. Historical private records remain partitioned
under their original recipient and are suppressed for both accounts by default;
the new owner receives a fresh current-state baseline, never the former owner's
login/death/skill history. Account-level achievement grants and premium remain with
the originating account. Until the transfer cut and both ownership publications are
proven, deny character-scoped access to both sides. Runtime transfer support is not
assumed merely because the wire documents its command name.

Proposed Platform retention (coordinator must accept before enablement):

| Data | Maximum retention / deletion |
|---|---|
| Current progress/timers | Latest entity while current ownership holds; superseded payloads ≤7 days |
| Skill history, death receipts/losses, PvP events | 90 days from occurred_at |
| Detailed recap actions | UNAVAILABLE in v1; no actions captured or retained |
| Game login history | 30 days |
| Raw acknowledged canonical payloads | ≤7 days, encrypted at rest and unavailable to ordinary application logs |
| Minimal record digest, key/sequence and epoch/ownership fences | While stream exists, plus 90 days after closure; no payload; reset/rebootstrap required before accepting older replay after expiry |
| Backups | Encrypted, bounded 35-day expiry; restore reapplies deletion/ownership fences before serving |

Account deletion purges payloads within 7 days and cancels reminders immediately;
retained replay fences are access-restricted pseudonymous security metadata, not
available history. Game's authoritative retention remains Game policy; export replay
must obey the recipient retention window, never resurrect erased content. Retention
expiry does not emit invented game tombstones or alter gameplay.

Never send passwords/hashes, tokens, cookies, certificates/private keys, raw IPs,
precise location, device identifiers/fingerprints, hardware IDs, user-agent strings,
GameSessionId, admission/lease capabilities, runtime topology, chat, mail, trade
notes, item inscriptions, counterpart AccountIds or unrestricted combat logs.
Country/device data are excluded from this feed; no raw source fallback. Counterpart
CharacterIds are minimized/redacted; names are resolved only through independently
permitted presentation data. No telemetry is an authentication factor.

## 9. Flags, versioning and rollout

Proposed flags, all **false** by default: Game `journal_progress_publish_enabled`;
Platform `playerdata.progress_ingest_enabled`, `playerdata.progress_read_enabled`,
`playerdata.progress_reminders_enabled`, `playerdata.game_login_history_enabled`.
Both sides also require an explicit enabled-family allowlist (default empty),
purpose-bound identity, accepted version and accepted source/epoch configuration.
A disabled route returns 503; disabled reads/reminders expose nothing. T80 security
actions retain their own existing capability gates; enabling history does not enable
revocation. Initial environments: testing/preproduction only, synthetic accounts.

Closed v1 rejects extra members/enums and unsupported versions with 400. A new
field, wider bound, unit change, scope change or enum requires an explicitly accepted
new wire version with separate descriptor negotiation/routing; no silent tolerant
reader. New producers must retain v1 behavior until both sides accept migration.
Never downgrade durable epoch/recovery/ownership fences on rollback. Baseline and
incremental data must use the same schema/catalog interpretation within an epoch.
Unknown catalog revisions suspend the affected entity/family. Turning flags off
retains pending outbox/digests and cancels reminders; rollback cannot expose an older
privacy allow or resurrect deleted history. Each family is accepted and enabled
independently; partial acceptance cannot close all G-11.

## 10. Golden payloads

These are synthetic golden schema examples. Fences render readable JSON; **wire
bytes are compact JSON in exactly this member order with no trailing newline**.
The authority, account partition and epochs must be configured in the test fixture;
its trusted clock is fixed at Unix millisecond 1790000000000.

Valid complete stamina record (character ownership projection_revision 12; sequence 1):

```json
{"contract_version":1,"operation":"PublishGameJournalProgressV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c04-7001-805b-3b1122334402","projection_revision":"12","projection_epoch":"1","recovery_generation":"3","family":"stamina","source_revision":"1","record_id":"01934f10-7c04-7001-805b-3b1122334403","entity_key":"stamina","character_revision":"9","occurred_at_unix_ms":"1790000000000","record_kind":"UPSERT","payload":{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}}
```

Matching valid acknowledgement:

```json
{"contract_version":1,"result":"accepted","record_id":"01934f10-7c04-7001-805b-3b1122334403","accepted_source_revision":"1"}
```

Valid baseline watermark after that sole record is durably applied:

```json
{"contract_version":1,"operation":"PublishGameJournalProgressWatermarkV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","recovery_generation":"3","family":"stamina","complete_through":"1","observed_at_unix_ms":"1790000000000","baseline_complete":true}
```

Valid family payload goldens, in §5 order. These are **payload objects**, not whole
requests; envelope subject/scope/entity rules remain mandatory.

```json
{"race_key":"rat","race_definition_revision":"r1","kill_count":100,"final_kill_threshold":100,"charm_points_awarded":5}
```

```json
{"boss_key":"boss/example","definition_revision":"r1","kill_count":"2","available_at_unix_ms":"1790086400000","last_death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334410"}
```

```json
{"quest_key":"quest/example","pinned_content_revision":"r1","definition_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":false,"tracks":[{"track_key":"quest/example/progress","value":"2"}]}
```

```json
{"achievement_key":"oteryn:achievement/example","definition_revision":"r1","progress":2,"target":10,"granted_at_unix_ms":null}
```

```json
{"achievement_key":"oteryn:achievement/example","definition_revision":"r1","progress":null,"target":null,"granted_at_unix_ms":"1790000000000"}
```

```json
{"build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411","cause":"training","policy_revision":"r1","skills":[{"skill":"sword","level_before":10,"progress_before":"9","level_after":11,"progress_after":"0"}]}
```

```json
{"death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334412","level_before":20,"level_after":19,"experience_lost":"1000","blessings_before":["blessing/example"],"blessings_after":[],"lost_items":[{"loss_index":0,"item_key":"item/example","quantity":1}],"recap_state":"UNAVAILABLE","window_ms":0,"actions":[]}
```

```json
{"death_occurrence_id":"01934f10-7c04-7001-805b-3b1122334412","role":"VICTIM","counterpart_character_id":null,"justification":"UNKNOWN","points_milli":null}
```

```json
{"skull":"RED","skull_until_unix_ms":"1790086400000","kill_block_until_unix_ms":null}
```

```json
{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}
```

```json
{"observed_state":"ACTIVE","effective_until_unix_us":"1790086400000000","authority_revision":"4"}
```

*house_rent golden deferred pending game-side schema verification.*

```json
{"login_event_id":"01934f10-7c04-7001-805b-3b1122334413","entered_at":"1790000000000","left_at":null,"end_reason":null}
```

Invalid full request: source_revision is a JSON number rather than a decimal string;
expect 400 and no outbox acknowledgement, projection write or freshness update.

```json
{"contract_version":1,"operation":"PublishGameJournalProgressV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c04-7001-805b-3b1122334402","projection_revision":"12","projection_epoch":"1","recovery_generation":"3","family":"stamina","source_revision":1,"record_id":"01934f10-7c04-7001-805b-3b1122334403","entity_key":"stamina","character_revision":"9","occurred_at_unix_ms":"1790000000000","record_kind":"UPSERT","payload":{"stamina_minutes":2400,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}}
```

Invalid family payload substitutions (400): above-maximum stamina; skull without
its required deadline; raw IP field; quest integer encoded noncanonically.

```json
{"stamina_minutes":2521,"content_revision":"9","build_occurrence_id":"01934f10-7c04-7001-805b-3b1122334411"}
```

```json
{"skull":"BLACK","skull_until_unix_ms":null,"kill_block_until_unix_ms":null}
```

```json
{"login_event_id":"01934f10-7c04-7001-805b-3b1122334413","entered_at":"1790000000000","left_at":null,"end_reason":null,"ip_address":"192.0.2.1"}
```

```json
{"quest_key":"quest/example","pinned_content_revision":"r1","definition_hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","completed":false,"tracks":[{"track_key":"quest/example/progress","value":"02"}]}
```

Rule-invalid watermark for the one-record fixture: valid schema but revision 2 is
missing, so 409 and no freshness advancement.

```json
{"contract_version":1,"operation":"PublishGameJournalProgressWatermarkV1","source_authority":"oteryn:journal:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","projection_epoch":"1","recovery_generation":"3","family":"stamina","complete_through":"2","observed_at_unix_ms":"1790000000000","baseline_complete":true}
```

Additional golden rule cases: replay the valid record unchanged → identical 200;
change its payload under the same record_id → 409; deliver epoch 1 after accepted
epoch 2 → 409; request another account's history → deny with no data; timeout after
commit then exact retry → one record; unsupported command → existing
`CHAR_CMD_UNSUPPORTED`, never a journal mutation.

## 11. Acceptance checklists and open questions

Platform acceptance (all unchecked until separately evidenced):

- [ ] Coordinator-approved schemas and Game source mapping for each enabled family;
      shared golden fixtures accepted by both repositories.
- [ ] Closed decoding, bounds, TLS purpose/account binding, signature/trust reuse and
      negative peer tests; all default-off flags exercised.
- [ ] Atomic digest/deduplication, reordered/gapped delivery, baseline promotion,
      truthful empty/stale/unavailable states and incomplete watermark refusal proven.
- [ ] T72 owner/other-account/team denial and transfer cut/cache/reminder invalidation;
      account versus character achievement separation proven.
- [ ] T77 progress/history, T78 receipt losses/PvP redaction and unavailable recap actions, T79 source-revalidated reminders,
      T80 admission/end history and unsupported security-action behavior proven.
- [ ] Erasure, retention, replay-after-erasure, backup/PITR and lower-fence refusal proven.
- [ ] Premium observation cannot grant value; no new gameplay commands or direct SQL;
      timeout reconciliation and restore amendment accepted before activation.

Game acceptance:

- [ ] Owner/coordinator explicitly accepts or revises each family, bound, unit and
      recipient/sale boundary; marks unsupported producers unavailable.
- [ ] Names/keys/counter semantics and catalog revisions mapped to exact owning types;
      no invented stage, cooldown, skull color, points or login/session authority.
- [ ] Consistent fenced baseline and transactional private outbox, stable occurrence
      IDs, contiguous export sequence, tombstones and no skipped watermark proven.
- [ ] Existing mTLS descriptor/transport, dedicated purpose and cert isolation proven;
      acknowledgement validation and bounded retry never block gameplay.
- [ ] NEW admission audit and timer/premium observations implemented
      only in separately authorized Game work; no assumed producer completion here.
- [ ] Restore/PITR, ownership publication cut and unsupported transfer consumer
      behavior tested against Platform's deny-on-ambiguity rule.
- [ ] Never-sent fields absent; counterpart redaction, exclusion of country/device data,
      retention-compatible replay and synthetic preproduction E2E accepted.

Open questions for the game owner (activation blockers for the affected family):

1. **Producer coverage:** which owners will supply NEW
   game admission/end audit, stamina receipt exports, premium observations and
   rent exports? Which families can be accepted first without declaring G-11 complete?
2. **Outbox cut:** can private streams share the existing account projection_epoch
   fence and allocate contiguous per-account/family export sequences transactionally?
   Confirm consistent baseline, tombstone, replay and restore-notice scope, including
   a Platform PITR fence. No enablement without an agreed recovery mechanism.
3. **Ownership/sale:** accept suppression of pre-sale character history for both
   parties, fresh current-state baseline for the buyer and retained account grants?
   Confirm the §4.3 receipt projection_revision mapping,
   and what proves the cut while TransferCharacterOwnership runtime is unsupported?
4. **Catalog/progress mapping:** confirm key alphabets, revision/hash sources,
   Bestiary charm points and Bosstiary interpretation, quest owner-visible tracks,
   account achievement grant versus character count, and skill bounds/cause set.
5. **Death/PvP privacy and bounds:** accept UNAVAILABLE recap actions in v1, 256 loss types,
   source redaction, Game-assigned KILL/ASSIST, and durable NONE/RED/BLACK skull only?
   What is the authoritative points_milli mapping? Action completeness is deferred.
6. **Timers/premium:** accept premium microseconds and other family milliseconds, conservative conversions, 2520-minute
   stamina constant maximum, no exported full-time prediction and reminder suppression on stale evidence?
   Should premium reminders use only Platform commercial authority and omit this
   Game observation family entirely?
7. **Login/security:** can Game mint non-secret admission audit occurrences without
   exporting GameSessionId, and supply the four-field entry/exit history? Which existing
   accepted revocation capability supports T80, and what exact completion proof can
   Platform show? History must remain independently usable if revocation is absent.
8. **Operational/privacy policy:** accept dedicated purpose/identity, route/depth/rate
   bounds, 3-second exchanges, 10-second freshness, 24-hour stale progress window,
   retention/deletion/backup durations and testing/preproduction-only rollout?
   Production remains disabled pending separate acceptance and authorization.

## 12. Pinned source evidence and validation scope

Game reference is read-only **Oteryn/Oteryn-Game main at
`2253b997`**. Owner-verified source names in §5.8–§5.12 are reproduced as supplied above;
no additional directory paths are inferred. Every older Game path below remains an **unverified** reference at
that commit ([source tree](https://github.com/Oteryn/Oteryn-Game/tree/2253b997)).
No Game mutation or push is part of this task.

| Game path (unverified at main `2253b997`) | Proposed evidence / limit (unverified) |
|---|---|
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` | TransferCharacterOwnership name, PLATFORM_BAZAAR_TRANSFER, existing pending/read/receipt routes, source revisions and account publication cut |
| `crates/gameplay/native-admission-source/src/character_commands.rs` and `crates/gameplay/durability/src/durability/character_command.rs` | Key, PendingEntry, canonical decimal/UUID wire checks, bounded polling; current consumer only decides CreateCharacter and refuses others with CHAR_CMD_UNSUPPORTED |
| `crates/gameplay/native-admission-source/src/descriptor.rs`, `http1_mtls.rs` | ProducerDescriptor, closed Operation route binding, TLS trust verifier and existing transport; no journal Operation exists yet |
| `crates/gameplay/native-admission-source/src/public_projections.rs` | Separate purpose-bound publication, held watermark on failure, 3-second exchange, retry/liveness pattern; public producer is not a private journal producer |
| `crates/gameplay/durability/src/durability/store/outbox.rs` and `apps/game-server/migrations/0087_character_commands.sql` | `store/outbox.rs` is Store-only; no journal/ingest outbox exists. Command receipt mapping is unverified; private outbox/emitter is NEW work |
| `crates/gameplay/durability/src/durability/bestiary_progress.rs` | BestiaryKillOccurrence, CommittedBestiaryKill, race_key/race_definition_revision, kill counts, CharacterRevision and receipt-owned transaction |
| `crates/protocol-oteryn/src/boss_encounter.rs`, `crates/gameplay/durability/src/durability/boss_encounter/cooldown_list.rs` | BossCooldownEntry.available_at_unix_ms, BosstiaryProgressEntry.kill_count and session-fenced cooldown owner; export cannot reuse an unproven browser session |
| `crates/gameplay/durability/src/durability/quest_state.rs`, `quest_state/storage.rs` | Named tracks/values, pinned_content_revision, definition_hash, completes, source receipts and fenced transitions; not a universal numeric stage |
| `apps/game-server/migrations/0574_character_achievement_progress.sql` | Character progress key/bound 100000 and separate existing account achievement grant owner |
| `apps/game-server/migrations/0030_character_build_state.sql`, `0181_hunting_stamina.sql`, `0438_resting_stamina_gain.sql` | Eight skill counters, mana_spent/tries, level bounds and receipt causes; stamina carried by Game build receipts |
| `apps/game-server/migrations/0016_character_death_receipts.sql` | PlayerDeathOccurrence, losses, blessings, experience and revision; this receipt alone cannot prove a last-seconds recap |
| `crates/gameplay/durability/src/durability/pvp_skull_storage.rs`, `apps/game-server/migrations/0142_pvp_death_consequences.sql` | Durable None/Red/Black, skull deadlines, contributor death transaction and unjustified points_milli |
| `apps/game-server/migrations/0914_house_rent.sql` | world_id/house_key, next_rent_at, warnings 0–7, rent_gold bound, payment_id and game-owned rent schedule |

Documentation-only validation: review the complete one-file diff, parse every fenced
JSON example, mark unverified pinned paths explicitly and execute
`python3 tools/validation/legacy_token_ratchet.py` before publication; required result
`ratchet ok`. Runtime/E2E is NOT_APPLICABLE to this proposal-only change; all unchecked
acceptance scenarios above remain future implementation evidence, not claimed tests.

## Future versions

Death recap action capture is deferred beyond v1. A separately accepted version
must define the capture owner, complete bounded action window, ordering, participant
redaction, ability mapping, completeness proof and retention. No populated actions
or COMPLETE recap state are accepted by v1, and no capture implementation is assumed.
