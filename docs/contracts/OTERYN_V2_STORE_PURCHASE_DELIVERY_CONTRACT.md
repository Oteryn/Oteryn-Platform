# Oteryn-v2 Store purchase delivery contract

## Status

`PROPOSED — PLATFORM/GAME STORE PURCHASE DELIVERY CONTRACT — NOT ACCEPTED — OTERYN-GAME GAP REGISTER §32 / Q4 RESOLVED BY OWNER DECISION S-1 (2026-10-15) — AWAITING EXACT-HEAD INDEPENDENT REVIEW`

- Task: **STORE-DELIVERY-CONTRACT-1** (Sol supervising architect).
- Subordinate to `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md` (the generic contract). This document applies the generic contract to in-game Store purchases and fills the gaps listed in §1. It relaxes no generic rule.
- Game-side inputs (read, not redefined):
  - `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md` §1-§3 (catalog is Game content; coin balance and purchase ledger are Platform's; delivery ownership open);
  - `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md` §4.3, §4.5 (D47, D49: account scope and portability; account inbox; claim is a DUR-03 MINT);
  - `docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md` §4.1 (Game status derived only from Platform entitlement through the `PROD-ENTITLEMENTS-01` consumer fence; a Game record is evidence, never a second authority);
  - `docs/architecture/ADR-0004-postgresql-and-data-ownership.md` §3-§5 (one owner per data set; cross-system effects through versioned API or outbox with idempotent consumers; no cross-database writes);
  - `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §2, §4.3, §5 (Platform-authored intent keyed by `operation_id`, Game-initiated consumption, durable Game receipt, `recovery_generation`, restore reconciliation);
  - `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` row STORE-ENT-1 and owner question Q4.
- **Blocking rule.** The open delivery question of Oteryn-Game gap register §32 (mechanics plan question Q4: who owns the Store inbox line) is **resolved** by owner decision S-1 (2026-10-15), which adopts option (a) of §4: Platform owns the "purchased, pending delivery" ledger, and Game only shows it and issues the item at claim. This contract records that decision; it does not take it. The owner decided S-1..S-9 on 2026-10-15 (§12); each is cited as `DECIDED` where it applies. The only step that remains before acceptance is an exact-head independent review of this document. Until this contract is accepted on an exact head, nothing here authorizes implementation, and STORE-ENT-1 stays blocked on that acceptance, no longer on §32.

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
5. how a Game or Platform restore interacts with delivered, reversed and reconciling Store deliveries (§9);
6. how a purchase is started from the in-game Store Buy button, paid from the Wallet and answered (§14).

## 2. Authority (unchanged, restated for the Store)

- **Platform is the single source of the purchase.** Platform alone records that a purchase happened: the order line, the Wallet debit of Tibia Coins (or a paid order), the price paid, the product/version bought, the entitlement it issued and every reversal. Game never records, infers or reconstructs a purchase. No Game record, receipt, inbox line, unlock fact or log proves that a purchase exists.
- **Game owns the in-game effect.** Game alone decides and writes the gameplay effect: the Store inbox display and the item issue at claim (§4, `DECIDED` S-1), the DUR-03 MINT of a claimed item, the usability of a Store cosmetic unlock on a world, and any in-game reclamation (§8).
- **The catalog is Game content** (catalog decision §1). Platform binds each purchase to an exact Game catalog revision and offer key; it never edits the catalog. There is a single price list, authored in the Game catalog and imported verbatim by Platform (`DECIDED` S-5).
- No distributed ACID transaction, no shared table, no cross-database write (ADR-0004 §4, §5).

## 3. Offer, product and fulfilment unit

A purchase of one Store offer creates one Platform order line and one Platform entitlement per fulfilment unit. Every fulfilment unit has exactly one generic delivery profile. Proposed mapping (to be confirmed per product/version before activation):

| Store offer kind | Proposed profile | Game effect | Reversal class proposed (§8) |
|---|---|---|---|
| Item or consumable (D47, D49 inbox line) | C — durable gameplay grant | inbox line; claim mints through DUR-03 | before claim: reversible; after claim: exact-item revoke or shortfall (`DECIDED` S-3) |
| Cosmetic unlock (outfit, addon, mount) | B — game-consumed account entitlement | usable on compatible worlds while the entitlement is usable (D49) | deny-future-use: stops working from next use (`DECIDED` S-8) |
| Premium time | B, existing `oteryn.premium_time` contract | Premium status (Premium activation §4.1) | deny-future-use (as that contract): stops working from next use (`DECIDED` S-8) |
| Tibia Coin package | E — Platform Wallet | none in Game | Wallet policy, never a negative balance (§8.4); spent coins follow S-4 (`DECIDED` S-8) |
| Character service (rename, transfer, …) | D — Character Authority command | per its own command contract | per that contract |

Rules:

- A bundle offer enumerates its child fulfilment units; each child has its own entitlement, profile and `delivery_operation_id`. A bundle is never one opaque grant.
- `quantity` is explicit per unit. A stack of N consumables is one fulfilment unit with quantity N, not N operations, unless the product version declares otherwise.
- Every unit binds `product_id`, `product_version`, `catalog_revision`, `offer_key`, and for items the item key and content revision it was sold under (D49 provenance).

## 4. Inbox line ownership

`DECIDED` S-1 (owner 2026-10-15): Platform is the source of truth for the "purchased, pending delivery" ledger; Game only shows it in the Store inbox and issues the item when it is claimed. This selects option (a) below; option (b) is not adopted and is kept only as the rejected alternative. The invariants hold as written:

- **Option (a), adopted: Platform entitlement line that Game reads.** The inbox line *is* the Platform Profile-C entitlement in state `ACTIVE`/`UNCLAIMED`. Game lists it through the delivery feed (§6) and, at claim time, mints and returns a receipt; Platform then marks the line `CLAIMED`.
- **Option (b), not adopted: Game inbox record.** Platform delivers the fulfilment unit; Game creates an account inbox record in one transaction with its receipt; the claim later mints from that record under its own claim operation.

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
- The exact member set, result codes, bounds and encoding are fixed by the wire contract that follows this contract (§32/Q4 resolved by S-1; transport `DECIDED` S-2: Game pulls); the semantic members above are binding.

## 6. Outbox and transport: at least once, idempotent apply

### 6.1 Platform side

- Platform writes the order line, the Wallet debit (Profile E mutator), the entitlement and a **delivery intent outbox row** in one Platform transaction. For a purchase started in game, that transaction is the one of §14.3. No intent exists without its purchase; no purchase with a Game effect exists without its intent.
- The intent is immutable once written. It stays `PENDING` until Platform holds a Game receipt for it. A timeout, a lost response, an expiry of a transport attempt or a missing receipt never marks it failed and never mints a new `operation_id` (generic contract: never mint X2 as a blind replacement).
- Store delivery intents do **not** carry a short technical TTL that turns a paid delivery into a rejection. `DECIDED` S-6: a paid but unclaimed purchase never expires automatically; if it cannot be delivered, support reviews it and may refund it.

### 6.2 Transport direction

`DECIDED` S-2 (owner 2026-10-15): Game pulls new purchases from Platform, polling the same way character commands work, over the existing Game→Platform mTLS client, as in commands contract §2 (list pending, read exact intent, publish receipt). Platform never calls Game and there is no inbound push endpoint on Game.

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
| item claimed, unit still present and untraded in the claiming character's possession | `RECLAIM` of exactly the minted unit identities (`DECIDED` S-3) | revoke the exact item if the player still holds it and has not transferred it |
| item claimed and consumed, traded, sold, destroyed or moved to another account | `FLAG` only: nothing is removed from any other player; the shortfall is recorded (`DECIDED` S-3); account locked pending resolution and sent to manual support review (`DECIDED` S-4) | Tibia chargeback model |
| Tibia Coins already spent on Store goods (Profile E reversal) | no negative Wallet balance; Platform records a reversal shortfall and, if policy chooses, reverses the downstream Store deliveries under this table; the account is locked pending resolution and sent to manual support review (`DECIDED` S-4) | Tibia chargeback model |

### 8.3 Invariants

- **No negative balances.** Neither the Platform Wallet nor any in-game balance, counter, stack or quantity becomes negative because of a reversal. What cannot be reclaimed is recorded as a shortfall, never as a debt balance.
- **No collateral loss.** A reversal never removes value from an account or character other than the one the unit was delivered to and claimed by, and never removes a unit other than the exact minted unit identities recorded in the claim receipt.
- **Fail closed, not destructive.** If Game cannot prove the exact unit is present and untouched, the result is `FLAG` with the shortfall, not a best-effort deletion.
- **Idempotent.** A reversal operation reclaims at most once; a replay republishes its receipt.
- **Ordered.** A reversal of a unit whose delivery is still `AMBIGUOUS` waits for the delivery receipt; Platform never sends `RECLAIM` for a claim it has no receipt for.
- **Unreclaimable value.** `DECIDED` S-4: if the item cannot be reclaimed (consumed, sold or spent), the account is locked pending resolution and the case goes to manual support review (Tibia chargeback model). The lock and the review are carried out by support through the operation and receipt path of §8.4; nothing is taken from third parties.

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
- **Difference from Character commands:** a paid Store delivery lost by the restore is decided again and normally **re-applied** under the new generation, because the customer still holds the purchase and the restored Game state proves the effect is absent. It is not expired into a rejection (S-6: it never expires automatically).
- A claim lost by the restore leaves the line claimable again; a minted unit lost by the restore is not re-minted automatically outside a new claim.
- A reversal lost by the restore is decided again against the restored state; if the reclaimed unit has reappeared it is reclaimed again under the same `reversal_operation_id`, otherwise flagged.
- Platform acknowledges the restore notice only after every reopened Store intent has a new receipt; Game opens admission only after that acknowledgement (as commands contract §5.4).

### 9.2 Platform restore

- Platform is the purchase authority, so a Platform restore that loses purchases is a commercial incident, not a Game decision. Game never recreates a purchase from its receipts.
- Game receipts whose `operation_id` Platform no longer knows are answered `404`; Game keeps them, stops republishing and alarms. The effect stays in Game until Platform re-establishes the purchase (from provider truth or its own ledger recovery) or issues an explicit reversal.
- A Platform restore must not lower any Profile-B fence (generic contract) and must not re-issue an `operation_id` already used (UUIDv7, never recycled).
- `DECIDED` S-7: after a Platform restore the purchase ledger is rebuilt from Platform's own ledger plus the payment provider. Game data is used only for comparison and alerting, never as a source.

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
7. refund after claim, unit present: exactly that unit reclaimed (S-3), once; if it cannot be reclaimed, shortfall recorded and the account locked for support review (S-4);
8. refund after the unit was traded: nothing removed from the receiver; shortfall flagged; no negative balance anywhere;
9. chargeback of a coin package whose coins were spent: Wallet never negative; shortfall recorded;
10. Game PITR after a delivery: reopened by generation; surviving receipt republished; lost delivery re-applied once;
11. Game PITR after a reversal: reversal re-decided; no double reclaim;
12. Platform restore that loses a purchase: Game keeps its receipt, alarms, nothing recreated;
13. dependency unavailable: no write, no receipt, intent stays `PENDING`;
14. cosmetic unlock revoked: unusable at next use, fence not lowered by restore.

Purchase initiation from Game (§14):

15. double click: the same Buy confirmation is sent twice under one `purchase_request_id`, both concurrently and in sequence. Result: one Wallet debit, one order line, one set of entitlements and intents, and the two answers are byte-identical;
16. lost response: Platform commits `ACCEPTED` and the answer is lost. Game retries the same `purchase_request_id` with the same binding. Platform returns the stored `ACCEPTED` answer byte-identical, debits nothing again and mints no new operation id. A retry under a new id is a conformance failure on the Game side;
17. concurrent purchases overspending: two different `purchase_request_id`s, whose total exceeds the balance, arrive at the same time. The Wallet row lock serializes them: one is `ACCEPTED` and the other `INSUFFICIENT_BALANCE`. The balance never becomes negative and the ledger holds exactly one debit;
18. stale quote: the offer's price, currency or `catalog_revision` differs from Platform's active import of the Game catalog. Platform answers `STALE_QUOTE` and writes no debit and no order line. A retry of the same id returns the same `STALE_QUOTE`. Game refreshes the offer, and the player's next confirmation uses a new id;
19. disabled flag: with the purchase surface or the offer kind disabled, nothing is written and nothing is debited. The answer is `PURCHASE_DISABLED`, or `503` while the whole surface is off. Turning the flag on later does not commit an earlier refused request.

## 12. Owner decisions

All nine questions were decided by the owner on 2026-10-15, approving the architect's recommendations. This section is the record; the body sections cite each decision.

| ID | Decided (owner 2026-10-15) | Rationale |
|---|---|---|
| S-1 | Platform is the source of truth for the "purchased, pending delivery" ledger. Game only shows it in the Store inbox and issues the item when it is claimed. | One purchase authority; Game never records a purchase (§2, §4). |
| S-2 | Game pulls new purchases from Platform, polling the same way character commands work. There is no inbound push endpoint on Game. | One integration pattern, no new Game listener (§6.2). |
| S-3 | On refund or chargeback after a claim, revoke the exact item if the player still holds it and has not transferred it. Otherwise record a shortfall; never take from third parties. | No collateral loss (§8.2, §8.3). |
| S-4 | If the item cannot be reclaimed (consumed, sold or spent), lock the account pending resolution and send it to manual support review. | Follows the Tibia chargeback model (§8.2, §8.3). |
| S-5 | There is a single price list, authored in Game's catalog; Platform imports it verbatim. | One price authority, no drift (§2). |
| S-6 | Paid but unclaimed purchases never expire automatically. If one cannot be delivered, support reviews it and may refund it. | A paid delivery is never turned into a rejection (§6.1, §9.1). |
| S-7 | After a Platform restore, the purchase ledger is rebuilt from Platform's own ledger plus the payment provider. Game data is only for comparison and alerting, never a source. | Platform stays the purchase authority (§9.2). |
| S-8 | Revocation per product: Premium time and cosmetics (outfits, mounts) stop working from their next use. Items follow S-3. Spent coins follow S-4. | Matches the reversal classes in §3 and §8.2. |
| S-9 | Launch order: Premium time and cosmetics first, items later. Real sales stay gated until payments are approved. | Premium snapshot contract keeps paid issuance unavailable until the Payments gate. |

## 13. Acceptance and rollout

- Oteryn-Game gap register §32 / mechanics plan Q4 (inbox line ownership) is resolved by owner decision S-1 (2026-10-15), which adopts option (a) of §4. No owner decision is open for this contract.
- What remains before acceptance is only an exact-head independent review of this document. Status stays **Proposed** until that review accepts it; acceptance belongs to the owner and the independent reviewer, never to the author.
- Rollout order (`DECIDED` S-9): Premium time and cosmetics first, items later; real sales stay gated until payments are approved.
- After acceptance, the steps are:
  1. a wire contract: members, codes and bounds, including the purchase request of §14;
  2. Game and Platform implementations, disabled by default;
  3. a shared-fixture `testing` stack, enabled per offer kind, never globally.
- STORE-ENT-1 stays blocked until this contract is accepted.

## 14. Purchase initiation from Game (in-game Store Buy button)

A player presses Buy in the in-game Store. This section fixes the semantics of that purchase; the members, codes, bounds and encoding belong to the wire contract.

### 14.1 Direction and authority

- Game sends a **Platform purchase request** over the existing Game→Platform mTLS client, in the same direction as the delivery feed (§6.2). `DECIDED` S-2 still holds: Platform never calls Game, and the native client never talks to Platform directly. The Game server forwards the player's confirmed Buy action.
- Platform alone decides whether the purchase happens (§2). Game's quote is a claim to be checked, never a price authority: the price is the one in Platform's verbatim import of the Game catalog (`DECIDED` S-5).
- Game asserts the account, character and world from its own authority: the current session and `AccountId`↔`CharacterId` ownership. Platform checks that the account exists and may buy. It records the character and world as Game-asserted purchase context and never treats them as ownership authority.

### 14.2 What a request binds

Every purchase request binds the members below, and Platform stores them with its answer:

- `account_id`, the Platform `AccountId`, copied and never minted by Game;
- `character_id` and `world_id` of the buying character;
- `catalog_revision` and `offer_key` of the offer as Game showed it;
- the quoted unit price and its currency: ordinary coins or transferable coins, from one closed set fixed by the wire contract;
- `quantity`;
- `purchase_request_id`: a client idempotency key. It is a UUIDv7 that Game creates once per confirmed Buy action and persists before it sends the first attempt.

### 14.3 Platform processing: one transaction

1. Look up `purchase_request_id`. If an answer is stored, compare the binding:
   - an equal binding returns the stored answer bytes and writes nothing;
   - a different binding is `CONFLICT`: Platform writes nothing, keeps the stored answer and raises an alarm.
2. Check the gates: the purchase surface and the offer kind are enabled, and the account exists and is not restricted, including the locks of §8.3.
3. Re-validate the offer against Platform's active import of the Game catalog:
   - the `catalog_revision` is the active one;
   - the offer exists, is purchasable and is mapped to a product version (§3);
   - the quoted price and currency equal the imported ones;
   - the total is `quantity × unit price`, without overflow.
4. Lock the buyer's Wallet row with the Wallet mutator's row lock and debit the total, using an idempotency key derived from `purchase_request_id`. The balance never becomes negative: a balance that does not cover the total is `INSUFFICIENT_BALANCE`.
5. Write the order line, the entitlement or entitlements of every fulfilment unit (§3) and every delivery intent outbox row (§6.1).
6. Store the answer with the binding under `purchase_request_id`.

Steps 1 to 6 are **one Platform transaction**. Either all of it commits or nothing does. No debit exists without its order line, entitlements and intents, and no intent exists without its debit.

### 14.4 Closed result set

| Result | Meaning | Written |
|---|---|---|
| `ACCEPTED` | the purchase committed; the answer carries the order id, the `delivery_operation_id` of every unit and the resulting balance | debit, order line, entitlements, intents, stored answer |
| `INSUFFICIENT_BALANCE` | the balance of the quoted currency does not cover the total | stored answer only |
| `STALE_QUOTE` | the `catalog_revision`, price or currency differs from Platform's active import | stored answer only |
| `OFFER_UNAVAILABLE` | the offer is unknown, not purchasable, not mapped to a product version, or its quantity is out of bounds | stored answer only |
| `ACCOUNT_RESTRICTED` | the account is unknown to Platform's buying gate, locked (§8.3) or otherwise barred from buying | stored answer only |
| `PURCHASE_DISABLED` | purchases or this offer kind are disabled | stored answer only |
| `CONFLICT` | the `purchase_request_id` is already stored with a different binding | nothing; alarm |
| `UNAVAILABLE` | a transient dependency failed (database, lock timeout, catalog import unavailable) | nothing |

- Every answer except `CONFLICT` and `UNAVAILABLE` is terminal for its `purchase_request_id`. An exact retry returns the same answer bytes and debits at most once, ever.
- `UNAVAILABLE` and a lost response are not terminal. Game retries the **same** `purchase_request_id` with the same binding until it gets a terminal answer, and never mints a new id for a retry. A new id means a new purchase and is used only for a new confirmed Buy action.
- A terminal refusal never turns into a purchase later. Re-enabling a flag, topping up the balance or a new catalog import does not commit an earlier refused request.

### 14.5 Balance and history for in-game display

- Game may read the buyer's current balance, per currency, to show it in the in-game Store.
- Game may also read a paged history of Wallet and Store transactions, newest first.
- Both reads are Platform-authoritative and read-only, and Game caches nothing as authority:
  - a displayed balance may be stale;
  - only the purchase answer of §14.4 decides a purchase;
  - a Game copy of a balance or history entry never proves a purchase or a balance (§2).
- History pages are bounded and cursor-based, so a page stays stable while new entries arrive. They carry no payment-provider payload, no secret and no data of another account.

## References

- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- Oteryn-Game: `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md`; `docs/architecture/ADR-0004-postgresql-and-data-ownership.md`; `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md`; `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md`; `docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md`; `docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §32; `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` (STORE-ENT-1, Q4)
- Issues: Oteryn/oteryn-platform #321 (Payments), #322 (Products/Entitlements), #924 (native game delivery)
