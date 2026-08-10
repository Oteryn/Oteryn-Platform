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
- entitlement issuance source/provenance, including voucher/redeem when adopted;
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

## Delivery profile axis

Every fulfilment unit chooses exactly one primary delivery profile from **A-E**. Composite products explicitly enumerate multiple child fulfilment units; each child still has exactly one primary profile.

Voucher/redeem is **not** a sixth delivery profile. It is a separate entitlement-issuance source/provenance axis described later in this contract. Therefore a voucher-funded entitlement can still have exactly one delivery profile A-E without semantic contradiction.

### Profile A — Platform-only entitlement

Examples: a Platform-only account capability, web preference tier or commercial right with no gameplay effect.

- Platform owns entitlement lifecycle and enforcement;
- no Oteryn-v2 mutation/delivery is required;
- a game-delivery state must not be fabricated merely for symmetry.

Payment, voucher/redeem or administrative authorization may issue the entitlement according to product policy, but gameplay remains unaffected.

### Profile B — Game-consumed account entitlement

Use for an account-level entitlement whose commercial lifecycle is Platform-owned but whose effect must be enforced by Oteryn-v2 gameplay, such as a future premium/VIP capability if adopted that way.

- Platform owns entitlement identity, commercial start/end/revocation and product policy;
- Oteryn-v2 owns gameplay enforcement/application of accepted entitlement state;
- Platform must expose an explicit versioned entitlement-state input/projection/command contract rather than direct game DB mutation;
- Oteryn-v2 must know canonical AccountId and entitlement revision/version strongly enough to reject stale/contradictory state;
- every activated Profile-B product/version must define a finite game-consumption authority policy and every consumed `active` representation must carry an authority-issued finite cutoff as defined below.

This profile does not choose push event, pull query, snapshot or command transport.

### Profile C — Durable gameplay grant

Use only for an explicitly approved product that creates durable gameplay state such as a future item/cosmetic/game-resource grant.

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

Required ordering:

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
- terminal game rejection does not consume the entitlement unless a separately accepted product contract explicitly defines a chargeable failed attempt;
- after terminal game completion, entitlement-consumption failure is reconciled by Platform entitlement idempotency and must never replay the character mutation;
- duplicate browser/service requests cannot consume one entitlement twice or create two game mutations;
- entitlement ownership does not bypass current Character Authority ownership/lifecycle/session eligibility.

### Profile E — Oteryn Coin package

Current architecture keeps the Oteryn Coins Wallet Platform-owned.

- delivery uses the approved Platform Wallet mutator/append-oriented idempotent ledger;
- it does not directly mutate Canary/Oteryn-v2 coin fields;
- payment/entitlement delivery and Wallet ledger result remain independently auditable;
- duplicate payment/provider/delivery events cannot credit the Wallet twice;
- refund/chargeback follows explicit commerce/Wallet policy rather than direct negative balance edits.

A future decision to move currency authority elsewhere requires a new architecture decision.

## Entitlement issuance source axis

Entitlement issuance provenance is independent from the delivery profile.

Supported semantic sources may include, when separately implemented/approved:

- verified paid order;
- voucher/redeem campaign;
- explicitly authorized administrator/support grant;
- migration/import with explicit provenance;
- another accepted Platform business source.

### Voucher / redeem issuance

Voucher/redeem success is an authorization/input to Platform entitlement issuance, **not** a delivery profile and not a bypass around fulfilment.

Rules:

- redemption creates/activates the approved Platform entitlement/order-equivalent state according to product policy;
- voucher plaintext is not retained where a verifier/hash design is sufficient;
- one-time/replay/concurrency rules are Platform-owned;
- the issued entitlement still uses exactly one declared delivery profile A-E;
- voucher redemption never directly calls a game mutation or Wallet balance SQL;
- delivery retry/reconciliation semantics remain determined by the entitlement's A-E profile, not by how it was issued.

This separation keeps rollout/reconciliation unambiguous: **issuance source answers why the entitlement exists; delivery profile answers how its value is fulfilled/enforced.**

## Entitlement identity and versioning

A Platform entitlement must bind enough immutable provenance to explain what was granted:

```text
Entitlement
  EntitlementId
  issuance_source_reference
  product_id
  product_version
  target_scope
  delivery_profile
  lifecycle_revision
  grant / activation / expiry / revocation semantics
```

