Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# G-10 private game journal: items and economic history, v1 proposal

## Purpose, authority and scope

This is a negotiation draft for G-10 in the gate table and §10 of
[the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md). T72 owns the
private `playerdata` ingest, deduplication, watermarks, retention and authorization;
T76 consumes its item search and bank/market/player-trade history projections.
Game owns authoritative custody, economic outcomes, source transactions, source
identities and ownership transitions. Platform owns private presentation and
rebuildable read models. The coordinator accepts or amends the producer contract;
Platform accepts the corresponding consumer mapping. Neither side's acceptance
alone enables a feature.

Scope is a complete read-only snapshot of equipment (including nested carried
containers), every town's depots, character inbox and eligible house custody at
successful durable logout, plus committed bank, market and player-trade history.
This is neither a live inventory nor proof that an item remains sellable. T76 may
link to a separately authorized Bazaar flow, but cannot list, reserve, move, mint,
destroy, transfer, withdraw, purchase or settle anything through this contract.
T74 may join catalogue/pricing facts for an explicitly estimated valuation; this
feed never asserts a market value or wallet balance. G-9 session analytics and
G-11 progression/login telemetry remain separate gates.

## Existing conventions and evidence

Read together with these Platform contracts:

- [Store wire](OTERYN_V2_STORE_WIRE_CONTRACT.md), §§2–4, 9: closed canonical JSON,
  integer strings, SHA-256 byte bindings, purpose-bound TLS 1.3 mTLS, bounded
  messages and exact retry semantics. **That wire is itself proposed**, not an
  accepted producer requirement inherited by this draft.
- [Public projection](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md): accepted
  authority split, replay/baseline, monotone source fences, truthful evidence
  states and fail-closed privacy. Its public exposure policy does not apply to
  these private records; its exact transport is deferred.
- [Character Authority commands](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md):
  accepted operation identity, ownership revalidation, terminal versus ambiguous
  results and reconciliation. Its architecture does not select cryptography.
  The concrete existing pull wire is also visible in
  `app/CharacterAuthority/CharacterCommandWire.php`.
- [Entitlement delivery](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): accepted
  separation of commercial truth from game effects, at-least-once delivery and
  no inferred success from timeout. It deliberately defers exact transport.

Game references below were inspected read-only at **857f15f376907c592bb28d854fccff18a15cdf87**
in `Oteryn/Oteryn-Game`, outside the Platform checkout. Paths are relative to that
repository and pinned to that revision, not claims about future main:

| Game path | Fact relied on / mapping constraint |
| --- | --- |
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` | Existing command contract and ownership-transfer boundary; reuse `TransferCharacterOwnership`, not a new transfer command |
| `crates/gameplay/native-admission-source/src/character_commands.rs` | `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1`, `PublishCharacterCommandReceiptV1`, purpose-specific mTLS, `Key`, canonical UUIDv7 and decimal counters; Game initiates HTTP |
| `crates/gameplay/durability/src/durability/store/outbox.rs` | `DeliveryReceipt`, `list_unacknowledged_store_receipts`, `mark_store_receipt_acknowledged`: durable outbox publication followed by acknowledgement |
| `crates/gameplay/economy/src/bank.rs`; `apps/game-server/migrations/0071_account_bank.sql` | Account/world bank, not character balance; `operation_occurrence_id`, `event_id`, chained entries, typed refusals; only committed operations create audit events; existing audit retention P30D |
| `crates/gameplay/economy/src/market.rs` | `TypedDefinitionRef`, offer identity, Buy/Sell, Filled/Cancelled/Expired, CreditHeld, amount bound 64,000 and gold total bound 999,999,999,999 |
| `crates/gameplay/economy/src/player_trade.rs`; `apps/game-server/migrations/0147_player_trade.sql` | Atomic two-leaf exchange; `occurrence_id`, `transaction_id`, `event_id` and participant receipts exist; session/command proof and before-state blobs must stay in Game |
| `crates/gameplay/items/src/depot.rs`; `crates/gameplay/durability/src/durability/character_depot.rs` | Town-partitioned custody, u16 town_id (including zero), 17 boxes and 15,000 hard entries; channel does not partition a depot |
| `apps/game-server/migrations/0076_character_inbox.sql` | Inbox custody and delivery evidence, rather than portal-created mail semantics |
| `apps/game-server/migrations/0025_house_item_custody.sql` | House item custody must be mapped explicitly, not inferred from house access rights |
| `crates/gameplay/domain/src/domain/item_definition.rs`; `apps/game-server/migrations/0010_item_mint_ground.sql` | Game-owned typed definition reference, rather than display name or client sprite as item identity |

The G-10 names and schemas below are **new proposed wire mappings**, not assertions
that a logout publisher, complete history feed or these routes already exist.
Existing receipts are evidence to build from; they do not by themselves prove a
complete export. No shared database access or new broker is proposed.

## Flow and transport

| Logical direction | Physical exchange | Authority/effect |
| --- | --- | --- |
| Game → Platform ingest | Game POSTs `PublishGameJournalRecordV1` to `/internal/v1/playerdata/journal/records` | One durable private record; response acknowledges storage, never a game effect |
| Game → Platform reconciliation query | Game POSTs `ReadGameJournalCursorV1` to `/internal/v1/playerdata/journal/cursor` | Reads Platform's durable contiguous cursor to resend missing records |
| Platform → Game command | **No new G-10 gameplay command.** Existing character-command pull/read/receipt exchange remains unchanged | `TransferCharacterOwnership` changes ownership under its own accepted contract; G-10 observes/fences that result |
| Platform → Game replay instruction | Cursor response on the Game-initiated connection | Requests resend after a watermark; cannot mutate gameplay or force a new logout |

All network calls originate in Game, as in the existing command and Store wires.
No browser calls an internal route. Normal portal reads use Platform storage and
never synchronously call Game or read Game SQL. These proposed routes use the
existing internal listener with TLS 1.3 mutual authentication and server identity
verification; no new port, bearer scheme or cryptographic primitive is selected.
Use a dedicated configured journal peer identity, distinct from Store, command,
public-projection, admission and entitlement identities, bound to exactly one
configured `source_authority` and environment. Missing, overlapping or inconsistent
configuration denies the surface with 503. Never trust client-supplied forwarding
headers for TLS identity; use the existing trusted terminator's verified values
(`SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, exact constant-time match of
`SSL_CLIENT_S_DN`, at most 128 printable ASCII bytes).

