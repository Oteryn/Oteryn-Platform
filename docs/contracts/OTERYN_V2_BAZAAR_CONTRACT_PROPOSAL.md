Status: PROPOSED by Platform, pending acceptance by the game owner (coordinator).

# Oteryn-v2 Bazaar contract proposal (G-7)

Contract ID: `oteryn-v2-bazaar-v1`; proposal revision 1, 2026-11-05.

## 1. Purpose, ownership and acceptance boundary

This is Platform's proposed answer to G-7 in [the Rust portal plan](../architecture/OTERYN_PORTAL_RUST_PLAN.md), gate table and §10. It is a reviewable wire and semantic proposal, not the game owner's promised wire, an accepted contract, an implementation task, or evidence that G-7 is delivered. Only the coordinator/game owner can accept the Game obligations. Platform must separately accept its consumer obligations on exact revisions. Every route, command, bound and signing profile newly named below is **PROPOSED**.

Platform owns listing/auction/offer policy, authenticated owner authorization, verified email/2FA/sudo gates, bid serialization, moderation, notifications, immutable commercial ledger and coin Wallet reservations/settlement. Game owns eligibility, Character locks, canonical ownership, item identity/attributes/custody, the transferable-item policy, account/world gold holds and ledger, durable effects and receipts. No shared SQL or page-render query to Game is permitted. An admin restriction can narrow Game eligibility; it cannot make an ineligible asset transferable. Gold is never a Platform Wallet balance.

| Portal module (§10) | Consumes |
|---|---|
| T58 | All command intents, Game receipts, catalog/status ingests, outbox reconciliation and restore fencing |
| T59 | Character eligibility, lock, verified immutable snapshot and existing ownership-transfer outcome |
| T60 | Transferability list, item escrow with attributes, offer/auction gold holds, delivery/return |
| T61 | Only the approved listing projection and its freshness/privacy state for search, filters and saved searches |
| T62 | Durable listing/saga transitions for notifications; never poll delivery as evidence of success |
| T63 | Committed sales and approved listing facts for prices/statistics; never failed/ambiguous sales or private maximum bids |
| T64 | Restrictive moderation, reports and audited fee/policy revisions; four-eyes cancellation cannot reverse a committed transfer |

T57 supplies the Bazaar domain; T22 supplies the Character command dependency. This proposal does not close either dependency or enable T58–T64. Coin settlement reuses the accepted transfer-before-Wallet-settlement invariant. D13 specifies free listing, 10% coin sale commission and 2.5% gold sale commission, with gold commission burned in Game. Gold rounding below requires Game acceptance; it cannot silently override D13.

### Sources and their actual status

Read together, without promoting a proposal to accepted authority:

- [Store wire](OTERYN_V2_STORE_WIRE_CONTRACT.md): **PROPOSED**, reused canonical bytes, SHA-256 bindings, Game-initiated pull/read/receipt, bounded paging, dedicated peer guard and reconciliation conventions. Bazaar adds its own purpose and routes; it does not reuse Store credentials or pretend Store is accepted.
- [Public game projections](OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md): accepted Platform authority/privacy/ordering semantics; transport deferred there. Reuse source epochs, per-subject revisions, high-watermarks, poison-record quarantine and privacy floors.
- [Character authority commands](OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md): accepted semantic command/result/idempotency/ambiguity boundary. Its eventual wire is already fixed by the accepted Game document below; `COMMITTED` on that wire means semantic `COMPLETED`.
- [Entitlement game delivery](OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md): accepted orchestration/saga and reserve-before-submit semantics; transport/signing deferred there. Bazaar custody is a move of existing assets, never an entitlement mint or a Store CLAIM.
- [Restore reconciliation](OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md): reuse the existing Game restore notice, recovery-generation lock and reopening rules without changing its bytes.
- [Native gateway login](OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md) §8.3/§9: existing pure Ed25519, unpadded base64url, immutable key-id binding and current/retiring key conventions. The four required contracts above do **not** specify signed Bazaar snapshots. §6 proposes a separate Game-signed profile using these primitives; it grants no access to admission signing keys or admission trust scopes.

### Pinned Game reference (read-only)