The exact schema is deferred.

For Profile B, a game-consumed representation additionally binds the finite authority evidence defined in **Profile-B finite authority lease**. The commercial entitlement interval and the game-consumption authority lease are related but distinct: the lease may never authorize beyond commercial expiry/revocation, and a finite lease is required even when commercial expiry is farther in the future.

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

Semantics must distinguish:

- `NOT_REQUIRED` — Platform-only delivery profile;
- `PENDING` — authorized fulfilment has not reached terminal target result;
- `APPLIED` / `COMPLETED` — authoritative target boundary confirms fulfilment;
- `REJECTED` — terminal authoritative target rejection with no later commit under that operation identity;
- `RETRYABLE_PENDING` — non-terminal target condition; reuse same operation identity;
- Platform-local `AMBIGUOUS` / `RECONCILING` — commit/result is unknown;
- `COMPENSATION_REQUIRED` when commercial reversal cannot be represented as a simple inverse;
- `MANUAL_RECONCILIATION_REQUIRED` where safe automatic policy is unavailable.

An entitlement may be commercially `active` while game delivery is pending/reconciling. UI and APIs must preserve that distinction.

## Idempotency and replay

At-least-once provider events, queue delivery and service retries are assumed possible.

Implementation must prove separate idempotency for:

- payment event processing under #321 before paid entitlement issuance;
- entitlement issuance under #322 regardless of issuance source;
- game delivery under this contract;
- Wallet mutation for coin packages;
- Character Authority operation for character services.

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

## Profile-B finite authority lease

Profile B uses Platform-owned commercial truth while preventing a previously accepted `active` representation from becoming indefinite authority during an outage. This is a **finite authorization lease**, not a statement that transport health is commercial truth.

Every game-consumed Profile-B representation must bind at least these semantic fields or an equivalently strong versioned structure:

```text
ProfileBEntitlementAuthority
  EntitlementId
  product_id + product_version
  canonical AccountId
  lifecycle_revision
  commercial_effective_from
  commercial_effective_until | no_scheduled_commercial_expiry
  commercial_state              # active / expired / revoked / other explicit state
  authority_issued_at
  authority_valid_until
  authority_policy_version
```

Required invariants:

- `authority_valid_until` is issued by the Platform entitlement authority and is finite for every representation that can authorize gameplay benefit;
- `authority_valid_until` may not exceed a known commercial expiry and can be shortened by product policy; it never extends an expired/revoked commercial state;
- an `active` entitlement with no scheduled commercial expiry still receives a finite authority lease and therefore requires periodic fresh authorization;
- exact encoding, transport and storage are deferred, but an implementation-defined or infinite lease is forbidden;
- each Profile-B product/version declares, before activation, a finite authority policy including the maximum stale/lease duration, refresh-before-expiry behavior and bounded clock/skew allowance (or an equivalently strong authority-issued/monotonic mechanism);
- numeric values are product/runtime rollout decisions and remain unknown until that product/version is approved, but **absence of finite values blocks activation**;
- a consumer must not infer additional grace after `authority_valid_until`; any allowed stale operation is already represented inside the finite issued bound.

### Profile-B authority states

Game-side enforcement must distinguish at least:

- `CURRENT_AUTHORITY` — accepted evidence is within its authority interval and the authority source is currently reachable/fresh according to the selected transport contract;
- `STALE_WITHIN_BOUND` — authority cannot currently be refreshed or evidence is not fresh, but the accepted representation has not passed `authority_valid_until`;
- `AUTHORITY_UNAVAILABLE` — authority cannot be obtained and there is no still-valid accepted representation for the requested decision;
- `EXPIRED` — commercial expiry or authority lease expiry forbids the benefit;
- `REVOKED` — a known authoritative lifecycle revision revokes the benefit;
- `SUPERSEDED` — evidence is older than the durable lifecycle high-water mark and is not eligible to authorize even if its local time bound has not elapsed.

Transport success/failure alone does not produce `active`, `expired` or `revoked` commercial truth. It only affects whether current authority can be obtained. When current authority is unavailable, the finite accepted evidence and product policy determine whether gameplay benefit remains temporarily usable.

### Revision and rollback fencing

Oteryn-v2 must retain a durable high-water fence for the highest accepted lifecycle revision for each entitlement identity (or an equivalently strong monotonic authority token).