Signing follows the concrete wire convention: mTLS authenticates the exchange;
SHA-256 binds canonical stored bytes and **is not a signature**. No detached HMAC,
JWT or signature field is invented. If a future accepted shared transport mandates
signed envelopes, adopt it through a jointly versioned revision before activation;
do not silently accept unsigned substitutes or treat a digest as authentication.
Purpose identity and persisted stream/revision fences prevent cross-purpose and
old-generation replay; business deduplication alone is insufficient.

Both routes are POST with `Content-Type: application/json`; all responses carry
`Cache-Control: no-store`. Reuse Store §2 canonical encoding: one UTF-8 object,
no BOM/newline/insignificant whitespace, no duplicate keys, no escapes/backslash,
exact listed member order and no additional members at any depth. Arrays preserve
source order. Integers are JSON integers only where specified; booleans are JSON
booleans. Decode depth ≤8 (flat item rows avoid recursive objects). Absent or
`identity` Content-Encoding only; no Transfer-Encoding. Content-Length, if present,
uses the existing decimal syntax and must equal actual length. Publish body ≤65,536
bytes; cursor request ≤1,024 bytes; response ≤1,024 bytes. Never truncate to fit.
A page that cannot fit must be split within the negotiated page bounds before its
identity is fixed; an unrepresentable snapshot is unavailable, never partial/empty.

Reuse command transport bounds: HTTP/1.1, connect 1 second, handshake 2 seconds,
exchange 3 seconds, one in-flight exchange per consumer. Proposed operational
rate: 120 requests/minute per peer, burst at most 120, 60-second window;
exponential retry with jitter
from 1 to 10 seconds, honoring `Retry-After` ≥1 second on 429/503. After 60 seconds
without progress raise a bounded diagnostic and continue durable reconciliation.
No elapsed timeout acknowledges or discards a record.

## Types and common envelope

All members are required; only explicit `null` is nullable. Table order is wire
order. Closed enums are case-sensitive. Lengths are UTF-8 bytes; ranges inclusive.

| Type | Exact representation |
| --- | --- |
| `id` | Non-nil canonical lowercase UUIDv7, as existing command wire; 36 bytes |
| `u64` | Decimal string `[1-9][0-9]{0,19}`, numeric value ≤18,446,744,073,709,551,615 |
| `cursor` | `"0"` or `u64` |
| `gold` | Decimal string `0` or nonzero decimal without leading zeros; ≤9,000,000,000,000,000; unit: one game gold piece, never Coins or fiat |
| `positive_gold` | `gold` >0 and ≤999,999,999,999 |
| `time` | `u64` Unix seconds UTC; source converts existing finer-grained timestamps by flooring; not ordering authority |
| `key` | ASCII `[A-Za-z0-9._:/-]{1,128}` |
| `definition_ref` | ASCII `[A-Za-z0-9._:/-]{1,512}`, exact source definition key/revision; a nonrepresentable source reference blocks export pending an accepted version |
| `house_key` | ASCII `oteryn:content.house.` followed by one or more `[a-z0-9_]`, total ≤512 bytes; revision-free, scoped by envelope world_id |
| `hex64` | Exactly 64 lowercase hexadecimal SHA-256 characters |
| `int[a,b]` | JSON integer in inclusive bounds |
| `family` | `ITEMS`, `BANK`, `MARKET`, `PLAYER_TRADE` |

