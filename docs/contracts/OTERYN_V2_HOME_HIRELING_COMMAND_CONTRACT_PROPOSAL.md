Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# Home hireling: private projection, expedition commands and Store delivery

Contract ID: `oteryn-v2-home-hireling-v1`. Proposed wire `contract_version: 1`;
command semantic version 1. Nothing here marks G-14 accepted or implemented.

## 1. Purpose, owners and authority

This is Platform's proposed boundary for G-14 in
[the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md), gate table and
§10. T92 (`crates/hirelings`) consumes helper ownership, professions/levels, an
explicit expedition reward table, current state/history, weekly limits and command
receipts. Its dependencies are T26 (Wallet), T29 (Store) and T36 (notifications).
T36 may notify an owner only after accepted Game evidence of a return; a portal
countdown is not evidence. T92 cannot enable itself merely because this document exists.

| Responsibility | Owner |
| --- | --- |
| AccountId, authenticated portal authorization, browser CSRF/rate limits, immutable command outbox, private presentation and notification deduplication | Platform |
| Product/version, order, entitlement and commercial delivery saga | Platform Store / ProductsEntitlements; Wallet remains its own authority |
| Hireling identity and owner CharacterId, house eligibility/custody, professions and levels, expedition eligibility, time, reward calculation, weekly accounting, chest and in-game dialogue | Oteryn-Game |
| Atomic gameplay effect, durable decision receipt, source revisions/epochs, transactional projection and receipt outboxes, restore fencing | Oteryn-Game |
| Acceptance of this proposal, gameplay rules, production enablement and unresolved questions | Game owner (coordinator), with Platform acceptance of the final consumer mapping |

The helper extends the existing hireling; it is not a second pet, character, NPC
identity system or inventory authority. Portal state is a rebuildable read model
plus an orchestration ledger. It never authorizes a grant or proves current ownership.
There is no shared SQL, direct game-table write, broker, inbound Game HTTP listener,
or synchronous Game query during page rendering. This docs-only proposal authorizes
no runtime/schema/workflow change, sale, deployment or activation.

## 2. Existing contracts and pinned Game evidence

Reuse these Platform contracts within their stated status and scope:

- [Store wire](OTERYN_V2_STORE_WIRE_CONTRACT.md): canonical bytes, mTLS peer guard,
  pending/read/receipt exchange, SHA-256 binding, immutable intents, receipt replay,
  restoration generations and separate purchase/delivery flags. **Its current status
  is PROPOSED**, so reuse is a proposal dependency, not a claim of accepted transport.
- [Public game projections](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md):
  source-owned revisions, transactional evidence, watermarks, rebuilds, poison/gap
  handling and privacy floors. This helper feed is private and adds no public family.
- [Character authority commands](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md):
  stable operation identity, current Game revalidation, terminal versus ambiguous
  outcomes, no blind replacement operation and restore reconciliation.
- [Entitlement delivery](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): payment,
  entitlement and gameplay truth stay separate; proposed durable helper grants use
  Profile C, not an infinite Profile-B authority lease.

