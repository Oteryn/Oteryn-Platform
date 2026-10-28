# Oteryn-v2 Store wire contract

## Status

`PROPOSED — oteryn-v2-store-wire-v1 — NOT ACCEPTED — DEFAULT-OFF — STACKED ON THE PROPOSED STORE PURCHASE DELIVERY CONTRACT`

- Task: **STORE-WIRE-1**. Governing issue: #1658.
- Wire of `docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md` (the *semantic contract*, STORE-DELIVERY-CONTRACT-1, PR #1625). This document fixes routes, members, types, bounds, result codes, encoding, digests, idempotency keys, paging and limits. It changes no rule of the semantic contract and decides nothing the owner has not decided (S-1..S-9, semantic contract §12).
- Pattern: the Character Authority command exchange (`docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`, `app/CharacterAuthority/CharacterCommandWire.php`) and the premium snapshot peer guard (`app/ProductsEntitlements/Http/GuardPremiumSnapshotPeer.php`). The restore notice is the existing `oteryn-v2-game-restore-reconciliation-v1` (`docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`).
- Fixtures: `docs/contracts/fixtures/store-wire-v1/` (§13).
- This contract stays Proposed until the semantic contract is accepted and an exact-head independent review accepts this document. Acceptance belongs to the owner and the reviewer, never to the author.

This contract authorizes no implementation, payment, real sale, Wallet credit or debit, Store activation, item mint, cosmetic unlock, Platform or Oteryn-Game runtime or schema change, deployment, or production mutation.

## 1. Principles

- **Direction** (`DECIDED` S-2). Every route is Game→Platform over the existing internal mTLS listener. Platform never calls Game. The native client never calls these routes; the Game server calls them on its player's behalf.
- **Authority.** Platform is the authority for the price check, the Wallet, the order, the entitlements and the pending-delivery ledger (`DECIDED` S-1, S-5). Game is the authority for in-game effects and reports them only through receipts.
- **Closed surface.** The route set (§3), message member sets, enums and result codes are closed. Any member, value or result code not listed answers `400`, never a best-effort accept.
- **Empty failures.** Every non-200 answer has an empty body. Every answer is `Cache-Control: no-store`.
- **Idempotency.** Every write carries a key fixed before the first attempt. An exact retry returns the same bytes and writes nothing (§9).

## 2. Encoding

### 2.1 Canonical JSON

Every request body, every 200 body, every intent and every receipt is canonical JSON:

- one UTF-8 JSON object, `Content-Type: application/json`, no BOM, no trailing newline;
- exactly the members listed for the message, in the listed order, none missing, none extra;
- no insignificant whitespace and no escape sequence: the body contains no backslash. The members' character sets below never need one;
- the bytes equal the re-encoding of the decoded value with `JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE`;
- decode depth at most 4.

A body that is not canonical answers `400`.

Transport rules, as on the command routes:

- `Content-Length`, when present, is `0|[1-9][0-9]{0,9}` and at most the message bound, else `413`;
- `Transfer-Encoding` is absent, else `400`;
- `Content-Encoding` is absent or `identity`, else `400`;
- the body is non-empty and within the bound, else `400`.

### 2.2 Types

| Type | Form |
|---|---|
| `uuid7` | canonical lower-case UUIDv7 `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `nil` | `00000000-0000-0000-0000-000000000000`, start of an operation cursor only |
| `uint64` | string `[1-9][0-9]{0,19}`, at most `18446744073709551615`: the command contract's encoding |
| `cursor64` | `"0"` or a `uint64` |
| `amount` | string `0\|[1-9][0-9]{0,18}`, at most `9223372036854775807` (fits a signed 64-bit integer) |
| `amount+` | an `amount` other than `"0"` |
| `delta` | string `0\|-?[1-9][0-9]{0,18}`, absolute value at most `9223372036854775807` |
| `hex64` | lower-case SHA-256 hex `^[0-9a-f]{64}$` |
| `int[a,b]` | JSON integer between `a` and `b` inclusive |
| `source_authority` | `[A-Za-z0-9._:/-]{1,128}`, equal to the configured `store.source_authority` |
| `offer_key` | `^[a-z0-9][a-z0-9._-]{0,63}$`, copied from the Game catalog |
| `grant_key` | `^[a-z0-9][a-z0-9._:-]{0,63}$` (item, cosmetic or `premium_time` key) |
| `product_id` | `^[a-z][a-z0-9_]{0,31}(\.[a-z][a-z0-9_]{0,31}){1,3}$`, at most 64 bytes |
| `currency` | `COINS` \| `TRANSFERABLE_COINS` (§10, open question Q1) |
| `unit_kind` | `PREMIUM_TIME` \| `COSMETIC` \| `ITEM` |
| `operation_kind` | `DELIVER` \| `CLAIM` \| `REVOKE_UNCLAIMED` \| `RECLAIM` \| `FLAG` |
| `reversal_reason` | `REFUND` \| `CHARGEBACK` \| `SUPPORT_CORRECTION` |

Times (`*_at`, `issued_at_source`) are `uint64` Unix seconds. Platform times are recorded and displayed, never used to select or order operations; ordering is by `source_revision`, `entry_id` or `recovery_generation`.

### 2.3 Digests

All digests are lower-case hex SHA-256 over **exact canonical bytes**:

- `binding_digest` = SHA-256 of the stored intent bytes exactly as `ReadStoreOperationIntentV1` serves them. It is the semantic contract's "canonical digest of every intent member" (§5).
- `request_digest` = SHA-256 of the exact purchase request bytes (§5.3).
- `receipt_digest` = SHA-256 of the exact stored receipt bytes (§5.8). `claim_receipt_digest` is the `receipt_digest` of a stored CLAIM receipt.
- `minted_units_digest` is Game's digest of its minted unit identity list. It is opaque to Platform, stored, and echoed back to Game through `claim_receipt_digest` so that Game reclaims exactly the units it recorded (`DECIDED` S-3).

## 3. Routes

All routes are `POST`, so that reads carry a body and every request is bound. The set is closed.

| # | Path | Operation | Request bytes | 200 body bytes | Flag (§11) |
|---|---|---|---|---|---|
| 1 | `/internal/v1/products-entitlements/store/balance/read` | `ReadStoreBalanceV1` | 512 | 512 | purchase |
| 2 | `/internal/v1/products-entitlements/store/history/read` | `ReadStoreHistoryPageV1` | 512 | 12288 | purchase |
| 3 | `/internal/v1/products-entitlements/store/purchases` | `PurchaseStoreOfferV1` | 1024 | 2048 | purchase |
| 4 | `/internal/v1/products-entitlements/store/operations/pending` | `ListPendingStoreOperationsV1` | 512 | 8192 | delivery |
| 5 | `/internal/v1/products-entitlements/store/operations/read` | `ReadStoreOperationIntentV1` | 512 | 4096 (intent) | delivery |
| 6 | `/internal/v1/products-entitlements/store/operations/receipt` | `PublishStoreDeliveryReceiptV1` | 1024 | 64 | delivery |
| 7 | `/internal/v1/game-auth/game-restore/notice` (existing, unchanged) | `PublishGameRestoreNoticeV1` | 512 | existing | restore (existing) |
| 8 | `/internal/v1/products-entitlements/store/operations/reconcile` | `ListStoreOperationStatesV1` | 512 | 12288 | delivery |

The `products-entitlements` prefix matches the premium snapshot read. Route 7 is not a new route: §7 states what the existing notice does to Store operations.

Every request on routes 1–6 and 8 begins with the same head:

| Member | Type | Rule |
|---|---|---|
| `contract_version` | integer | `1` |
| `operation` | string | the operation of the route |
| `source_authority` | `source_authority` | equal to the configured value, else `401` |

## 4. Peer identity and guard

### 4.1 One dedicated Store identity

- Routes 1–6 and 8 accept exactly one client certificate subject: `products-entitlements.store.mtls_client_identity`. It is a printable ASCII string of 1–128 bytes. This is the semantic contract's "dedicated mTLS identity for the Store consumer" (§10).
- The identity is **dedicated**. If it equals any identity configured for another internal purpose, the Store surface is misconfigured and answers `503` on every Store route. The other purposes are:
  - `game-auth.native_evidence`;
  - `game-auth.character_bootstrap_intent`;
  - the account-characters publishers;
  - `game-auth.native_runtime_status` / `native_scope_assignment`;
  - `products-entitlements.premium_snapshot`;
  - the character command consumer;
  - `character-authority.restore.identities`;
  - the public-projection consumer.

  The other surfaces apply their own rule in the same way.
- Route 7 keeps its own identity list (`character-authority.restore.identities`), as the restore contract defines it.

Splitting the Store identity in two (purchase reads and writes versus delivery feed) is a least-privilege option that the owner or Game may ask for (open question Q7). The wire does not depend on it.

### 4.2 Guard (mirrors `GuardPremiumSnapshotPeer`)

In this order, for every Store route:

1. **`503`** if the route's surface is disabled or misconfigured (§11). This includes a missing or overlapping identity, a missing source authority, a bound out of range, and an environment outside the allowed list.
2. **`401`** unless all of these hold:
   - `SSL_CLIENT_VERIFY` is `SUCCESS`;
   - `SSL_PROTOCOL` is `TLSv1.3`;
   - `SSL_CLIENT_S_DN` is a string of at most 128 bytes;
   - `hash_equals(configured identity, SSL_CLIENT_S_DN)` is true.

   These values come from the trusted TLS terminator, as for the premium snapshot read.
3. **`429`** with `Retry-After` (seconds, at least 1) when the per-peer limiter refuses. The limiter key is `products-entitlements-store:` + SHA-256 of the identity, with a 60-second window, and its limit is `store.requests_per_minute`.
4. Transport and canonical checks (§2.1), then the head. A wrong `source_authority` answers `401`.
5. The route handler.

Per-account limits (§12) apply inside the handler, after the body is decoded.

## 5. Messages

Member tables list members in canonical order. Every member is required unless the type says `| null`, in which case the member is present with `null`.

### 5.1 Balance read: `ReadStoreBalanceV1`

Request: head, then `account_id` (`uuid7`, Platform `AccountId`).

200 body:

| Member | Type | Rule |
|---|---|---|
| `contract_version` | integer | `1` |
| `result` | string | `OK` |
| `account_id` | `uuid7` | equal to the request |
| `balances` | array 1–2 | one entry per currency Platform holds, `COINS` first; no currency twice |
| `balances[].currency` | `currency` | |
| `balances[].available` | `amount` | spendable now |
| `balances[].reserved` | `amount` | held, for example by an auction escrow; not spendable |
| `read_at` | `uint64` | Platform read time, display only |

Result codes:

| Status | Meaning |
|---|---|
| `200` `OK` | the balance |
| `400` | malformed or not canonical |
| `401` | wrong peer or source authority |
| `404` | Platform knows no such account |
| `413` | `Content-Length` above 512 |
| `429` | peer or account limit, with `Retry-After` |
| `503` | surface off or misconfigured, or store unavailable |

An account without a Wallet row reads `"0"`; Platform does not create the row on a read.

Platform v1 holds exactly one currency, `COINS`, which is the existing Wallet `available_balance` / `reserved_balance` of the account's Platform identity. `TRANSFERABLE_COINS` appears only after the owner decides Q1 (§10).

The balance is display only (semantic contract §14.5). Game caches nothing as authority.

### 5.2 History page read: `ReadStoreHistoryPageV1`

Request members after the head:

| Member | Type | Rule |
|---|---|---|
| `account_id` | `uuid7` | |
| `before_entry_id` | `cursor64` | `"0"` = newest page; otherwise only entries with a smaller `entry_id` |
| `max_entries` | `int[1,32]` | |

200 body:

| Member | Type | Rule |
|---|---|---|
| `contract_version` | integer | `1` |
| `result` | string | `OK` |
| `account_id` | `uuid7` | equal to the request |
| `entries` | array 0–32 | strictly descending `entry_id` |
| `entries[].entry_id` | `uint64` | Platform's Wallet ledger entry sequence, never reused |
| `entries[].kind` | enum | `STORE_PURCHASE`, `STORE_REVERSAL`, `COIN_PACKAGE_CREDIT`, `COIN_PACKAGE_REVERSAL`, `AUCTION_RESERVE`, `AUCTION_RELEASE`, `AUCTION_SETTLE`, `OPERATOR_ADJUSTMENT`, `OTHER` |
| `entries[].currency` | `currency` | |
| `entries[].available_delta` | `delta` | |
| `entries[].reserved_delta` | `delta` | |
| `entries[].order_id` | `uuid7 \| null` | non-null iff `kind` is `STORE_PURCHASE` or `STORE_REVERSAL` |
| `entries[].offer_key` | `offer_key \| null` | same rule |
| `entries[].quantity` | `int[1,100] \| null` | same rule |
| `entries[].created_at` | `uint64` | display only |
| `next_before_entry_id` | `cursor64` | the last returned `entry_id`, or `"0"` when no older entry exists |

- **Paging.** The cursor is the ledger sequence, so a page is stable while new entries arrive: new entries only ever get larger ids.
- **Exposure.** The page exposes only the account's own entries. It never exposes a provider reference, a payment payload, an idempotency key, metadata, operator identity, or another account's data.
- **Unmapped entries.** A Wallet operation type with no mapping is `OTHER`.

Result codes are those of §5.1. A `413` is above 512 bytes. The 200 bound is 12288 bytes: the worst case at 32 entries is 11666 bytes (fixture manifest `worst_case_bytes`).

### 5.3 Purchase: `PurchaseStoreOfferV1`

Request (semantic contract §14.2):

| Member | Type | Rule |
|---|---|---|
| head | | `operation` = `PurchaseStoreOfferV1` |
| `purchase_request_id` | `uuid7` | Game-minted once per confirmed Buy action and persisted before the first attempt |
| `account_id` | `uuid7` | the buyer's Platform `AccountId` from Game's session authority |
| `character_id` | `uuid7` | buying character; recorded as Game-asserted context, never ownership authority |
| `world_id` | `uuid7` | the character's world; recorded |
| `catalog_revision` | `uint64` | revision of the Game catalog import the offer was shown from (open question Q2) |
| `offer_key` | `offer_key` | |
| `currency` | `currency` | quoted currency |
| `unit_price` | `amount+` | quoted unit price |
| `quantity` | `int[1,100]` | wire ceiling; the offer's own maximum may be lower |

Platform processes the request in one transaction, in the semantic contract's §14.3 order. Each check that refuses ends processing; the first refusal wins, and the result codes it produces are fixed:

1. **Stored answer.** If `purchase_request_id` has a stored answer, compare request bytes. Equal bytes return the stored answer bytes: `200`, nothing written. Different bytes return `409` (`CONFLICT`): nothing written, the stored answer is kept, and the alarm `products_entitlements.store_purchase_request_conflict` is raised.
2. **Account.** Platform does not know the account, or it is locked (semantic contract §8.3, S-4), or it is otherwise barred from buying: `ACCOUNT_RESTRICTED`.
3. **Catalog revision.** `catalog_revision` is not the active import revision: `STALE_QUOTE`.
4. **Offer.** `offer_key` is unknown in the active revision, or not purchasable, or not mapped to a product version: `OFFER_UNAVAILABLE`.
5. **Kind enabled.** A unit kind of the offer is not in `store.enabled_unit_kinds`: `PURCHASE_DISABLED`.
6. **Quote.** `currency` or `unit_price` differs from the import: `STALE_QUOTE`.
7. **Quantity and currency.** Any of these gives `OFFER_UNAVAILABLE`:
   - `quantity` is above the offer's maximum;
   - the currency is not held by the Wallet (Q1);
   - `quantity × unit_price` is above `9223372036854775807`;
   - the units of the offer exceed 8.
8. **Balance.** Lock the Wallet row (the Wallet mutator's insert-or-ignore plus `lockForUpdate`). If `available` is below the total: `INSUFFICIENT_BALANCE`.
9. **Commit.** All of the following happen in this same transaction, and the answer is `ACCEPTED`:
   - debit `available` by the total, with operation type `store_purchase` and idempotency key `store-purchase:` + `purchase_request_id`;
   - write the order line;
   - write one entitlement per fulfilment unit;
   - write one DELIVER intent per fulfilment unit (§5.5);
   - write the stored answer.

A refusal in steps 2–8 stores its answer (with the request digest) and writes nothing else.

**Answer** (200, at most 2048 bytes). Every answer carries `contract_version` (`1`), `purchase_request_id`, `request_digest` (`hex64`, §2.3) and `result`, and ends with `decided_at` (`uint64`). An `ACCEPTED` answer adds these members between `result` and `decided_at`:

| Member | Type | Rule |
|---|---|---|
| `order_id` | `uuid7` | |
| `currency` | `currency` | equal to the request |
| `total_price` | `amount+` | `quantity × unit_price` |
| `available_after` | `amount` | the debited currency's `available` after commit |
| `units` | array 1–8 | one per fulfilment unit, in product-version order |
| `units[].unit_kind` | `unit_kind` | |
| `units[].entitlement_id` | `uuid7` | |
| `units[].delivery_operation_id` | `uuid7` | the DELIVER intent's `operation_id` |

**Closed result set:**

| Result | Wire | Terminal |
|---|---|---|
| `ACCEPTED` | `200` JSON | yes |
| `INSUFFICIENT_BALANCE` | `200` JSON | yes |
| `STALE_QUOTE` | `200` JSON | yes |
| `OFFER_UNAVAILABLE` | `200` JSON | yes |
| `ACCOUNT_RESTRICTED` | `200` JSON | yes |
| `PURCHASE_DISABLED` | `200` JSON | yes |
| `CONFLICT` | `409`, empty | for that id; never retried |
| `UNAVAILABLE` | `503`, empty | no; retry the same id and bytes |

The transport failures do not decide anything and none of them writes anything:

- `400` (malformed) and `413` are Game defects: Game never retries them unchanged.
- `401` is a configuration fault.
- `429` is not terminal: Game retries the same id and bytes after `Retry-After`.

Game rules (semantic contract §14.4):

- on `503`, `429`, a timeout or a lost response, Game resends the identical bytes under the same `purchase_request_id` until it receives a terminal answer;
- Game never mints a new id for a retry;
- a new id means a new confirmed Buy action.

### 5.4 Pending list: `ListPendingStoreOperationsV1`

Request members after the head:

| Member | Type | Rule |
|---|---|---|
| `after_source_revision` | `cursor64` | |
| `after_operation_id` | `uuid7` or `nil` | `nil` iff `after_source_revision` is `"0"` |
| `max_entries` | `int[1,32]` | |

200 body: `contract_version` (`1`) and `pending`, an array of 0–32 entries. Each entry has `operation_kind`, `operation_id` (`uuid7`) and `source_revision` (`uint64`). Entries are strictly ascending by (`source_revision` numerically, `operation_id`) and lie after the cursor. The bound is 8192 bytes; the worst case is 4290.

- `source_revision` is Platform's monotone Store outbox sequence, assigned at intent creation and never reused.
- "Pending" means no current receipt. This includes intents reopened by a restore (§7) and CLAIM intents waiting for the player (§6.2).
- Game pages until it gets an empty page, then rescans from (`"0"`, `nil`) on its own interval. Reading an intent many times is safe (§9).

Result codes: `200`, `400`, `401`, `413`, `429`, `503`.

### 5.5 Exact read: `ReadStoreOperationIntentV1` and the intent

Request members after the head: `issuer_authority` (constant `OTERYN_PLATFORM_STORE_AUTHORITY`), `operation_kind`, `operation_id`.

200 body: the stored intent bytes, verbatim and byte-identical on every read. The bound is 4096 bytes; the worst case is 833.

Result codes:

| Status | Meaning |
|---|---|
| `404` | no intent with that issuer, kind and id |
| `400`, `401`, `413`, `429`, `503` | as on the other routes |

**Intent.** Platform writes the intent once and never changes it. Every intent has the same opening members and the same closing members; the kind-specific binding goes between them.

| Position | Member | Type | Rule |
|---|---|---|---|
| opening | `contract_version` | integer | `1` |
| opening | `operation_kind` | `operation_kind` | |
| opening | `issuer_authority` | string | `OTERYN_PLATFORM_STORE_AUTHORITY` |
| opening | `source_revision` | `uint64` | |
| opening | `operation_id` | `uuid7` | Platform-minted, never reused |
| closing | `issued_at_source` | `uint64` | Platform time, recorded only |
| closing | `audience` | string | `OTERYN_GAME_STORE` |

Store intents carry no `expires_at`: a paid delivery never expires (`DECIDED` S-6).

Kind-specific binding, in order:

| Kind | Members |
|---|---|
| `DELIVER` | `order_id`, `entitlement_id`, `account_id`, `product_id`, `product_version` (`uint64`), `unit_kind`, `catalog_revision`, `offer_key`, `grant_key`, `grant_revision` (`uint64`), `quantity` (`int[1,65535]`) |
| `CLAIM` | `delivery_operation_id`, `entitlement_id`, `account_id`, `product_id`, `product_version`, `unit_kind` (= `ITEM`), `grant_key`, `grant_revision`, `quantity` |
| `REVOKE_UNCLAIMED` | `delivery_operation_id`, `entitlement_id`, `account_id`, `product_id`, `product_version`, `unit_kind` (= `ITEM`), `reversal_reason` |
| `RECLAIM` | `delivery_operation_id`, `claim_operation_id`, `claim_receipt_digest`, `entitlement_id`, `account_id`, `product_id`, `product_version`, `unit_kind` (= `ITEM`), `reversal_reason` |
| `FLAG` | `delivery_operation_id`, `reversal_operation_id`, `entitlement_id`, `account_id`, `product_id`, `product_version`, `unit_kind` (= `ITEM`), `reversal_reason`, `shortfall_quantity` (`int[1,65535]`) |

Notes on the binding members:

- `grant_key` and `grant_revision` are the item, cosmetic or premium grant and the content revision it was sold under (D49 provenance, semantic contract §3).
- For `PREMIUM_TIME`, `quantity` is the number of days in the product version.

### 5.6 Receipt: `PublishStoreDeliveryReceiptV1`

The body is one StoreDeliveryReceipt (semantic contract §5), at most 1024 bytes; the worst case is 1006. Members in order:

1. the head, with `operation` = `PublishStoreDeliveryReceiptV1`;
2. `issuer_authority`, `operation_kind`, `operation_id`, `binding_digest`;
3. `outcome` (`COMMITTED` | `REJECTED`) and `result_code`;
4. the echo members `account_id`, `entitlement_id`, `product_id`, `product_version`, each equal to the intent;
5. the committed members, only when `COMMITTED`;
6. `recovery_generation` (`uint64`) and `decided_at` (`uint64`, Game transaction time).

Committed members per kind:

| Kind | Committed members |
|---|---|
| `DELIVER` | `delivered_quantity` (`int[1,65535]`, = intent `quantity`) |
| `CLAIM` | `character_id`, `world_id`, `minted_quantity` (= intent `quantity`), `minted_units_digest` (`hex64`) |
| `REVOKE_UNCLAIMED` | `revoked_quantity` (= the line's quantity) |
| `RECLAIM` | `character_id`, `world_id`, `reclaimed_quantity` (`int[0,65535]`), `shortfall_quantity` (`int[0,65535]`); their sum = the claim receipt's `minted_quantity` |
| `FLAG` | none |

Result codes (closed):

| Code | Outcome | Kinds |
|---|---|---|
| `STORE_OK` | COMMITTED | DELIVER, CLAIM, REVOKE_UNCLAIMED, FLAG |
| `STORE_RECLAIMED` | COMMITTED | RECLAIM, `shortfall_quantity` = 0 |
| `STORE_RECLAIM_SHORTFALL` | COMMITTED | RECLAIM, `shortfall_quantity` > 0; fail closed, nothing taken from anyone else (§8.3) |
| `STORE_ACCOUNT_NOT_FOUND` | REJECTED | DELIVER |
| `STORE_GRANT_UNKNOWN` | REJECTED | DELIVER, CLAIM |
| `STORE_LINE_REVOKED` | REJECTED | CLAIM |
| `STORE_LINE_ALREADY_CLAIMED` | REJECTED | REVOKE_UNCLAIMED |
| `STORE_LINE_NOT_FOUND` | REJECTED | CLAIM, REVOKE_UNCLAIMED, RECLAIM, FLAG |
| `STORE_INTENT_INVALID` | REJECTED | all |
| `STORE_OPERATION_CONFLICT` | REJECTED | all |
| `STORE_UNSUPPORTED` | REJECTED | all; the only code for a kind Game does not support |

An incompatible world is not a receipt. The claim fails closed in Game, mints nothing, and the CLAIM intent stays pending (D49, semantic contract §4 invariant 4).

A paid DELIVER rejection never expires the purchase. It moves the unit to support review, which may refund it (`DECIDED` S-6).

Platform handling, in one transaction under the shared restore lock (§7):

| Status | Meaning |
|---|---|
| `200` `{"contract_version":1,"result":"accepted"}` | the first receipt for the key, or an exact republication of the stored current receipt |
| `400` | not canonical, wrong member set for outcome and kind, code not allowed for the kind, an echo or quantity mismatch, or a RECLAIM sum or code mismatch |
| `401`, `413`, `429` | as on the other routes |
| `404` | no intent for (`issuer_authority`, `operation_kind`, `operation_id`). Game keeps its receipt, stops republishing and alarms (semantic contract §9.2). Platform alarm: `products_entitlements.store_receipt_without_intent` |
| `409` | different bytes for a key that already has a current receipt, or a `binding_digest` that is not the stored intent's. Alarm: `products_entitlements.store_receipt_conflict` |
| `409` | a `recovery_generation` below the latest restore notice's `new_recovery_generation`. Alarm: `products_entitlements.store_receipt_lost_to_restore` (§7) |
| `503` | surface off or store unavailable; Game republishes later |

The receipt key is (`issuer_authority`, `operation_id`). `operation_kind` must equal the intent's, and the four echo members must equal the intent's.

### 5.7 Restore notice: `PublishGameRestoreNoticeV1` (existing)

The bytes and the answer are unchanged. See §7.

### 5.8 Reconcile listing: `ListStoreOperationStatesV1`

The request has the same members and cursor rule as §5.4.

200 body: `contract_version` (`1`) and `operations`, an array of 0–32 entries. Entries are ascending by (`source_revision`, `operation_id`) and cover **every** Store operation in any state. Each entry has these members:

| Member | Type | Rule |
|---|---|---|
| `operation_kind` | `operation_kind` | |
| `operation_id` | `uuid7` | |
| `source_revision` | `uint64` | |
| `state` | enum | `PENDING` (no receipt yet), `REOPENED` (current receipt superseded by a restore, no new one yet), `COMMITTED`, `REJECTED` |
| `receipt_digest` | `hex64 \| null` | SHA-256 of the stored current receipt; null iff `PENDING` or `REOPENED` |
| `recovery_generation` | `uint64 \| null` | of the current receipt; null iff `PENDING` or `REOPENED` |

The bound is 12288 bytes; the worst case is 9061.

This is the semantic contract's periodic reconcile (§9.3), by operation identity and never by time. Game compares each entry with its own receipt store. Drift is an operator alarm with a bounded metric, never an automatic grant or deletion (S-7: Game data is only for comparison). The kinds of drift are:

- a Game receipt the listing lacks;
- a `COMMITTED` digest that is not Game's stored receipt;
- `PENDING` where Game holds a receipt, which Game's outbox fixes by republishing.

Result codes: `200`, `400`, `401`, `413`, `429`, `503`.

## 6. Operation flow

### 6.1 Every fulfilment unit gets a DELIVER intent

Every fulfilment unit of an `ACCEPTED` purchase gets one DELIVER intent, `PREMIUM_TIME` and `COSMETIC` included. This keeps semantic contract §6.1 literally true: no purchase with a Game effect exists without its intent.

- **ITEM.** A committed DELIVER means Game shows the line in the account's Store inbox. Nothing is minted. The line is Platform's entitlement (`DECIDED` S-1, option (a)).
- **COSMETIC and PREMIUM_TIME.** A committed DELIVER means Game has registered the grant in its consumer view. Usability still comes only from the Platform entitlement through the Profile-B fence (premium snapshot read; generic contract). A Game record is evidence, never authority.

Whether the owner wants Profile-B units to bypass the intent instead is open question Q5.

### 6.2 Items: DELIVER, then CLAIM

1. When Platform stores a committed ITEM DELIVER receipt, it writes in the same transaction one CLAIM intent with a new Platform `operation_id`. That id is distinct from `delivery_operation_id`, which keeps one receipt per key (semantic contract §5).
2. The CLAIM intent stays pending until the player claims, and it never expires (S-6).
3. When the player claims, Game mints into one `CharacterInventory` (DUR-03) and publishes the CLAIM receipt with `character_id`, `world_id` and `minted_units_digest`. Platform marks the line `CLAIMED`.
4. Reconcile metrics treat a pending CLAIM as waiting for the player, not as delivery lag.

### 6.3 Reversals (semantic contract §8)

Each case below applies only to ITEM units:

- **Unclaimed line.** Platform writes a REVOKE_UNCLAIMED intent with a new `reversal_operation_id`. Game serializes it against the CLAIM on the line:
  - if the revoke commits first, Game also publishes the pending CLAIM as `REJECTED` / `STORE_LINE_REVOKED`;
  - if the claim committed first, the revoke is `REJECTED` / `STORE_LINE_ALREADY_CLAIMED`, and Platform continues with the next case.
- **Claimed line.** Platform writes a RECLAIM intent that binds `claim_operation_id` and `claim_receipt_digest`. Game reclaims only the exact recorded units still present and untraded in the claiming character's possession (S-3). The receipt is `STORE_RECLAIMED`, or `STORE_RECLAIM_SHORTFALL` with the shortfall.
- **Shortfall.** Platform locks the account pending resolution and opens manual support review (S-4). It writes a FLAG intent with the shortfall, and Game records its append-only audit row (§8.4). The FLAG never removes anything.

For `PREMIUM_TIME` and `COSMETIC`, a reversal is an entitlement revocation through the Profile-B fence (S-8). There is no Game reversal intent.

Platform never sends RECLAIM without a stored CLAIM receipt, and never sends REVOKE_UNCLAIMED before a stored DELIVER receipt.

## 7. Restore: reuse of the existing notice

There is no Store-specific notice. The existing `PublishGameRestoreNoticeV1` (route 7) covers Store operations as its "Deliveries (A4)" section foresees: selection by generation, superseded history kept, one new outcome, and acknowledgement after all are answered.

- **Wire.** The notice bytes and the answer are unchanged. `reopened` and `outstanding` count character commands and Store operations together.
- **Reopen.** In the notice's transaction, Platform reopens every Store intent whose current receipt has `recovery_generation` ≥ H, and carries every Store intent an earlier notice reopened that still lacks a new receipt. It uses restore-reopen rows of the same shape. This covers DELIVER, CLAIM and reversals (semantic contract §9.1).
  - A lost paid DELIVER is decided again and normally re-applied (S-6).
  - A lost CLAIM leaves the line claimable.
  - A lost reversal is decided again against the restored state.
- **Serialization.** A Store receipt publication takes the `character_restore_locks` row shared before it locks the intent. The notice takes it exclusively, as for command receipts.
- **Lost to restore.** A Store receipt below the latest notice's `new_recovery_generation` answers `409` with `products_entitlements.store_receipt_lost_to_restore`. Only an exact republication of a still-current receipt answers as before.
- **Exchange availability.** A notice that would reopen Store operations while the Store delivery exchange cannot serve them answers `503` and commits nothing. The alarm is `character_authority.restore_notice_store_exchange_unavailable`. The Store delivery routes (4, 5, 6, 8) open while the restore surface is enabled, within the Store environment limit and identity checks, even with purchases off.
- **Contract amendment.** The restore contract must be amended to say this before Store delivery is enabled; that is a precondition, not an owner decision. Until then, a stack with Store intents must not enable the restore surface. With no Store intents, every notice still acknowledges at once.
- **Platform restore.** The restore contract's open item PLATFORM-PITR-FENCE-1 applies to Store too (open question Q6). With purchases enabled, a Platform point-in-time restore is not covered by this contract version.

## 8. Paging summary

| Route | Order | Cursor | Page | End |
|---|---|---|---|---|
| history | `entry_id` descending | `before_entry_id` (`"0"` = newest) | ≤ 32 | `next_before_entry_id` = `"0"` |
| pending | (`source_revision`, `operation_id`) ascending | (`after_source_revision`, `after_operation_id`), start (`"0"`, `nil`) | ≤ 32 | empty page |
| reconcile | same as pending | same as pending | ≤ 32 | empty page |

Cursors are not opaque: they are the values of the last entry returned. Platform never skips an entry with a value above the cursor.

## 9. Idempotency keys

| Key | Minted by | Scope | Exact retry | Conflicting reuse |
|---|---|---|---|---|
| `purchase_request_id` | Game, once per confirmed Buy | stored answer and request digest | same answer bytes, one debit ever | `409`, alarm |
| Wallet `idempotency_key` = `store-purchase:` + `purchase_request_id` | Platform | Wallet ledger (≤ 120 bytes) | no second debit | Wallet `idempotency_conflict`, rolled back, `503` with alarm |
| `operation_id` of each intent | Platform, UUIDv7 | intent, receipt key | same intent bytes | never re-issued, also not after a Platform restore |
| receipt (`issuer_authority`, `operation_id`) | Game, at most one current | Platform receipt store | `200`, nothing written | `409`, alarm |
| `restore_event_id` | Game | restore notice | existing contract | existing contract |

## 10. Currency

The owner asked for ordinary and transferable coins. Platform's Wallet today holds one balance per identity ("Oteryn Coins", not legacy transferable coins), so the wire reserves both values and v1 behaves as follows:

- `COINS` is the existing Wallet balance;
- `TRANSFERABLE_COINS` is not held, so an offer quoted in it answers `OFFER_UNAVAILABLE` and the balance read omits it.

Semantics, spend order and Wallet storage for transferable coins are open question Q1. Adding them changes no member.

## 11. Configuration and rollout flags

Proposed `config/products-entitlements.php` section `store`. Every flag defaults to off or empty:

| Key | Env | Default | Effect |
|---|---|---|---|
| `purchase_enabled` | `PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED` | `false` | routes 1–3; off is `503` |
| `delivery_enabled` | `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED` | `false` | routes 4–6 and 8; off is `503`, unless the restore surface opens them (§7) |
| `enabled_unit_kinds` | `PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS` | `[]` | JSON list over `unit_kind`; a kind not listed is `PURCHASE_DISABLED` |
| `mtls_client_identity` | `PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY` | none | §4.1 |
| `source_authority` | `PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY` | none | §3 |
| `requests_per_minute` | `PRODUCTS_ENTITLEMENTS_STORE_REQUESTS_PER_MINUTE` | `600` (1–6000) | per peer |
| `purchases_per_account_per_minute` | `PRODUCTS_ENTITLEMENTS_STORE_PURCHASES_PER_ACCOUNT_PER_MINUTE` | `10` (1–60) | route 3; `429`, nothing stored |
| `reads_per_account_per_minute` | `PRODUCTS_ENTITLEMENTS_STORE_READS_PER_ACCOUNT_PER_MINUTE` | `60` (1–600) | routes 1–2 |
| `max_pending_operations` | `PRODUCTS_ENTITLEMENTS_STORE_MAX_PENDING_OPERATIONS` | `1000` (1–100000) | route 3 answers `503` (`UNAVAILABLE`, nothing written) while this many DELIVER intents are pending |

The surface is misconfigured, and answers `503`, in any of these cases:

- `purchase_enabled` is true while `delivery_enabled` is false;
- an identity is missing, malformed or overlapping;
- the source authority is missing;
- a bound is out of range;
- `enabled_unit_kinds` holds an unknown value;
- the environment is not `testing` or `preproduction`.

The environment limit is the command routes' limit. It makes a production sale impossible under this contract version.

Rollout order (`DECIDED` S-9), each step gated on its conformance scenarios (semantic contract §11) on the shared-fixture `testing` stack:

1. Delivery exchange and the restore amendment (§7) with no purchases.
2. `purchase_enabled` with `enabled_unit_kinds = ["PREMIUM_TIME","COSMETIC"]`, funded only by operator-granted coins. This needs Q3 and Q4 answered first.
3. `ITEM`, after the item scenarios (claim, refund before and after claim, trade, Game restore) pass.
4. Real sales: not authorized by this contract version. They need the Payments gate (#1236) and a later contract version that lifts the environment limit.

Rollback at any step: turn the flag off. Stored answers, intents and receipts stay, and pending deliveries resume when the delivery surface is re-enabled.

## 12. Size and rate limits

| Message | Bound (bytes) | Worst case (bytes) |
|---|---|---|
| balance request / response | 512 / 512 | 257 / 323 |
| history request / response | 512 / 12288 | 319 / 11666 |
| purchase request / answer | 1024 / 2048 | 627 / 1575 |
| pending request / response | 512 / 8192 | 337 / 4290 |
| read request / intent | 512 / 4096 | 356 / 833 |
| receipt / answer | 1024 / 64 | 1006 / 42 |
| reconcile request / response | 512 / 12288 | 337 / 9061 |
| restore notice | 512 (existing) | — |

- Entry ceilings: 32 per page and 8 units per purchase.
- Quantity ceilings: 100 for a purchase and 65535 for a unit.
- JSON depth: at most 4.
- Rate limits are in §11. A `429` always carries `Retry-After` and never decides a purchase.

## 13. Fixtures

`docs/contracts/fixtures/store-wire-v1/` holds:

- `manifest.json`, which carries:
  - the routes, bounds and worst cases;
  - the result codes, member order and cross-field rules;
  - each valid example with its canonical size and SHA-256;
  - schema-level invalid cases (`invalid_cases`) and rule-level ones (`rule_cases`) with their expected status;
- a JSON Schema (draft 2020-12, closed objects) per message;
- valid examples for all eight messages.

Fixture files are pretty-printed for review. Canonical bytes are their compact re-encoding in file member order. Every receipt's `binding_digest` is the SHA-256 of its intent fixture, and the RECLAIM intent's `claim_receipt_digest` is that of the CLAIM receipt fixture.

## 14. Conformance additions (wire level)

Beyond semantic contract §11:

1. a non-canonical body (whitespace, reordered members, an escaped character) answers `400` on every route;
2. a peer with TLS 1.2, a failed verification, a 129-byte subject or another identity answers `401`; an overlapping identity disables the surface (`503`);
3. a purchase retried after `503` with identical bytes debits once, and a retry with any changed byte is `409`;
4. two concurrent first deliveries of the same purchase bytes commit one order and return byte-identical answers;
5. a receipt whose `binding_digest` is not the stored intent's is `409` and writes nothing;
6. a receipt below the latest notice's new generation is `409` with the lost-to-restore alarm;
7. pending and reconcile paging across concurrent inserts misses no entry above the cursor and returns none twice within one scan;
8. a history page read while new entries are written stays identical for the same `before_entry_id`.

## 15. Open questions

The following are genuinely new. None is decided here, and each blocks only what it names.

1. **Q1, transferable coins.** The Wallet holds one balance per identity, with no ordinary/transferable split. The owner needs to decide the semantics, the spend order and the Wallet storage of `TRANSFERABLE_COINS`. Until then, offers quoted in it answer `OFFER_UNAVAILABLE`.
2. **Q2, `catalog_revision` source.** The native catalog contract has no `store_offer` capability. Which Game catalog import carries offers, prices and the revision Game quotes? This blocks route 3.
3. **Q3, Premium time v1 fit.** `oteryn.premium_time` v1 allows one entitlement per account and `OPERATOR_GRANT` issuance only, and a revocation removes all remaining time. That conflicts with one entitlement per fulfilment unit and with a per-purchase refund. Does Premium time need a v2 product contract before step 2 of §11?
4. **Q4, cosmetic authority read.** No Profile-B read contract exists for cosmetics, yet S-9 launches cosmetics first. A Game consumer fence read for cosmetics is needed before step 2.
5. **Q5, Profile-B intents.** This wire gives Premium time and cosmetics a DELIVER intent (§6.1) so that semantic contract §6.1 holds literally. Confirm this, or amend §6.1 so that Profile-B units have no intent.
6. **Q6, Platform PITR.** PLATFORM-PITR-FENCE-1 applies to Store. A Platform restore that loses a purchase could let a Game retry under the same `purchase_request_id` debit and deliver again. That conflicts with S-7 (rebuild from Platform's ledger plus the provider). A fence is needed before purchases are enabled on a stack that can be restored.
7. **Q7, identity split.** Does Game want separate purchase and delivery identities (§4.1)?
8. **Q8, account to Wallet mapping.** Platform must map `AccountId` to the Wallet's identity for the row lock. This is internal to Platform and needs no owner decision, only confirmation in the implementation task.

## References

- `docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- `docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`
- `app/CharacterAuthority/CharacterCommandWire.php`, `app/ProductsEntitlements/Http/GuardPremiumSnapshotPeer.php`, `app/Wallet/WalletMutator.php` (patterns, read only)