`AccountId`, `CharacterId`, `WorldId`, operation, event and instance identities map
to canonical native IDs, never compatibility numeric IDs. Game must confirm every
existing ID's UUID version before accepting this mapping; do not remint an
existing source identity merely to pass validation. `HouseId` and town identifiers remain
game-owned: house keys are opaque and towns retain their existing u16 IDs, not
fabricated UUIDs or channel partitions.

Publish envelope:

| Member | Type / rule |
| --- | --- |
| `contract_version` | integer `1` |
| `operation` | literal `PublishGameJournalRecordV1` |
| `source_authority` | `key`, exact configured authority |
| `account_id` | `id`; recipient account, never inferred by joining a historical character name |
| `world_id` | `id` |
| `character_id` | `id` for ITEMS/MARKET/PLAYER_TRADE; `null` for BANK |
| `ownership_epoch` | `u64` for character families; `null` for BANK; proposed monotone game-owned ownership incarnation, not runtime-scope generation |
| `family` | `family` |
| `recovery_generation` | `u64`, existing restore-reconciliation generation semantics |
| `source_revision` | `u64`, contiguous per stream defined below |
| `record_id` | `id`, fixed durably before first publication |
| `occurred_at` | `time`, durable logout time for ITEMS, committed economic event time otherwise |
| `coverage_start_at` | `time` ≤ `occurred_at`; earliest covered economic time, or logout time for ITEMS |
| `kind` | `ITEMS_BEGIN`, `ITEMS_PAGE`, `ITEMS_END`, `BANK_ENTRY`, `MARKET_EVENT`, `PLAYER_TRADE_EVENT`; must match family |
| `payload` | Exactly the matching object below |

Stream key is `(source_authority, account_id, world_id, character_id,
ownership_epoch, family, recovery_generation)`. BANK is account/world scoped and
must not be copied once per character. Character-family history is confined to
one ownership incarnation. Proposed `ownership_epoch` requires game-owner
agreement and a durable mapping to authoritative ownership transitions; existing
`character_revision` alone cannot order all economic/item changes.

## Logout snapshot records

All four roots belong to one durable logout cut; ongoing market delivery, mail,
house changes and nested moves must be included or excluded consistently at that
cut. Game must capture the source cut and export/outbox atomically or from a
provably reproducible committed revision. Crash/disconnect without that proof
produces no purported complete logout. A house snapshot is not proof that house
contents belong to the character: Game exports only custody eligible under the
accepted ownership policy. Visitors, guests and guild access alone confer no
export right. If eligibility is unknown the snapshot cannot become complete.

`ITEMS_BEGIN` payload:

| Member | Type / rule |
| --- | --- |
| `snapshot_id` | `id`, same for begin/pages/end |
| `character_revision` | `u64`, game Character Authority revision at cut, not item sequence |
| `page_count` | `int[0,16384]` |
| `item_count` | `int[0,524288]`, flattened rows including containers |
| `depots` | Array of 0–256 unique `int[0,65535]` town IDs, numeric ascending order; every eligible town included, even empty |
| `houses` | Array of 0–256 unique `house_key` house IDs, lexicographic order; every eligible house included, even empty |

Equipment and inbox roots are always included implicitly. Roots missing from these
lists are not evidence that a previously known depot/house is empty; Game must
prove the lists complete. If accepted Game bounds exceed these proposal bounds,
version the mapping rather than silently dropping roots.

`ITEMS_PAGE` payload, ordered members: `snapshot_id: id`, `page_index:
int[0,16383]`, `items: array[1..32]` of ItemRow. Page indices begin at zero without
gaps; every page has at most 32 rows and must fit the byte bound. The exporter
fixes page boundaries before BEGIN: for nonempty snapshots,
`page_count ≤ item_count ≤ 32 × page_count`. Zero items means zero pages and
zero pages requires zero items. Rows are globally sorted by `item_instance_id` ASCII.

ItemRow (every field present):

