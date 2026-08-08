# Oteryn-v2 Entitlement / Game Delivery Contract

## Status

`ACCEPTED PLATFORM CONSUMER / COMMERCIAL-ORCHESTRATION ARCHITECTURE CONTRACT — PAYMENT, GAME TRANSPORT AND RUNTIME IMPLEMENTATION DEFERRED`

This contract defines how Oteryn Platform may turn a Platform-owned product/entitlement decision into a gameplay-affecting outcome without transferring gameplay authority to Platform, treating payment as delivery proof, or writing native game persistence directly.

It is subordinate to ADR 0031 and to the owning Wallet, Payments, Character Authority and public-projection contracts.

It does **not** select a payment provider, game transport, IDL, broker, Oteryn-v2 entitlement storage/enforcement mechanism, Platform schema/worker framework, product catalogue contents, pricing, tax/legal policy or production rollout.

## Core authority split

Three truths are intentionally separate:

```text
PAYMENT / ORDER TRUTH                 PLATFORM ENTITLEMENT TRUTH              GAME DELIVERY / ENFORCEMENT TRUTH

Payments (#321)                       ProductsEntitlements                    Oteryn-v2 gameplay authority
- provider settlement                 - product/version                       - gameplay mutation result
- verified provider events            - EntitlementId                        - game enforcement/applicability
- refund/dispute/chargeback            - grant/expiry/revocation               - durable gameplay grant receipt
- payment reconciliation              - reservation/consumption              - authoritative character mutation
        |                                      |                                      |
        +------------ authorizes --------------+------------- requests/feeds ----------+
                                               |
                                               v
                                    customer/account presentation
```

Rules:

- payment success can authorize entitlement issuance under Platform product policy; it is not proof that gameplay delivery occurred;
- an active Platform entitlement is not proof that Oteryn-v2 has already applied or observed the gameplay effect;
- an Oteryn-v2 gameplay receipt is not proof that money settled or that a refund/chargeback cannot occur later;
- public/account UI state is presentation only and cannot promote one truth into another;
- no distributed ACID transaction across provider, Platform and Oteryn-v2 is assumed.

## Platform authority

Oteryn Platform owns, when implemented under #322:

- product catalogue/version and localized commercial presentation;
- order-line/product-version binding;
- Platform `EntitlementId` or equivalent stable entitlement identity;
- entitlement grant, activation, expiry, reservation, consumption, cancellation and revocation lifecycle;
- delivery workflow identity/state and commercial reconciliation;
- voucher/redeem redemption state;
- customer entitlement/service history;
- Platform Wallet mutation through its existing approved mutator/ledger boundary;
- Platform business/product policy deciding whether a game-affecting delivery may be requested.

Platform does not become authoritative for gameplay state merely because it owns the commercial entitlement.

## Oteryn-v2 authority

Oteryn-v2 owns, when a product affects gameplay:

- authoritative gameplay mutation;
- game-runtime enforcement of accepted game-consumed entitlement state;
- current game-state eligibility for a requested grant/service effect;
- durable gameplay grant/result facts;
- authoritative character mutation through the existing Character Authority boundary;
- game-internal concurrency, locking/fencing, persistence and runtime applicability.

Platform must not emulate these decisions by directly writing game tables or relying on stale game projections.

## Canonical identity

Native entitlement/game-delivery semantics use canonical identities:

- Platform `AccountId` for account-scoped target context;
- game-owned `CharacterId` for character-scoped target context;
- canonical `WorldId` / `ChannelId` only when a product legitimately requires topology scope;
- stable Platform entitlement identity (`EntitlementId` semantic role; exact encoding deferred);
- stable Platform delivery-operation identity for one semantic fulfilment attempt.

Rules:

- browser-supplied account/character/world identifiers are request input only and never establish authorization;
- target ownership and eligibility are resolved/revalidated server-side;
- Canary numeric account/player/world IDs remain Legacy Canary Compatibility details only;
- product SKU/display name is not stable entitlement identity;
- a game receipt binds to the exact entitlement/delivery operation and target so it cannot be replayed for another account, character or product revision.