Read-only source: [Oteryn-Game at
857f15f376907c592bb28d854fccff18a15cdf87](https://github.com/Oteryn/Oteryn-Game/tree/857f15f376907c592bb28d854fccff18a15cdf87).
Every Game path below refers to that commit, never floating `main`:

| Game path | Evidence used |
| --- | --- |
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`, §§2–6 | Game-initiated `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1`, `PublishCharacterCommandReceiptV1`; dedicated mTLS purpose, exact receipts, 300-second intent TTL, recovery fence |
| `crates/gameplay/native-admission-source/src/character_commands.rs` | `Key`, `PendingEntry`, `CommandDescriptor`, `http1_mtls::exchange`, compiled paths, page/rescan limits; these existing Character operations are not extended silently |
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md`, §§3–6; `crates/gameplay/native-admission-source/src/public_projections.rs` | `Family`, publisher, epoch/revision replacement, `complete_through`, acknowledgement vocabulary and fail-closed freshness |
| `crates/gameplay/native-admission-source/src/store_wire.rs`; `crates/gameplay/durability/src/durability/store/outbox.rs` | Store exact field/byte validation; `list_unacknowledged_store_receipts` / `mark_store_receipt_acknowledged`; persisted receipts survive failed publication |
| `crates/gameplay/dispatch/mechanics/0425-hireling.mechanic`, `0610-hireling-management.mechanic` | Existing mechanic registrations: command 220 / capability 165 and command 470 / capability 250 respectively |
| `docs/contracts/protocol-oteryn/v1/hireling-v1.md`, `hireling-management-v1.md` | Summon/dismiss, identity list, return-to-lamp, appearance and transient actor projection; current session-derived authority, not portal authority |
| `apps/game-server/migrations/0405_hireling.sql`, `0719_hireling_identity.sql`, `0720_store_hireling_delivery.sql` | Existing persistence lineage, UUIDv7 `hireling_id`, `owner_character_id`, optional `house_key`, Store reservations/effects and atomic receipt/outbox proof |
| `crates/gameplay/durability/src/durability/house_hireling/model.rs` | `HirelingSnapshot`, `MAX_HIRELINGS = 10`, durable identity separate from NPC actor handles/positions |
| `crates/gameplay/domain/src/domain/hireling.rs` | Existing `Skill::{Banker,Cook,Steward,Trader}` IDs 1001–1004 are service apprenticeships, not expedition professions |
| `crates/gameplay/durability/src/durability/store/hireling_model.rs` | `HirelingStoreAction::{Hire,Unlock,Rename,SexChange}`, `HirelingStorePurchase`, `HirelingServiceDeliveryPlan`; existing character-wide unlocks and immutable customization |

The plan's references 0425/0610 denote mechanics, not SQL migration filenames at
this pin. G-14's professions, levelled expeditions, reward/weekly accounting, chest
and portal messages below are **new proposals**, not existing Game types. Existing
service skill IDs and character-wide unlock scope retain their meaning. The plan's
three helpers per account differs from the Game's ten identities per character:
acceptance must resolve that discrepancy without deleting existing hirelings (Q1).

## 3. Directions, transport, authentication and signing

All network exchanges are **Game → Platform**, HTTP/1.1 over TLS 1.3 using the
existing `http1_mtls::exchange` model, server certificate validation and a dedicated
purpose-bound client certificate. Logical command intent flows **Platform → Game**
in the response to Game polling; there is no Platform-initiated connection to Game.

Proposed paths/operation names below are new, compiled, closed names following the
existing owners' conventions. They are not aliases for the existing Character,
public-projection or Store routes. Every path is POST, every response `no-store`.

| Logical flow | Operation | Proposed Platform path | Request / 200 response ceiling, bytes |
| --- | --- | --- | --- |
| Game → Platform private catalog ingest | `PublishHirelingRewardTableV1` | `/internal/v1/game-private/hirelings/reward-table` | 262144 / 256 |
| Game → Platform private owner snapshot ingest | `PublishHirelingStateV1` | `/internal/v1/game-private/hirelings/state` | 131072 / 256 |
| Game → Platform expedition/history ingest | `PublishHirelingExpeditionV1` | `/internal/v1/game-private/hirelings/expedition` | 32768 / 256 |
| Game → Platform completeness ingest | `PublishHirelingWatermarkV1` | `/internal/v1/game-private/hirelings/watermark` | 512 / 256 |
| Platform → Game command discovery response | `ListPendingHirelingCommandsV1` | `/internal/v1/game-auth/hireling-commands/pending` | 512 / 8192 |
| Platform → Game exact intent response | `ReadHirelingCommandIntentV1` | `/internal/v1/game-auth/hireling-commands/read` | 512 / 4096 |
| Game → Platform authoritative receipt | `PublishHirelingCommandReceiptV1` | `/internal/v1/game-auth/hireling-commands/receipt` | 2048 / 256 |
| Game → Platform periodic command comparison | `ListHirelingCommandStatesV1` | `/internal/v1/game-auth/hireling-commands/reconcile` | 512 / 16384 |
| Store entitlement/delivery | Existing Store wire operations | Existing Store paths, unchanged | Existing Store limits |

Separate dedicated identities for private projection publisher and helper command
consumer; neither is the Store, public projection, LCFA, admission/evidence, runtime,
restore or Character-command identity. Exact configured certificate subject plus
configured `source_authority` determines allowed scope; request fields cannot select
trust or another shard. One publisher owns a configured source namespace; accounts
have one configured helper authority. Cross-authority migration needs a future contract.

Use the Store peer guard order: disabled/misconfigured `503`; require trusted TLS
terminator `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, exact configured
subject of 1–128 printable ASCII bytes (`401` otherwise); per-peer limiter `429`
with positive `Retry-After`; then transport/schema/source checks. Overlapping
identities, unknown environments or unsafe trusted-time basis fail closed. Testing
and preproduction only; production PKI/enablement needs separate acceptance.
Proposed limits: 600 requests/minute/peer (range 1–6000), 10 new portal commands/minute/
account (1–60), one command exchange in flight, at most 40 unreceipted commands per
authority. Refuse excess issuance before persisting a new intent.

TLS authenticates both peers and protects wire integrity. These inherited exchanges
have **no detached application signature, JWT or new signing key**. SHA-256 binding
and content digests are comparisons, not signatures or authentication. Never accept
an offline JSON payload as authenticated because it has a digest. If a later owner
requires signed artifacts, that needs an explicit parent-compatible revision and
key/audience/rotation design; no placeholder signature field is proposed here.

Command connect/handshake/exchange timeouts: 1/2/3 seconds; ingest: 1/2/5 seconds.
Command poll every 1 second, backoff up to 10 seconds on `429`/`503`, honoring a longer
`Retry-After`. Publication retries double 1 to 60 seconds, with per-subject defects
held at least 5 minutes and alarmed, as public projections. Timeouts convey no outcome.

## 4. Encoding and common types

Exact member sets and listed order, UTF-8 JSON objects, `application/json`, no BOM,
trailing newline, insignificant whitespace, duplicate keys, escapes/backslashes,
unknown members or unknown enum values. JSON integer means integer token, never
float/exponent/boolean. All members are required; null is forbidden except the two
explicit reconcile fields in §7. Pretty printing is not accepted on the wire.
Maximum nesting depth 8 (the new nested helper snapshot bound, not a change to parent
routes). `Content-Length`, if present, is canonical decimal, at most the route bound;
over-bound is `413`. No `Transfer-Encoding`; `Content-Encoding` absent or `identity`.
Empty, malformed or otherwise over-bound bodies are refused with no state change.
Digests are SHA-256 of exact canonical UTF-8 bytes, lowercase hex.

| Type | Exact representation / inclusive bounds |
| --- | --- |
| `uuid7` | Lowercase non-nil UUIDv7, `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `nil` | `00000000-0000-0000-0000-000000000000`, initial operation cursor only |
| `u64` | Decimal string `[1-9][0-9]{0,19}`, at most `18446744073709551615` |
| `cursor` | `"0"` or `u64` |
| `time` | `u64`, Unix seconds UTC, never milliseconds; event/expiry semantics specified below |
| `int[a,b]` | JSON integer a through b |
| `key` | `[a-z0-9][a-z0-9._-]{0,63}`, case sensitive, not a display label |
| `item_key` | `oteryn:item.` followed by 1–116 of `[a-z0-9._-]`, total at most 128 ASCII bytes; must resolve in the pinned Game content revision |
| `house_key` | 1–128 bytes `[A-Za-z0-9._:/-]`, Game catalogue key, paired with WorldId |
| `authority` | 1–128 bytes `[A-Za-z0-9._:/-]`, exact configured source |
| `hex64` | 64 lowercase hexadecimal digits |
| `hireling_name` | 3–18 ASCII letters/spaces, copied from Game, never supplied in an expedition command; presentation only, escaped in HTML |

Canonical AccountId is Platform-issued; CharacterId, HirelingId, ExpeditionId and
ChestId are Game-issued UUIDv7. Existing hireling identity is reused, never minted
from a name, house, actor ID or numeric service skill. `world_id` is canonical
Registry WorldId. `house_key` identifies no account and alone grants no access.
No ChannelId is needed for these cross-channel Game-owned house facts.

Common request head **H** is, in order: `contract_version` (integer 1), `operation`
(route's exact name), `source_authority` (`authority`). Every schema below expands H
inline, never as a JSON `head` member. Common projection prefix **P** is H followed
by the listed subject fields, then `projection_epoch` (`u64`), `projection_revision`
(`u64`), `source_observed_at` (`time`, frozen when the revision is assigned).

## 5. Game → Platform projection record schemas

### 5.1 Reward table: `PublishHirelingRewardTableV1`

P subject fields: `world_id` (`uuid7`), `reward_table_key` (`key`),
`reward_table_revision` (`u64`, immutable table version). Then, in order:

| Field | Type / rule |
| --- | --- |
| `content_revision` | `u64`; Game catalogue revision interpreting every item/profession/destination |
| `availability` | `ACTIVE` or `RETIRED`; retirement blocks new starts but preserves pinned expeditions |
| `weekly_policy_revision` | `u64`; immutable weekly policy meaning |
| `week_anchor_at` | `time`; Monday 00:00:00 UTC; windows are `[anchor + n*604800, anchor + (n+1)*604800)` seconds |
| `weekly_expedition_limit` | `int[1,168]`; successful starts per AccountId across characters/helpers/worlds on this authority |
| `weekly_item_caps` | Array 1–32 of `ItemAmount`, sorted unique `item_key`; cap quantities per AccountId/week |
| `rows` | Array 1–32 of `RewardRow`, sorted unique `reward_row_key` |

`ItemAmount`, exact order: `item_key` (`item_key`), `quantity` (`int[1,65535]`, item
units, no currency conversion). `RewardRow`, exact order: `reward_row_key` (`key`),
`profession_key` (`key`), `minimum_level` (`int[1,100]`), `maximum_level`
(`int[1,100]`, ≥ minimum), `destination_key` (`key`), `duration_seconds`
(`int[3600,43200]`, 1–12 hours), `profession_xp` (`int[0,1000000]`, Game XP units),
`rewards` (1–32 `ItemAmount`, sorted unique item key).

Proposed v1 is deterministic: one selected row yields exactly its listed quantities
and XP, with no paid chance, probability, hidden roll or portal-computed loot. Distinct
row keys are selectable quotes. Game checks the helper's unlocked profession/level
and the quoted table/row at execution. Every reward must have an applicable weekly
cap. Caps/limits are shared account policy: tables must agree on one active
`weekly_policy_revision`; a contradictory table quarantines new starts.
Rows/content/policy bindings of a table version are immutable; only availability
may transition ACTIVE → RETIRED at a newer projection revision. At most one version
of a table key is active per World. Older versions are independent projection
subjects, retained for reconciliation. Tables and referenced item definitions remain readable internally while any live
expedition/receipt depends on them. A revision change never reprices a running expedition.
Localized destination/profession labels belong to versioned content presentation,
not mutable wire identity. New profession keys do not reuse service IDs 1001–1004.
The 262144-byte table ceiling covers 32 rows × 32 maximum-length item keys
and amounts plus row/envelope fields; 131072 bytes covers ten helper/chest lists
plus weekly counters. Collection cardinalities and byte ceilings both apply.

### 5.2 Owner snapshot: `PublishHirelingStateV1`

P subject fields: `account_id`, `character_id` (both `uuid7`). Then `world_id`
(`uuid7`), `hirelings` (array 0–10 of `Helper`, sorted unique hireling_id).
It replaces the character's complete helper list. Empty means no presently
owner-visible helper on that character; it is not an account-deletion receipt.
A removed/transferred character must also publish an empty snapshot to the former
AccountId. Platform independently requires current accepted ownership evidence.

`Helper` members, in order:

| Field | Type / rule |
| --- | --- |
| `hireling_id` | `uuid7`; existing `HirelingSnapshot.id` |
| `owner_character_id` | `uuid7`, equals outer character_id |
| `name` | `hireling_name` copied verbatim from Game |
| `hireling_revision` | `u64`; changes on each helper state/profession/chest transition |
| `placement` | Array 0–1: object `house_key` (`house_key`); empty = lamp/unplaced, no position exported |
| `professions` | Array 0–16 of `Profession`, sorted unique profession_key |
| `availability` | `READY`, `BUSY`, `UNPLACED`, `BLOCKED`; advisory, never execution permission |
| `active_expedition_ids` | Array 0–1 `uuid7`; at most one unfinished expedition per helper |
| `chest` | Array 0–1 of `Chest`; absent if not provisioned; never means zero items |

`Profession`: `profession_key` (`key`), `level` (`int[1,100]`), `experience`
(`cursor`, Game XP units), `next_level_experience` (`cursor`, zero iff max level;
otherwise greater than experience). Game owns XP thresholds and advancement.

`Chest`: `chest_id` (`uuid7`, stable Game-owned logical helper chest),
`chest_revision` (`u64`), `capacity_units` (`int[1,2097120]`, summed item units),
`used_units` (`int[0,2097120]`, ≤ capacity), `items` (0–32 `ItemAmount`, sorted
unique keys, sum quantities = used_units). No stack slot, mutable inventory blob,
item-instance ID, tradable mint token or authority handle crosses the boundary.
A chest beyond these bounds makes this scope unavailable; never truncate it silently.

### 5.3 Expedition/history: `PublishHirelingExpeditionV1`

P subject fields: `account_id`, `character_id`, `expedition_id` (`uuid7`). Then:

| Field | Type / rule |
| --- | --- |
| `world_id`, `hireling_id` | `uuid7` each |
| `expedition_revision` | `u64`; Game state revision of this expedition |
| `reward_table_key`, `reward_row_key` | `key` each |
| `reward_table_revision`, `content_revision`, `weekly_policy_revision` | `u64` each, pinned at start |
| `profession_key`, `destination_key` | `key` each, equal to pinned row |
| `level_at_start` | `int[1,100]`, within pinned row's bounds |
| `state` | `RUNNING`, `READY_TO_COLLECT`, `COLLECTED`, `CANCELLED`, `HIDDEN` |
| `started_at`, `returns_at`, `week_start_at` | `time` each; returns = start + row duration; week containing start |
| `origin` | `PORTAL` or `GAME_DIALOGUE`, origin of the start; collection may use either surface |
| `start_operation_ids` | 0–1 `uuid7`; one for PORTAL, empty for GAME_DIALOGUE or HIDDEN |
| `collect_operation_ids` | 0–1 `uuid7`; one iff collected through portal |
| `rewards` | 0–32 `ItemAmount`; pinned row's exact rewards except empty for CANCELLED/HIDDEN |
| `profession_xp_awarded` | `int[0,1000000]`; zero before COLLECTED, then pinned row XP |
| `collected_at` | Array 0–1 `time`; one iff COLLECTED |
| `chest_ids` | Array 0–1 `uuid7`; one iff COLLECTED |

HIDDEN is a privacy tombstone: rewards empty, XP zero, operation/time/chest optional
arrays empty. Its original identifiers/pinned start facts remain internal only.
CANCELLED means Game durably guarantees no collection; it is never inferred by the
portal and v1 has no portal cancel command. It retains original start facts but has
empty rewards and no collection arrays. The projection provides bounded history
(one record per expedition), not a rolling list whose eviction means deletion.

### 5.4 Weekly account counters: included with each state publication

Append `weekly` after `hirelings`: exact object `weekly_policy_revision` (`u64`),
`week_start_at` (`time`), `week_end_at` (`time`, start + 604800),
`weekly_expedition_limit` (`int[1,168]`), `started_count` (`int[0,168]`, ≤ limit),
`items` (1–32 objects, sorted unique item_key, each `item_key`, `cap_quantity`
`int[1,65535]`, `reserved_quantity` `int[0,65535]`, `delivered_quantity`
`int[0,65535]`; reserved + delivered ≤ cap). These are an account-wide snapshot
repeated for each character; append `weekly_revision` (`u64`) inside `weekly` last.
Compare account counters by (epoch, weekly_revision), not by character revision;
equal pair/different weekly content is a contradiction. Missing policy/counters
means unavailable, not a zero counter. Counters are presentation only.

At successful start Game increments started_count and reserves all row reward units
atomically under the account/week budget. Collection moves reserved to delivered
in that **original week**, once, even after a week boundary. A new week grants no
second claim on an old expedition. Successful starts remain counted after cancellation;
Game releases cancelled undelivered reward reservations with a durable decision.
Old-window reservations remain until resolved. Policy changes take effect at a week
boundary; all in-game and portal paths use the same accounting/locks. XP/level,
receipt, expedition, chest and budget transitions commit together or not at all.

### 5.5 Watermark

H then `family` (one of `hireling_reward_table`, `hireling_state`,
`hireling_expedition`), `projection_epoch` (`u64`), `complete_through` (`time`),
`observed_at` (`time`). Exact same meaning as public projections: no committed
change at/before complete_through is undelivered in that family/authority; it cannot
pass outstanding outbox work or a resync start. A watermark is not an expedition result.

## 6. Platform → Game command intent schemas

### 6.1 Discovery and exact read

Pending request: H, `after_source_revision` (`cursor`), `after_operation_id`
(`uuid7` or `nil`, nil iff revision is zero), `max_entries` (`int[1,32]`).
200: `contract_version` (1), `pending` (0–32 objects, each `command` (closed command
set below), `operation_id` (`uuid7`), `source_revision` (`u64`)). Sort ascending
numeric revision then UUID bytes, strictly after cursor, no duplicate operation IDs.

Read request: H, `issuer_authority` (constant `OTERYN_PLATFORM_HIRELING_AUTHORITY`),
`command`, `operation_id`. 200 returns exact stored immutable intent bytes.
Unknown intent is `404`; different command for that issuer/id is `409`, with alarm.

### 6.2 Common immutable intent and command-specific binding

Members in this exact order:

| Field | Type / rule |
| --- | --- |
| `contract_version` | integer 1 |
| `command` | `StartHirelingExpedition` or `CollectHirelingExpedition` |
| `command_semantic_version` | integer 1 |
| `issuer_authority` | `OTERYN_PLATFORM_HIRELING_AUTHORITY` |
| `issuer_decision_id` | `uuid7`, durable Platform authorization decision |
| `source_revision` | `u64`, Platform outbox sequence, unique per issuer, allocated under commit-order lock |
| `operation_id` | `uuid7`, Platform-minted once per confirmed semantic action |
| `account_id`, `character_id`, `world_id`, `hireling_id` | `uuid7` each, authorized target, revalidated in Game |
| `expected_hireling_revision` | `u64`, optimistic check, not final authority |
| command binding | Fields below, inserted here |
| `issued_at_source`, `expires_at_source` | `time` each; issued < expires ≤ issued + 300 seconds |
| `audience` | `OTERYN_GAME_HIRELINGS` |

Start binding: `reward_table_key` (`key`), `reward_table_revision` (`u64`),
`reward_row_key` (`key`). No reward amounts, duration override, new level, chest
address, price or client clock are accepted. Game allocates ExpeditionId on commit.

Collect binding: `expedition_id` (`uuid7`), `expected_expedition_revision` (`u64`).
No destination inventory selector: **collect atomically credits the helper's Game
chest**, never the portal Wallet, browser, arbitrary character or mailbox. Proposal:
READY_TO_COLLECT means returned and awaiting this one credit; no automatic second
credit on return. Later removal of chest items happens through Game inventory rules,
outside these portal commands. Q3 asks the owner to confirm this interpretation.

Game checks existing receipt/binding before freshness or preconditions. For a new
decision it proves current account/security evidence, AccountId↔CharacterId ownership,
helper ownership, World/house authority, deployment/time/recovery fences, policy and
content availability, table/level/profession eligibility and revision preconditions.
For new operations, an issued_at_source more than 5 seconds ahead of trusted
Game time is INTENT_INVALID; expiry is checked conservatively inside the deciding
transaction, never from a browser clock. Unavailable or ambiguous trusted time
leaves the operation pending without effects. Unknown dependency/time/ownership **never grants**. Definitively foreign targets
produce a generic NOT_ELIGIBLE receipt; unavailable evidence stays pending.

Start requires placement in an eligible owned house, READY, no active expedition,
active table, budget and account helper-slot eligibility. Collect requires the
same current owner, correct expedition/helper/world, authoritative returned state
and chest capacity. Game serializes these with dialogue, lamp return, house transfer/
eviction, character transfer/deletion, Store changes and inventory writes. It may
refuse with a typed conflict rather than project stale runtime placement. A helper
cannot be silently duplicated or transported by a receipt. Details need Q1/Q4 acceptance.

### 6.3 Idempotency keys and durable effects

| Identity | Scope / behavior |
| --- | --- |
| Portal `idempotency_key` | UUIDv7 minted once before browser submission, persisted by Platform under (authenticated AccountId, key); same action bytes return same operation_id/status; changed target/action/quote is conflict; refresh, double click and HTTP retry reuse it |
| `operation_id` | Stable internal command id; key (issuer_authority, operation_id) across both command kinds, never reused even after Platform restore |
| `binding_digest` | SHA-256 of complete exact intent bytes; Game stores complete binding and receipt atomically with effect and receipt outbox |
| ExpeditionId | Game UUIDv7; unique successful start and at most one reward-credit receipt regardless of number of collect operation IDs |
| `receipt_digest` | SHA-256 of stored canonical publication bytes; immutable within a recovery generation |
| Projection identity | (source_authority, family, subject, projection_epoch, projection_revision); weekly scope additionally ordered by weekly_revision |
| Notification | (AccountId, ExpeditionId, `RETURNED`); duplicate publications do not notify twice |
| Store delivery operation / EntitlementId | Existing Store identities, never reused as helper expedition operation IDs |

A browser key is private orchestration data, not a credential, and is not sent to
Game. Source revision identifies the immutable outbox record, not a gameplay fence.
Exact retries return the same decision bytes and produce no XP, item, counter,
revision or notification increment. Reusing an operation with changed bytes keeps
the original receipt, grants nothing and alarms; do not overwrite it with a second
rejection. A second collect ID for an already credited expedition is terminal
ALREADY_COLLECTED with no second reward. Check the expedition's claim identity in
addition to command idempotency.

## 7. Game → Platform command receipts and reconciliation

Receipt members, in order: H (`PublishHirelingCommandReceiptV1`),
`issuer_authority` (constant above), `command`, `command_semantic_version` (1),
`operation_id` (`uuid7`), `binding_digest` (`hex64`), `outcome` (`COMMITTED` or
`REJECTED`), `result_code` (table below), `account_id`, `character_id`, `world_id`,
`hireling_id` (`uuid7`, all equal intent), then committed-only members, then
`recovery_generation` (`u64`), `decided_at` (`time`, Game transaction time).
All echo fields must match the stored intent; rejected receipts carry no gameplay facts.

Committed-only members, in order: `expedition_id` (`uuid7`; equals collect target),
`hireling_revision` (`u64`), `expedition_revision` (`u64`), `projection_epoch`
(`u64`), `state_projection_revision` (`u64`), `expedition_projection_revision`
(`u64`), `weekly_revision` (`u64`), `chest_ids` (0–1 `uuid7`, empty on start,
one on collect), `chest_revisions` (0–1 `u64`, same cardinality).
The receipt binds source revisions for UX reconciliation; actual item amounts are
in the pinned reward table/expedition projection, not a second grant instruction.

| Result code | Outcome / meaning |
| --- | --- |
| `HIRELING_OK` | COMMITTED, all effects durable |
| `HIRELING_NOT_ELIGIBLE` | REJECTED, no owner/subject/house disclosure to foreign callers |
| `HIRELING_STALE_PRECONDITION` | REJECTED, helper/expedition revision changed |
| `HIRELING_TABLE_UNAVAILABLE` | REJECTED, definitive retired/unknown quote |
| `HIRELING_BUSY` | REJECTED, start conflicts with ongoing expedition |
| `HIRELING_WEEKLY_LIMIT` | REJECTED, starts or reward cap exhausted |
| `HIRELING_NOT_RETURNED` | REJECTED, collect too early under Game time |
| `HIRELING_ALREADY_COLLECTED` | REJECTED, this expedition already credited by another action |
| `HIRELING_CHEST_FULL` | REJECTED, no partial credit or XP award |
| `HIRELING_STATE_CONFLICT` | REJECTED, definite lifecycle/session/house/inventory conflict |
| `HIRELING_INTENT_EXPIRED` | REJECTED, no prior receipt and technical TTL elapsed |
| `HIRELING_INTENT_INVALID` | REJECTED, authenticated readable intent with invalid binding/time |
| `HIRELING_UNSUPPORTED` | REJECTED, unsupported command or semantic version with readable identity |

HIRELING_BUSY and HIRELING_WEEKLY_LIMIT apply only to start.
HIRELING_NOT_RETURNED, HIRELING_ALREADY_COLLECTED and HIRELING_CHEST_FULL apply
only to collect. HIRELING_TABLE_UNAVAILABLE applies to a start quote; a missing
pinned running-expedition table is an unavailable dependency, never permission to
substitute a new table. Other codes apply to both commands.

These are proposed closed codes, not additions to `CHAR_CMD_*` or `STORE_*`.
Unsupported/malformed input without a trustworthy operation identity is quarantined,
not fabricated into an unbound receipt. An unsupported command name may be echoed
only if it is 1–64 ASCII letters/digits/underscore and matches Platform's stored
intent. No generic internal error may be terminal unless no-commit is proven.

Publication 200 body: `{"contract_version":1,"result":"accepted"}` for first
stored or byte-identical current receipt. Platform validates intent, digest, echoes,
command/outcome member set, code and generation in one transaction. `400` schema/
echo error; `401` peer/source; `404` no intent; `409` binding/current receipt conflict
or invalid recovery generation; `413` size; `429` limit; `503` unavailable/off.
All non-200 responses are empty and never acknowledge.
All new routes share `400` malformed/canonical/version/schema failure, `401` wrong
peer/source, `413` length ceiling, `429` limiter and `503` disabled/misconfigured/
unavailable behavior. Ingest 200 is only accepted/superseded with the exact bodies
in §8; schema refusals store nothing and do not advance a watermark. A receipt
conflict preserves the first stored bytes. A read/list failure never decides a command. For `404`/`409` Game retains
the receipt, stops republishing that operation and alarms; neither side repairs the
other's truth. Platform remains unresolved. `429`/`503`/loss retries exact bytes.

Reconcile request is H (`ListHirelingCommandStatesV1`) plus the pending cursor and
max_entries. Response is `contract_version` (1), `operations` (0–32 entries, same
ordering/cursor, **all** states). Each entry: `command`, `operation_id`,
`source_revision`, `state` (`PENDING`, `REOPENED`, `COMMITTED`, `REJECTED`),
`receipt_digest` (`hex64 | null`), `recovery_generation` (`u64 | null`); last two
null iff PENDING/REOPENED. Game compares these with its durable receipts, republishes
missing acknowledgements and alarms on contradictions. Matching state alone is
not grant proof; never mint/delete/repair gameplay from a comparison.

Network timeout, missing/404 read, lost acknowledgement, process crash or exhausted
retry budget produces Platform-local AMBIGUOUS/RECONCILING, **not REJECTED**. No
receipt means pending, including after TTL: only Game can issue the terminal expired
receipt. Retry/reconcile the same operation. A 300-second-old pending operation is
alarmed; after 24 hours unresolved it requires operator reconciliation, remains
nonterminal and cannot be replaced blindly. Transient Game dependency failure rolls
back all effects, stores no terminal receipt and leaves the original operation pending.
A genuinely terminal rejection guarantees that ID cannot later commit; a new user
attempt requires fresh gates, a fresh browser key and a fresh operation ID.

## 8. Ordering, watermarks, replay and restore

Command lists are bounded discovery, not ownership/current-state evidence. Page from
("0", nil), continue by last key, restart after a short page, and rescan after at
most eight full pages or 30 seconds, whichever comes first (Character consumer
convention). Issuance locks revision allocation through commit; no fresh intent may
commit behind the cursor. Reopened commands retain original revisions/times and
must be found by rescan. Slow or expired work is never omitted merely for being old.

Projection revisions advance in the transaction that changes their source. Apply
complete replacements by (epoch, revision), scoped to subject. Lower pair answers
200 `{"contract_version":1,"result":"superseded"}`; equal content answers accepted;
equal pair with different content is `409`, preserving the first. Content comparison
excludes only `source_observed_at`, as in public projections; all other fields are
bound. Higher pair atomically replaces. Revisions for complete snapshots may skip
because intermediate versions coalesce; do not invent a contiguous event stream.
Missing expedition records, unacknowledged outbox rows or incomplete baselines still
block family completeness; row count equality alone is not proof of replay completion.

Watermarks are sent at least every 10 seconds, monotone within epoch; accept equal
or advancing complete_through, refuse regressions/contradictory or future-dated time.
Observed_at ≥ complete_through and no later than Platform now + 1 second. For testing/
preproduction, fresh iff the current epoch has a complete baseline and
`platform_now - complete_through + 1 <= 30` seconds and complete_through ≤ now + 1.
Check at every serve, including caches. No stale display window: an unknown/stale/
invalid source, ownership or privacy decision hides private facts and disables new
commands. Other independent family scopes may continue only when causality permits.
A receipt may show its terminal command status while the detail projection says
"updating" until its cited revisions arrive; never fabricate the new chest/level.

Game replay uses its durable source/outbox; no time-based deduplication. Operator
resync publishes every authorized subject/table and retained expedition into a new
Platform candidate generation at a known baseline cut, then drains the committed
tail. Tombstones remove former-owner entries. Completeness watermark stays below
the cut until every baseline and tail record through it is acknowledged. Platform
switches its read pointer atomically only after source integrity, current ownership
and privacy fences pass; rollback cannot revive an older owner allow. V1 has no
portal-driven Game resync command. Replay beyond retained expedition history must
report that history unavailable, not silently synthesize it.

Use the existing [Game restore reconciliation contract](OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md)
and its `PublishGameRestoreNoticeV1`, never a new helper restore route. Acceptance
must register this command family in that notice's recovery barrier before activation.
A Game restore raises the nonrollback projection epoch, invalidates all lower-epoch
helper families, and reopens affected receipts by recovery_generation, never by
wall-clock dates. Preserve superseded receipt history. Game fences all decisions
until reconciliation; survived receipts republish exact bytes, lost receipts decide
the original intent under the new generation and original expiry. No blind regrant
of chest loot/XP. Only a notice-authorized reopen permits replacement receipt bytes;
reject receipts below the current restore generation floor except exact replay of a
still-current survived receipt under the parent rule. Do not acknowledge the restore
barrier or reopen affected features while helper decisions remain ambiguous.

Invalid/poison records stay quarantined with bounded reason/reference; no watermark
passes a causally required undelivered record. Unknown or ambiguous state denies.
Counters, epochs, identity conflicts and unbounded/overflow quantities fail closed;
arrival time and UI timestamps never order mutations.

## 9. Store entitlement delivery and in-game conversation

Purchase remains a Store purchase: product/version → order → EntitlementId → stable
Store delivery operation → Game receipt. Reuse `PurchaseStoreOfferV1`,
`ReadStoreOperationIntentV1`, `PublishStoreDeliveryReceiptV1` and the existing Store
receipt outbox/idempotency and restore behavior; no helper purchase command, direct
Wallet debit, entitlement boolean in a browser or duplicated delivery protocol.
Payment/order success is not a helper-created receipt.

The pinned Game already has `HirelingStoreAction` reservations and effects for
Hire/Unlock/Rename/SexChange. Preserve those action bindings, canonical names and
character-wide service unlock meanings. **The referenced Platform Store v1 unit_kind
set is only PREMIUM_TIME/COSMETIC/ITEM. It has no helper/profession grant binding.**
Do not smuggle a helper grant as ITEM/COSMETIC or reinterpret the numeric Game unlock
ID as a Platform EntitlementId. Before T92 purchase activation the two owners must
accept a Store wire/product mapping amendment binding canonical AccountId,
CharacterId/WorldId, product/version, EntitlementId, delivery operation and immutable
HirelingStoreAction customization, with exact receipt facts linking resulting
HirelingId/unlock. Q5 is a blocking dependency; this proposal adds no undocumented
Store members or enum values. Existing lamp delivery, reservation and receipt proof
must be reused, not bypassed by a portal helper row. Prices, refunds and extra-helper/
profession/outfit offers need approved product policy. No automatic destructive
clawback of a deployed helper or already credited rewards on a payment reversal.

In-game conversation uses the existing hireling/NPC dialogue authority and fenced
session. It offers the same approved profession/row choices and collection/chest
semantics as portal commands, with Game-local command identity/receipt. It runs the
same account-week budget, helper/expedition claim lock and chest transition. Dialogue
receipts do not impersonate Platform issuer operations; their effects are exported
with `origin: GAME_DIALOGUE` and empty portal start IDs. A collect by dialogue races
portal collect safely: one wins and a later different portal operation gets
ALREADY_COLLECTED. Existing command 220/470 binary messages/capabilities keep their
current meaning; adding expedition dialogue/wire requires Game-owned acceptance.

## 10. Privacy, retention and flags

Only the currently authenticated owner may read a helper, expedition, quote or
receipt. Resolve AccountId from the session server-side, prove current accepted
AccountId↔CharacterId ownership independently of this projection, and enforce
object authorization on every read/mutation and notification. Former owners cannot
read retained receipts through historical account associations. Foreign, missing,
hidden and unauthorized objects have indistinguishable owner-facing responses.
Unknown ownership/privacy dependencies deny. Private pages/APIs are `no-store`,
excluded from shared caches, public indexes, rankings, house pages and share links.
No public join of this feed to account/alternate-character data is permitted.

Never send credentials, cookies/tokens, payment/provider/voucher payloads, IP/email,
full dialogue/chat text, other accounts' identities, home coordinates, ChannelId,
node/topology, session/lease/fencing tokens, actor handles, full inventory/character
blobs, loot RNG state, unpublished reward rules or item-instance authority. Only
AccountId and the canonical subject IDs needed by the private schemas cross this
channel. Logs contain operation/family, result class and timings, not IDs/names/bodies;
metrics have bounded labels. Quarantine payloads are owner-restricted, never ordinary logs.

Proposed retention, requiring Q6 acceptance before enablement:

- Current state/active table: retain while applicable; on owner removal immediately
  fence serving and purge derived private views within 24 hours. Tombstone/revision
  fences remain to prevent resurrection. Backups inherit access controls and deletion policy.
- Expedition detail: 90 days after terminal collection/cancellation; unresolved
  expeditions remain until reconciled. Game can resync retained history; older history
  explicitly unavailable. A hidden subject is denied immediately, independent of cleanup.
- Full immutable command intents/receipts: at least 90 days after terminal resolution
  and until all commercial/restore obligations finish, whichever is later. Never
  age out pending/ambiguous work. Thereafter keep the minimal ID, fingerprint, terminal
  outcome, recovery/revision fence and claim deduplication marker for the authority's
  lifetime; purge private detail under approved erasure policy. Old IDs can never
  become new effect authority. Game's durable claim markers survive normal pruning.
- Bounded quarantine diagnostic payloads: at most 7 days; keep safe reference/reason
  while unresolved. No mandatory retention of raw request bodies in general logs.

All proposed helper flags default **false**: `hirelings.projection_enabled`,
`hirelings.commands_enabled`, `hirelings.start_enabled`, `hirelings.collect_enabled`,
`hirelings.purchase_enabled`, `hirelings.dialogue_expeditions_enabled`. Allowed
sources/identities default empty; no wildcard. These are semantic configuration
names for later mapping, not new environment variables or current Rust APIs.
New command issuance requires global commands + specific command flag + fresh
projection/policy/ownership gates. Purchase also requires accepted Store amendment
and Store purchase/delivery flags. Dialogue enablement requires the same Game rules.

Disabling issuance blocks new work but preserves pending records and existing
reconciliation. Keep a separately controlled `hirelings.reconciliation_enabled`
(default false) for pending list/read/receipt/reconcile and restore recovery after
issuance is disabled; if this exchange is off, routes return `503` and no outcome is
inferred. Projection-off returns `503`, private presentation unavailable. Rollback
never destroys outboxes/receipts or retries uncertain commands through another authority.

## 11. Versioning, compatibility and acceptance

No parent operation's fields, limits, enums, identity purpose or byte order change.
This new family is off until both owners accept schemas/fixtures at pinned producer
and consumer revisions. Any changed member/path/type/bound/result, weekly meaning,
chest timing, privacy, idempotency or receipt terminality requires a new contract
version and owner review. A new profession/table row within accepted bounds is a
new content/table revision; changing existing revision content is a conflict.
Unsupported versions fail closed; no best-effort decoding, automatic downgrade or
mixed-version authority. Retain old decoders and immutable decision bindings while
old operations/expeditions can reconcile; never translate an ambiguous operation
into a different version/new ID. Negotiation/enablement is explicit configuration,
not a client-selected transport.

Platform acceptance checklist (future implementation; none claimed by this draft):

- [ ] Owner accepts final Game mapping and Store amendment; pin contract/producer/consumer SHAs.
- [ ] T92 stays off until G-14 evidence passes; T26/T29/T36 keep their own authority.
- [ ] mTLS purpose separation, all size/member/type/null/version/unknown-source negative cases.
- [ ] Owner/foreign/former-owner read and command denial, CSRF, rate limits, current ownership failure.
- [ ] Durable browser key/operation mapping, same-key conflict, outbox and no timeout replacement.
- [ ] Source replacement, weekly revision fence, tombstones, watermarks, epoch/gap/quarantine and baseline+tail rebuild.
- [ ] Receipt/digest/generation validation, exact replay, terminal rejection versus ambiguous UX, post-commit response loss.
- [ ] Receipt-cited projection convergence, one return notification, private cache/log/retention behavior.
- [ ] Restore reopen and reconciliation, issuance-off drain and rollback without duplicate effects.

Game acceptance checklist (future implementation; none claimed by this draft):

- [ ] Coordinator resolves Q1–Q7 and confirms existing hireling/service/Store type mapping.
- [ ] Deterministic explicit table, professions/levels, account-week budgets and overflow/bound proofs.
- [ ] Current account/character/helper/house authority, trusted-time and recovery fences; unknown denies.
- [ ] Start/collect decision, XP, chest, quota, receipt and both outboxes atomic; no partial rewards.
- [ ] Same ID replay, changed binding refusal, two collect IDs, dialogue/portal and concurrent helper/week races.
- [ ] Capacity-full retry as a new user attempt only after terminal receipt; no loss/second credit.
- [ ] Weekly boundary, late collection, cancellation, policy retirement and table changes preserve pinned rewards.
- [ ] Lamp return, house eviction/transfer, character transfer/delete and session races have explicit results.
- [ ] Receipt survives failed publication; projection watermark cannot pass undelivered effects.
- [ ] Restore fence/epoch, expired lost intent, survived receipt replay, retention and operator resync tested.
- [ ] Shared golden vectors (including every limit and one past it), exact-revision cross-repo integration before enablement.

## 12. Golden JSON vectors

These are synthetic, complete candidate messages; each fenced JSON block is compact
canonical bytes except the Markdown newline, which is **not** part of the payload.
Examples are not production identifiers or accepted gameplay/product policy.

Valid reward table (one deterministic row; no chance field):

```json
{"contract_version":1,"operation":"PublishHirelingRewardTableV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","reward_table_key":"home-v1","reward_table_revision":"7","projection_epoch":"1","projection_revision":"7","source_observed_at":"1790000000","content_revision":"9","availability":"ACTIVE","weekly_policy_revision":"1","week_anchor_at":"1789948800","weekly_expedition_limit":7,"weekly_item_caps":[{"item_key":"oteryn:item.tibia.i2148","quantity":700}],"rows":[{"reward_row_key":"miner-1-short","profession_key":"miner","minimum_level":1,"maximum_level":10,"destination_key":"quarry","duration_seconds":3600,"profession_xp":10,"rewards":[{"item_key":"oteryn:item.tibia.i2148","quantity":100}]}]}
```

Valid owner snapshot before start (weekly account state, empty provisioned chest):

```json
{"contract_version":1,"operation":"PublishHirelingStateV1","source_authority":"oteryn:game:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"20","source_observed_at":"1790000000","world_id":"01934f10-7c02-7001-805b-3b1122334401","hirelings":[{"hireling_id":"0192a000-0000-7000-8000-000000000010","owner_character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Alex","hireling_revision":"3","placement":[{"house_key":"home.17"}],"professions":[{"profession_key":"miner","level":1,"experience":"0","next_level_experience":"100"}],"availability":"READY","active_expedition_ids":[],"chest":[{"chest_id":"0192a000-0000-7000-8000-000000000030","chest_revision":"1","capacity_units":1000,"used_units":0,"items":[]}]}],"weekly":{"weekly_policy_revision":"1","week_start_at":"1789948800","week_end_at":"1790553600","weekly_expedition_limit":7,"started_count":0,"items":[{"item_key":"oteryn:item.tibia.i2148","cap_quantity":700,"reserved_quantity":0,"delivered_quantity":0}],"weekly_revision":"1"}}
```

Valid start intent:

```json
{"contract_version":1,"command":"StartHirelingExpedition","command_semantic_version":1,"issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expected_hireling_revision":"3","reward_table_key":"home-v1","reward_table_revision":"7","reward_row_key":"miner-1-short","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_HIRELINGS"}
```

Valid committed start receipt; binding_digest hashes the exact start block above:

```json
{"contract_version":1,"operation":"PublishHirelingCommandReceiptV1","source_authority":"oteryn:game:primary","issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","command":"StartHirelingExpedition","command_semantic_version":1,"operation_id":"0192a000-0000-7000-8000-000000000001","binding_digest":"63b6e07ae0fb33daaa02f306618440169a6ae708cb53e3bc96aca181d5a01cba","outcome":"COMMITTED","result_code":"HIRELING_OK","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expedition_id":"0192a000-0000-7000-8000-000000000020","hireling_revision":"4","expedition_revision":"1","projection_epoch":"1","state_projection_revision":"21","expedition_projection_revision":"1","weekly_revision":"2","chest_ids":[],"chest_revisions":[],"recovery_generation":"3","decided_at":"1790000002"}
```

Valid collect intent (after Game has published READY_TO_COLLECT revision 2):

```json
{"contract_version":1,"command":"CollectHirelingExpedition","command_semantic_version":1,"issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a2","source_revision":"18","operation_id":"0192a000-0000-7000-8000-000000000002","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expected_hireling_revision":"5","expedition_id":"0192a000-0000-7000-8000-000000000020","expected_expedition_revision":"2","issued_at_source":"1790003610","expires_at_source":"1790003910","audience":"OTERYN_GAME_HIRELINGS"}
```

Valid committed collect receipt (same successful execution as the next record):

```json
{"contract_version":1,"operation":"PublishHirelingCommandReceiptV1","source_authority":"oteryn:game:primary","issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","command":"CollectHirelingExpedition","command_semantic_version":1,"operation_id":"0192a000-0000-7000-8000-000000000002","binding_digest":"b12dff40a880f5b018789bcbdc47e05f1338a513594e9de26176ba145d36eda5","outcome":"COMMITTED","result_code":"HIRELING_OK","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expedition_id":"0192a000-0000-7000-8000-000000000020","hireling_revision":"6","expedition_revision":"3","projection_epoch":"1","state_projection_revision":"23","expedition_projection_revision":"3","weekly_revision":"3","chest_ids":["0192a000-0000-7000-8000-000000000030"],"chest_revisions":["2"],"recovery_generation":"3","decided_at":"1790003611"}
```

Valid resulting expedition record (actual rewards credited once to chest):

```json
{"contract_version":1,"operation":"PublishHirelingExpeditionV1","source_authority":"oteryn:game:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","expedition_id":"0192a000-0000-7000-8000-000000000020","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790003611","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expedition_revision":"3","reward_table_key":"home-v1","reward_row_key":"miner-1-short","reward_table_revision":"7","content_revision":"9","weekly_policy_revision":"1","profession_key":"miner","destination_key":"quarry","level_at_start":1,"state":"COLLECTED","started_at":"1790000002","returns_at":"1790003602","week_start_at":"1789948800","origin":"PORTAL","start_operation_ids":["0192a000-0000-7000-8000-000000000001"],"collect_operation_ids":["0192a000-0000-7000-8000-000000000002"],"rewards":[{"item_key":"oteryn:item.tibia.i2148","quantity":100}],"profession_xp_awarded":10,"collected_at":["1790003611"],"chest_ids":["0192a000-0000-7000-8000-000000000030"]}
```

Valid terminal refusal for that collect shape when the chest is full in an
alternative execution; it cannot coexist with a committed outcome for the same ID:

```json
{"contract_version":1,"operation":"PublishHirelingCommandReceiptV1","source_authority":"oteryn:game:primary","issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","command":"CollectHirelingExpedition","command_semantic_version":1,"operation_id":"0192a000-0000-7000-8000-000000000002","binding_digest":"b12dff40a880f5b018789bcbdc47e05f1338a513594e9de26176ba145d36eda5","outcome":"REJECTED","result_code":"HIRELING_CHEST_FULL","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","recovery_generation":"3","decided_at":"1790003611"}
```

Valid private watermark:

```json
{"contract_version":1,"operation":"PublishHirelingWatermarkV1","source_authority":"oteryn:game:primary","family":"hireling_state","projection_epoch":"1","complete_through":"1790003611","observed_at":"1790003612"}
```

Invalid pending request: numeric cursor, nil mismatch and page size over 32; `400`,
no progress or command state change:

```json
{"contract_version":1,"operation":"ListPendingHirelingCommandsV1","source_authority":"oteryn:game:primary","after_source_revision":17,"after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":33}
```

Invalid collect intent: null revision and client-selected reward amount; schema
refusal/quarantine, never a grant, even though IDs are well formed:

```json
{"contract_version":1,"command":"CollectHirelingExpedition","command_semantic_version":1,"issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a2","source_revision":"18","operation_id":"0192a000-0000-7000-8000-000000000002","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expected_hireling_revision":null,"expedition_id":"0192a000-0000-7000-8000-000000000020","expected_expedition_revision":"2","reward_quantity":9999,"issued_at_source":"1790003610","expires_at_source":"1790003910","audience":"OTERYN_GAME_HIRELINGS"}
```

Invalid conflicting reuse: this otherwise well-formed start uses the valid start's
operation_id but changes its quote row. Game keeps the first binding/receipt,
executes nothing new and alarms; no replacement receipt:

```json
{"contract_version":1,"command":"StartHirelingExpedition","command_semantic_version":1,"issuer_authority":"OTERYN_PLATFORM_HIRELING_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","world_id":"01934f10-7c02-7001-805b-3b1122334401","hireling_id":"0192a000-0000-7000-8000-000000000010","expected_hireling_revision":"3","reward_table_key":"home-v1","reward_table_revision":"7","reward_row_key":"miner-1-long","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_HIRELINGS"}
```

## 13. Open questions for the Game owner (coordinator)

1. **Identity, ownership and limits:** accept reusing character-owned HirelingId and
   a shared AccountId budget? Is the plan's three-per-account cap for expedition-enabled
   helpers only, or all existing identities? How do the existing ten-per-character
   identities, character-wide service unlocks and transfers migrate without loss?
2. **Gameplay table/progression:** accept fixed deterministic rewards/XP, 1–12-hour
   rows, levels 1–100 and the proposed bound of 16 professions? Supply approved
   profession/destination keys, content/row revisions, XP curves, weekly start/item
   caps and confirm Monday UTC reset/policy changes at boundaries. Example numbers
   are fixtures, not a requested economy decision.
3. **Return versus collection:** confirm READY_TO_COLLECT followed by explicit
   collect-to-chest (rather than automatic chest credit at return and collect meaning
   withdrawal). Confirm chest capacity measured in item units, 32 item kinds, and
   Game-only withdrawal. Any different meaning must revise these schemas before acceptance.
4. **Lifecycle/concurrency:** which house, placement and session conditions allow
   portal start/collect? What happens during lamp return, eviction/house transfer,
   character sale/deletion and uncollected returns? Who may cancel, and how are old
   account-week reservations and former-owner private history resolved?
5. **Store mapping blocker:** which approved product/grant profile and Store wire
   amendment carries `HirelingStoreAction` customization, new expedition-profession
   unlocks and resulting identity facts? Confirm Profile C, game-side reservation
   before debit, product limits and refund/reversal handling. No existing unit kind
   is assumed to authorize it.
6. **Operational/privacy policy:** accept the proposed freshness, cadence, backlog,
   wire ceilings, retention and minimal lifetime deduplication fences? Confirm
   erasure handling, dedicated source/peer purposes and replay/resync sizing. A
   production PKI, retention and measured SLO decision remains required separately.
7. **Acceptance and restore integration:** accept the proposed names and new private
   families without adding fields to existing public/Character routes? Identify the
   Game owner/module and exact implementation/fixture revisions; approve registration
   in the existing restore barrier and the two-sided test/enablement packet. G-14 and
   T92 command/purchase activation remain blocked until these decisions are explicit.
