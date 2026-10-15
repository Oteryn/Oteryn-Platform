# Oteryn-v2 Store purchase delivery contract

## Status

`PROPOSED — PLATFORM/GAME STORE PURCHASE DELIVERY CONTRACT — NOT ACCEPTED — BLOCKED BY OTERYN-GAME GAP REGISTER §32 (STORE-ENT-1)`

- Task: **STORE-DELIVERY-CONTRACT-1** (Sol supervising architect).
- Subordinate to `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` (the generic contract). This document applies the generic contract to in-game Store purchases and fills the gaps listed in §1. It relaxes no generic rule.
- Game-side inputs (read, not redefined):
  - `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md` §1-§3 (catalog is Game content; coin balance and purchase ledger are Platform's; delivery ownership open);
  - `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md` §4.3, §4.5 (D47, D49: account scope and portability; account inbox; claim is a DUR-03 MINT);
  - `docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md` §4.1 (Game status derived only from Platform entitlement through the `PROD-ENTITLEMENTS-01` consumer fence; a Game record is evidence, never a second authority);
  - `docs/architecture/ADR-0004-postgresql-and-data-ownership.md` §3-§5 (one owner per data set; cross-system effects through versioned API or outbox with idempotent consumers; no cross-database writes);
  - `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §2, §4.3, §5 (Platform-authored intent keyed by `operation_id`, Game-initiated consumption, durable Game receipt, `recovery_generation`, restore reconciliation);
  - `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` row STORE-ENT-1 and owner question Q4.
- **Blocking rule.** STORE-ENT-1 is blocked on the §32 delivery decision. This contract does **not** take that decision. Every point the owner must decide is listed in §12 and marked `OPEN` where it appears. Until the owner decides them and this contract is accepted on an exact head, nothing here authorizes implementation.

This contract authorizes no payment provider, real charge, Wallet credit or debit, Store activation, inbox, item mint, cosmetic unlock, Platform or Oteryn-Game runtime/schema change, deployment or production mutation.

## 1. Why this contract exists (gap against the generic contract)

The generic contract already fixes, and this contract keeps:

| Point | Generic contract section |
|---|---|
| three separate truths (payment, entitlement, game delivery) | Core authority split |
| Platform owns product, entitlement identity, lifecycle, delivery saga | Platform authority |
| Oteryn-v2 owns gameplay mutation, enforcement, durable grant result | Oteryn-v2 authority |
| one stable `delivery_operation_id`, no blind second grant | Delivery operation envelope; Profile C |
| per-layer idempotency, at-least-once assumed | Idempotency and replay |
| `AMBIGUOUS` / `RECONCILING`, never mint X2 | Ambiguous outcomes and reconciliation |
| four reversal classes; no silent deletion; no negative Wallet balance | Refund, chargeback, expiry and revocation after delivery |
| Profile-B fence cannot be lowered by restore | Profile B revision fencing; Rollout and rollback |

It leaves open, and Oteryn-Game §32 lists as open, the following. This contract proposes semantics for each:

1. how a Store offer maps to a product, a delivery profile and a fulfilment unit (§3);
2. the shape and key of the Game-side delivery receipt (§5);
3. the outbox and transport by which a Store delivery reaches Game and its receipt returns (§6);
4. what a refund or chargeback does to in-game value that was already delivered, claimed, consumed or traded, without negative balances and with an audit trail (§8);
5. how a Game or Platform restore interacts with delivered, reversed and reconciling Store deliveries (§9).

## 2. Authority (unchanged, restated for the Store)

- **Platform is the single source of the purchase.** Platform alone records that a purchase happened: the order line, the Wallet debit of Tibia Coins (or a paid order), the price paid, the product/version bought, the entitlement it issued and every reversal. Game never records, infers or reconstructs a purchase. No Game record, receipt, inbox line, unlock fact or log proves that a purchase exists.
- **Game owns the in-game effect.** Game alone decides and writes the gameplay effect: the account inbox entry when Game holds it (§4, `OPEN` S-1), the DUR-03 MINT of a claimed item, the usability of a Store cosmetic unlock on a world, and any in-game reclamation (§8).
- **The catalog is Game content** (catalog decision §1). Platform binds each purchase to an exact Game catalog revision and offer key; it never edits the catalog. Who is price authority at purchase time is `OPEN` S-5.
- No distributed ACID transaction, no shared table, no cross-database write (ADR-0004 §4, §5).

## 3. Offer, product and fulfilment unit

A purchase of one Store offer creates one Platform order line and one Platform entitlement per fulfilment unit. Every fulfilment unit has exactly one generic delivery profile. Proposed mapping (to be confirmed per product/version before activation):

| Store offer kind | Proposed profile | Game effect | Reversal class proposed (§8) |
|---|---|---|---|
| Item or consumable (D47, D49 inbox line) | C — durable gameplay grant | inbox line; claim mints through DUR-03 | before claim: reversible; after claim: `OPEN` S-3 |
| Cosmetic unlock (outfit, addon, mount) | B — game-consumed account entitlement | usable on compatible worlds while the entitlement is usable (D49) | deny-future-use |
| Premium time | B, existing `oteryn.premium_time` contract | Premium status (Premium activation §4.1) | deny-future-use (as that contract) |
| Tibia Coin package | E — Platform Wallet | none in Game | Wallet policy, never a negative balance (§8.4) |
| Character service (rename, transfer, …) | D — Character Authority command | per its own command contract | per that contract |

Rules:

- A bundle offer enumerates its child fulfilment units; each child has its own entitlement, profile and `delivery_operation_id`. A bundle is never one opaque grant.
- `quantity` is explicit per unit. A stack of N consumables is one fulfilment unit with quantity N, not N operations, unless the product version declares otherwise.
- Every unit binds `product_id`, `product_version`, `catalog_revision`, `offer_key`, and for items the item key and content revision it was sold under (D49 provenance).

## 4. Inbox line ownership — both options, one set of invariants

Who holds the account inbox line is the §32 decision (`OPEN` S-1, plan Q4). This contract defines invariants that hold under either option:

- **Option (a) Platform entitlement line that Game reads.** The inbox line *is* the Platform Profile-C entitlement in state `ACTIVE`/`UNCLAIMED`. Game lists it through the delivery feed (§6) and, at claim time, mints and returns a receipt; Platform then marks the line `CLAIMED`.
- **Option (b) Game inbox record.** Platform delivers the fulfilment unit; Game creates an account inbox record in one transaction with its receipt; the claim later mints from that record under its own claim operation.

Invariants for both:

1. One fulfilment unit gives at most one inbox line and at most `quantity` minted units, ever.
2. The line binds the exact `delivery_operation_id`, `EntitlementId`, account, profile family (D49), item key and content revision.
3. The claim is a DUR-03 MINT into one `CharacterInventory`, idempotent per line and unit, fenced like the reward claim; the line is the source cause.
4. An incompatible world fails the claim closed, mints nothing, keeps the line claimable (D49).
5. A Game line is evidence about Platform authority, never a second commercial authority (Premium activation §4.1 rule, `PROD-ENTITLEMENTS-01` §2.1). A Game line whose Platform entitlement is revoked is not claimable (§8).

## 5. Game-side delivery receipt

Every Store delivery operation and every in-game reclamation operation (§8) that reaches Game produces exactly one durable Game receipt, following the Character Authority command pattern (commands contract §4.3, §5.2).

```text
StoreDeliveryReceipt
  issuer_authority             configured Platform issuer constant
  operation_id                 = Platform delivery_operation_id (or reversal_operation_id), UUIDv7
  operation_kind               DELIVER | CLAIM | REVOKE_UNCLAIMED | RECLAIM | FLAG
  entitlement_id
  product_id + product_version
  account_id                   Platform AccountId, copied, never minted by Game
  character_id                 CLAIM / RECLAIM only
  world_id                     CLAIM / RECLAIM only
  binding_digest               canonical digest of every intent member
  outcome                      COMMITTED | REJECTED
  result_code                  closed set, fixed by the wire contract
  effect_summary               e.g. minted units, inbox line id, reclaimed units, shortfall units
  recovery_generation          Game recovery fence asserted by the deciding transaction
  decided_at                   Game transaction time
```

Rules:

- **Key.** The receipt is keyed by `(issuer_authority, operation_id)`. One key, one receipt, stored in the same PostgreSQL transaction as the effect (or as the rejection). A deterministic rejection is stored too, so a rejected operation can never commit later.
- **Exact retry.** Reading an intent whose key already has a receipt with an equal `binding_digest` republishes the stored receipt bytes and writes nothing.
- **Conflicting reuse.** The same key with a different binding writes nothing, publishes nothing new, keeps the stored receipt and alarms (generic contract: one operation ID cannot be reused for another entitlement, target, product version or intent).
- **Byte identity.** Every publication of one receipt is byte-identical.
- **Distinct identities.** In option (b) the Game claim has its own `operation_id`, correlated with, never equal to, the delivery `operation_id`. A Profile-D service keeps its Character Authority `operation_id` distinct, as the generic contract requires.
- The exact member set, result codes, bounds and encoding are fixed by the wire contract that follows the §32 decision (`OPEN` S-2); the semantic members above are binding.

## 6. Outbox and transport: at least once, idempotent apply

### 6.1 Platform side

- Platform writes the order line, the Wallet debit (Profile E mutator), the entitlement and a **delivery intent outbox row** in one Platform transaction. No intent exists without its purchase; no purchase with a Game effect exists without its intent.
- The intent is immutable once written. It stays `PENDING` until Platform holds a Game receipt for it. A timeout, a lost response, an expiry of a transport attempt or a missing receipt never marks it failed and never mints a new `operation_id` (generic contract: never mint X2 as a blind replacement).
- Store delivery intents do **not** carry a short technical TTL that turns a paid delivery into a rejection. Whether an undelivered paid intent ever expires, and into what, is `OPEN` S-6.

### 6.2 Transport direction

Proposed default, to keep one integration pattern: Game-initiated pull over the existing Game→Platform mTLS client, as in commands contract §2 (list pending, read exact intent, publish receipt). Platform never calls Game and Game exposes no new inbound listener. The transport choice is `OPEN` S-2.

### 6.3 Game side

- The Game consumer may see each intent any number of times (list rescans, retries, restarts). Applying is idempotent by §5: the first commit wins; every later read republishes.
- Game writes the receipt and a **receipt outbox row** in the deciding transaction (ADR-0004 §3 transactional outbox). The outbox publishes the receipt at least once until Platform answers `accepted` or the same-bytes answer.
- A dependency that is unavailable (account-security evidence, recovery fence, content, world) is transient: no write, no receipt, the intent stays `PENDING`.

### 6.4 Platform receipt handling

- Platform stores the first receipt for a key and marks the intent terminal. An identical republication is accepted again. Different bytes for a key not reopened by a restore (§9) are refused and alarmed on both sides; neither side rewrites the other's record.
- Platform updates entitlement delivery state from the receipt only: `APPLIED` on `COMMITTED`, `REJECTED` on a terminal rejection, and keeps `AMBIGUOUS`/`RECONCILING` while it has none. Customer presentation follows the generic contract's presentation rules (`entitlement active` ≠ `item in inbox` ≠ `item claimed`).

## 7. Lifecycle states of one Store fulfilment unit

```text
Platform:  PURCHASED ─► ENTITLEMENT_ACTIVE ─► DELIVERY_PENDING ─► DELIVERED ─► (CLAIMED, items only)
                                                   │                  │              │
                                                   ▼                  ▼              ▼
                                             RECONCILING      REVERSAL_REQUESTED ─► REVERSED | REVERSAL_PARTIAL | MANUAL_REVIEW
```

- `DELIVERED` means a stored `COMMITTED` Game receipt for the delivery operation.
- `CLAIMED` exists only for item lines and means a stored `COMMITTED` claim receipt.
- Reversal states are Platform's; their in-game effect is decided only by a Game receipt of a reversal operation (§8).

## 8. Refunds and chargebacks

### 8.1 Ownership

- The provider refund/chargeback truth is Payments'. The decision to revoke an entitlement is ProductsEntitlements'. The in-game effect of that revocation is Game's (generic contract).
- A reversal is never an edit: it is a new Platform `reversal_operation_id` (UUIDv7), its own intent, its own Game receipt, its own audit records. It references the original `delivery_operation_id` and `EntitlementId` and never reuses either as its key.

### 8.2 What happens to delivered value (proposed per state)

| State at reversal | Proposed Game effect | Notes |
|---|---|---|
| intent `PENDING`, no receipt | Platform cancels the intent only if Game later returns a stored rejection; otherwise it issues `REVOKE_UNCLAIMED` after the delivery receipt arrives | Platform never assumes non-delivery from silence |
| delivered, inbox line unclaimed | `REVOKE_UNCLAIMED`: the line becomes non-claimable; nothing is minted | reversible; the line stays as history |
| cosmetic unlock | entitlement revoked through the Profile-B fence; the unlock becomes unusable at next use or login | deny-future-use; no appearance or character write beyond what the consumer contract already does |
| Premium time | as `oteryn.premium_time` (deny-future-use) | unchanged |
| item claimed, unit still present and untraded in the claiming character's possession | `RECLAIM` of exactly the minted unit identities, or `FLAG` only | which one is `OPEN` S-3 |
| item claimed and consumed, traded, sold, destroyed or moved to another account | `FLAG` only: nothing is removed from any other player; the shortfall is recorded | `OPEN` S-3, S-4 for follow-up sanctions |
| Tibia Coins already spent on Store goods (Profile E reversal) | no negative Wallet balance; Platform records a reversal shortfall and, if policy chooses, reverses the downstream Store deliveries under this table | `OPEN` S-4 |

### 8.3 Invariants

- **No negative balances.** Neither the Platform Wallet nor any in-game balance, counter, stack or quantity becomes negative because of a reversal. What cannot be reclaimed is recorded as a shortfall, never as a debt balance.
- **No collateral loss.** A reversal never removes value from an account or character other than the one the unit was delivered to and claimed by, and never removes a unit other than the exact minted unit identities recorded in the claim receipt.
- **Fail closed, not destructive.** If Game cannot prove the exact unit is present and untouched, the result is `FLAG` with the shortfall, not a best-effort deletion.
- **Idempotent.** A reversal operation reclaims at most once; a replay republishes its receipt.
- **Ordered.** A reversal of a unit whose delivery is still `AMBIGUOUS` waits for the delivery receipt; Platform never sends `RECLAIM` for a claim it has no receipt for.
- **Not automatic in-game sanctions.** Account restriction, ban or trade lock after a chargeback is a moderation/fraud decision outside this contract (`OPEN` S-4).

### 8.4 Audit trail

- Platform keeps an append-only record per reversal: provider event reference (no secrets, no full payload), reason class, actor (system, operator with RBAC/MFA), the original and reversal operation IDs, the Game receipt and the resulting shortfall.
- Game keeps an append-only critical-mutation audit row per `RECLAIM` and `FLAG` (ADR-0004 §3) with the unit identities, the receipt key and the `recovery_generation`.
- Logs carry no payment secret, no provider payload, no AccountId or character name beyond what the existing privacy rules allow (commands contract §8).
- Manual corrections by support use the same operation and receipt path; there is no direct table edit on either side.

## 9. Restore and reconcile

### 9.1 Game restore

Commands contract §5.4 and OPERABILITY ruling 10 apply, with one Store-specific difference:

- Every Store receipt carries `recovery_generation`. After a Game restore notice with restored generation H, Platform reopens as `PENDING` every Store intent (delivery, claim, reversal) whose receipt carries generation ≥ H, selected by generation only, never by clock.
- A receipt that survived in the restored store is republished byte-identical: no-op.
- **Difference from Character commands:** a paid Store delivery lost by the restore is decided again and normally **re-applied** under the new generation, because the customer still holds the purchase and the restored Game state proves the effect is absent. It is not expired into a rejection (subject to `OPEN` S-6).
- A claim lost by the restore leaves the line claimable again; a minted unit lost by the restore is not re-minted automatically outside a new claim.
- A reversal lost by the restore is decided again against the restored state; if the reclaimed unit has reappeared it is reclaimed again under the same `reversal_operation_id`, otherwise flagged.
- Platform acknowledges the restore notice only after every reopened Store intent has a new receipt; Game opens admission only after that acknowledgement (as commands contract §5.4).

### 9.2 Platform restore

- Platform is the purchase authority, so a Platform restore that loses purchases is a commercial incident, not a Game decision. Game never recreates a purchase from its receipts.
- Game receipts whose `operation_id` Platform no longer knows are answered `404`; Game keeps them, stops republishing and alarms. The effect stays in Game until Platform re-establishes the purchase (from provider truth or its own ledger recovery) or issues an explicit reversal.
- A Platform restore must not lower any Profile-B fence (generic contract) and must not re-issue an `operation_id` already used (UUIDv7, never recycled).
- Whether Platform may re-import Game receipts to rebuild delivery state after its own restore is `OPEN` S-7.

### 9.3 Periodic reconcile

- Platform and Game reconcile by operation identity, not by time: Platform lists intents without receipt older than a bound; Game lists receipts not acknowledged. Drift (purchase without delivery, delivery without purchase, claim without line, reversal without receipt) is an operator alarm with a bounded metric, never an automatic grant or deletion.

## 10. Security and privacy

Inherited from the generic contract (Security boundary) and commands contract §3, §8: a dedicated mTLS identity for the Store consumer; issuer accepted only from configuration; closed routes; bounded messages; no secrets, provider payloads or voucher plaintext in intents, receipts or logs.

## 11. Conformance scenarios (to prove before activation)

1. duplicate delivery intent read twice, receipt lost once: one inbox line, byte-identical receipts;
2. browser double purchase with one `operation_id`: one Wallet debit, one entitlement, one delivery;
3. same `operation_id` with a changed binding: no write, alarm;
4. claim twice on two worlds concurrently: one mint;
5. claim on an incompatible world: nothing minted, line still claimable;
6. refund before claim: line non-claimable, nothing minted, audit rows on both sides;
7. refund after claim, unit present: exactly that unit reclaimed or flagged (per S-3), once;
8. refund after the unit was traded: nothing removed from the receiver; shortfall flagged; no negative balance anywhere;
9. chargeback of a coin package whose coins were spent: Wallet never negative; shortfall recorded;
10. Game PITR after a delivery: reopened by generation; surviving receipt republished; lost delivery re-applied once;
11. Game PITR after a reversal: reversal re-decided; no double reclaim;
12. Platform restore that loses a purchase: Game keeps its receipt, alarms, nothing recreated;
13. dependency unavailable: no write, no receipt, intent stays `PENDING`;
14. cosmetic unlock revoked: unusable at next use, fence not lowered by restore.

## 12. Open owner decisions (not decided here)

| ID | Question | Notes |
|---|---|---|
| S-1 | Who owns the account inbox line: (a) a Platform entitlement line that Game reads, or (b) a Game inbox record? | §32 delivery decision; plan Q4. §4 invariants hold either way. |
| S-2 | Transport and wire: reuse the Game-initiated pull pattern of the Character commands contract, or another transport? Exact members, codes and bounds. | §6.2 proposes pull; not decided. |
| S-3 | After a claim, on refund or chargeback: reclaim the exact unit when still present and untraded, or only flag? | §8.2 |
| S-4 | Consequences of a shortfall (consumed, traded, spent coins): only record, account restriction, trade lock, or support review? Fraud policy. | moderation/fraud; outside this contract's authority |
| S-5 | Price authority at purchase: the Game catalog revision's `price_coins` as imported by Platform, or a Platform-side price that may differ? | catalog decision §1-§2, follow-up 1 |
| S-6 | Can an undelivered paid intent ever expire, and if so into refund, manual review or nothing? | §6.1, §9.1 |
| S-7 | After a Platform restore, may Platform rebuild delivery state from Game receipts? | §9.2 |
| S-8 | Per-product reversal class for each Store offer kind, and the confirmation of the §3 profile mapping. | generic contract requires it per product/version |
| S-9 | Commercial activation order: which Store offer kinds ship first, and whether paid issuance waits for the Payments gate. | Premium snapshot contract keeps paid issuance unavailable |

## 13. Acceptance and rollout

- Status stays **Proposed** until the owner takes the §32 delivery decision and answers §12, and an exact-head independent review accepts this contract.
- After acceptance: a wire contract (members, codes, bounds), then Game and Platform implementations, disabled by default, then a shared-fixture `testing` stack, per offer kind, never globally.
- STORE-ENT-1 stays blocked until then.

## References

- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- Oteryn-Game: `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`; `docs/architecture/ADR-0004-postgresql-and-data-ownership.md`; `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md`; `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md`; `docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md`; `docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §32; `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` (STORE-ENT-1, Q4)
- Issues: Oteryn/oteryn-platform #321 (Payments), #322 (Products/Entitlements), #924 (native game delivery)