All Game paths cited here are relative to [Oteryn/Oteryn-Game at `857f15f376907c592bb28d854fccff18a15cdf87`](https://github.com/Oteryn/Oteryn-Game/tree/857f15f376907c592bb28d854fccff18a15cdf87), inspected outside the Platform checkout. No Game changes or pushes are authorized.

| Game path | Fact relied on; limit of evidence |
|---|---|
| `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §§2–7 | Accepted `oteryn-game-character-authority-commands-v1`: pull transport, exact `TransferCharacterOwnership` intent/receipt, three Bazaar purposes, TTL, recovery and deduplication |
| `crates/gameplay/native-admission-source/src/character_commands.rs` | Existing `ListPendingCharacterCommandsV1`, `ReadCharacterCommandIntentV1`, `PublishCharacterCommandReceiptV1`, compiled paths, bounded mTLS consumer; its current consumer explicitly decides `CreateCharacter` only, so a declared transfer wire is not proof of enabled transfer execution |
| `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §§3–5.1 | Accepted push/mTLS, transactional outbox, epoch fence, operator resync and completeness-time watermark |
| `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` §§3–7; `crates/gameplay/native-admission-source/src/public_projections.rs` | Dedicated public-purpose identity; WorldId without placement, vocation IDs, source ordering and hiding locked/transferring profiles |
| `crates/gameplay/durability/src/durability/character_authority.rs` | Canonical current Character owner/world/revision and recovery-fenced authority; no inference of a Bazaar lock API |
| `apps/game-server/migrations/0071_account_bank.sql`; `crates/gameplay/economy/src/bank.rs` | Account/World bank, operation binding and immutable paired ledger, amount/balance ceiling 999999999999 gold; ordinary DEPOSIT/WITHDRAW/TRANSFER are not Bazaar holds |
| `apps/game-server/migrations/0157_depot_transfers.sql`; `crates/gameplay/durability/src/durability/character_depot.rs` | Existing item move receipts/lines, exact instance, quantity, state revision and definition checks, Character/World depot custody; not a cross-account Bazaar escrow API |
| `crates/gameplay/domain/src/domain/item_definition.rs` | `TypedDefinitionRef = (family, production_key, revision_ref)` and definition-specific stack maximum; weight unit centi-oz |
| `crates/gameplay/content/src/content/reference_playable.rs`; `crates/gameplay/durability/src/durability/player_trade.rs` | `tradeable` exists as a known/unknown content fact. A market flag is not a substitute for live transferability; existing player-trade policy is not automatically Bazaar policy |
| `crates/gameplay/durability/src/durability/store/codec.rs`; `crates/gameplay/durability/src/durability/store/outbox.rs` | Existing durable binding digest, stored receipt, recovery generation, unacknowledged receipt publication/ack pattern; Bazaar requires its own records and must not mint via Store |

The pinned reference proves building blocks, not G-7 lock, signed snapshot, gold hold or Bazaar escrow support. New names in §§3–7 are proposal names, not claims about existing Rust symbols.

## 2. Direction, transport and authenticated audience

Logical **Platform → Game commands** are immutable Platform outbox intents returned to Game's pull consumer. All HTTP connections remain **Game → Platform**. Game effects, snapshots and receipts return as **Game → Platform ingest**. No new inbound Game listener, browser-to-internal call, broker, bearer token or shared database transport is introduced.

Reuse HTTP/1.1 over TLS 1.3 with client certificates and the existing `http1_mtls::exchange` architecture. Dedicated proposed Bazaar certificate subjects, one per authority host, are allowlisted for the configured `source_authority` and these routes only. Identity must differ from Store, public projection, native evidence, runtime, account projection, character commands and restore identities. Existing transfer and restore routes keep their existing identities. Fields cannot select peer trust. Trust terminator-supplied `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, exact configured `SSL_CLIENT_S_DN` (1–128 printable ASCII bytes), never user-forwarded equivalents. Missing/overlapping identities or source scope make the surface unavailable.

Connect 1 s, handshake 2 s, command exchange 3 s, ingest exchange 5 s; one in-flight exchange per consumer, independent catalog/status publication lanes. Poll commands every 1 s, back off on 429/503 up to 10 s, honor `Retry-After` (positive integer seconds). Rescan from the start after eight full pages or 30 s, whichever comes first, skipping already known keys. A rescan is required because restore may reopen an old key. No HTTP timeout proves failure of an effect.

All proposed routes are `POST`, return `Cache-Control: no-store`, use exact operation names below and the same closed-body guard as Store/Character commands. Failures have empty bodies. Status order: 503 disabled/misconfigured, 401 wrong TLS peer/source authority, 429 peer limit, then encoding/schema/handler. Proposed peer limit 120 requests/minute (60 s window, keyed by purpose plus SHA-256 of peer subject), owner-command issuance limit 20/minute/account and pending backlog 24 per issuer; exhaustion refuses new issuance without writing an intent. Operational tuning requires a reviewed compatibility revision. No account identifiers in limiter logs/metric labels.

### Encoding and scalar types

Each wire body is one UTF-8 JSON object, exact listed members in listed order, no BOM, duplicate keys, unknown members, insignificant whitespace, backslashes, trailing newline, floats or exponent notation. All members are required; only an explicit `| null` permits a present `null`. Arrays are ordered as stated and never silently truncated. Content-Type is `application/json`; Transfer-Encoding is absent, Content-Encoding absent or `identity`. Content-Length, if present, is canonical unsigned decimal at most the route byte limit; above limit is 413. Actual bodies above the limit are also 413; malformed/empty bodies or other encoding violations are 400. Proposed maximum JSON depth is 12 (needed for signed snapshot sections); this explicitly differs from Store's shallow messages. Byte ceilings include all nesting and signature envelope.

| Type | Exact form, units and inclusive bounds |
|---|---|
| `id` | Canonical lowercase non-nil UUIDv7, 36 ASCII bytes, pattern `[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}` anchored to the whole string |
| `u64` | String of 1–20 decimal digits, no leading zero, numeric 1..18446744073709551615 |
| `z64` | `"0"` or `u64` |
| `time` | `u64` Unix seconds UTC, never milliseconds or an ordering key |
| `gold` | Canonical decimal string, 0..999999999999, base gold units (one gold coin = 1); same bank ceiling as the pinned Game |
| `gold+` | `gold` greater than zero |
| `coin+` | Canonical non-zero decimal string, 1..9223372036854775807, whole Platform Wallet coin units |
| `n[a,b]` | JSON integer, a..b |
| `hash` | 64 lowercase hexadecimal characters, SHA-256 over exact canonical bytes |
| `key` | ASCII `[a-z0-9][a-z0-9._:/-]{0,127}`, 1–128 bytes; opaque stable Game key, no localized/free text |
| `authority` | ASCII `[A-Za-z0-9._:/-]{1,128}` equal to configured peer namespace |
| `name` | 2–29 ASCII letters/spaces under existing CHAR-NAME-1 and public-projection name rule; no leading/trailing/double space, slash or escape |
| `b64` | Canonical unpadded base64url `[A-Za-z0-9_-]+`, re-encoding decoded bytes must be identical |
| `nil` | `00000000-0000-0000-0000-000000000000`, cursor start only |

Definition record `Definition`, exact order: `family` (`key`), `production_key` (`key`), `revision_ref` (`key`). These copy `TypedDefinitionRef`; they are not a numeric client sprite/item ID. A Game definition outside these proposed bounds is unsupported, never shortened or remapped. All referenced AccountIds are Platform-issued; CharacterIds/item instances are Game-issued; WorldIds are Platform Registry IDs. Game-minted listing-lock, escrow and hold IDs are `id`s. No ChannelId, node, coordinates, session-generation or lease token crosses this surface.

`binding_digest` is SHA-256 of the **whole stored intent**; `receipt_digest` is SHA-256 of the **whole stored receipt**; `snapshot_hash` is SHA-256 of the **snapshot object alone** (§6), excluding signature envelope; `attributes_hash` hashes the entire retained canonical Game attribute state, including non-exportable fields, not just the visible attributes. These are distinct hashes. Never calculate a digest by parsing and reordering input. Synthetic fixtures in §10 use the same byte rules.

## 3. Closed routes and exchange records

Prefix `B = /internal/v1/bazaar`. All requests start with `Head = contract_version` (JSON integer `1`), `operation` (exact route operation), `source_authority` (`authority`), in that order. `Head` and table composition are notation only: no JSON `head` member exists.

| Path after B | Operation | Request/200 response maximum bytes | Logical direction |
|---|---|---|---|
| `/commands/pending` | `ListPendingBazaarCommandsV1` | 512 / 8192 | Platform → Game intents index |
| `/commands/read` | `ReadBazaarCommandIntentV1` | 512 / 4096 | Platform → Game exact command |
| `/commands/receipt` | `PublishBazaarCommandReceiptV1` | 262144 / 256 | Game → Platform durable decision |
| `/commands/reconcile` | `ListBazaarCommandStatesV1` | 512 / 12288 | Platform → Game durable receipt comparison |
| `/transferability` | `PublishBazaarTransferabilityV1` | 262144 / 256 | Game → Platform full policy snapshot |
| `/status` | `PublishBazaarListingStatusV1` | 2048 / 256 | Game → Platform full listing custody status |
| `/watermark` | `PublishBazaarWatermarkV1` | 512 / 256 | Game → Platform completeness/liveness |

### Pull/list/read/reconcile

Pending request: `Head`, `after_source_revision` (`z64`), `after_operation_id` (`id` or `nil`, nil iff revision is `"0"`), `max_entries` (`n[1,32]`). Response: `contract_version` (`1`), `pending` (0–32 entries). Entry exact order: `command` (§4 enum), `operation_id` (`id`), `source_revision` (`u64`). Strict ascending numeric revision then UUID byte order, unique IDs, above cursor. Cursor is last entry; short/empty page restarts at zero. PENDING means no current terminal receipt, even after expiry.

Read request: `Head`, `issuer_authority` (constant `OTERYN_PLATFORM_BAZAAR_AUTHORITY`), `command` (§4 enum), `operation_id` (`id`). Response is the exact immutable intent (§4), verbatim. 404 absent key; 409 key bound to different command. Neither means the Game effect is absent.

Reconcile request has the pending request shape with operation `ListBazaarCommandStatesV1`. Response: `contract_version` (`1`), `operations` (0–32 entries). Each entry: `command`, `operation_id`, `source_revision` as above, then `state` (`PENDING|REOPENED|COMMITTED|REJECTED`), `receipt_digest` (`hash | null`), `recovery_generation` (`u64 | null`). Last two are null iff PENDING/REOPENED. Includes every operation, same cursor/order, not just pending. This is a comparison feed, not permission to execute. Game republishes its original receipt if Platform is pending; different digests or a missing key with a Game receipt require operator reconciliation, never another mutation.

### Receipt and ingest acknowledgements

Receipt success and accepted ingests: exact `{"contract_version":1,"result":"accepted"}`. A strictly older catalog/status/watermark returns `{"contract_version":1,"result":"superseded"}` with no state change. No superseded result for command receipts. Identical current receipt bytes return accepted without another side effect. Only after this response may Game acknowledge its outbox row. A lost acknowledgement causes byte-identical publication.

Common statuses: 200 success as defined; 400 schema/bound/semantic echo violation; 401 wrong purpose/peer/source or unverifiable signature; 404 receipt without stored intent or missing exact read; 409 conflicting reuse/digest/generation; 413 byte excess; 429 limit with Retry-After; 503 disabled, unsafe authority, unknown signing trust or dependency unavailable. 404/409 receipt conflicts retain both records, stop that key's automatic publication and alarm. 400/401/413 require defect/configuration repair, not an unchanged busy retry. 429/503/network loss retry the same bytes/key. HTTP status alone never supplies a terminal Game refusal.

## 4. Proposed Platform → Game command schema

Intent common prefix, in order:

| Member | Type / rule |
|---|---|
| `contract_version` | integer `1` |
| `command` | one command in the table below |
| `variant` | constant `PLATFORM_BAZAAR` |
| `issuer_authority` | constant `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| `issuer_decision_id` | `id`, immutable Platform authorization decision |
| `source_revision` | `u64`, monotone issuer outbox sequence |
| `operation_id` | `id`, fixed before first publication |
| `listing_id` | `id`, Platform listing/workflow identity |
| `seller_account_id` | `id`, original economic seller, never an escrow account |
| `world_id` | `id`, fixed asset/funding world |

Then the exact command-specific members below, followed by common tail `issued_at_source` (`time`), `expires_at_source` (`time`), `audience` (constant `OTERYN_GAME_BAZAAR_AUTHORITY`). All members non-null unless specified. Technical attempt TTL is `0 < expires - issued ≤ 300` seconds; `issued` cannot exceed trusted Game time by more than 5 s. Check stored operation binding/receipt **before** expiry so a late retry returns the original result. An unexecuted expired attempt gets a durable rejection, not silent deletion. Asset custody and holds have no automatic TTL; listing lifetime (1–14 days for Character auction, 14 days for item notice) is Platform policy, not command expiry.

| `command` | Exact additional members in order | Committed `facts` (§5) |
|---|---|---|
| `LockBazaarCharacter` | `character_id` (`id`), `expected_character_revision` (`u64`), `eligibility_revision` (`u64`), `snapshot_schema_version` (integer `1`) | `CharacterLockFacts` |
| `BindBazaarCharacterTransfer` | `character_id` (`id`), `lock_id` (`id`), `expected_lock_revision` (`u64`), `transfer_operation_id` (`id`), `transfer_binding_digest` (`hash`), `purpose` (existing `BAZAAR_LISTING_ESCROW\|BAZAAR_SETTLEMENT\|BAZAAR_ESCROW_RETURN`), `sale_id` (`id \| null`), `funding` (`Funding \| null`), last two non-null iff SETTLEMENT | `TransferBindingFacts` |
| `UnlockBazaarCharacter` | `character_id` (`id`), `lock_id` (`id`), `expected_lock_revision` (`u64`) | `UnlockFacts` |
| `EscrowBazaarItem` | `seller_character_id` (`id`), `item_instance_id` (`id`), `expected_state_revision` (`u64`), `quantity` (`n[1,4294967295]`), `transferability_revision` (`u64`) | `ItemEscrowFacts` |
| `DeliverBazaarItem` | `escrow_id` (`id`), `expected_escrow_revision` (`u64`), `buyer_account_id` (`id`), `buyer_character_id` (`id`), `escrow_receipt_digest` (`hash`), `sale_id` (`id`), `funding` (`Funding`) | `ItemMoveFacts` |
| `ReturnBazaarItem` | `escrow_id` (`id`), `expected_escrow_revision` (`u64`), `seller_character_id` (`id`), `escrow_receipt_digest` (`hash`) | `ItemMoveFacts` |
| `HoldBazaarGold` | `buyer_account_id` (`id`), `buyer_character_id` (`id`), `offer_id` (`id`), `hold_purpose` (`OFFER\|RESERVE`), `amount` (`gold+`), `expected_hold_revision` (`z64`) | `GoldHoldFacts` |
| `ReleaseBazaarGold` | `buyer_account_id` (`id`), `hold_id` (`id`), `expected_hold_revision` (`u64`), `release_reason` (`OUTBID\|DECLINED\|CANCELLED\|UNSOLD\|EXCESS\|TERMINAL_REFUSAL`), `amount` (`gold+`) | `GoldReleaseFacts` |
| `PayoutBazaarGold` | `buyer_account_id` (`id`), `hold_id` (`id`), `expected_hold_revision` (`u64`), `sale_id` (`id`), `gross_amount` (`gold+`), `fee_bps` (integer `250`), `fee_amount` (`gold`), `seller_amount` (`gold+`), `delivery_operation_id` (`id`), `delivery_receipt_digest` (`hash`) | `GoldPayoutFacts` |

The 4096-byte intent limit applies even if individual fields meet their limits. All commands revalidate current authoritative account/Character/world eligibility at execution. Seller cannot be buyer. Never trust caller-supplied attributes, display names, balances or owner assertions. Delivery/return conserves exact assets; neither command mints a replacement.

`Funding` exact order: `currency` (`GOLD|COINS`), `reservation_id` (`id`), `hold_revision` (`u64 | null`), `gross_amount` (`gold+` when GOLD, `coin+` when COINS). GOLD names the Game hold and its expected revision; COINS names a Platform Wallet reservation and requires null hold_revision. Platform must prove its coin reservation locally; Game does not treat it as Game currency or credit gold for it. Funding binds listing, unique sale, parties and exact gross, which may be below the held maximum. The maximum remains held until payout and explicit EXCESS release.

For GOLD, DeliverBazaarItem atomically verifies/fences that hold before moving the item; settlement BindBazaarCharacterTransfer atomically verifies/fences it before the transfer can become executable. Both verify retained seller headroom, require gross no greater than the hold, and move the hold into DISPOSITION_PENDING for that sale. Game rejects competing releases, payouts or sale bindings while this disposition can commit. A terminal delivery/transfer refusal may clear the disposition fence only with durable no-late-commit evidence; the funds remain held. COMMITTED delivery advances to AWAITING_PAYOUT; only the matching Payout may consume it. These disposition fences are internal state changes and do not change the public hold_revision: that revision advances on create, replacement amount, Release and Payout only. Thus Payout can bind the known funding hold revision without modifying the existing Character receipt wire; fenced holds refuse all other amount changes. Exact replay does not fence twice. For coin sales Platform's listing/Wallet transaction must analogously fence release before making a handoff intent listable. These interlocks prevent an out-of-order release from leaving an executable unfunded transfer. A later payout's sale_id, currency, amount and receipt must match the fenced disposition. This is a saga hold, not a distributed transaction.

### Character eligibility, lock and transfer integration

Game determines eligibility atomically with the lock and signed snapshot: current seller owner, live Character, offline/no admission or session, lifecycle eligible, no conflicting service/transfer/delete/lock, supported snapshot, valid account, no blocking penalty, housing prepared, guild obligations resolved, no unhandled market/mail/depot obligations and any accepted cooldown. It returns typed reasons (§5), not private sanction detail. Existing transfer guards still run. The coordinator must accept the exact obligations matrix (Q1); unknown obligations deny listing. Platform enforces the accepted 30-day resale cooldown independently; Game must not certify a cooldown it cannot prove.

A committed lock persists in Game and prevents login/admission, gameplay writes, rename/delete/world transfer, item/bank changes attributable to the Character and competing ownership mutations. It binds listing, original seller, world, Character revision and snapshot hash. Taking a snapshot before acquiring the lock is forbidden. Cross-account bank changes do not make a Character snapshot a bank-balance authority. Account-wide entitlements are excluded. If a supposedly frozen fact changes, Game marks listing status INVALID and fences sale pending reconciliation; it never silently substitutes snapshot bytes.

Use **existing** `TransferCharacterOwnership`, with its unchanged route, member set and result codes, for every ownership leg. Preserve the existing listing-escrow → settlement/escrow-return saga: original seller → configured native escrow AccountId (`BAZAAR_LISTING_ESCROW`), then escrow → buyer (`BAZAAR_SETTLEMENT`) or escrow → seller (`BAZAAR_ESCROW_RETURN`). Escrow AccountId is an accepted deployment allocation, never minted in this proposal. The Character lock survives listing-escrow transfer; it is released only by explicit unlock after final authoritative ownership receipt (or after proved no-transfer cancellation).

`BindBazaarCharacterTransfer` proposes the necessary interlock without adding fields to the existing transfer wire. Platform first stores the exact existing transfer intent as **not yet listable**, then issues a bind command with its ID and SHA-256 of every original intent byte. Game validates the lock, purpose and listing provenance and stores a one-use digest binding. When it later reads the existing transfer intent, Game validates the complete bytes against that digest, its current owner, the permitted escrow/buyer/seller destination, decision provenance, purpose and TTL before any ownership write. A digest alone cannot certify those unread fields. Bind COMMITTED permits Platform to publish that transfer. Game transfer execution must require this binding for locked Characters, consume it atomically with its normal ownership result and advance the lock revision. Timeout after binding keeps the lock; it is not a transfer receipt. Unused binding grants no execution beyond either its bind intent deadline or the bound transfer's TTL; a new attempt requires terminal no-commit evidence for the old attempt and a new bind. An expired bind command cannot authorize an expired transfer.

This interlock and atomic cross-command enforcement require Game acceptance/implementation; the existing transfer wire alone does not provide them. No unlocked direct seller-to-buyer path is inferred. Game must reject unlock while a transfer authorization can still execute; after settlement/return it verifies the exact current final owner. Competing settlement/return bindings serialize on the lock, and one must be terminally refused before the other may become executable.

### Item custody and transferability

Escrow removes the exact selected quantity from seller spendable custody under Game transaction/fence and records immutable attributes plus seller/Character/world, definition revision, quantity and state revision. Stack splitting follows existing Game identity/conservation rules; a nonstackable item must have quantity 1. Content's proven stack maximum remains an upper bound. Proposed v1 accepts whole empty containers only, never nested contents, timed/decaying items, bound/quest/nontransferable items or an attribute state Game cannot conserve. Unsupported features deny, never strip attributes.

A transferable-definition list (§7) is advisory for UI and pre-submit gates. Game revalidates its current admitted Content and instance restrictions during escrow; missing/unknown `tradeable` is denied for Bazaar even where another trade profile allows unknown. Platform can narrow the list and minimum value but cannot broaden it. A new deny policy suspends affected listings; it does not delete escrow or prevent eventual safe return.

Deliver and return serialize on one escrow record: exactly one terminal move wins. Both validate receipt digest, current revision, world and current target ownership. Delivery is into Game-owned durable Character inbox/custody, with every attribute conserved, not a ground drop or best-effort inventory insert. Full inventory/offline buyer cannot lose an asset: if the accepted durable destination cannot receive it, the effect stays pending or is terminally refused with escrow intact. The coordinator must confirm destination/capacity rules (Q3). Seller return uses the original seller's currently authorized Character in that world; missing/transferred/deleted targets require support custody reconciliation, never transfer to the new owner of a stale CharacterId.

### Gold holds and payout

Funds come exclusively from the buyer's **AccountId + WorldId bank**, as in the existing bank contract; no inventory gold, guild bank, other world or Portal coin conversion. Game must extend its single bank authority so ordinary withdrawals/transfers/fees also respect held funds. A second parallel balance in Bazaar is forbidden. Invariants: `0 ≤ held_total ≤ bank_total ≤ 999999999999`; available = bank_total − held_total; concurrent bank and Bazaar operations serialize on the same authority rows.

One hold per (`listing_id`, `offer_id`, buyer, world). `expected_hold_revision="0"` creates; later Hold with that key is an atomic **replacement total**, not an additive debit. It may increase the maximum while preserving existing held funds; it cannot lower the total or change OFFER/RESERVE. A RELEASED, payout-in-progress or paid hold cannot be replenished; a later offer needs a new offer_id and authorization. A decrease is an explicit partial Release; reserve selection is Platform saga state. OFFER holds secure a submitted maximum/negotiated offer; RESERVE secures a fixed-price or accepted buy-order match before handoff. Neither constitutes a payment. Full maximum must be held before an offer becomes eligible/visible as accepted. An increase that cannot be fully funded rejects with the old hold unchanged. Reserve the new winning bidder before releasing the old bidder, so failed funding never removes the funded winner.

Release removes exactly `amount` from that hold, credits availability only (bank total unchanged), and advances revision. Zero remaining marks RELEASED. Over-release or stale revision rejects atomically. A release is prohibited while the sale's delivery/transfer can still commit or while payout is pending/ambiguous. Holds do not auto-release at listing end, command TTL, network timeout, moderation suspension or feature rollback.

Payout is one Game transaction: prove COMMITTED asset delivery/settlement for this sale, debit `gross_amount` from the exact remaining hold and buyer bank total, credit `seller_amount` to seller's same-world bank, burn `fee_amount`, write immutable linked ledger/receipt/outbox and advance hold revision. It must reference a current-generation receipt that Game itself holds; Platform's assertion alone is insufficient. For Characters this is the **settlement** TransferCharacterOwnership receipt, never listing escrow; for items it is DeliverBazaarItem. Buyer/seller/world/asset/listing must match. Game must retain the cross-channel evidence to validate both forms.

Proposed exact rounding: `fee_amount = ceil(gross_amount × 250 / 10000)` in checked wide integer arithmetic, `seller_amount = gross_amount − fee_amount > 0`. Below-minimum net payout is refused before listing/bid acceptance, not discovered after handoff. No commission cap; never overflow, use floating point or charge commission on an unsold listing. Proposed headroom interlock: HoldBazaarGold also reserves same-world seller bank credit capacity for the maximum net proceeds, using the accepted fee formula, in that same Game transaction. A replacement increase reserves its full increased headroom or rejects unchanged; Release recomputes headroom from the remaining potential gross under the exact fee formula and reduces unused headroom only when no handoff can commit, and Payout consumes the exact net headroom while retaining the remainder until EXCESS release. Every ordinary seller bank credit must respect outstanding headroom reservations; they are not spendable gold and do not credit the seller. Multiple funded offers may therefore conservatively reserve multiple capacities. Seller balance plus all reserved incoming net credits must remain at/below the bank ceiling. This new interlock needs Game acceptance (Q4); a balance preflight alone cannot protect payout after delivery. The remaining maximum above gross stays held until an explicit EXCESS release after payout. Payout retries cannot credit/burn twice. Amount conservation is buyer debit = seller credit + burn.

## 5. Game → Platform durable decision schemas

Receipt exact common fields in order: `Head` (operation `PublishBazaarCommandReceiptV1`), `issuer_authority` (Bazaar constant), `command` (intent command), `operation_id` (`id`), `binding_digest` (`hash`), `listing_id` (`id`), `outcome` (`COMMITTED|REJECTED`), `result_code` (below), `facts` (typed object for committed command, **null** for rejected), `recovery_generation` (`u64`), `decided_at` (`time`). Subject/echo values must match the bound intent. No accepted-pending HTTP acknowledgement is a terminal receipt. Transient dependency failure leaves an intent PENDING; optional internal retry state never changes this terminal wire.

Facts exact member order follows the table. Fields are non-null except where shown. Every returned revision is a Game monotone state revision; every `*_id` is `id`.

| Record | Exact members and types |
|---|---|
| `CharacterLockFacts` | `character_id` (`id`), `lock_id` (`id`), `lock_revision` (`u64`), `eligibility_revision` (`u64`), `signed_snapshot` (`SignedSnapshot`, §6) |
| `TransferBindingFacts` | `character_id`, `lock_id`, `lock_revision` (`u64`), `transfer_operation_id`, `transfer_binding_digest` (`hash`) |
| `UnlockFacts` | `character_id`, `lock_id`, `lock_revision` (`u64`), `current_account_id` (`id`) |
| `ItemEscrowFacts` | `escrow_id`, `escrow_revision` (`u64`), `seller_character_id`, `item` (`Item`, §6) |
| `ItemMoveFacts` | `escrow_id`, `escrow_revision` (`u64`), `destination_account_id`, `destination_character_id`, `delivery_id` (`id`, durable Game custody movement), `item` (`Item`, exact conserved definition/quantity/attributes; current state revision) |
| `GoldHoldFacts` | `hold_id`, `hold_revision` (`u64`), `buyer_account_id`, `offer_id`, `hold_purpose` (`OFFER\|RESERVE`), `held_amount` (`gold+`, replacement total) |
| `GoldReleaseFacts` | `hold_id`, `hold_revision` (`u64`), `released_amount` (`gold+`), `remaining_held` (`gold`), `hold_state` (`ACTIVE\|RELEASED`, RELEASED iff zero) |
| `GoldPayoutFacts` | `hold_id`, `hold_revision` (`u64`), `sale_id`, `gross_amount` (`gold+`), `fee_amount` (`gold`), `seller_amount` (`gold+`), `remaining_held` (`gold`), `buyer_ledger_entry_id` (`id`), `seller_ledger_entry_id` (`id`), `burn_entry_id` (`id`), `delivery_operation_id`, `delivery_receipt_digest` (`hash`) |

`BAZAAR_OK` is the only COMMITTED code. Closed REJECTED set, applicable commands:

| Code | Commands / guaranteed no-effect meaning |
|---|---|
| `BAZAAR_UNSUPPORTED` | All; unknown command/schema/profile/version, no attempt effect |
| `BAZAAR_INTENT_INVALID` | All; bad semantic binding/audience; no attempt effect |
| `BAZAAR_INTENT_EXPIRED` | All; not previously decided and expired before execution |
| `BAZAAR_OPERATION_CONFLICT` | All; fresh key conflicts with a revision/binding; never overwrites an existing receipt |
| `BAZAAR_OWNER_MISMATCH` | Lock, Bind, Unlock, Escrow, Deliver, Return, Hold; current owner differs |
| `BAZAAR_NOT_FOUND` | All; required Character/asset/lock/hold not present with terminal no-commit proof |
| `BAZAAR_ACCOUNT_RESTRICTED` | Lock, Escrow, Hold, Deliver; account policy forbids a new acquisition/listing; never blocks necessary safe release/return merely due to suspension |
| `BAZAAR_SESSION_CONFLICT` | Lock, Bind, Escrow; authoritative active session/admission conflict |
| `BAZAAR_LIFECYCLE_CONFLICT` | Lock, Bind, Unlock, Escrow, Deliver, Return; conflicting lifecycle/service/custody state |
| `BAZAAR_HOUSING_NOT_PREPARED` | Lock, Bind; unresolved housing disposition |
| `BAZAAR_GUILD_OBLIGATION` | Lock, Bind; unresolved guild obligation |
| `BAZAAR_MARKET_OBLIGATION` | Lock, Bind; unresolved market/mail/depot obligation |
| `BAZAAR_COOLDOWN` | Lock; resale cooldown authoritatively unsatisfied |
| `BAZAAR_STALE_REVISION` | All; expected state/policy revision does not match |
| `BAZAAR_ITEM_NOT_TRANSFERABLE` | Escrow, Deliver; actual instance/policy denies |
| `BAZAAR_ATTRIBUTES_UNSUPPORTED` | Escrow; cannot conserve/export supported attribute representation |
| `BAZAAR_QUANTITY_INVALID` | Escrow; stack/count invalid |
| `BAZAAR_DESTINATION_UNAVAILABLE` | Deliver, Return; durable target cannot receive, escrow remains intact |
| `BAZAAR_INSUFFICIENT_GOLD` | Hold; full amount cannot be held, old hold unchanged |
| `BAZAAR_HOLD_CONFLICT` | Hold, Release, Payout; hold state/amount/offer mismatch or release forbidden |
| `BAZAAR_DELIVERY_NOT_COMMITTED` | Payout; referenced exact effect is terminally absent/rejected; uncertain effect is pending, not this refusal |
| `BAZAAR_PAYOUT_INVALID` | Payout; arithmetic/fee/binding invalid, no gold effect |
| `BAZAAR_SELLER_BANK_LIMIT` | Hold, Payout; protected seller headroom unavailable, no gold effect |

A definitive REJECTED means this operation cannot later commit; any pre-existing lock/hold/escrow remains as before the attempt. Unknown codes are contract defects and deny progress. Only a durable terminal refusal permits a new attempt after current gates and bounded policy; it never implicitly releases another operation's assets/funds. Game stores the decision and outbox in the same transaction as the effect, including refusals. Existing character-transfer refusals stay `CHAR_CMD_*`, never translated on that wire; Platform maps them only in its saga.

## 6. Signed Character snapshot and item attribute records

### Snapshot signing

`SignedSnapshot`, exact order: `snapshot` (`CharacterSnapshot`), `snapshot_hash` (`hash`), `alg` (constant `Ed25519`), `kid` (`key`, max 64 bytes), `signature` (`b64`, exactly 86 characters decoding to 64 bytes).

Use existing pure Ed25519 (RFC 8032) and canonical unpadded base64url conventions, with a separate proposed signature purpose. Exact signing input is the ASCII prefix `oteryn-v2-bazaar-character-snapshot-v1.` followed by unpadded base64url of the **exact canonical snapshot bytes**. `snapshot_hash` must independently equal their SHA-256. The domain prefix, issuer and audience prevent reuse as a login/admission/command token. This is an attestation of a locked sale snapshot, never a bearer credential, ownership proof or indefinite sale authorization.

Game holds the private key outside repositories/images/database/logs; Platform has only an explicitly provisioned Game public-key allowlist scoped to (`OTERYN_GAME_BAZAAR_AUTHORITY`, this profile, `kid`). Reuse current/retiring key and immutable kid→key binding conventions: at most two trusted keys, never reuse/re-trust revoked kids, deny unknown/revoked keys, halt on unavailable trust evidence. No payload-provided public key/URL, no admission key reuse, no weaker signature fallback. Trust is checked at verification and before publication/sale; trust-policy invalidation hides/fences affected listings. A signature's `kid` or snapshot age alone cannot renew trust or unlock assets. Provisioning/rotation/revocation ownership is an explicit acceptance blocker (Q5), not an invented new signing-trust endpoint. Retries publish stored signature bytes, not re-sign with a newer kid.

### `CharacterSnapshot`

Exact field order below. All sections are part of the hash/signature. Complete supported sections only: `null` means **unsupported/not exportable**, never zero/empty; an empty supported collection means known empty. If a required section exceeds a bound or a supported fact is unknown, refuse the lock with no listing effect, never truncate or invent zeros. Arrays hold no duplicate IDs/keys. This v1 deliberately omits unsupported future tabs rather than using an arbitrary JSON object.

| Member | Type / units and bounds |
|---|---|
| `contract_version` | integer `1` |
| `issuer_authority` | constant `OTERYN_GAME_BAZAAR_AUTHORITY` |
| `audience` | constant `OTERYN_PLATFORM_BAZAAR_AUTHORITY` |
| `source_authority` | `authority` |
| `listing_id`, `seller_account_id`, `character_id`, `world_id`, `lock_id` | each `id`, in this order, equal to lock decision/intent |
| `lock_revision`, `character_revision`, `eligibility_revision`, `projection_epoch` | each `u64`, in this order; epoch follows Game restore fence |
| `captured_at` | `time`, lock/snapshot transaction time |
| `name` | `name` |
| `vocation` | `n[1,6]`, existing base IDs: Druid 1, Knight 2, Monk 3, Paladin 4, Sorcerer 5, None 6 |
| `level` | `u64` |
| `experience` | `z64`, experience points |
| `created_at` | `time \| null`, authoritative creation time only |
| `health`, `health_max`, `mana`, `mana_max` | each `z64`, points; current ≤ maximum |
| `capacity_centi_oz` | `z64`, hundredths of an ounce |
| `speed` | `z64`, Game speed units |
| `blessings` | array 0–32 `key`, sorted lexically |
| `skills` | array 0–16 `Skill`, sorted by `key` |
| `world_transfer_eligible` | boolean or null (null means unsupported/unproven; never true by default) |
| `appearance` | `Appearance \| null` |
| `items` | array 0–512 `Item` (no null entries), or whole member null; sorted item_instance_id; only Character-owned transferable sale contents, no other Character/account custody |
| `store_grants` | array 0–128 `key` or null, sorted; only durable Character-bound grants that survive ownership transfer |
| `outfits`, `mounts`, `achievements`, `quests` | each array 0–512 `Progress` or null, in this order, sorted by key; only transfer-surviving Character-bound state |

`Skill`: `key` (closed `fist|club|sword|axe|distance|shielding|fishing|magic`), `level` (`z64`), `progress_bps` (`n[0,10000]`, 100 basis points = 1%). `Progress`: `key` (`key`), `value` (`z64`, Game-defined count/state under the pinned key's version; no localized description). `Appearance`: `look_type` (`n[0,65535]`), `head`, `body`, `legs`, `feet` (each `n[0,255]`), `addons` (`n[0,3]`), `mount` (`n[0,65535]`), in that order. Unknown key semantics cannot become a filter/statistic until Game's catalog defines them. Empty `skills` is permitted only if no supported skills exist; it cannot mask a read failure.

No bank gold field is sent: bank belongs to Account+World and cannot follow a sold Character. T59's optional gold tab must stay absent unless a later accepted version defines transferable Character-owned physical gold separately. No account-wide premium time, account-bound achievements, email, identity link, private inventory text or another Character's assets may be smuggled into optional tabs. Item sale contents and transfer-surviving grants must match what ownership transfer actually conveys (Q2).

### `Item`

Exact order: `item_instance_id` (`id`), `world_id` (`id`), `definition` (`Definition`), `state_revision` (`u64`), `quantity` (`n[1,4294967295]`, constrained further by definition), `attributes_hash` (`hash`), `attributes` (0–32 `Attribute`, sorted by key).

`Attribute` exact order: `key` (closed enum below), `value` (`z64`). Supported proposed exported keys: `tier` (0..255), `upgrade_level` (0..255), `charges` (0..4294967295), `attack` (0..65535), `defense` (0..65535), `armor` (0..65535); value remains a decimal **string**. Unknown attributes are retained intact by Game but cannot be presented as known. This schema does not authorize dropping unexported state. If any non-exportable attribute changes tradeability, time behavior, buyer rights or valuation, v1 refuses that instance until an accepted schema can represent it. Free text, provenance identifying other owners, enchantment scripts and arbitrary key/value blobs are never exported. Game preserves the full immutable attribute state internally across escrow/delivery/return and proves its hash unchanged; Platform never constructs an item from this projection.

## 7. Game → Platform policy/status/watermark ingests

### Transferability full snapshot

Request exact order: `Head` (`PublishBazaarTransferabilityV1`), `world_id` (`id`), `projection_epoch` (`u64`), `projection_revision` (`u64`), `source_observed_at` (`time`), `eligibility_revision` (`u64`), `definitions` (0–1024 entries, sorted by Definition tuple). Entry: `definition` (`Definition`), `tradeable` (boolean), `max_quantity` (`n[1,4294967295]`), `allowed_attribute_keys` (0–6 keys from §6 Attribute enum, sorted lexically). `tradeable=false` explicitly denies; absent definition also denies. Snapshot lists supported known decisions only; unknown content is absent, never true. One complete world list per revision, no paging/truncation. A list beyond bounds blocks activation pending a versioned chunk protocol; no partial world becomes eligible. `eligibility_revision` identifies the full Game ruleset used by instance checks, not merely this flag.

### Full listing custody status

Request exact order: `Head` (`PublishBazaarListingStatusV1`), `listing_id`, `seller_account_id`, `world_id` (each `id`), `projection_epoch`, `projection_revision` (each `u64`), `source_observed_at` (`time`), `asset_kind` (`CHARACTER|ITEM`), `asset_id` (`id`, CharacterId or escrowed item_instance_id), `state` (`LOCKED|ESCROWED|DELIVERED|RETURNED|UNLOCKED|INVALID`), `custody_id` (`id`, lock or escrow ID), `custody_revision` (`u64`), `snapshot_hash` (`hash | null`, non-null iff CHARACTER), `last_operation_id` (`id | null`), `reason_code` (`null|POLICY_CHANGED|STATE_CHANGED|RESTORE_RECONCILING|AUTHORITY_UNAVAILABLE`). `last_operation_id` is non-null for DELIVERED/RETURNED/UNLOCKED and references the exact terminal command/transfer; INVALID requires a non-null reason; other states have null reason. Initial LOCKED/ESCROWED may reference their command or use null. ITEM cannot be LOCKED/UNLOCKED; CHARACTER cannot be ESCROWED (its lock remains LOCKED across the escrow-account transfer). Status is an owner-only projection/reconciliation signal; a DELIVERED row is **not** a substitute for the command receipt required to pay out.

### Completeness watermark

Request exact order: `Head` (`PublishBazaarWatermarkV1`), `projection_epoch` (`u64`), `complete_through` (`time`), `observed_at` (`time`). Reuse the account/public projection completeness-time convention: T proves every applicable catalog/status change committed at/before T is acknowledged. It does not prove ownership, payment or command completion. Compute conservatively using oldest undelivered outbox change and maximum transaction duration; keep below resync start until full resync finishes. Publisher sends at least every 10 s. Proposed testing/preproduction freshness: S=30 s, clock_uncertainty=1 s. Reject `complete_through > observed_at` or either time more than uncertainty after Platform now. Stale if `now - complete_through + uncertainty > S`, future watermark beyond uncertainty, unsafe clock, or watermark epoch differs from highest seen. On stale/invalid/unavailable evidence, deny new listing/bids/sale handoff and hide unproven listings from public search; retain locks/holds and allow exact reconciliation and safe compensation. Production timing remains an acceptance blocker, not an infinite stale lease.

## 8. Identifiers, ordering, replay and recovery

Platform allocates one stable `listing_id`, distinct `offer_id`/`sale_id`, and one immutable `operation_id` per semantic step. Retrying the same step does not allocate another ID. Preserve the existing Platform transfer-step convention `bazaar:{auction}:{purpose}:{attempt}` for existing transfer callers; for proposed Rust G-7 steps, use `bazaar:{listing_id}:{command}:{step_id}:{attempt}` as a local unique issuance key, with `step_id` an `id` and attempt a positive decimal 1..8. The key maps transactionally to exactly one operation ID and full intent digest. It is not a wire credential. Different offers/releases have different step IDs; a timeout never increments attempt. Existing transfer operation IDs stay in their existing issuer namespace.

Game uniqueness is (`issuer_authority`, `operation_id`) across commands. The first accepted binding is immutable. Exact duplicate returns the same stored result bytes, including revisions/signature/timestamps; conflicting reuse writes no new effect or receipt, retains the original and alarms. Receipt publication keys use the same pair, plus current recovery-generation evidence. All writes use transactional outboxes, at-least-once delivery, durable deduplication and receipts; there is no exactly-once network promise.

Platform assigns `source_revision` under the issuer lock and holds it to transaction commit, refusing fresh keys at/below its stored maximum. Intent order is a discovery order, not proof that another effect has committed. Game serializes by Character lock, escrow, hold and account/world bank authority. Cross-step prerequisites require matching authoritative receipts, never adjacent sequence numbers or timestamps. Pending feeds may have gaps because terminal rows are omitted; do not demand contiguous pending revisions.

Policy ordering scope is (`source_authority`, world_id); status scope is (`source_authority`, listing_id). Compare (`projection_epoch`, `projection_revision`) numerically as uint64. Greater pair replaces full content; equal pair with equal canonical content accepts without effects and preserves the first source observation; equal pair with different content is 409/quarantined; lower pair is superseded. Content excludes Head/source observation/ordering fields and includes every remaining member. Gaps in per-subject revisions are safe for complete snapshots; missing prerequisite custody/receipt evidence still blocks sale. Poison data never advances the accepted subject pair or completeness claim.

Within one epoch, watermark `complete_through` and `observed_at` never decrease: older pairs are superseded, equal pairs are idempotent, and a later observation with unchanged completeness cannot extend the freshness cutoff. A watermark is accepted only after all catalog/status rows covered by its time cut are acknowledged.

A higher epoch in **any** status, policy or watermark immediately invalidates all older-epoch Bazaar projections. Rebuild off-path from Game full resync, reconcile receipt/custody/hold digests, current privacy floor and watermark, then switch only after completeness is proven. Operator-triggered replay/resync follows the existing projection pattern; Platform does not invent a resync endpoint. Empty feeds are not proof of an empty world.

Game restore uses the existing `/internal/v1/game-auth/game-restore/notice` exchange and its original wire/identity. Extend its affected-saga handling to Bazaar under the same restore lock: receipts below the new `recovery_generation` are historical, affected operations reopen REOPENED, and listing/funding/settlement pause. Game proves its recovery fence before replay, publishes reconciled stored or terminal replacement outcomes as that contract permits, and resyncs lock/escrow/hold/status state. Old-generation receipts cannot authorize payout/unlock. A restored buyer bank or item store cannot be repaired by blindly replaying an irreversible payout/delivery; drift requires operator reconciliation with both ledgers. Any missing external epoch/generation fence denies authority. Rollback cannot lower these floors.

## 9. Saga failure, timeout, privacy and rollout

### Required saga ordering

1. Platform authenticates and gates the owner, resolves authoritative identity, stores one command and local issuance key atomically. Publish no listing before committed lock+verified snapshot or committed item escrow. For Characters, also require the existing listing-escrow transfer COMMITTED before activation.
2. Before accepting a bid/offer/match, reserve full maximum in the coin Wallet or observe COMMITTED gold hold. An unfunded/ambiguous offer cannot win. Auction/proxy-bid rules remain Platform-owned.
3. At close/match, freeze a unique sale, expected parties/currency, exact funded amount, fees, asset receipt/hash and mutually exclusive disposition under Platform locking. Check destination and seller payout headroom before handoff. Funds remain reserved.
4. Deliver item or bind and execute the existing Character settlement transfer. Observe exact current-generation COMMITTED receipt. Timeout/503/lost response stays AMBIGUOUS/reconciling with funds and custody protected; query/replay the **same operation**. Never pay, return or release based on silence, expiry, a projection row or `not found` without terminal no-late-commit proof.
5. Only after delivery/ownership COMMITTED, settle the coin Wallet with its own idempotency keys or issue PayoutBazaarGold. Payout ambiguity keeps sale pending and reconciles payout only; it never re-runs asset transfer. After payout release excess maximum, then explicitly unlock delivered Character. Notify success/statistics only once the whole sale is durably complete.
6. Unsold/cancelled/refused flows require no executable/ambiguous delivery/payout. Return item or execute existing Character escrow-return transfer, reconcile its receipt, then unlock and release remaining holds once safe. Moderation pause retains custody; four-eyes invalidation cannot claw back a sold asset or credit replacement gold. Any post-sale reversal requires a separate accepted compensation contract.

No distributed ACID is assumed. Pending/AMBIGUOUS, REJECTED, COMMITTED, stale/unavailable evidence and manual reconciliation are distinct user states. Dependency uncertainty denies new authority; it is never a zero balance, absent asset, eligible Character or completed sale. Alarm when any pending effect remains unresolved for 300 s; escalate operationally after 900 s without releasing funds. Bounded retries/backoff do not turn escalation into terminal no-effect evidence.

### Privacy and retention (proposed acceptance values)

Internal envelopes, raw signed snapshots, AccountId linkage, instance/lock/hold IDs, maximum offers, gold amounts, bank entries and command receipts are owner-only: authenticated seller sees their listing; buyer sees only their own offer/hold and acquired asset receipt. Buyer cannot browse seller account data; seller cannot see another buyer's maximum or bank balance. Staff access requires narrow audited RBAC/recent MFA; support cannot bypass Game effects. Never return internal ingests verbatim from a public route or use a shared public cache for them.

Public Character sale facts require a separate Platform listing projection and explicit seller publication authorization, bounded by current field/privacy policy. It may show approved name/world/level/skills/items, capture date and snapshot hash, not seller AccountId, signature envelope or account grouping. Public item attributes are the approved visible subset only. If publication authorization/privacy freshness is absent, keep owner-only and block T59/T61 public output. Revocation removes search/filter/notification payloads and cached representations; an old signed snapshot cannot override a newer privacy deny. T63 retains anonymous aggregated prices only after privacy filtering; private bidder histories never become public statistics.

Never send credentials, private/public admission tokens, private signing keys, recovery keys, email/IP/device identifiers, chat/mail text, secret quests, private topology/placement, raw inventory database blobs, bank balance/history, other account Characters, free-form item inscriptions, provider data or session/lease/fencing secrets. Internal correlation IDs are pseudonymous operational identifiers, never metric labels or public ownership proof.

Proposed retention: active/pending/reconciling operation bindings, receipts, outboxes, locks/escrows/holds and ledger references remain until terminal and reconciled; never age out unresolved value. Serving/projection copies of the full private snapshot/exported attributes expire 30 days after terminal close/return; owner access ends then (earlier on lawful deletion where no hold applies), and public listing removes optional detail then. The immutable receipt embedding those facts is a separate restricted replay/audit record: it is not a serving copy and stays under the approved financial evidence retention policy. Redacted operational audit expires after 90 days, subject to a documented legal hold. Amount ledger, request/receipt digests, operation identity tombstones and restore/high-water fences are retained durably for deduplication; retain only the minimal binding/digest necessary, restrict access and apply the approved financial retention policy before production. Detailed payload deletion never permits an old operation to execute again. If approved law/policy requires deleting embedded receipt facts as well, first accept a new wire version separating snapshot payload retention from immutable receipt references; v1 cannot both erase embedded facts and promise byte-identical full receipts. Until that choice is accepted, retention/production enablement stays blocked (Q6). Raw rejected poison payloads are encrypted/restricted, retained at most 7 days unless an incident hold applies; diagnostic reason/digest only remains. Backups must expire/redact consistently. These durations require both owners' privacy/financial approval (Q6); no runtime is authorized until that policy reconciles payload deletion with exact-retry evidence.

### Flags and compatibility

Proposed flags `bazaar.ingest_enabled`, `bazaar.character_listing_enabled`, `bazaar.item_escrow_enabled`, `bazaar.gold_holds_enabled`, `bazaar.gold_payout_enabled`, `bazaar.public_listings_enabled` default **false** independently on both sides where applicable. Existing Character transfer command/saga flags remain off unless separately enabled. Missing identity, signer trust, escrow account allocation, policy, destination capability, recovery fence or accepted producer/consumer version returns unavailable; a broad Bazaar switch cannot override a missing dependency. Initial acceptance/qualification covers testing/preproduction only; production remains off pending its own evidence.

Stopping new listings/bids must not orphan existing value. Keep an explicitly authorized reconciliation/release/return lane under the same authentication, flags and fences until all outstanding operations resolve; do not silently enable it when a disabled surface returns 503. Emergency shutdown may pause even recovery, but retains records/funds and escalates. Rollback routes no uncertain operation through another authority and does not unlock/release by migration or direct SQL.

Wire `contract_version=1` is distinct from proposal revision and snapshot schema version. Exact member sets, paths, enum additions, scalar bounds, canonicalization, meaning, signature input or result changes require a new wire version and owner acceptance; no tolerant unknown fields or best-effort downgrade. Unsupported optional snapshot sections use the explicitly specified null form, not invented members. Existing Character command and restore versions remain unchanged; adapters must validate their native receipts exactly. Pin golden fixtures, producer and consumer SHAs at acceptance. Accepting this proposal alone is not G-7 completion: require enabled-capability evidence and exact-revision cross-repository conformance before marking the gate satisfied.

## 10. Golden payloads

These blocks are complete JSON wire objects, not templates. Each line inside a block is canonical; the Markdown newline is not a wire byte. IDs and times are synthetic. Valid structural examples assume matching authenticated peers, configured authorities, current policy/recovery state and corresponding stored intents. Golden signing vector is specified immediately below; no private key belongs in this document.

### Valid: full gold maximum hold intent

```json
{"contract_version":1,"command":"HoldBazaarGold","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","listing_id":"0192a000-0000-7000-8000-000000000010","seller_account_id":"0192a000-0000-7000-8000-000000000011","world_id":"0192a000-0000-7000-8000-000000000012","buyer_account_id":"0192a000-0000-7000-8000-000000000013","buyer_character_id":"0192a000-0000-7000-8000-000000000014","offer_id":"0192a000-0000-7000-8000-000000000015","hold_purpose":"OFFER","amount":"10000","expected_hold_revision":"0","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}
```

### Valid: hold receipt, SHA-256 bound to the preceding intent

```json
{"contract_version":1,"operation":"PublishBazaarCommandReceiptV1","source_authority":"oteryn:game:primary","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","command":"HoldBazaarGold","operation_id":"0192a000-0000-7000-8000-000000000001","binding_digest":"d7410d244496b996ec293ca23bd3b0b4fc7d32dc23d1a047c9cc49604d1bb90c","listing_id":"0192a000-0000-7000-8000-000000000010","outcome":"COMMITTED","result_code":"BAZAAR_OK","facts":{"hold_id":"0192a000-0000-7000-8000-000000000016","hold_revision":"1","buyer_account_id":"0192a000-0000-7000-8000-000000000013","offer_id":"0192a000-0000-7000-8000-000000000015","hold_purpose":"OFFER","held_amount":"10000"},"recovery_generation":"3","decided_at":"1790000002"}
```

### Valid: item escrow intent and item facts with attributes

```json
{"contract_version":1,"command":"EscrowBazaarItem","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a2","source_revision":"18","operation_id":"0192a000-0000-7000-8000-000000000002","listing_id":"0192a000-0000-7000-8000-000000000020","seller_account_id":"0192a000-0000-7000-8000-000000000011","world_id":"0192a000-0000-7000-8000-000000000012","seller_character_id":"0192a000-0000-7000-8000-000000000021","item_instance_id":"0192a000-0000-7000-8000-000000000022","expected_state_revision":"7","quantity":1,"transferability_revision":"4","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}
```

This separate `Item` record is structurally valid; `attributes_hash` here is SHA-256 of the synthetic complete attribute state `{"tier":"2"}`. It is not a whole receipt.

```json
{"item_instance_id":"0192a000-0000-7000-8000-000000000022","world_id":"0192a000-0000-7000-8000-000000000012","definition":{"family":"item","production_key":"fixture-sword","revision_ref":"v1"},"state_revision":"8","quantity":1,"attributes_hash":"fd8184df56101f486d8766c87c216edddf9c25c4f29ba4c48eb48219dc137c19","attributes":[{"key":"tier","value":"2"}]}
```

### Valid: signed locked Character snapshot

The following standalone `SignedSnapshot` uses a disposable example Ed25519 public key `e8c30390441a54642382f15a784087a42dc75f85f775fdfc9e2a25b0f08a9806`, assigned the synthetic kid `fixture-example-1`. Only this public verification key and signature are retained; the ephemeral example private key was discarded. It must never be trusted in a deployed stack. The signature verifies the §6 domain-prefixed canonical bytes with that public key. Its unsupported tabs are null, not fabricated game facts.

```json
{"snapshot":{"contract_version":1,"issuer_authority":"OTERYN_GAME_BAZAAR_AUTHORITY","audience":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","source_authority":"oteryn:game:primary","listing_id":"0192a000-0000-7000-8000-000000000030","seller_account_id":"0192a000-0000-7000-8000-000000000011","character_id":"0192a000-0000-7000-8000-000000000031","world_id":"0192a000-0000-7000-8000-000000000012","lock_id":"0192a000-0000-7000-8000-000000000032","lock_revision":"1","character_revision":"7","eligibility_revision":"4","projection_epoch":"3","captured_at":"1790000002","name":"Aldric","vocation":4,"level":"100","experience":"1000000","created_at":null,"health":"500","health_max":"500","mana":"200","mana_max":"200","capacity_centi_oz":"10000","speed":"220","blessings":[],"skills":[{"key":"distance","level":"75","progress_bps":5000}],"world_transfer_eligible":null,"appearance":null,"items":null,"store_grants":null,"outfits":null,"mounts":null,"achievements":null,"quests":null},"snapshot_hash":"26d9a91adcd5a5b430194f1c876ca7d6e5d7e308a3e6cf0eed5f9926123c38c2","alg":"Ed25519","kid":"fixture-example-1","signature":"hNOaX-8--zY8dW-lC1geGshkvtFOvNgKd3XbSCxEOnmIQprN_v66f9j4UdEQ5ORvVJ5PBZs_v-21nt_SpgjsBA"}
```

### Valid: unchanged existing ownership-transfer intent

Served on the existing character-command read route with its dedicated identity, **not** the Bazaar command read route. The Game lock binding must name the hash of these exact bytes. This fixture is a settlement leg after a committed listing-escrow leg; `expected_from_account_id` is the allocated synthetic escrow account. Expiry is a technical attempt deadline, not permission to release the Character.

```json
{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000b1","source_revision":"19","operation_id":"0192a000-0000-7000-8000-000000000003","character_id":"0192a000-0000-7000-8000-000000000031","expected_from_account_id":"0192a000-0000-7000-8000-000000000040","to_account_id":"0192a000-0000-7000-8000-000000000013","purpose":"BAZAAR_SETTLEMENT","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}
```

### Valid: item delivery refusal followed by a distinct return attempt

These are structurally valid complete messages. The all-ones escrow receipt digest is a synthetic prerequisite: this scenario assumes Game and Platform hold the matching escrow receipt under that digest. A real digest mismatch denies the attempt. Delivery refusal commits no movement; escrow remains at revision 1, allowing this separately authorized return. The return intent alone does not prove a return completed.

```json
{"contract_version":1,"command":"DeliverBazaarItem","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a3","source_revision":"20","operation_id":"0192a000-0000-7000-8000-000000000004","listing_id":"0192a000-0000-7000-8000-000000000020","seller_account_id":"0192a000-0000-7000-8000-000000000011","world_id":"0192a000-0000-7000-8000-000000000012","escrow_id":"0192a000-0000-7000-8000-000000000023","expected_escrow_revision":"1","buyer_account_id":"0192a000-0000-7000-8000-000000000013","buyer_character_id":"0192a000-0000-7000-8000-000000000014","escrow_receipt_digest":"1111111111111111111111111111111111111111111111111111111111111111","sale_id":"0192a000-0000-7000-8000-000000000024","funding":{"currency":"COINS","reservation_id":"0192a000-0000-7000-8000-000000000025","hold_revision":null,"gross_amount":"10000"},"issued_at_source":"1790000010","expires_at_source":"1790000310","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}
```

```json
{"contract_version":1,"operation":"PublishBazaarCommandReceiptV1","source_authority":"oteryn:game:primary","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","command":"DeliverBazaarItem","operation_id":"0192a000-0000-7000-8000-000000000004","binding_digest":"d5076e9f28cb4e630e5dd69a161aa7c1b2c01afc33517e31e3f15b5d9a9a6f59","listing_id":"0192a000-0000-7000-8000-000000000020","outcome":"REJECTED","result_code":"BAZAAR_DESTINATION_UNAVAILABLE","facts":null,"recovery_generation":"3","decided_at":"1790000011"}
```

```json
{"contract_version":1,"command":"ReturnBazaarItem","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a4","source_revision":"21","operation_id":"0192a000-0000-7000-8000-000000000005","listing_id":"0192a000-0000-7000-8000-000000000020","seller_account_id":"0192a000-0000-7000-8000-000000000011","world_id":"0192a000-0000-7000-8000-000000000012","escrow_id":"0192a000-0000-7000-8000-000000000023","expected_escrow_revision":"1","seller_character_id":"0192a000-0000-7000-8000-000000000021","escrow_receipt_digest":"1111111111111111111111111111111111111111111111111111111111111111","issued_at_source":"1790000020","expires_at_source":"1790000320","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}
```

### Valid: empty full transferability snapshot and completeness watermark

The empty definition list denies all new item escrow in that world; it is not an ingest outage.

```json
{"contract_version":1,"operation":"PublishBazaarTransferabilityV1","source_authority":"oteryn:game:primary","world_id":"0192a000-0000-7000-8000-000000000012","projection_epoch":"3","projection_revision":"4","source_observed_at":"1790000002","eligibility_revision":"4","definitions":[]}
```

```json
{"contract_version":1,"operation":"PublishBazaarWatermarkV1","source_authority":"oteryn:game:primary","projection_epoch":"3","complete_through":"1790000000","observed_at":"1790000003"}
```

### Invalid: future completeness (400; do not advance watermark)

```json
{"contract_version":1,"operation":"PublishBazaarWatermarkV1","source_authority":"oteryn:game:primary","projection_epoch":"3","complete_through":"1790000004","observed_at":"1790000003"}
```

### Invalid: unknown operation member and noncanonical revision (400)

```json
{"contract_version":1,"operation":"PublishBazaarTransferabilityV1","source_authority":"oteryn:game:primary","world_id":"0192a000-0000-7000-8000-000000000012","projection_epoch":"3","projection_revision":"04","source_observed_at":"1790000002","eligibility_revision":"4","definitions":[],"allow_unknown_items":true}
```

### Invalid: tampered Character level with original hash and signature

Reject before publishing or settling: `snapshot_hash` no longer matches, and signature verification also fails. Rehashing without a new authoritative Game signature cannot repair it.

```json
{"snapshot":{"contract_version":1,"issuer_authority":"OTERYN_GAME_BAZAAR_AUTHORITY","audience":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","source_authority":"oteryn:game:primary","listing_id":"0192a000-0000-7000-8000-000000000030","seller_account_id":"0192a000-0000-7000-8000-000000000011","character_id":"0192a000-0000-7000-8000-000000000031","world_id":"0192a000-0000-7000-8000-000000000012","lock_id":"0192a000-0000-7000-8000-000000000032","lock_revision":"1","character_revision":"7","eligibility_revision":"4","projection_epoch":"3","captured_at":"1790000002","name":"Aldric","vocation":4,"level":"101","experience":"1000000","created_at":null,"health":"500","health_max":"500","mana":"200","mana_max":"200","capacity_centi_oz":"10000","speed":"220","blessings":[],"skills":[{"key":"distance","level":"75","progress_bps":5000}],"world_transfer_eligible":null,"appearance":null,"items":null,"store_grants":null,"outfits":null,"mounts":null,"achievements":null,"quests":null},"snapshot_hash":"26d9a91adcd5a5b430194f1c876ca7d6e5d7e308a3e6cf0eed5f9926123c38c2","alg":"Ed25519","kid":"fixture-example-1","signature":"hNOaX-8--zY8dW-lC1geGshkvtFOvNgKd3XbSCxEOnmIQprN_v66f9j4UdEQ5ORvVJ5PBZs_v-21nt_SpgjsBA"}
```

### Invalid: conflicting semantic retry (no second hold)

This complete intent reuses the first hold's operation ID but changes amount. With the first binding stored, it is a conflict: retain the first receipt, no additional hold, alarm; never overwrite with a new rejection receipt.

```json
{"contract_version":1,"command":"HoldBazaarGold","variant":"PLATFORM_BAZAAR","issuer_authority":"OTERYN_PLATFORM_BAZAAR_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","listing_id":"0192a000-0000-7000-8000-000000000010","seller_account_id":"0192a000-0000-7000-8000-000000000011","world_id":"0192a000-0000-7000-8000-000000000012","buyer_account_id":"0192a000-0000-7000-8000-000000000013","buyer_character_id":"0192a000-0000-7000-8000-000000000014","offer_id":"0192a000-0000-7000-8000-000000000015","hold_purpose":"OFFER","amount":"10001","expected_hold_revision":"0","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_BAZAAR_AUTHORITY"}
```

Additional required negative golden transformations of the signed vector: change `level` without rehashing → hash mismatch; recompute hash without re-signing → signature failure; change seller/listing/lock/world → binding failure even with another valid signature; unknown/revoked kid → unavailable/denied; replay below epoch floor → superseded and cannot publish. Changing hold amount to JSON number `10000`, string `"010000"`, null or `"1000000000000"` is invalid encoding/type/range, never coerced. Negative payout vector: gross `"10000"`, fee `"249"`, seller `"9751"` is `BAZAAR_PAYOUT_INVALID` because exact 250-bps fee is `"250"`.

## 11. Acceptance checklist (all unchecked)

### Game owner / coordinator

- [ ] Accept/amend this exact proposal and lock the Game producer contract/revision; confirm new operation names and bounded wire capabilities, with runtime still off.
- [ ] Resolve Q1–Q7; accept eligibility/cooldown, escrow account and lock-to-existing-transfer interlock, supported snapshot tabs and surviving assets.
- [ ] Prove offline/admission/write lock races and exact-snapshot capture atomicity; existing transfer requires the bound operation and conserves the lock across all three purposes.
- [ ] Prove item split/nonstackable/attribute conservation, delivery-versus-return exclusion, unavailable destination and policy withdrawal behavior, including suspended/missing seller recovery.
- [ ] Prove full bank authority honors OFFER/RESERVE holds across Characters/worlds, concurrent withdrawal/fees/other bids, replacement/partial release, headroom and payout/burn conservation.
- [ ] Prove atomic effect+receipt+outbox, exact/conflicting retry, no receipt rewrite, timeout-before/during/after commit, expired unexecuted attempt and durable terminal refusal.
- [ ] Prove correct mTLS purpose isolation, trusted terminator guard, bounded rate/body/depth/schema checks, Ed25519 vector, key rotation/revocation and absence of payload/key logging.
- [ ] Prove catalog/status outbox ordering, freshness watermark, restore/epoch/generation fences, resync and drift escalation without blind delivery/payout replay.
- [ ] Supply exact Game code/test SHAs, retention/privacy/runbook decisions, and cross-repository qualification evidence before marking G-7 delivered.

### Platform owner / Rust portal consumer

- [ ] Accept Game's final wire through the coordinator; keep T58–T64 unavailable until dependencies and exact compatible versions are proven.
- [ ] Validate canonical encodings/member sets, hashes/signature/identity/audience, matching receipts, all units/limits/nulls, and contradictory/gap/unknown evidence denial.
- [ ] Prove owner/foreign-object denial, verified-email/2FA/sudo listing gates, no self-bid, account restrictions, admin narrowing and immutable commercial ledger.
- [ ] Prove publish-after-lock/escrow, full-maximum-funding-before-accepted-bid, winner funding before outbid release, delivery-before-payout, existing escrow transfer sequence and payout-only reconciliation after delivery.
- [ ] Prove same local issuance key returns the same operation, terminal refusal versus ambiguity, mutually exclusive return/settlement, partial/excess release and no timeout-driven release.
- [ ] Prove T59/T60 approved listing views, T61 privacy-filtered search, T62 terminal-only notifications, T63 committed/anonymous stats and T64 four-eyes moderation with no uncontracted reversal.
- [ ] Prove owner-only raw data, current privacy-floor enforcement/cache invalidation and retention/deletion while preserving replay fences; unsupported sections render unavailable, not zero.
- [ ] Pin shared valid/invalid golden fixtures on both exact heads; exercise duplicate/conflict/out-of-order/poison/restore and interrupted-saga integration in testing/preproduction.
- [ ] Obtain separate enablement/production authority, keep flags off by default, and document recovery/rollback with all outstanding locks, escrows and holds accounted for.

## 12. Open questions for the game owner

1. **Eligibility and lock/transfer interlock:** Which house, guild, market, mail, depot, penalty, cooldown and account rules are authoritative listing blockers? Will Game accept `BindBazaarCharacterTransfer` plus atomic enforcement on the unchanged existing transfer command and the current escrow-account sequence? Who allocates the native escrow AccountId and guarantees its capacity?
2. **Snapshot and transfer contents:** Which exact optional sections/keys are supported today, and which items/store grants/quests/achievements actually survive sale? Are the proposed bounds/name/definition/attribute encodings sufficient? Confirm that Account+World bank gold and account-wide entitlements never follow a Character. Is the proposed Game-signed snapshot profile acceptable?
3. **Item custody:** Which durable inbox/custody destination can receive a Bazaar move with every attribute preserved, including full-inventory/offline cases? What is the safe owner-authorized return destination if the original seller Character no longer belongs to that seller? Which timed/bound/container/attribute cases must remain unsupported?
4. **Gold authority and economics:** Accept Account+World bank holds, OFFER/RESERVE meaning, replacement-total increases, explicit partial releases, 999999999999 ceiling and exact fee rounding? How will seller headroom be reserved before handoff, and how will Game validate cross-channel delivery receipts before payout? Resolve small gross amounts with zero net and existing junior/recipient restrictions before activation.
5. **Trust and recovery operations:** Who provisions/rotates/revokes the separate Game snapshot keys, publishes trusted public-key evidence to Platform, and owns emergency suspension? Which existing restore scope/fence covers bank, item custody, lock and character-command receipts together? Confirm no proposed signature or transfer binding weakens existing admission/Character authority.
6. **Privacy and retention:** Accept owner-only raw data, explicit seller publication projection, 30-day serving snapshot/90-day redacted audit/7-day poison retention, with separately restricted immutable receipt evidence and durable minimal deduplication evidence? What legal financial-retention policy and approved public fields constrain T59–T64 and deletion of exact-retry payloads?
7. **Wire/rollout acceptance:** Accept the proposed route/command names, field sets, bounds, refusal taxonomy, per-world full policy list ceiling, polling/freshness/timeout limits and flags? What measured production limits, multi-host authority allocation and exact producer/consumer SHAs will be required? Until answered and accepted, G-7 remains pending and all affected capabilities remain off.