- a representation below the high-water revision cannot authorize benefit after reconnect, process restart, cache replay, delayed delivery, projection rollback or storage restoration;
- a newer known `expired`/`revoked` revision immediately supersedes every older `active` representation regardless of the older representation's `authority_valid_until`;
- equal revision with conflicting immutable lifecycle/effective/authority fields fails closed and requires reconciliation rather than choosing the more permissive value;
- authority refresh must not move the high-water revision backwards;
- rollback of the Platform projection or game-side cache to an older snapshot does not reset the high-water fence by implication;
- if durable revision-fence integrity cannot be proven after restart/recovery, Profile-B authorization fails closed until current authority is re-established.

This fence prevents resurrection of a **known** newer lifecycle decision. The finite lease separately bounds the case where a newer revocation/expiry exists at Platform but is temporarily unobservable because of a partition.

### Clock and skew semantics

Authority validity is evaluated from Platform-issued timestamps/validity evidence, not from a client clock.

- browser/client time never extends entitlement authority;
- a game node may use local wall clock only under the product/version's declared bounded skew policy, or use an equivalently strong monotonic/authority-issued expiry mechanism;
- uncertainty beyond the declared skew bound reduces usable authority rather than extending it;
- a backward local-clock jump, VM snapshot restore or process restart must not make previously expired authority valid again;
- if the implementation cannot prove safe time evaluation, it fails closed and refreshes/re-establishes current authority.

### Admission, reconnect and running-session boundary

The contract distinguishes **authorization of benefit** from **forced session termination**.

- new login/admission, reconnect and re-enabling a Profile-B gameplay benefit require evidence that is not `EXPIRED`, `REVOKED`, `SUPERSEDED` or beyond `authority_valid_until`;
- `STALE_WITHIN_BOUND` may authorize only what the exact product/version policy explicitly permits and only until the existing `authority_valid_until` cutoff;
- after that cutoff, no new or continued Profile-B gameplay benefit is authorized by the stale representation;
- whether the owning runtime disconnects a player, keeps the connection while disabling the entitlement benefit, or performs another safe transition remains product/session policy and is not selected here;
- deferring forced-disconnect semantics must never be interpreted as permission to continue the commercially controlled benefit beyond the finite authority bound.

## Premium / VIP expiry and revocation

A future premium/VIP capability using Profile B inherits the finite authority lease above.

- Platform owns commercial entitlement lifecycle revision and effective interval/revocation decision;
- Oteryn-v2 owns gameplay enforcement based on accepted bounded entitlement evidence;
- exact premium/VIP benefits, numeric lease/max-stale duration, refresh lead, clock-skew value and forced-disconnect implementation remain product/runtime decisions;
- those numeric policy values must nevertheless be finite and explicitly declared before the product/version can be activated;
- a known revocation/expiry supersedes stale allow state by durable revision fence;
- an unseen revocation during partition is bounded by the last accepted `authority_valid_until` rather than by an implicit/infinite grace period.

## Refund, chargeback, expiry and revocation after delivery

A commercial reversal is not automatically the inverse of a gameplay mutation.

Each product version must classify post-delivery reversal policy as one of:

1. **reversible** — an accepted game/Platform operation can safely revoke the effect;
2. **compensating** — a separate explicit compensation operation/state is required;
3. **deny-future-use** — irreversible historical value is not silently removed, but future use/access is denied according to policy;
4. **manual reconciliation** — operator decision under explicit RBAC/MFA/audit controls.

Rules:

- never silently delete items, mutate character state or create a negative Wallet balance merely because a provider reports a chargeback;
- provider refund/chargeback truth remains owned by Payments;
- Platform entitlement revocation remains owned by ProductsEntitlements;
- gameplay compensation/revocation remains owned by Oteryn-v2 when gameplay state is affected;
- every automated reversal has its own stable operation identity/idempotency and typed result/reconciliation semantics.

## Character-service entitlement integrity

A single-use service protects two independent effects:

- Platform entitlement usage capacity;
- authoritative Character Authority mutation.

Minimum behavior:

- reserve entitlement under Platform transaction/locking/idempotency before submitting the game operation;
- reservation binds to one intended CharacterId/service/product version and one character-operation correlation;
- concurrent requests cannot reserve the same one-use entitlement twice;
- ambiguous game state keeps the reservation held/recovery-visible;
- terminal rejection releases/transitions reservation according to product policy;
- terminal game completion consumes the entitlement once;
- consumption retry cannot repeat the game mutation.