## Product delivery profiles

Every product version that can be fulfilled must choose exactly one primary delivery profile. Composite products explicitly enumerate several child fulfilments; they do not silently mix authority.

### Profile A — Platform-only entitlement

Examples: a Platform-only account capability, web preference tier or commercial right with no gameplay effect.

Authority:

- Platform owns entitlement lifecycle and enforcement;
- no Oteryn-v2 mutation/delivery is required;
- a game-delivery state must not be fabricated merely for symmetry.

Payment, voucher or administrative authorization may create/revoke the entitlement according to Platform policy, but gameplay remains unaffected.

### Profile B — Game-consumed account entitlement

Use for an account-level entitlement whose commercial lifecycle is Platform-owned but whose effect must be enforced by Oteryn-v2 gameplay, such as a future premium/VIP capability if adopted that way.

Authority:

- Platform owns the entitlement identity, commercial start/end/revocation and product policy;
- Oteryn-v2 owns gameplay enforcement/application of the accepted entitlement state;
- Platform must expose an explicit versioned entitlement-state input/projection/command contract rather than direct game DB mutation;
- Oteryn-v2 must know the applicable canonical AccountId and entitlement revision/version strongly enough to reject stale/contradictory state.

This profile does not choose push event, pull query, snapshot or command transport.

### Profile C — Durable gameplay grant

Use only for an explicitly approved product that creates durable gameplay state such as a future item/cosmetic/game-resource grant.

Authority:

- Platform owns commercial entitlement and fulfilment saga;
- Oteryn-v2 owns the authoritative gameplay grant transaction and result;
- each grant uses one stable delivery operation identity;
- exact retries of the same semantic grant reuse the same identity;
- duplicate/replayed delivery cannot create a second gameplay grant;
- materially different target/product/revision under one delivery identity fails closed;
- timeout after possible commit becomes ambiguous/reconciling and must not mint a second blind grant.

No durable gameplay grant type is approved merely because this generic profile exists.

### Profile D — Character-service entitlement

Use for a single-use/right-to-use product such as a future paid rename, deletion-related service or world transfer.

The entitlement **does not perform the character mutation**. The actual mutation is routed through `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md` and the product-specific character lifecycle contract.

Required ordering for a one-use service:

```text
eligible Platform entitlement
  -> reserve entitlement use
  -> submit authoritative Character Authority operation
  -> terminal game COMPLETED
  -> consume entitlement reservation
  -> service fulfilment completed
```

Rules:

- while the game operation is pending, retryable or ambiguous, the entitlement remains reserved;
- terminal game rejection does not consume the entitlement unless an explicit product policy defines a separately chargeable failed attempt, which requires its own accepted commercial contract;
- after terminal game completion, entitlement-consumption failure is reconciled by Platform entitlement idempotency and must never replay the character mutation;
- duplicate browser/service requests cannot consume one entitlement twice or create two game mutations;
- entitlement ownership does not bypass current Character Authority ownership/lifecycle/session eligibility.

### Profile E — Oteryn Coin package

Current architecture keeps the Oteryn Coins Wallet Platform-owned.

Therefore a purchased/adopted coin package:

- delivers through the existing approved Platform Wallet mutator/append-oriented idempotent ledger;
- does not directly mutate Canary/Oteryn-v2 coin fields;
- payment/entitlement delivery and Wallet ledger result remain independently auditable;
- duplicate payment/provider/delivery events cannot credit the Wallet twice;
- refund/chargeback follows explicit commerce/Wallet policy rather than direct negative balance edits.

A future decision to move gameplay currency authority elsewhere requires a new architecture decision; this contract does not make that change.

### Profile F — Voucher / redeem product

Voucher/redeem success is an authorization/input to the same entitlement lifecycle, not a bypass around it.

Rules:

- redemption creates/activates the approved Platform entitlement/order-equivalent state according to product policy;
- voucher plaintext is not retained where a verifier/hash design is sufficient;
- one-time/replay/concurrency rules are Platform-owned;
- the redeemed entitlement still uses its declared delivery profile A-E;
- a voucher code never directly calls game mutation or Wallet balance SQL.

## Entitlement identity and versioning

A Platform entitlement must bind enough immutable provenance to explain what was granted:

```text
Entitlement
  EntitlementId
  source_order_or_redemption
  product_id
  product_version
  target_scope
  lifecycle_revision
  grant / activation / expiry / revocation semantics
```

The exact schema is deferred.

A product version change does not silently reinterpret already issued entitlements. Migration/upgrade requires explicit compatibility or replacement rules.

## Delivery operation envelope

Every game-affecting fulfilment has one stable semantic operation identity.

```text
EntitlementDeliveryOperation
  delivery_operation_id
  EntitlementId
  product_id + product_version
  delivery_profile + semantic_version
  canonical target scope
  requested intent / entitlement revision
  correlation / causation context
```

Rules:

- all retries of the same semantic delivery reuse the same `delivery_operation_id`;
- a different operation ID means a new fulfilment attempt and must never be minted solely because a response was lost;
- duplicate delivery cannot double-grant durable gameplay value;
- one operation ID cannot be reused for another entitlement, target, product version or materially different intent;
- operation identity is not an authentication credential.

Character-service Profile D may correlate one entitlement-delivery operation with one Character Authority `operation_id`; the two IDs have distinct domain roles and must not be conflated unless a future contract explicitly chooses one shared identity with equivalent guarantees.

## Delivery outcome semantics

Platform tracks **delivery state** separately from **entitlement state**.

The exact machine vocabulary may differ, but semantics must distinguish:

- `NOT_REQUIRED` — Platform-only delivery profile;
- `PENDING` — authorized fulfilment has not reached a terminal game/Wallet result;
- `APPLIED` / `COMPLETED` — authoritative target boundary confirms fulfilment;
- `REJECTED` — terminal authoritative target rejection with no later commit under that operation identity;
- `RETRYABLE_PENDING` — non-terminal target condition; reuse same operation identity;
- Platform-local `AMBIGUOUS` / `RECONCILING` — commit/result is unknown;
- `COMPENSATION_REQUIRED` or equivalent — commercial reversal/revocation cannot be represented as a simple inverse of the applied gameplay effect;
- `MANUAL_RECONCILIATION_REQUIRED` where safe automatic policy is unavailable.

An entitlement may be commercially `active` while game delivery is pending/reconciling. UI and APIs must preserve that distinction instead of presenting false completion.

## Idempotency and replay

At-least-once provider events, queue delivery and service retries are assumed possible.

Implementation must prove:

- payment event idempotency under #321 before paid entitlement issuance;
- entitlement issuance idempotency under #322;
- game-delivery idempotency under this contract;
- Wallet mutation idempotency for coin packages;
- Character Authority idempotency for character services.

Idempotency keys from one layer are not automatically proof for another layer. Cross-layer correlation is required, but each authority owns its own terminal effect.

## Ambiguous outcomes and reconciliation

For gameplay-affecting delivery:

```text
submit delivery operation X
  -> COMPLETED/APPLIED      => record authoritative target result
  -> terminal REJECTED      => record no-delivery outcome
  -> RETRYABLE/PENDING      => retry/reconcile X
  -> timeout/lost response  => mark Platform delivery AMBIGUOUS
                               reconcile X
                               never mint X2 as blind replacement
```

A `not found` result is not automatically proof that a new operation is safe unless the target contract guarantees the original cannot later materialize.

Platform entitlement/order state must remain sufficient to explain why delivery was attempted while target receipts remain authoritative for whether gameplay/Wallet mutation actually occurred.

## Premium / VIP expiry and revocation

For a game-consumed account entitlement:

- Platform owns the commercial entitlement lifecycle revision and effective interval/revocation decision;
- Oteryn-v2 owns gameplay enforcement based on accepted current entitlement evidence;
- stale/unavailable entitlement evidence must be explicitly represented; game behavior must not silently extend commercial authority forever;
- exact offline grace, cache TTL, reconnect requirement, current-session handling or forced disconnect behavior is product/runtime policy and remains deferred;
- revoking an entitlement does not by itself authorize terminating an existing gameplay session unless the owning runtime/session policy explicitly says so;
- delayed stale `active` state must not override a newer revocation once the revision order is known.

Future implementation must define a bounded fail-safe behavior appropriate to the product without converting this generic architecture into a forced-session policy.

## Refund, chargeback, expiry and revocation after delivery

A commercial reversal is not automatically the inverse of a gameplay mutation.

Each product version must classify its post-delivery reversal policy as one of:

1. **reversible** — an accepted game/Platform operation can safely revoke the granted effect;
2. **compensating** — a separate explicit compensation operation/state is required;
3. **deny-future-use** — delivered irreversible historical value is not silently removed, but future entitlement use/access is denied according to accepted policy;
4. **manual reconciliation** — operator decision is required under explicit RBAC/MFA/audit controls.

Rules:

- never silently delete items, mutate character state or create a negative Wallet balance merely because a provider reports a chargeback;
- provider refund/chargeback truth remains owned by Payments;
- Platform entitlement revocation state remains owned by ProductsEntitlements;
- gameplay compensation/revocation remains owned by Oteryn-v2 when gameplay state is affected;
- every automated reversal must have its own stable operation identity/idempotency and typed result/reconciliation semantics.

## Character-service entitlement integrity

A single-use character service has two distinct resources that must not be double-spent:

- Platform entitlement usage capacity;
- authoritative Character Authority mutation.

Minimum concurrency behavior:

- reserve the entitlement under Platform transaction/locking/idempotency before submitting the game operation;
- one entitlement reservation binds to one intended CharacterId/service/product version and one character operation correlation;
- concurrent requests cannot reserve the same one-use entitlement twice;
- ambiguous game state keeps the reservation held/recovery-visible;
- terminal rejection releases or transitions the reservation according to product policy;
- terminal game completion settles/consumes the entitlement once;
- consumption retry cannot repeat the game mutation.

This is a saga, not distributed ACID.

## Eligibility and target ownership

An entitlement says the customer has a commercial right; it does not guarantee current game eligibility.

For target-scoped use:

- Platform verifies the authenticated user and current approved AccountId/CharacterId ownership authority before initiating delivery/use;
- Oteryn-v2 revalidates current game-owned target ownership/state when gameplay is mutated;
- stale entitlement/service queue target snapshots are not authority;
- transfer/rename/delete/session conflicts are handled by the owning Character Authority/product contract;
- a product may remain owned even when temporarily ineligible for use; that is different from delivery rejection or entitlement revocation.

## Presentation and projections

Platform may expose customer-facing entitlement and delivery history, but presentation must preserve truth domains.

Examples:

- `payment pending` != `entitlement active`;
- `entitlement active` != `game effect applied`;
- `game delivery reconciling` != `failed`;
- `refund requested` != `game effect already revoked`;
- unavailable game-delivery evidence != zero/no entitlement.

Public entitlement exposure, if ever adopted, requires explicit privacy policy. AccountId/current ownership/payment details are not public by default.

## Security boundary

Future game-delivery transport must provide:

- authenticated service identity;
- authorized audience/scope;
- replay protection beyond business idempotency;
- bounded schema/size/version validation;
- canonical target validation;
- secret-safe logs and traces;
- correlation across entitlement/delivery/target result without exposing provider secrets or voucher plaintext;
- rate/abuse controls appropriate to fulfilment operations.

The exact cryptographic or transport primitive is deferred.

## Observability and reconciliation

Future implementation should expose bounded metrics for:

- entitlement issuance/activation/revocation;
- delivery pending/applied/rejected/ambiguous age;
- duplicate/replayed/conflicting delivery operations;
- entitlement reservations stuck awaiting character operation reconciliation;
- premium/VIP state propagation lag;
- compensation/manual-reconciliation backlog;
- Wallet delivery mismatch for coin packages;
- provider-payment vs entitlement vs game-delivery drift.

