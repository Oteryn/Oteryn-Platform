Status: PROPOSED revision 2 by Platform, pending acceptance by the game owner (coordinator).

# Character services command contract proposal (G-3)

## Revision 2 changes

Owner review via the programme coordinator: **ACCEPT-WITH-NOTES**. This revision records the answers without changing the pending-acceptance status.

- Game chooses deletion effective time; 60 days is Game policy, with offline cancellation strictly before that time. Released names remain locked for 30 days (D183).
- The v2 owner-only service projection has independent revisions under the shared epoch fence; service changes do not bump LCFA v1 revisions.
- Reservation precedes the command; settlement follows its receipt. Restore loss earns a free replacement service, with no game reversal on chargeback.
- Sex change remains unsupported with its flag off; world transfer is outside this slice. Game commits to a new repeatable-service table; restore horizon equals backup retention.
- Oversize requests return 400; public-family and online-time boundaries follow G-4 and G-8. Owner answers are recorded in §11.

## 1. Purpose, authority and evidence

This proposes the G-3 extension of GAME-CHAR-CMD for portal module **T46**, `crates/characters/src/manage*`, in §10 and the gate table of [the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md). T46 depends on T22 (characters), T26 (Wallet) and G-3. Its in-slice services are schedule deletion, cancel deletion and rename. Sex change is unsupported (flag off); world transfer is outside this slice. Reserved shapes for those two commands below do not authorize issuance or activation. Creation and Bazaar account ownership transfer retain their existing contracts. World transfer preserves AccountId and CharacterId and changes WorldId; it is not `TransferCharacterOwnership` or a Channel move.

Platform owns authenticated owner UX, CSRF/recent-auth/rate limits, product flags, price and charge reservation, entitlement reservation/consumption, orchestration, audit and privacy. Game Character Authority owns current ownership, lifecycle, name registry, sex, world placement, eligibility, serialization, execution and durable receipts. The game owner/coordinator accepts or amends this proposal and supplies the producer contract and implementation. Acceptance of this document alone does not implement or activate G-3.

Platform references read for this proposal:

- [Character Authority command contract](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md): operation identity, authority, terminality, ambiguity and races.
- [Entitlement game delivery contract](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): single-use service reservation before mutation and consumption after completion.
- [Public game data projection contract](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md): independent public projection and privacy fences.
- [Store wire contract](OTERYN_V2_STORE_WIRE_CONTRACT.md): canonical byte encoding, decimal strings, mTLS peer guard and outbox conventions. This document is itself PROPOSED; it is precedent, not acceptance authority. Store purchase's debit-before-delivery is not substituted for this service's reserve-before-command saga.
- `app/CharacterAuthority/CharacterCommandWire.php`: existing route names, issuer/audience, bounds and receipt shapes.
- [Game restore reconciliation contract](OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md): recovery-generation selection and reopening.

Read-only Game evidence is pinned to **Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87**, retrieved outside the Platform checkout. Paths below refer to that commit, not moving main:

| Game path | Evidence used |
|---|---|
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`, §§3–8, 12–15 | Accepted GAME-CHAR-CMD-0: Game pulls intents, exact bytes, receipts, replay and restore; a changed wire needs a new contract version |
| `crates/gameplay/native-admission-source/src/character_commands.rs` | `Key`, `PendingEntry`, `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1`, `PublishCharacterCommandReceiptV1`; poll/rescan/backoff bounds |
| `crates/gameplay/durability/src/durability/character_command.rs` | `CreateWire`, `CreateIntent`, canonical binding, transaction and recovery fence; only creation is decided at this pin |
| `apps/game-server/migrations/0087_character_commands.sql` | `game_character_command_receipts`, immutable receipt bytes, operation uniqueness and per-variant revision uniqueness |
| `crates/gameplay/native-admission-source/src/account_characters.rs` | `CharacterSummary`, `AccountSnapshot`, `WatermarkFacts`, `complete_through`, purpose-specific publisher identity |
| `crates/gameplay/durability/src/durability/account_characters_projection.rs` | `game_character_account_projection_outbox`, snapshot read, acknowledgement and epoch/revision semantics |
| `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` | LCFA owner-only snapshot, epoch fence, resync and liveness; v1 has no deletion timestamps |
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md`; `crates/gameplay/native-admission-source/src/public_projections.rs` | Separate public snapshots, tombstones, world-only placement and watermarks |
| `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, §6.1 | CHAR-NAME-1, global comparison key and accepted 30-day released-name hold |
| `apps/game-server/migrations/0134_character_sex.sql`; `crates/gameplay/durability/src/durability/character_appearance/character_sex.rs` | Stored sex 1 = Female, 2 = Male, historical NULL denies; creation sex is currently immutable |

**Owner decisions:** the deletion period of 60 days is Game policy rather than a wire invariant. Released-name locks last **30 days**, following Game decision D183 (`docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md:171`), after rename or terminal deletion. Sex change remains unsupported and its flag stays off; creation sex remains immutable. World transfer is outside this slice. Updated producer contracts and exact-revision conformance remain activation prerequisites.

## 2. Directions and transport

Logical **Platform → Game command** means Platform commits an immutable intent and Game pulls it. Physical HTTP is always **Game → Platform**. There is no inbound game listener, browser-to-game command, shared SQL, broker, or new signing primitive.

Logical **Game → Platform ingest** has three independent flows: durable command receipts, private service-state snapshots proposed in §6, and existing unchanged LCFA/public projection publications. A receipt confirms one operation; a snapshot describes current state; neither replaces the other.

Reuse HTTP/1.1 over TLS 1.3, purpose-bound mTLS certificates and configured source authorities. Character-command certificates are accepted only on command routes; the private projection publisher has a separate dedicated identity, disjoint from commands, LCFA, public projections, Store, runtime and native evidence. Only Character Authority hosts hold these identities. A body cannot select a peer identity, audience or authority. Platform uses the existing trusted TLS terminator guard (`SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, exact configured certificate subject, constant-time comparison). Reject spoofed forwarding headers; validate server certificate/hostname on Game. No bearer credentials, application signatures, signed browser payloads or HMAC keys are introduced. Trust is channel authentication plus immutable full-intent binding and durable replay evidence; a SHA-256 fingerprint is not a signature.