| Member | Type / rule |
| --- | --- |
| `item_instance_id` | `id`, unique in complete snapshot |
| `definition_family` | `key`, exact Game typed definition family; not rewritten to a portal enum |
| `definition_production_key` | `definition_ref`, exact accepted catalogue key |
| `definition_revision_ref` | `definition_ref`, exact accepted catalogue revision |
| `quantity` | `int[1,4294967295]`, source units; validated against definition's stack rules |
| `root` | `EQUIPMENT`, `DEPOT`, `INBOX`, `HOUSE` |
| `town_id` | `int[0,65535]` for DEPOT, otherwise `null`; must appear in begin.depots |
| `depot_box` | `int[1,17]` for DEPOT, otherwise `null` |
| `house_id` | `house_key` for HOUSE, otherwise `null`; must appear in begin.houses |
| `spatial_position_hex` | For HOUSE only: lowercase hex encoding of Game spatial_position bytes, even length 2–256; otherwise `null`; opaque, never decoded as portal coordinates |
| `equipment_slot` | `int[1,255]` for EQUIPMENT, otherwise `null`; game-owner slot mapping required before activation |
| `parent_item_instance_id` | `id` or `null`; parent in same snapshot and same root coordinates |
| `placement_ordinal` | `u64`; unique among siblings at identical root coordinates |

No cycles, orphan parents, duplicated custody or depth >32. For HOUSE,
placement_ordinal maps to stack_ordinal and sibling uniqueness also
includes spatial_position_hex; the pinned house custody permits leaves only, so
HOUSE rows require null parent and no children. Root metadata is
inherited on nested rows; equipment_slot identifies the ancestor equipment slot.
Quantity is not a monetary amount. Unknown definition mapping blocks valuation
and snapshot activation; it does not create a guessed item. Item text, custom
labels, enchantment secrets and arbitrary serialized attributes are not exported.

`ITEMS_END` payload, ordered: `snapshot_id: id`, `page_count: int[0,16384]`,
`item_count: int[0,524288]`, `pages_sha256: hex64`. Counts exactly equal BEGIN and
observed rows. Digest is SHA-256 of the concatenation of exact canonical
ITEMS_PAGE **payload object** bytes in page_index order, with no separator;
zero pages hashes the empty byte string. The authenticated envelope binds this
digest to account, character, ownership epoch and source generation.
All BEGIN/PAGE/END envelopes must agree on snapshot ID, stream key, occurred_at
and coverage_start_at. Only validated END atomically swaps T76's complete
snapshot pointer. BEGIN or PAGE ACK means durable staging, never complete visibility.

## Economic records

Only committed facts are exported; a rejection, attempted trade, pending match or
transport timeout cannot become an economic event. Counterpart accounts, names
and character IDs are omitted. Trade/transfer IDs correlate the owner's own
records internally but confer no access to another participant's projection.
Economic ItemLine is the ordered object `item_instance_id: id`,
`definition_family: key`, `definition_production_key: definition_ref`,
`definition_revision_ref: definition_ref`, `quantity: int[1,4294967295]`. ItemLine lists sort
by instance ID and contain no duplicate instance in one list.

`BANK_ENTRY` payload:

| Member | Type / rule |
| --- | --- |
| `operation_occurrence_id` | `id`, existing bank operation identity |
| `event_id` | `id`, existing committed audit event |
| `entry_id` | `id`, existing owner's bank ledger entry |
| `previous_entry_id` | `id` or `null` only for first source ledger entry; a retention-window boundary is not a new ledger origin |
| `kind` | `DEPOSIT`, `WITHDRAW`, `TRANSFER` |
| `direction` | `CREDIT` or `DEBIT`; DEPOSIT only CREDIT, WITHDRAW only DEBIT |
| `amount` | `positive_gold`; DEPOSIT ≤20,000,000; WITHDRAW ≤1,009,999 |
| `balance_before` | `gold`, ≤999,999,999,999 (existing account-bank ceiling) |
| `balance_after` | `gold`, ≤999,999,999,999; exactly before plus/minus amount without overflow/underflow |

BANK shows this account's entry, not a character asset inherited by a buyer. Its
ledger coverage is explicitly bounded by coverage_start_at; no preceding entry
inside that interval may be absent. Existing additional debit causes, fees or
held credits must not be mislabelled as DEPOSIT/WITHDRAW/TRANSFER: supporting
those needs an accepted mapping extension before a complete-bank-history claim.

`MARKET_EVENT` payload:

| Member | Type / rule |
| --- | --- |
| `event_id` | `id`, game committed event |
| `offer_id` | `id`, existing Game offer identity |
| `side` | `BUY` or `SELL`, this owner's offer |
| `state` | `OPEN`, `FILLED`, `CANCELLED`, `EXPIRED`, `CREDIT_HELD` |
| `end_state` | `FILLED`, `CANCELLED`, `EXPIRED` only when CREDIT_HELD; otherwise `null` |
| `definition_family` | `key`, exact source family |
| `definition_production_key` | `definition_ref` |
| `definition_revision_ref` | `definition_ref` |
| `amount` | `int[1,64000]`, original offer units |
| `remaining` | `int[0,64000]`, ≤amount; FILLED and CREDIT_HELD/FILLED require zero |
| `piece_price` | `positive_gold`; amount × piece_price ≤999,999,999,999 |
| `fee_gold` | `gold`, ≤2,500,000; fee actually recorded for this event, zero if none |
| `held_credit_gold` | `gold`; zero unless CREDIT_HELD |

