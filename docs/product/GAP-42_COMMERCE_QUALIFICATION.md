# GAP-42 Store delivery and coin market qualification

Platform records Store purchase decisions, delivery intents and receipts, coin transfers, market reservations, and coin settlements. Players can review their own records in **Shop → Balance and history**. Staff with `admin.access`, `payments.reconcile`, and staff MFA can use **Commerce → Store and coin operation status**.

| Record | Reference to use | What the status confirms |
| --- | --- | --- |
| Store delivery, claim, or reversal | Game delivery reference from the player's purchase history, or the operation ID from the Store reconcile feed | The Store order, operation state, binding digest, current receipt and its recovery generation, and the last applicable restore event. Pending and reopened operations are awaiting Game evidence. |
| Coin transfer | Wallet operation UUID from the player's history or the original wire request | An immutable Platform coin movement, including amount and both accounts. |
| Coin market reservation, trade, or release | Offer UUID, trade UUID, or cancellation UUID from the original wire request | The committed coin movement and the current reservation remainder. Game gold is shown as unavailable. |

If a lookup finds no committed record, keep the original reference. A timeout may leave a request in flight. Look up the same operation again or let its original caller retry with the same key and exact payload. Keep unresolved trades in Game's durable queue until both sides reconcile. Staff can use the operation status page to compare evidence while investigating a dispute. The page has no controls for settlement, gold release, receipt submission, or balance correction.

## Default-off configuration

- The player history is part of `OTERYN_SHOP_ENABLED`. Browser transfers use the existing testing-only Store transfer configuration and Wallet authorization.
- The operator lookup is part of `OTERYN_ADMIN_COMMERCE` and the staff panel. It reads historical Platform records even when their write flags are off.
- Game Store purchase and delivery use `PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED` and `PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED`, the configured Store mTLS identity, and the existing environment restrictions. Use the owner-approved Premium Time configuration with operator-granted coins for the qualification run.
- Native coin transfers and market operations use `PORTAL_COIN_TRANSFERS_ENABLED` and `PORTAL_COIN_MARKET_ENABLED` with their own mTLS identities. Use an isolated testing stack and test funding. Leave real-payment paths off.

## Platform and Game qualification

1. Import and activate the approved Game-authored quote on the isolated stack. Submit one funded purchase twice with the same bytes and reference. Expect one debit, one order and the original response. Send conflicting payloads, stale quotes and unaffordable requests and verify that they add no debit.
2. Deliver once in Game and submit the exact receipt twice. Compare the intent and receipt digests in the staff view and confirm the result in the buyer's history. A receipt with mismatched binding must be refused. After a restore, the same operation must reopen and refuse superseded receipts. Confirm the new generation with a fresh Game decision and verify the original purchase remains a single debit.
3. Submit one qualifying transfer twice with the same key. Confirm one debit and one credit. Try conflicting key reuse, insufficient funds and negative input. Compare the operation with both players' Wallet histories and the staff view.
4. Reserve an offer, partially settle it with a stable trade UUID, and cancel the remainder. Retry each request. Confirm the buyer's coin credit and the seller's released hold occur once. Repeat with a response lost during settlement and compare the original trade UUID across the Platform record and Game's queue before resolving the gold side. Preserve the Platform record and escalate any discrepancy with the trade, reservation and restore references.

Game still needs to qualify catalogue and receipt production, one-time fulfilment across retries and restore, character resolution, market matching and expiry, durable replay and uncertainty handling, and gold recovery for disputed or partially settled trades. A matched Game gold record and a restore drill with both services are required to close the end-to-end qualification.

The Platform wire contracts and owner decisions define the qualification requirements. [Store purchase and delivery](../contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md), [Store owner decisions](../contracts/OTERYN_V2_STORE_OWNER_DECISIONS_AMENDMENT_2026-11-05.md), [Currency decision](../contracts/OTERYN_V2_STORE_CURRENCY_D1_AMENDMENT_2026-11-12.md), [Coin transfer and market](../contracts/OTERYN_V2_COIN_TRANSFER_AND_MARKET_CONTRACT.md).