Logs must not contain payment secrets, voucher plaintext, bearer credentials, complete provider payloads or unnecessary game/private state.

## Legacy Canary Compatibility

Current/historical Canary premium/account/coin fields or direct write adapters do not define the native target.

Rules:

- no new native product is delivered by direct/shared Canary SQL by implication;
- Canary numeric IDs remain adapter details;
- compatibility delivery must be explicitly named, least-privileged, reversible and have removal criteria;
- an ambiguous native delivery cannot fail over blindly to Canary/direct SQL because the native operation may already have committed;
- migration occurs per product/delivery profile after native producer/consumer idempotency and reconciliation are proven.

## Rollout and rollback

Cut over per product/delivery profile, not globally.

Before native activation of one game-affecting profile prove:

1. exact product/version and entitlement semantics;
2. canonical target identity and server-side ownership validation;
3. delivery operation idempotency/conflicting-reuse behavior;
4. terminal/non-terminal/ambiguous result semantics;
5. target reconciliation after timeout-before-receive, during execution and after commit;
6. entitlement reservation/consumption ordering for single-use services;
7. refund/revocation/compensation behavior for already delivered value;
8. customer presentation of pending/reconciling states;
9. legacy consumer inventory and rollback/removal gate;
10. exact producer/consumer revision compatibility.

Rollback may redirect **new** deliveries to a proven compatibility path only when safe. An already submitted native delivery must first reach a terminal/reconciled or explicitly fenced state.

## Validation requirements for later implementation

At minimum prove:

- provider/browser return cannot directly grant value;
- duplicate/out-of-order paid-order authorization cannot issue duplicate entitlement;
- duplicate entitlement delivery cannot duplicate gameplay/Wallet value;
- same delivery operation with changed entitlement/target fails closed;
- timeout before/after authoritative target commit reconciles without double grant;
- entitlement active vs delivery pending/reconciling renders truthfully;
- single-use character-service concurrent requests consume at most once and mutate the character at most once;
- game mutation completion precedes entitlement consumption for the service saga;
- coin packages use Wallet mutator/idempotency only;
- premium/VIP revocation supersedes stale allow state by revision without inventing session-disconnect behavior;
- refund/chargeback never silently performs an uncontracted irreversible game mutation;
- logs/audit redact secrets/voucher/provider/private target data;
- exact-head cross-domain E2E exists before any product activation.

## Deferred details

Still intentionally `UNKNOWN` / owned elsewhere:

- real payment provider and provider contract;
- exact ProductId/EntitlementId/delivery-operation encoding;
- Platform ProductsEntitlements schema and worker/job design;
- exact Oteryn-v2 entitlement/grant service, storage and enforcement architecture;
- event/query/command transport and serialization;
- exact premium/VIP benefits, TTL/offline grace and in-session expiry behavior;
- exact durable gameplay grant catalogue;
- exact refund/chargeback policy per product;
- exact legal/tax/currency policy;
- staging/production rollout values.

These unknowns do not permit direct game-table writes or collapsing payment, entitlement and game-delivery truth.

## Non-authorization

This contract authorizes no:

- payment provider selection or real charges;
- provider webhook activation;
- Laravel runtime/database/worker implementation;
- Wallet credit/debit;
- voucher creation/redemption;
- premium/VIP activation;
- item/cosmetic/game-resource grant;
- character-service launch;
- Oteryn-v2 or Canary repository write;
- game-delivery endpoint deployment;
- staging/production mutation.

## References

- ADR 0031 — Native Oteryn-v2 Integration vs Legacy Canary Compatibility
- Issue #321 — Payments foundation owner
- Issue #322 — Products/Entitlements/Vouchers implementation owner
- Issue #924 — focused native game-delivery architecture owner
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- `docs/architecture/MODULE_CATALOG.md` — ProductsEntitlements / Wallet ownership boundaries