Each committed partial fill emits a new event with updated remaining even if state
stays OPEN. State/remaining records are absolute observations, not additive sales.
A later event leaving CREDIT_HELD records actual resolution; portal never treats
held credit as spendable. Bank and market records may describe the same transaction;
do not sum them together as independent wealth. Cross-family atomic visibility
is not promised. Bazaar commercial history remains owned by Platform's existing
Bazaar/Wallet contracts, not fabricated from Game market events.

`PLAYER_TRADE_EVENT` payload, ordered: `occurrence_id: id`, `transaction_id: id`,
`event_id: id`, `given: array[1..1] of ItemLine`, `received: array[1..1] of ItemLine`.
This deliberately matches the pinned atomic two-leaf exchange; Game must confirm
whether later supported trade shapes require a new version. Given and received
IDs differ; each quantity is additionally ≤100, the pinned leaf-exchange bound. One committed occurrence yields one owner-scoped view for each side;
not two swaps. No offered/cancelled UI negotiation is included.

## Cursor, ordering, replay and failure

`ReadGameJournalCursorV1` request has the ordered members `contract_version: 1`,
`operation: "ReadGameJournalCursorV1"`, `source_authority: key`, `account_id: id`,
`world_id: id`, `character_id: id|null`, `ownership_epoch: u64|null`, `family:
family`, `recovery_generation: u64`. Null rules and stream authorization are exactly
as on publish. An unknown or unregistered stream returns 409; only a proven
registered stream with no stored records may return cursor zero. Response is `contract_version: 1`, `after_source_revision: cursor`,
`last_record_id: id|null` (null iff cursor is zero). A cursor is a storage ACK,
not owner authorization or proof that a staged snapshot is visible.

Publication response for a first valid write or exact replay is always:

```json
{"contract_version":1,"result":"accepted"}
```

Game fixes record IDs, ordering revisions and bytes durably with the source
transaction/export cut and an outbox; a restart cannot regenerate them. Source
revision starts at 1 per stream and increments by one; no gaps for filtered
foreign records. Each snapshot's BEGIN, pages and END occupy consecutive revisions
without interleaving another snapshot. Economic coverage_start_at is constant per
stream generation and replay includes every committed supported fact since that
boundary. A retained history can be empty only after a separately verified initial
baseline/reconciliation proves completeness; silence is never that proof. This
proposal does not invent an empty-history event or an implicit bootstrap success.

Idempotency keys are `(stream key, source_revision)` and independently
`(source_authority, recovery_generation, record_id)`. Store canonical bytes or their
SHA-256 binding durably with both keys. Exact retries acknowledge without a second
row/effect. Changed bytes under either key conflict, including changed timestamp,
recipient, root, schema or amount. A source economic ID also deduplicates within
its owner view across rebuilds; republishing it must not add a second ledger event.
Two participant views of a trade legitimately share source event IDs.

Platform commits record/staging, dedup keys and contiguous watermark atomically
before ACK. Game marks its outbox acknowledged only after validating the exact
200 response. On lost response Game reads the cursor and/or resends the identical
record. Cursor advancement never authorizes deleting the source ledger. A lower
revision can only be accepted as an exact known duplicate; higher than next
revision is 409 and cannot advance the cursor. Do not use wall-clock arrival order.

| HTTP / condition | Semantics and next step |
| --- | --- |
| 400 | Malformed/noncanonical, unsupported version/member/enum, wrong units or impossible invariants; no cursor advancement; quarantine reference and repair through owner-approved corrected source generation, not changed bytes under same ID |
| 401 | Peer/source authority failure; no ingestion or existence disclosure; repair configuration, never retry with another purpose's credential |
| 409 | Identity conflict, gap, incompatible snapshot or fenced epoch/generation; no new cursor advancement; query cursor/reconcile same stream; conflicting bytes require operator/game-owner resolution |
| 413 | Oversized body; no ingestion; fix exporter before assigning new accepted source identities, do not truncate |
| 429 | Rate limited; no ACK; honor Retry-After and retry same record |
| 503 | Disabled, unsafe clock/configuration/authority, dependency or storage unavailable; no ACK; bounded backoff with same identity |
| Timeout, disconnect, unexpected response, 5xx | Unknown/ambiguous commit; deny success and reconcile same keys; never fabricate empty history or a new source event |