This is a saga, not distributed ACID.

## Eligibility and target ownership

An entitlement says the customer has a commercial right; it does not guarantee current game eligibility.

- Platform verifies authenticated user and current approved AccountId/CharacterId ownership authority before use;
- Oteryn-v2 revalidates current game-owned target ownership/state when gameplay is mutated;
- stale entitlement/service-queue target snapshots are not authority;
- transfer/rename/delete/session conflicts are handled by the owning Character Authority/product contract;
- a product may remain owned while temporarily ineligible for use.

## Presentation and projections

Customer-facing presentation must preserve truth domains:

- `payment pending` != `entitlement active`;
- `entitlement active` != `game effect applied`;
- `game delivery reconciling` != `failed`;
- `refund requested` != `game effect already revoked`;
- unavailable delivery evidence != zero/no entitlement.

Public entitlement exposure, if ever adopted, requires explicit privacy policy. AccountId/current ownership/payment details are not public by default.

## Security boundary

Future game-delivery transport must provide:

- authenticated service identity;
- authorized audience/scope;
- replay protection beyond business idempotency;
- bounded schema/size/version validation;
- canonical target validation;
- secret-safe logs/traces;
- correlation across entitlement/delivery/target result without exposing provider secrets or voucher plaintext;
- rate/abuse controls appropriate to fulfilment operations.

Exact cryptographic/transport primitive is deferred.

## Observability and reconciliation

Future implementation should expose bounded metrics for:

- entitlement issuance source and lifecycle;
- delivery pending/applied/rejected/ambiguous age;
- duplicate/replayed/conflicting delivery operations;
- entitlement reservations stuck awaiting character-operation reconciliation;
- Profile-B authority refresh lag, `STALE_WITHIN_BOUND` age and lease-expiry denial;
- revision-fence conflicts/rollback attempts and authority-unavailable decisions;
- compensation/manual-reconciliation backlog;
- Wallet delivery mismatch for coin packages;
- provider-payment vs entitlement vs game-delivery drift.

Logs must not contain payment secrets, voucher plaintext, bearer credentials, complete provider payloads or unnecessary game/private state.

## Legacy Canary Compatibility

Current/historical Canary premium/account/coin fields or direct write adapters do not define the native target.

- no new native product is delivered by direct/shared Canary SQL by implication;
- Canary numeric IDs remain adapter details;
- compatibility delivery must be explicitly named, least-privileged, reversible and have removal criteria;
- an ambiguous native delivery cannot fail over blindly to Canary/direct SQL;
- migration occurs per delivery profile after native producer/consumer idempotency and reconciliation are proven.

## Rollout and rollback

Cut over per delivery profile/product version, not globally and not by issuance source.

Before native activation prove:

1. exact product/version and entitlement semantics;
2. issuance source/provenance behavior separately from delivery profile;
3. canonical target identity and server-side ownership validation;
4. delivery operation idempotency/conflicting-reuse behavior;
5. terminal/non-terminal/ambiguous result semantics;
6. reconciliation after timeout-before-receive, during execution and after commit;
7. entitlement reservation/consumption ordering for single-use services;
8. refund/revocation/compensation behavior for delivered value;
9. customer presentation of pending/reconciling states;
10. legacy consumer inventory and rollback/removal gate;
11. exact producer/consumer revision compatibility;
12. for every Profile-B product/version, finite `authority_valid_until` semantics and explicit finite max-stale/refresh/skew policy;
13. durable lifecycle-revision high-water fencing across restart/cache replay/rollback;
14. admission/reconnect and running-session benefit behavior at `STALE_WITHIN_BOUND` and lease expiry.

Profile B is **not activation-ready** while any finite authority-policy value or required high-water/time-evaluation guarantee is undefined. This contract intentionally does not choose those numeric values.

Rollback may redirect **new** deliveries to a proven compatibility path only when safe. An already submitted native delivery must first reach a terminal/reconciled or explicitly fenced state. Profile-B rollback must not restore an older entitlement snapshot below the durable lifecycle high-water mark or re-authorize evidence whose finite lease has expired.

## Validation requirements for later implementation

At minimum prove:

