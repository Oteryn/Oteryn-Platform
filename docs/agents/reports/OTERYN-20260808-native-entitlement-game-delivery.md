# OTERYN-20260808 — Native Entitlement / Game Delivery Architecture Review

## Result

`PASS — COMMERCIAL ENTITLEMENT AND GAMEPLAY DELIVERY AUTHORITY SEPARATED`

Issue #924 resolves the Platform-side native boundary for products that may affect Oteryn-v2 gameplay without implementing Payments, ProductsEntitlements runtime, Wallet mutation or game delivery.

## Evidence reviewed

- ADR 0031 Platform-vs-game authority split;
- `MODULE_CATALOG.md` ownership for ProductsEntitlements, Wallet and Integration;
- Issue #321 provider-neutral Payments boundary;
- Issue #322 Products/Entitlements/Vouchers scope and delivery invariants;
- accepted `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`;
- current main after Issue #919 closeout;
- open PR/branch search for overlapping entitlement/game-delivery ownership.

No overlapping open PR or branch was found for this focused native boundary.

## Accepted truth separation

Three states remain independent:

1. **payment/order truth** — owned by Payments/provider reconciliation;
2. **Platform entitlement truth** — owned by ProductsEntitlements;
3. **game delivery/enforcement truth** — owned by Oteryn-v2 when gameplay is affected.

Consequences:

- paid != entitlement delivered;
- entitlement active != game effect already applied;
- game receipt != payment settled forever;
- refund/chargeback != game effect automatically reversed;
- public/account presentation cannot collapse these truth domains.

## Delivery profiles

### Platform-only

No game mutation. Platform owns lifecycle and enforcement.

### Game-consumed account entitlement

Platform owns entitlement lifecycle; Oteryn-v2 owns gameplay enforcement using an accepted revisioned entitlement state. Premium/VIP may use this profile if separately adopted.

### Durable gameplay grant

Platform owns commercial fulfilment workflow; Oteryn-v2 owns authoritative gameplay grant/result. Delivery is idempotent and ambiguous outcomes reconcile by the same delivery operation identity.

### Character-service entitlement

Entitlement authorizes but does not perform the mutation. The game operation goes through the Character Authority command contract.

Required one-use ordering:

```text
reserve entitlement
  -> submit Character Authority operation
  -> terminal game COMPLETED
  -> consume entitlement
```

Ambiguous game state keeps the entitlement reserved; entitlement-consumption retry cannot replay the game mutation.

### Oteryn Coin package

Current Wallet authority remains Platform-owned. Coin packages use approved Wallet mutators/append-oriented idempotent ledger, not game persistence.

### Voucher/redeem

Redemption creates/activates the declared Platform entitlement/order-equivalent state and then follows the underlying delivery profile. It never bypasses game/Wallet authority.

## Stable delivery identity

Every game-affecting delivery has one stable Platform delivery-operation identity bound to:

- EntitlementId semantic identity;
- product/version;
- delivery profile/version;
- canonical AccountId/CharacterId/topology scope as applicable;
- target intent and correlation.

Exact retries reuse the same identity. Duplicate/replayed delivery cannot double-grant. Conflicting reuse fails closed. The delivery ID is not an authentication credential.

## Entitlement state vs delivery state

The contract requires distinct states so Platform can represent:

- active entitlement + pending game delivery;
- active entitlement + ambiguous/reconciling delivery;
- revoked entitlement + gameplay compensation pending;
- completed game delivery + later payment refund/chargeback.

This avoids treating one subsystem as proof of another.

## Premium/VIP handling

Platform owns commercial start/end/revocation revision. Oteryn-v2 owns gameplay enforcement.

The contract intentionally does not choose:

- cache TTL/offline grace;
- in-session expiry behavior;
- reconnect requirement;
- forced disconnect policy.

A newer revocation must supersede a stale allow state once revision order is known, but revocation alone does not authorize disconnecting an existing session without the owning runtime/session policy.

## Refund / chargeback safety

A commercial reversal is not automatically an inverse game mutation.

Each product version must choose an explicit policy:

- reversible;
- compensating operation;
- deny future use/access;
- manual reconciliation.

Direct item deletion, silent character edits or negative Wallet corrections are forbidden without the owning explicit contract.

## Character service integrity

A single-use service protects two independent effects:

- Platform entitlement consumption;
- authoritative game character mutation.

Platform reserves the entitlement before game submission. Oteryn-v2 resolves the mutation. Only terminal game success consumes the entitlement under normal one-use semantics. Ambiguity keeps reservation/recovery state visible.

This is a saga, not distributed ACID.

## Compatibility

Canary premium/account/coin fields and numeric IDs remain `Legacy Canary Compatibility` only. No native product may use shared/direct game SQL by convenience.

An ambiguous native delivery cannot fall back blindly to Canary because the native operation may already have committed.

## Product impact

### #322

The generic native delivery authority is now defined at architecture level. #322 still owns actual product catalogue, entitlement/voucher/service history persistence, UI/API, Wallet integration and customer-facing implementation.

### #321

Payments remains separate. Provider settlement can authorize paid entitlement issuance but browser return/provider state never directly proves game delivery.

### Character services

Future paid rename/delete/transfer service products reuse `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md` for the actual mutation and keep entitlement reservation/consumption separate.

## Deferred decisions

Not selected here:

- payment provider;
- Platform schema/queues/workers;
- exact IDs/IDL/transport;
- Oteryn-v2 entitlement/grant storage or enforcement implementation;
- product catalogue/benefits/pricing;
- premium/VIP in-session expiry behavior;
- durable gameplay-grant catalogue;
- refund/chargeback policy per product;
- deployment/production activation.

## Validation classification

Architecture/documentation only:

- runtime/browser E2E: `NOT_APPLICABLE` — no executable payment, entitlement, Wallet, game-delivery, UI or deployment behavior changes;
- exact-head full-diff self-review and repository-selected CI are required;
- Oteryn-v2 and Canary remain read-only;
- no production/payment authority is created by this task.