Non-200 bodies are empty. Diagnostics contain bounded reason codes and references,
not private payloads. A handler that cannot prove a no-commit failure remains
ambiguous to Game even if HTTP failed; deduplication resolves the retry.

Within a stream, gaps/poison records fence visibility of any state dependent on the
gap. Other independent streams may progress. Replay is at-least-once; no exactly-once
network promise. Retain acknowledged export bytes for 30 days, unacknowledged bytes
until resolution within approved privacy retention; approaching expiry without
resolution stops completeness claims and escalates rather than retaining forever.
Game cursor rescans at least every 30 seconds recover Platform restore or ACK loss.
If requested history predates retained export evidence, do not skip to the tail:
mark history unavailable and negotiate a verified replacement baseline with a new
fenced generation and explicit coverage boundary. A full logout snapshot can
replace item state but cannot repair missing economic history.

Use the existing restore notice and recovery_generation reconciliation convention
from `OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`; a new or unknown generation
is never auto-trusted. Fence affected serving until restore notice/ownership and
baseline-plus-tail proof agree. Rebuild off-path, validate complete snapshot and
retained history, then atomically switch projection generation. Rollback cannot
lower source/ownership/privacy fences or resurrect deleted data. Exact baseline
exchange and generation admission are acceptance prerequisites, not a license to
hand-edit cursors. Preserve fences/dedup evidence for the whole replay lifetime;
after payload erasure, reject expired replays without restoring private content.

## Owner-only access, transfer and retention

No public feed, sharing link, guild/team/staff dashboard, public search index, CDN or
analytics export receives this data. Authenticate the Identity, resolve AccountId
server-side and require current authoritative ownership for every character read,
search, download and cache hit. Staff status alone is insufficient. BANK requires
current authenticated access to the account, independently of character ownership.
Use private no-store responses; no browser-supplied AccountId can select a tenant.
Internal ingest authentication does not authorize a portal user.

A committed `TransferCharacterOwnership` must fence the prior ownership epoch
before it can serve again. Seller loses character-family access; buyer receives
only a new complete snapshot and history from the new epoch, never seller's
pre-sale history. Delayed seller records are rejected/fenced, not rebound to buyer.
Bank history remains with the original account. During pending/ambiguous transfer,
ownership evidence outage, house eligibility uncertainty or unapplied privacy fence,
deny affected access to both candidate owners. No last-known owner allow fallback.
Reclaim provenance is not itself item ownership: HOUSE eligibility must be
proven by the agreed game policy, never inferred from reclaim_subject alone.
The existing authoritative transfer result remains the truth; G-10 does not delay
or redefine the commercial settlement saga. Activation requires proof that the
ownership fence reaches private serving paths at the authoritative cutoff.

Retention uses trusted UTC with uncertainty ≤30 seconds; use the latest plausible
current time to expire data conservatively. Future source times beyond 30 seconds
are invalid; unsafe clock evidence denies serving and ingestion until reconciled.
Proposed retention bounds, pending coordinator/privacy approval: latest complete
logout snapshot serveable for at most 30 days from occurred_at, always labelled
“at logout” with time; never labelled live. Retain at most the current and one
previous complete snapshot, deleting the previous within 24 hours after replacement.
Staging expires after 24 hours without a valid END. Economic projection/export
payloads expire 30 days after occurred_at; no extension on retry, rebuild or restore.
Platform never changes Game's authoritative ledger/legal-hold retention. An expired
payload's replay fence is minimal non-content metadata retained for 31 days after
payload expiry, then replay below a durable per-stream retention floor is refused.
Unresolved records cannot bypass these privacy limits: expiry becomes an explicit
gap/unavailable state. Backups must age out within 30 days of deletion and restore
must reapply deletion/ownership fences before any serving. Legal-hold policy for
this private copy requires separate approval; no implicit indefinite hold.

On account/character erasure or ownership change, immediately hide affected records
and purge payloads/caches/staging within 24 hours, retaining only minimal scoped
fences needed to deny replay. Never send credentials, cookies, IP/device identifiers,
GameSessionId, gameplay lease/admission material, private topology, chat/mail/item
text, trade before-state blobs, other participant account/name/character identity,
payment/provider data or unrestricted game attributes. Do not log bodies; log
aggregate counts, age, bounded codes and non-content correlation references.

T72 distinguishes `fresh`, `stale`, `unavailable`, `invalid`, `rebuilding` evidence
from `present` and proven `empty`. An incomplete snapshot is unavailable; a snapshot
older than 30 days is unavailable, never empty. Freshness means a verified logout
cut, never current in-game state. History exposes its covered interval and gap
status, never promises an all-time ledger. Unknown or ambiguous authorization,
completeness, custody, restore or expiry state denies the affected output.

## Feature flags, compatibility and activation