- provider/browser return cannot directly grant value;
- voucher redemption cannot bypass declared delivery profile;
- duplicate/out-of-order issuance source event cannot issue duplicate entitlement;
- duplicate entitlement delivery cannot duplicate gameplay/Wallet value;
- same delivery operation with changed entitlement/target fails closed;
- timeout before/after authoritative target commit reconciles without double grant;
- entitlement active vs delivery pending/reconciling renders truthfully;
- single-use character-service concurrent requests consume/mutate at most once;
- game mutation completion precedes entitlement consumption for service saga;
- coin packages use Wallet mutator/idempotency only;
- Profile-B outage **before** lease expiry yields only the product-declared `STALE_WITHIN_BOUND` behavior and never extends the existing cutoff;
- Profile-B outage **after** lease expiry denies new/continued entitlement benefit unless fresh current authority is obtained;
- an active older revision delivered after a newer revoke/expiry is rejected as `SUPERSEDED`;
- commercial expiry during an authority outage cannot be extended by a later authority lease or local-clock behavior;
- reconnect/new admission with stale evidence is accepted only when explicitly allowed within the finite bound and denied after it;
- process restart with cached active evidence preserves the lifecycle high-water fence and finite lease cutoff;
- out-of-order revisions and projection/cache rollback cannot reduce the lifecycle high-water mark or resurrect benefit;
- equal revision with conflicting lifecycle/authority facts fails closed and reconciles;
- local-clock backward jump, excessive skew or VM snapshot restoration cannot make expired Profile-B authority valid again;
- known revocation supersedes stale allow state immediately without inventing forced-disconnect policy;
- refund/chargeback never silently performs uncontracted irreversible game mutation;
- logs/audit redact secrets/voucher/provider/private target data;
- exact-head cross-domain E2E exists before product activation.

### Profile-B contract validation matrix

| Scenario | Required authority classification | Required enforcement result |
| --- | --- | --- |
| Platform unavailable, accepted active evidence still before `authority_valid_until` | `STALE_WITHIN_BOUND` | Only explicitly declared bounded product behavior; no lease extension |
| Platform unavailable, `authority_valid_until` passed | `EXPIRED` or `AUTHORITY_UNAVAILABLE` | No new or continued entitlement benefit from stale evidence |
| Newer revoke/expiry revision already known, delayed older active arrives | `SUPERSEDED` | Reject older active; durable high-water mark unchanged |
| Commercial expiry occurs before authority lease cutoff | `EXPIRED` | Commercial expiry wins; authority lease cannot extend it |
| Reconnect with stale active evidence inside finite bound | `STALE_WITHIN_BOUND` | Permit only if product policy explicitly permits reconnect inside bound |
| Reconnect after finite bound | `EXPIRED` or `AUTHORITY_UNAVAILABLE` | Deny entitlement benefit until fresh current authority |
| Process restart with cached active evidence | prior bounded state plus preserved revision fence | Never reset lease/revision history; fail closed if fence/time integrity is unproven |
| Out-of-order lifecycle revisions | `CURRENT_AUTHORITY`/`SUPERSEDED` by monotonic order | Highest accepted revision wins; no rollback |
| Projection/cache rollback to older snapshot | `SUPERSEDED` | Older snapshot cannot lower high-water revision or restore authority |
| Local-clock uncertainty exceeds declared skew | authority not safely evaluable | Fail closed / refresh authority; uncertainty never adds grace |

## Deferred details

Still intentionally `UNKNOWN` / owned elsewhere:

- real payment provider and provider contract;
- exact ProductId/EntitlementId/delivery-operation encoding;
- Platform ProductsEntitlements schema and worker/job design;
- exact Oteryn-v2 entitlement/grant service, storage and enforcement architecture;
- event/query/command transport and serialization;
- exact premium/VIP benefits and exact finite numeric lease/max-stale/refresh/skew/current-session transition values for each future product version;
- exact durable gameplay-grant catalogue;
- exact refund/chargeback policy per product;
- exact legal/tax/currency policy;
- staging/production rollout values.

These unknowns do not permit infinite/implementation-defined Profile-B grace, direct game-table writes or collapsing payment, entitlement and game-delivery truth. A Profile-B product/version remains blocked from activation until its finite numeric authority policy is explicitly approved.

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
- Issue #944 — Profile-B bounded stale-authority lease repair
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- `docs/architecture/MODULE_CATALOG.md` — ProductsEntitlements / Wallet ownership boundaries