This proposes a **version 2 wire profile**, not an in-place addition to frozen v1. Compiled routes retain the established shapes with a versioned prefix:

| POST path | Operation | Request / response maximum bytes |
|---|---|---|
| `/internal/v2/game-auth/character-commands/pending` | `ListPendingCharacterCommandsV2` | 512 / 8192 |
| `/internal/v2/game-auth/character-commands/read` | `ReadCharacterCommandIntentV2` | 512 / 4096 |
| `/internal/v2/game-auth/character-commands/receipt` | `PublishCharacterCommandReceiptV2` | 1024 / 256 |
| `/internal/v2/game-auth/character-services/state` | `PublishCharacterServiceStateV2` | 2048 / 256 |
| `/internal/v2/game-auth/character-services/watermark` | `PublishCharacterServiceWatermarkV2` | 512 / 256 |

These paths/operations are proposed compiled names, absent at the inspected pin. Existing v1 paths, messages and consumers remain byte-for-byte unchanged; v1 lists never expose v2 operations. Both versions share the issuer-wide operation identity namespace. Upgraded Game must process v1 and v2 without separate writers deciding the same key. Deployment requires explicit mutual version support, never probing by issuing a paid command. The existing create-specific receipt storage is evidence of the pattern, not a reusable service schema: its unique character_id and create-only variant constraints cannot represent repeated services on one character. **Producer commitment:** Game will add a new table for repeatable services; migration `apps/game-server/migrations/0087_character_commands.sql` permits only one entry per character and cannot serve this purpose. Game owns the new storage design and implementation.

Reuse connect 1 s, TLS handshake 2 s, exchange 3 s, one in-flight exchange per consumer/publisher. Command poll is 1 s, backoff on 429/503 at most 10 s. The service-state publisher reuses LCFA's 1 s idle, 3 s watermark cadence, at most 10 s between watermark attempts. Timeouts mean no proven delivery.

Every body is one UTF-8 canonical JSON object, no BOM/newline, whitespace, escapes/backslash, duplicate/unknown/missing members; member order is the table order. Nesting at most 3. Use compact unescaped UTF-8/slashes encoding as the existing wires. `Content-Type: application/json`; reject Transfer-Encoding and non-identity Content-Encoding. Content-Length, if present, is canonical decimal of at most 10 digits within the route bound. Empty/oversize actual bodies and declared oversize return 400. All responses are `Cache-Control: no-store`. Non-200 bodies are empty.

## 3. Types and envelopes

All fields are required and non-null unless explicitly marked nullable. Fields belonging to another variant are **absent**, not null.

| Type | Exact representation and bounds |
|---|---|
| `uuid7` | String matching `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`; non-nil |
| `u64` | String matching `[1-9][0-9]{0,19}`, value 1..18446744073709551615; never JSON number |
| `cursor` | `"0"` or `u64` |
| `time` | `u64`, Unix seconds, not milliseconds |
| `authority` | String `[A-Za-z0-9._:/-]{1,128}`, exact configured source authority |
| `command` | String `[A-Za-z0-9_]{1,64}`; supported set in §4 |
| `wire_name` | NFC UTF-8 string 1..64 bytes; no Cc/Cf, quote or backslash |
| `requested_name` | ASCII letters in words separated by one space, 2..29 bytes, no outer spaces; `^[A-Za-z]+( [A-Za-z]+)*$` plus byte bound |
| `sex` | JSON integer 1 (Female) or 2 (Male), matching Game's `Sex`; unknown is not guessed |
| `hex64` | String `[0-9a-f]{64}`, SHA-256 of canonical bytes, internal fingerprint only |

Common request head, in order: `contract_version` (integer exactly 2), `operation` (exact route operation), `source_authority` (`authority`). List request appends `after_source_revision` (`cursor`), `after_operation_id` (`uuid7`, or nil `00000000-0000-0000-0000-000000000000` only with cursor `"0"`), `max_entries` (integer 1..32). A zero cursor requires nil and a positive cursor requires non-nil.