Proposed independent flags `game_journal_items_export_enabled` (Game),
`playerdata_g10_ingest_enabled` (Platform), `playerdata_g10_items_enabled` and
`playerdata_g10_history_enabled` (Platform) are all **false by default** in every
environment. Missing configuration equals disabled. Disabling presentation hides
and invalidates affected private output immediately; disabling ingest returns 503
and leaves outbox records unacknowledged. Re-enabling needs current ownership,
retention and replay checks. No automatic fallback to shared SQL or another feed.

Contract identifier: `oteryn-v2-game-journal-items-v1`, body contract_version 1 and
route /v1. Exact field sets are closed: even additional optional-looking members
are rejected. A compatible operational limit reduction requires both sides to
negotiate it while disabled; meaning, ID encoding, ownership/retention policy,
units, enum, record shape, ordering or digest changes require a new contract
version and fixtures. Never silently reinterpret an old version. Pin exact
producer/consumer SHAs and accepted contract revision before activation. Parallel
shadow consumption may compare evidence but may not mix versions/generations into
one authoritative snapshot. Unsupported versions deny, not downgrade.

## Golden payloads

These are synthetic, single-line canonical wire bytes; code-fence newlines are
not transmitted. UUID values illustrate shape only and confer no authority.

Valid cursor request (known account/world bank stream):

```json
{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"ownership_epoch":null,"family":"BANK","recovery_generation":"1"}
```

Valid empty snapshot pair (stream cursor initially zero, trusted owner epoch 1;
equipment/inbox included, no eligible depots/houses). END proves empty, not BEGIN:

```json
{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","ownership_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000004","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_BEGIN","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","character_revision":"1","page_count":0,"item_count":0,"depots":[],"houses":[]}}
```

```json
{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","ownership_epoch":"1","family":"ITEMS","recovery_generation":"1","source_revision":"2","record_id":"019a0000-0000-7000-8000-000000000006","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"ITEMS_END","payload":{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_count":0,"item_count":0,"pages_sha256":"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}
```

Valid committed account-bank entry; source ledger origin proven:

```json
{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"ownership_epoch":null,"family":"BANK","recovery_generation":"1","source_revision":"1","record_id":"019a0000-0000-7000-8000-000000000007","occurred_at":"1793836800","coverage_start_at":"1793836800","kind":"BANK_ENTRY","payload":{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":"100","balance_before":"0","balance_after":"100"}}
```

Valid nonempty ITEMS_PAGE payload object (one carried container; the surrounding
snapshot declares one item/page). Definition references and family `ITEM` below are synthetic
catalogue bindings; real activation requires accepted source definitions:

```json
{"snapshot_id":"019a0000-0000-7000-8000-000000000005","page_index":0,"items":[{"item_instance_id":"019a0000-0000-7000-8000-00000000000b","definition_family":"ITEM","definition_production_key":"oteryn:content.item.backpack","definition_revision_ref":"rev-1","quantity":1,"root":"EQUIPMENT","town_id":null,"depot_box":null,"house_id":null,"spatial_position_hex":null,"equipment_slot":3,"parent_item_instance_id":null,"placement_ordinal":"1"}]}
```

Valid MARKET_EVENT payload object for a fully filled sell offer:

```json
{"event_id":"019a0000-0000-7000-8000-00000000000c","offer_id":"019a0000-0000-7000-8000-00000000000d","side":"SELL","state":"FILLED","end_state":null,"definition_family":"ITEM","definition_production_key":"oteryn:content.item.gold_coin","definition_revision_ref":"rev-1","amount":10,"remaining":0,"piece_price":"2","fee_gold":"0","held_credit_gold":"0"}
```

Valid PLAYER_TRADE_EVENT payload object, one owner perspective:

```json
{"occurrence_id":"019a0000-0000-7000-8000-00000000000e","transaction_id":"019a0000-0000-7000-8000-00000000000f","event_id":"019a0000-0000-7000-8000-000000000010","given":[{"item_instance_id":"019a0000-0000-7000-8000-000000000011","definition_family":"ITEM","definition_production_key":"oteryn:content.item.gold_coin","definition_revision_ref":"rev-1","quantity":10}],"received":[{"item_instance_id":"019a0000-0000-7000-8000-000000000012","definition_family":"ITEM","definition_production_key":"oteryn:content.item.platinum_coin","definition_revision_ref":"rev-1","quantity":1}]}
```

Invalid cursor request: BANK must use null character/epoch (400; no cursor change):

```json
{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":"019a0000-0000-7000-8000-000000000003","ownership_epoch":"1","family":"BANK","recovery_generation":"1"}
```

Invalid economic payload object (for BANK_ENTRY): numeric amount instead of string,
and impossible credit arithmetic. Each defect independently rejects with 400:

```json
{"operation_occurrence_id":"019a0000-0000-7000-8000-000000000008","event_id":"019a0000-0000-7000-8000-000000000009","entry_id":"019a0000-0000-7000-8000-00000000000a","previous_entry_id":null,"kind":"DEPOSIT","direction":"CREDIT","amount":100,"balance_before":"0","balance_after":"99"}
```

Additionally, exact replay of the valid bank envelope is 200 with one entry;
changing its amount/balance to `"101"` under the same keys is 409; delivering revision
3 before 2 is 409; replaying epoch 1 after a committed ownership epoch 2 is fenced
409 and cannot serve either owner's obsolete data. Authentication failures always
precede content/existence disclosure.

## Acceptance checklist — Game owner / coordinator

- [ ] Accept/amend this exact version; confirm existing facts versus new export work and identify producer owner and pinned implementation SHA.
- [ ] Confirm canonical ID/TypedDefinitionRef/house/town/slot mappings, all bounds, nested inventory completeness and eligible house custody policy.
- [ ] Prove one reproducible logout cut across all roots under concurrent delivery, market settlement, house changes, crash and disconnect; no partial snapshot END.
- [ ] Confirm account/world bank semantics, fee/debit coverage, market partial-fill/held-credit mapping, player-trade two-leaf scope and history availability/retention.
- [ ] Define durable ownership_epoch mapping and transfer cutoff notification/fence, including delayed export and buyer/seller races; no session proof exported.
- [ ] Prove source transaction/export-cut plus outbox durability, fixed identities/bytes, duplicate/conflict behavior, resend after lost ACK and bounded replay.
- [ ] Accept the cursor, generation admission, initial baseline/empty-history proof and restore/baseline-plus-tail exchange; prove gaps outside replay window stay unavailable.
- [ ] Qualify dedicated mTLS purpose/environment, canonical validators, limits, timeout, disabled paths and secret-safe diagnostics on exact producer/consumer revisions.
- [ ] Approve retention/privacy policy and off-by-default rollout; acceptance does not authorize production activation or Game changes in this task.

## Acceptance checklist — Platform

- [ ] Accept exact producer contract and pin producer/consumer/fixture SHAs; implement nothing from an unresolved field or assumption.
- [ ] T72 proves atomic ingest/watermark/dedup ACK, exact-byte conflicts, restart/restore replay, quarantine/gap handling and bounded staging/retention erasure.
- [ ] T76 proves count/digest/root/graph validation and atomic END visibility, item search and clearly dated logout state; no mutation or guessed valuation.
- [ ] Test owner versus foreign account, staff, sold character, delayed old epoch, pending/ambiguous transfer, unknown ownership and cache/download/search bypasses.
- [ ] Test account bank does not follow character sale, market held credit is not spendable, economic events deduplicate and cross-family records are not double-counted.
- [ ] Test all valid/invalid fixtures, conflicting keys, reordered/unknown members, excessive size/depth, wrong version, timeout before/after commit, duplicate ACK and expired replay.
- [ ] Prove initial baseline, empty-history evidence, replay-window exhaustion, restore generation fencing and rollback never restore older privacy/ownership authority.
- [ ] Prove no public/CDN/log leakage, deletion/backup expiry, separate default-off flags, truthful unavailable/invalid/stale states and exact-revision cross-repository qualification before activation.

## Open questions for the game owner

1. Can Game provide an atomic/reproducible logout cut across equipment, every town depot, inbox and house custody? What is the supported crash/disconnect path and maximum export size?
2. Which house items are eligible private custody, including co-owners, guests and guild halls? Confirm town, house, equipment-slot and TypedDefinitionRef wire representations and UUID versions.
3. Will Game accept a durable ownership_epoch and a synchronous serving fence at `TransferCharacterOwnership` cutoff? Which existing ownership evidence carries it, and how do restore and account erasure fence queued exports?
4. Which bank ledger causes beyond DEPOSIT/WITHDRAW/TRANSFER must be covered (fees, market settlement, held credit), and are market partial-fill events and player-trade receipts complete and replayable for 30 days? Is the pinned two-leaf trade shape still the intended v1 scope?
5. Accept or amend the proposed limits, 30-day payload/replay window, snapshot staging/backup deletion limits, request rate/deadline and dedicated mTLS purpose. Who approves private-copy legal holds?
6. What exact initial/replacement baseline exchange proves complete or empty retained history, admits recovery generations and supplies tail replay after either side restores? Until accepted and qualified, these states remain unavailable.

This document alone closes no G-10 gate and authorizes no runtime, schema, workflow,
Game repository write, deployment or product activation. All proposal values await
coordinator acceptance; unknown or ambiguous state denies.