List 200 body: `contract_version` (2), `pending` (array 0..32). Entry fields in order: `command`, `operation_id` (`uuid7`), `source_revision` (`u64`). Keys are unique and strictly ascending by numeric revision then UUID bytes. Read request appends `issuer_authority` (constant `OTERYN_PLATFORM_CHARACTER_AUTHORITY`), `command`, `operation_id`. Read 200 body is stored intent bytes verbatim. Pending includes expired and restore-reopened intents until a current receipt exists.

Intent prefix in order:

| Field | Type / meaning |
|---|---|
| `contract_version` | Integer 2; also fixes these command semantics |
| `command`, `variant` | Exact pair from §4 |
| `issuer_authority` | `OTERYN_PLATFORM_CHARACTER_AUTHORITY` |
| `issuer_decision_id` | `uuid7`, immutable Platform authorization decision |
| `source_revision` | `u64`, issued under Platform issuer lock, committed in increasing order |
| `operation_id` | `uuid7`, one semantic attempt |
| `character_id` | `uuid7`, game-owned CharacterId |
| `account_id` | `uuid7`, expected current owner, not an owner reassignment |
| `expected_character_revision` | `u64`, optimistic game revision; Game must match under lock |

Append the command-specific fields in §4, then the suffix in order: `issued_at_source` (`time`), `expires_at_source` (`time`), `audience` (exact `OTERYN_GAME_CHARACTER_AUTHORITY`). Require `issued < expires <= issued + 300`, checked without integer overflow. Issued time more than 5 s ahead of trusted Game time is invalid. Unavailable/ambiguous trusted time permits no execution. The technical 300 s TTL is **not** the deletion grace period.

## 4. Commands and authoritative transitions

All commands revalidate account security, ownership, expected revision, lifecycle, recovery/process fences and session eligibility in the deciding transaction. Pending deletion disallows gameplay and other paid services; cancellation remains eligible while offline and inside the grace interval. Game serializes commands and admission with existing portfolio-before-CharacterLease lock order; any extra domain lock order needs game review. Public projection/entitlement freshness never forces eligibility.

| Command / variant | Fields after intent prefix, in order | COMMITTED effect |
|---|---|---|
| `ScheduleCharacterDeletion` / `PLATFORM_USER_DELETE` | None | `ACTIVE` → `DELETION_PENDING`; Game chooses authoritative start and deadline |
| `CancelCharacterDeletion` / `PLATFORM_USER_CANCEL_DELETE` | `deletion_operation_id` (`uuid7`, exact committed scheduling operation) | `DELETION_PENDING` → `ACTIVE`, same deletion generation only |
| `RenameCharacter` / `PLATFORM_USER_RENAME` | `requested_name` (`requested_name`) | Atomically reserve new global comparison key and change name, preserving identity |
| `ChangeCharacterSex` / `PLATFORM_USER_SEX_CHANGE` | `requested_sex` (`sex`) | Change game-owned current sex and required appearance state atomically, preserving identity; unsupported, flag off; reserved shape only |
| `TransferCharacterWorld` / `PLATFORM_USER_WORLD_TRANSFER` | `expected_from_world_id` (`uuid7`), `target_world_id` (`uuid7`, different from source) | Outside this slice; reserved shape only, no issuance or execution |

Deletion policy: Game chooses `deletion_effective_at`; 60 days is Game policy, not a fixed wire duration. Platform checks only `deletion_effective_at > deletion_scheduled_at` for the timestamp relationship and denies ingest otherwise; it neither computes nor enforces a fixed offset. Cancellation can commit only while the character is out of session and trusted transaction time is strictly before `deletion_effective_at`. At equality or later cancellation refuses, even if finalizer has not run. Game owns automatic finalization; Platform never sends a finalize command. Finalizer and cancel serialize so exactly one wins. Finalization requires trusted game time at or beyond the deadline; unsafe/ambiguous time delays finalization and denies cancellation until authority can be established, never shortens the grace interval. Until game evidence arrives Platform never infers final deletion from a local countdown. A second schedule is a lifecycle conflict, not a deadline extension. Finalized CharacterId is never reused. Game emits a private tombstone and removes/hides public projections; preservation/anonymization of durable gameplay references is game-owned.

Rename follows CHAR-NAME-1's global ASCII-lowercase-with-spaces-removed comparison key and final atomic name reservation. Same current comparison key, same current sex, same destination world, stale deletion generation and stale revisions refuse rather than consume a paid service for a no-op. Released-name retention is 30 days after rename or terminal deletion (D183); Platform cannot reserve names independently.

World transfer is outside this slice and must not be issued or enabled. The reserved shape needs a separate owning contract before use, including a second eligible world. Game must account for housing, guild, market, mail, depot, inventory, bank, quests and other world-specific obligations, offline/session state and source/destination rulesets. Until it defines safe disposition for every applicable obligation, it refuses. No partial transfer, copied CharacterId, inferred ChannelId or silent item/gold loss is allowed.

## 5. Receipts, refusal and paid-service saga

Receipt common prefix in order: `contract_version` (2), `operation` (`PublishCharacterCommandReceiptV2`), `source_authority` (`authority`), `command`, `operation_id` (`uuid7`), `outcome` (`COMMITTED` or `REJECTED`), `result_code` (closed set below).

A COMMITTED receipt appends `character_id` (`uuid7`), `account_id` (`uuid7`), `character_revision` (`u64`, resulting), `projection_epoch` (`u64`), `projection_revision` (`u64`, affected account's resulting independent services revision under the shared epoch fence; not an LCFA v1 revision), then these exact per-command facts:

| Command | Additional members in order |
|---|---|
| Schedule deletion | `lifecycle` = `DELETION_PENDING`, `deletion_operation_id` (`uuid7`, equals this operation), `deletion_scheduled_at` (`time`), `deletion_effective_at` (`time`, Game-chosen and strictly greater than `deletion_scheduled_at`) |
| Cancel deletion | `lifecycle` = `ACTIVE`, `deletion_operation_id` (`uuid7`, equals referenced schedule) |
| Rename | `previous_name` (`wire_name`), `name` (`wire_name`, accepted new display name) |
| Sex change | `previous_sex` (`sex`), `sex` (`sex`, equals requested) |
| World transfer | `from_world_id` (`uuid7`), `world_id` (`uuid7`, equals destination) |

Every receipt ends with `recovery_generation` (`u64`), `decided_at` (`time`, game transaction time). REJECTED has **only prefix and suffix**, no result facts. COMMITTED requires `CHAR_CMD_OK`. This maps wire COMMITTED to semantic COMPLETED in the Platform authority contract. No pending receipt state is added: transient failure stores nothing and leaves the intent pending. Old consumers may return `CHAR_CMD_UNSUPPORTED`; it never authorizes fallback.

Closed REJECTED codes (all terminal no-commit for this attempt except contradictory key evidence, which requires investigation):

| Codes | Applicability / meaning |
|---|---|
| `CHAR_CMD_INTENT_INVALID`, `CHAR_CMD_INTENT_EXPIRED`, `CHAR_CMD_UNSUPPORTED`, `CHAR_CMD_OPERATION_CONFLICT` | All; malformed intent, technical expiry, mixed version/capability, identity contradiction; existing names reused |
| `CHAR_CMD_ACCOUNT_NOT_ELIGIBLE`, `CHAR_CMD_CHARACTER_NOT_FOUND`, `CHAR_CMD_OWNER_MISMATCH`, `CHAR_CMD_CHARACTER_IN_SESSION` | All; existing account/subject/session denials |
| `CHAR_CMD_STALE_PRECONDITION`, `CHAR_CMD_LIFECYCLE_CONFLICT` | All; proposed revision/lifecycle denials |
| `CHAR_CMD_DELETION_WINDOW_CLOSED` | Cancel only; deadline reached or finalization won |
| `CHAR_CMD_NAME_INVALID`, `CHAR_CMD_NAME_UNAVAILABLE` | Rename only; existing names reused |
| `CHAR_CMD_SEX_CHANGE_NOT_ALLOWED` | Sex change only; proposed current-sex/appearance eligibility denial |
| `CHAR_CMD_WORLD_NOT_ACCEPTED`, `CHAR_CMD_CHARACTER_NOT_TRANSFERABLE`, `CHAR_CMD_HOUSING_NOT_PREPARED` | World transfer only; existing vocabulary extended to this profile |

A missing authoritative dependency, unknown eligibility, unsafe clock, unreconciled restore, database deadlock/serialization failure or loss of process proof commits nothing and stores no receipt; Game retries the same operation. Never turn an uncertain post-commit failure into rejection. No raw database errors or gameplay-obligation details cross the boundary.

Receipt 200 acknowledgement is exactly `{"contract_version":2,"result":"accepted"}`. Exact republication is a no-op. 400 = malformed/invalid echo or actual/declared oversize, 401 = authentication/source mismatch, 404 = no intent, 409 = conflicting receipt or restore-generation violation, 429 = throttled with Retry-After in seconds, 503 = disabled/misconfigured/unavailable. List/read use the same applicable failures; read 409 also means command mismatch. Guards run before handling: disabled/misconfigured 503, then peer 401, rate 429, encoding/schema checks. On receipt 404/409 Game retains its receipt, stops automatic publication of that key and alarms; Platform denies progress and investigates. Other timeouts/429/503 retry identical bytes. No HTTP status alone settles money.

Paid rename uses this saga; unsupported sex change and out-of-slice world transfer cannot enter it:

1. Authenticate and authorize the owner server-side; require current accepted owner evidence, product policy, CSRF/recent-auth as applicable and no conflicting service/Bazaar operation. Browser identifiers never authorize the action.
2. In one Platform transaction, lock Wallet/service capacity, reserve the accepted coin charge (or reserve an already purchased one-use entitlement), allocate the operation and immutable decision, and append the intent/outbox. **No command becomes listable before the reservation commits.** Insufficient funds, unknown price, stale ownership or reservation failure means no intent. Free schedule/cancel create no charge reservation.
3. Game pulls, revalidates and commits/refuses with a durable receipt. Platform keeps funds/entitlement reserved while pending or AMBIGUOUS; no replacement operation, automatic release at timeout or second charge.
4. Valid current COMMITTED causes exactly one reserved-charge settlement and/or entitlement consumption in Platform's receipt-handling transaction, with durable downstream work. The game mutation is not repeated if settlement retries. A terminal matching REJECTED releases the reservation once; the saga never debits before the command. Contradictory identity evidence stays held for manual reconciliation, never blind refund.
5. Restore can reopen previously terminal operations. Keep original financial history; use a separately idempotent compensation tied to operation and new recovery generation after authoritative reconciliation. If a completed service is lost through restore, grant the service again for free after reconciliation. Never debit twice; a chargeback causes no in-game reversal.

Internal Platform reservation record (not wire), exact logical fields: `operation_id`, `issuer_decision_id`, `account_id`, `character_id` (all uuid7); `command` (paid command); `product_id` (Store wire product_id, at most 64 bytes); `product_version` (integer 1..2147483647); `entitlement_id` (uuid7 or null, null for direct Wallet service); `currency` (exact `COINS`); `amount` (positive decimal string 1..9223372036854775807, integer coin units); `reservation_id` (uuid7); `intent_digest` (hex64); `state` (`RESERVED`, `SETTLED`, `RELEASED`, `RECOVERY_REQUIRED`); `created_at` (time). Product price, Wallet/payment IDs and entitlement IDs are never sent to Game. Amount must be zero nowhere in a paid reservation.

Proposed internal financial idempotency keys are `character-service:reserve:<operation_id>`, `character-service:settle:<operation_id>` and `character-service:release:<operation_id>` (literal lowercase prefixes plus UUID, at most 62 ASCII bytes). Restore compensation is `character-service:compensate:<operation_id>:<recovery_generation>` (at most 86 bytes). Wallet ledger uniqueness is the authority for money, not these strings alone. Browser double-submission maps to the same stored operation. Two distinct paid attempts for one character cannot reserve concurrently while either can commit.

## 6. Private service-state ingest (proposed companion projection)

LCFA v1 is unchanged: its `CharacterSummary` lacks sex, deletion deadline and character revision. Do not add these members to v1 or infer a deadline from `UNAVAILABLE`. This proposes a separate minimal owner-only companion using LCFA's push/outbox/epoch/watermark pattern, not a public feed. It is required for pending deletion across devices, later automatic finalization and changed state outside a portal receipt.

Snapshot field order:

| Field | Type / invariant |
|---|---|
| Common request head | version 2, `PublishCharacterServiceStateV2`, configured projection authority |
| `account_id`, `character_id` | `uuid7`, owner-scoped subject |
| `projection_epoch`, `projection_revision` | `u64`, shared non-rollback epoch fence with LCFA, but an independent per-account services revision namespace; service changes must not advance LCFA v1 revisions |
| `source_observed_at` | `time`, source revision assignment time, not retry time |
| `character_revision` | `u64`, current game revision |
| `world_id` | `uuid7`, current or last world for tombstone |
| `name` | `wire_name`, current or last display name |
| `sex` | `sex` or null; null for historical unknown, never defaulted |
| `lifecycle` | `ACTIVE`, `DELETION_PENDING`, `DELETED`, `REMOVED_FROM_ACCOUNT` |
| `deletion_operation_id` | `uuid7` or null |
| `deletion_scheduled_at`, `deletion_effective_at` | `time` or null |

The three deletion fields are non-null exactly for DELETION_PENDING/DELETED, and null for ACTIVE/REMOVED_FROM_ACCOUNT. Game chooses the deadline; Platform requires `deletion_effective_at > deletion_scheduled_at` and denies ingest otherwise, without requiring any fixed duration. DELETED is the durable finalization tombstone; absence is never a tombstone. REMOVED_FROM_ACCOUNT revokes the old owner's private entry after ownership transfer without revealing the new owner; its sex is null, and last name/world are used only to invalidate the old entry. This record cannot grant admission, prove current ownership or expose another account. Sex change is unsupported and is never offered.

Watermark exact fields: common head (operation `PublishCharacterServiceWatermarkV2`), `projection_epoch` (`u64`), `complete_through` (`time`), `observed_at` (`time`). 200 replies for snapshot/watermark are `{"contract_version":2,"result":"accepted"}` or `{"contract_version":2,"result":"superseded"}`; all other statuses follow §5, with 409 for equal ordered key/different content. No operation id is fabricated for automatic finalization.

Game records service projection work in the mutation transaction, including finalization and old-owner removal. Use the established account projection outbox/resync model; the new publisher needs its **own delivery acknowledgement**, so clearing LCFA work cannot discard undelivered service state. Snapshot is the complete current state of one `(account_id, character_id)`, replacing that entry. Order by `(projection_epoch, projection_revision)` within that subject; services revisions may skip for a subject, so a numerical gap alone is not message loss. Equal pair/equal content is idempotent, preserving first source_observed_at; equal pair/different content quarantines the scope. Lower pairs are superseded. Receipt and snapshot arrival order has no authority: a newer snapshot cannot be overwritten by an older receipt. Receipt facts may update the operation display immediately but never fabricate an account/public projection.

## 7. Ordering, replay, restore and timeouts

Intent/receipt key is `(issuer_authority, operation_id)` across every command and wire version. Full canonical intent bytes are the immutable binding; internal `intent_digest = SHA-256(bytes)` aids comparison but does not replace byte checks. Reusing an operation with a different command, owner, revision or intent cannot mutate or overwrite a receipt. Per `(issuer_authority, variant)`, equal source_revision with another operation is a contradiction. Lower unseen unexpired command revisions may still execute: source revision is not a global execution fence.

Reuse command pagination/rescan: resume by the last `(source_revision, operation_id)`, rescan from zero after a short page, at most 8 full pages or 30 s. Preserve the 256 wire pending ceiling and the **40 enabled backlog** limit across v1/v2 combined, issuer-locked issuance order and no fresh commits behind the cursor. Reopened operations keep original revision and TTL; they may reappear behind the cursor. Retain known-key retry state without hiding later work. No timestamp orders commands or replaces receipt evidence.

The companion projection shares LCFA's externally persisted non-rollback epoch fence; a restore raises above every published epoch before publication resumes. Seeing a higher epoch in a snapshot or watermark invalidates all older-epoch service entries. Resync must enumerate all live subjects and required tombstones/removals, not just changed roots. At-least-once replay is safe. A poison record holds its scope unavailable and its undelivered work holds back completeness; never acknowledge and skip it to make freshness green.

The companion watermark covers **only service-state delivery**. Reuse `complete_through`: no later than the oldest undelivered committed change or source now minus maximum transaction duration, and below resync start until full resync completes. Require complete_through <= observed_at; neither may exceed Platform now plus configured clock uncertainty. For testing/preproduction reuse LCFA's S = 30 s and its configured accepted clock uncertainty; no implicit production value. Stale if `platform_now - complete_through + uncertainty > S`, future bound violated, epoch not highest, fence unavailable or scope quarantined. Deny owner service actions on stale/unknown evidence; show unavailable, not empty/eligible. Healthy service-state watermark does not prove LCFA/public projection freshness and vice versa.

Game restore uses the existing restore notice route and recovery-generation rules without another notice format: receipt generation >= restored generation reopens affected operations and preserves superseded receipt history. Surviving receipts replay unchanged. Lost receipts are decided against original intent/expiry under the new recovery fence; do not recreate an expired mutation. Platform accepts replacement receipts only for explicitly reopened keys and the permitted generation. Admission and publication obey the established restore handshake. A rollback cannot erase operation deduplication or resurrect an old privacy allow.

A 3 s exchange timeout, missing receipt, expired local UI timer, empty pending page or read 404 does not prove no commit. Platform marks AMBIGUOUS/recovery-required, holds charge/service capacity and reconciles the **same** operation through pending re-listing and stored-receipt republication. Proposed operational escalation is after 300 s without terminal evidence; that is an alert, not cancellation or refund. No finite retry budget turns uncertainty into success/refusal. Reconciliation is required before any new semantic attempt that conflicts with the uncertain one.

## 8. Privacy, retention, flags and compatibility

Private records and operation status are available only to the authenticated current account owner after server-side object authorization, and to narrowly authorized internal recovery operators with audit. Loss/ambiguity of current ownership denies access; old owner's history must not expose the new owner's state. Owner responses are no-store. Public rename/delete/transfer changes travel through accepted public projection families, with stable CharacterId invalidation and Platform's current privacy floor. Stale old-name/world/search/ranking/guild/presence variants cannot resurrect deleted/private facts. Unknown purge acknowledgement keeps affected public output hidden.

Never send credentials, session/lease/fencing tokens, GameSessionId, Identity IDs, email/IP/device/MFA data, private node/Channel/position data, payment/provider data, price/balance, coin reservation, entitlement/voucher secrets, arbitrary character blobs, inventory or gameplay-obligation details. Route logs contain only operation name, command, result class and timings, never account/character identifiers, names or payloads. Metrics have bounded labels, no per-user identifiers.

Proposed retention, pending owner/legal approval: unresolved intent/receipt bindings and financial recovery evidence are retained until reconciled, with 90-day operator review rather than automatic deletion. Payload access is restricted; any legal erasure conflict freezes processing for review. Terminal detailed service payloads/old names/deletion timestamps are purged 90 days after finalization of the operation **and** resolution of associated financial/restore disputes, whichever is later. Keep minimal immutable operation identity, canonical binding fingerprint, terminal result and generation anti-replay tombstones for the lifetime of the authority/restore history; expired ids must never become new commands after payload purge. Game must reject retired/unknown expired operations rather than execute them. Private current ACTIVE/DELETION_PENDING state persists while needed; deletion/removal tombstones persist through acknowledged resync and replay/backup horizon, then erase private fields. The supported restore horizon equals Game backup retention; its configured duration must be pinned before activation; no premature deduplication GC. Financial ledger retention is Platform-owned and separate from game payload retention. The name lock lasts 30 days (D183), independently of the 90-day audit period.

Proposed flags, all **false** by default: Platform `character_services_v2.enabled`, and `schedule_deletion`, `cancel_deletion`, `rename`, `sex_change`, `world_transfer` beneath that namespace; Game `platform.character_commands.services_v2_enabled` plus corresponding per-command capability settings; companion projection `character_services_v2.projection_enabled` on both sides. These are proposed settings, not claims that code exists. Unknown flag/capability/environment denies. Production stays off until separately approved. Paid flags also require Wallet/entitlement readiness; sex change remains unsupported with its flag off, and world transfer stays off as outside this slice.

Stopping new submissions must leave receipt ingest, pending consumption, reconciliation and projection delivery available to drain existing operations. Scheduling deletion cannot be enabled without cancellation, and cancellation stays available for already pending deletions when new scheduling is disabled. Disabling everything during an incident makes state unavailable and requires recovery; it cannot undo a command or forfeit the grace period silently. Rollback never routes ambiguous operations through another authority.

Version 2 is closed-schema: incompatible member, type, enum, path, bound or response changes require a new negotiated contract_version and fixtures. New variants/codes are not assumed ignorable. Keep v1 servicing its own pending intents. No downgrade of v2 to v1, reinterpretation of ownership transfer as world transfer, or automatic unsupported-command retry under a fresh ID. Coordinator acceptance must pin exact producer and consumer revisions before enablement.

New public families belong only under `/internal/v2/game-public/*`, using the V2 family enum defined by [G-4](OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md). The v1 watermark family enum remains closed and unchanged. G-4 drops its own `character_activity` family. The single source of online time is the G-8 online-interval family: intervals are owner-only and only world totals are public. This service companion introduces no activity or online-time source.

## 9. Golden payloads

These compact JSON blocks are exact canonical bytes when copied without the fenced-block newline. UUID/time values are synthetic. They validate the proposal, not current v1 parsers. Intent expiry is technical; the deletion deadline is game-issued.

Valid schedule intent:

```json
{"contract_version":2,"command":"ScheduleCharacterDeletion","variant":"PLATFORM_USER_DELETE","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","expected_character_revision":"8","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Valid committed schedule receipt; the example uses a Game-chosen deadline, not a required offset:

```json
{"contract_version":2,"operation":"PublishCharacterCommandReceiptV2","source_authority":"oteryn:character-authority:primary","command":"ScheduleCharacterDeletion","operation_id":"0192a000-0000-7000-8000-000000000001","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_revision":"9","projection_epoch":"1","projection_revision":"43","lifecycle":"DELETION_PENDING","deletion_operation_id":"0192a000-0000-7000-8000-000000000001","deletion_scheduled_at":"1790000002","deletion_effective_at":"1795270402","recovery_generation":"3","decided_at":"1790000002"}
```

Valid cancel intent with a new operation referring to that schedule:

```json
{"contract_version":2,"command":"CancelCharacterDeletion","variant":"PLATFORM_USER_CANCEL_DELETE","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a2","source_revision":"18","operation_id":"0192a000-0000-7000-8000-000000000002","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","expected_character_revision":"9","deletion_operation_id":"0192a000-0000-7000-8000-000000000001","issued_at_source":"1790000100","expires_at_source":"1790000400","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Valid rename intent (assumes ACTIVE state and committed Platform reservation):

```json
{"contract_version":2,"command":"RenameCharacter","variant":"PLATFORM_USER_RENAME","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a3","source_revision":"19","operation_id":"0192a000-0000-7000-8000-000000000003","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","expected_character_revision":"10","requested_name":"Aldric Vale","issued_at_source":"1790000200","expires_at_source":"1790000500","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Reserved sex-change shape, valid JSON but unsupported: flag off, no issuance or execution:

```json
{"contract_version":2,"command":"ChangeCharacterSex","variant":"PLATFORM_USER_SEX_CHANGE","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a4","source_revision":"20","operation_id":"0192a000-0000-7000-8000-000000000004","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","expected_character_revision":"11","requested_sex":1,"issued_at_source":"1790000300","expires_at_source":"1790000600","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Reserved world-transfer shape, valid JSON but outside this slice: no issuance or execution:

```json
{"contract_version":2,"command":"TransferCharacterWorld","variant":"PLATFORM_USER_WORLD_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a5","source_revision":"21","operation_id":"0192a000-0000-7000-8000-000000000005","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","expected_character_revision":"12","expected_from_world_id":"01934f10-7c02-7001-805b-3b1122334401","target_world_id":"01934f10-7c02-7001-805b-3b1122334402","issued_at_source":"1790000400","expires_at_source":"1790000700","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Valid refusal; no result facts and no subsequent commit under this operation:

```json
{"contract_version":2,"operation":"PublishCharacterCommandReceiptV2","source_authority":"oteryn:character-authority:primary","command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","outcome":"REJECTED","result_code":"CHAR_CMD_NAME_UNAVAILABLE","recovery_generation":"3","decided_at":"1790000202"}
```

Valid owner-only pending-deletion snapshot and watermark:

```json
{"contract_version":2,"operation":"PublishCharacterServiceStateV2","source_authority":"oteryn:character-services:primary","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"43","source_observed_at":"1790000002","character_revision":"9","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","sex":null,"lifecycle":"DELETION_PENDING","deletion_operation_id":"0192a000-0000-7000-8000-000000000001","deletion_scheduled_at":"1790000002","deletion_effective_at":"1795270402"}
```

```json
{"contract_version":2,"operation":"PublishCharacterServiceWatermarkV2","source_authority":"oteryn:character-services:primary","projection_epoch":"1","complete_through":"1790000002","observed_at":"1790000003"}
```

Invalid list request: revision is a JSON number instead of a decimal string; 400, no cursor advance:

```json
{"contract_version":2,"operation":"ListPendingCharacterCommandsV2","source_authority":"oteryn:character-authority:primary","after_source_revision":0,"after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}
```

Invalid receipt: COMMITTED with refusal code and missing committed facts; 400, no settlement:

```json
{"contract_version":2,"operation":"PublishCharacterCommandReceiptV2","source_authority":"oteryn:character-authority:primary","command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","outcome":"COMMITTED","result_code":"CHAR_CMD_NAME_UNAVAILABLE","recovery_generation":"3","decided_at":"1790000202"}
```

Invalid rename intent: account_id is null, violating the required owner type; deterministic CHAR_CMD_INTENT_INVALID if the operation is safely attributable and undecided, never a mutation:

```json
{"contract_version":2,"command":"RenameCharacter","variant":"PLATFORM_USER_RENAME","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a6","source_revision":"22","operation_id":"0192a000-0000-7000-8000-000000000006","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":null,"expected_character_revision":"10","requested_name":"Aldric Vale","issued_at_source":"1790000200","expires_at_source":"1790000500","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

Additional required negative vectors: changed requested_name under the valid rename's operation_id is an operation conflict; a deadline at or before the scheduled time is invalid ingest; any strictly later Game-chosen deadline satisfies the wire timestamp relationship; requested_sex 0/3/null is invalid intent; same source/destination world refuses; extra payment/signature/Channel fields fail exact-schema validation. Unknown ownership, unsafe time, a missing watermark or an unaccepted version never becomes an allow.

## 10. Acceptance checklists

Platform:

- [ ] Confirm T46/T22/T26 mapping, approved product prices/policies and current owner authorization; all service flags default off.
- [ ] Demonstrate atomic charge/entitlement reservation plus intent commit; insufficient funds and concurrent double-submit produce no unreserved command.
- [ ] Validate all exact schemas/byte limits, certificate-purpose separation, mixed versions and unsupported receipts with shared golden bytes.
- [ ] Prove one settlement/consumption on committed receipt, one release on terminal refusal, no release/new operation on timeout, and no repeated mutation during financial retry.
- [ ] Prove receipt-before-projection and projection-before-receipt behavior, stale/unknown owner denial, privacy fences and old-owner access revocation.
- [ ] Prove pagination, combined backlog, same-key conflicts, replay, restore reopening and compensation; retain evidence beyond supported restore horizon.
- [ ] Exercise owner UX/E2E for all in-slice services including grace countdown without invented finalization, cancellation boundary, session conflicts and recovery; no public owner/payment disclosure.

Game owner/coordinator and producer:

- [ ] Accept/amend the exact v2 profile, companion projection, command names/codes and per-command capability advertisements; pin owning contract and producer revision.
- [ ] Implement Game-chosen deletion time, offline cancel-before-effective semantics and D183 30-day name locks; keep sex change unsupported and off.
- [ ] Add the new repeatable-service table rather than reuse migration 0087 receipt storage; prove independent services revisions under the shared epoch without bumping LCFA v1.
- [ ] Keep world transfer outside this slice and disabled; any future transfer contract needs separate review.
- [ ] Prove mutation + immutable receipt + audit + relevant outboxes commit atomically under ownership, revision, session and recovery fences.
- [ ] Prove duplicate/conflicting operation, concurrent rename/name reservation, cancel/finalize boundary, delete/paid-service/Bazaar/session races and dependency-failure no-commit cases.
- [ ] Prove durable receipt republication after response loss, unknown-version refusal, epoch-fenced resync/tombstones, watermark completeness and restore-generation handshake.
- [ ] Agree production PKI, safe clock bounds, retention/replay/restore horizon and shared exact-revision fixtures/E2E with Platform before separate activation approval.

## 11. Open questions for the game owner — answered decisions

The owner supplied these answers via the programme coordinator (ACCEPT-WITH-NOTES). Closed questions are replaced by their answers:

1. **Q1 — ANSWERED:** use v2 with an independent services revision and delivery acknowledgement under the shared LCFA epoch fence. Do not bump LCFA v1 revisions for services.
2. **Q2 — ANSWERED:** 60 days is Game policy, not wire. Game chooses the effective time; Platform requires effective > scheduled. Cancellation requires trusted time strictly before effective and a character out of session. Name locks last 30 days under D183, after rename or terminal deletion.
3. **Q3 — ANSWERED:** sex change is disabled and unsupported; its flag stays off. Existing v1 is unchanged and gains no sex-change support.
4. **Q4 — ANSWERED:** world transfer is outside this slice; reserved shapes grant no implementation or activation authority.
5. **Q5 — ANSWERED:** yes to the typed refusal set, mandatory `expected_character_revision` and shared 40-operation enabled backlog. Cancellation requires the authoritative account/session gates and an out-of-session character.
6. **Q6 — ANSWERED:** yes to reserve → receipt → settle/consume and rejection release. There is no debit-before-command/refund branch. A service lost through restore is granted again for free after reconciliation; chargeback causes no in-game reversal.
7. **Q7 — ANSWERED:** restore horizon equals backup retention. Pin the configured retention duration before activation and preserve replay/deduplication evidence across that horizon.

Production certificate allocation, clock/freshness configuration and recovery escalation remain deployment prerequisites in the acceptance checklist; no values or assignments are invented here.

This is a docs-only proposal. It authorizes no code, schema, workflow, Game repository writes, real charge, deployment, service activation or contract acceptance on behalf of the game owner.
