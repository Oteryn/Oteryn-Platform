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
- every product/version selecting Profile B must declare a finite authority-lease policy before activation; no implicit, infinite or implementation-defined stale/offline grace is permitted;
- accepted Profile-B evidence must carry an authority-issued finite cutoff that Oteryn-v2 can evaluate without interpreting transport availability as commercial truth.

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
  effective_from / effective_until
  Profile-B authority lease metadata when game-consumed
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

## Profile B finite authority lease, expiry and revocation

Profile B separates **commercial entitlement truth** from the **finite lifetime of game-consumable evidence**. A transport outage does not itself revoke an entitlement, but an old `active` decision cannot authorize gameplay benefit forever while Platform authority is unreachable.

### Required authority evidence

Every accepted Profile-B authority representation must bind at least:

```text
ProfileBEntitlementAuthority
  AccountId
  EntitlementId
  product_id + product_version
  entitlement_state
  lifecycle_revision
  authority_revision
  effective_from
  effective_until
  authority_issued_at
  authority_valid_until
```

Semantics:

- `lifecycle_revision` monotonically orders commercial grant/activation/expiry/revocation changes for one entitlement;
- `authority_revision` monotonically orders refreshed authority representations within the same lifecycle revision so an older lease cannot replace a newer lease after replay, cache rollback or restart;
- `effective_from` / `effective_until` describe the commercial interval; an open-ended commercial interval does **not** make game-consumable authority open-ended;
- `authority_issued_at` and `authority_valid_until` are Platform-authority-issued values, not browser/client timestamps;
- `authority_valid_until` is mandatory and finite for an `active` Profile-B representation;
- `authority_valid_until` must never exceed a known commercial `effective_until` and must obey the finite maximum lease declared by that exact product/version;
- a refresh may issue a later `authority_revision` and later cutoff without changing commercial `lifecycle_revision`, but replay of an older authority revision cannot move the cutoff forward;
- revoked/expired evidence is terminal for that revision ordering and is never converted back to `active` by transport retry semantics.

The exact wire encoding remains deferred. The semantic fields and ordering guarantees do not.

### Product/version authority policy

Before a product/version may activate Profile B it must declare finite values/behavior equivalent to:

```text
max_authority_lease     finite > 0
refresh_before          finite, earlier than authority_valid_until
max_clock_skew          finite
stale_within_bound      explicit allow-or-deny behavior before cutoff
authority_expired       deny entitlement benefit until acceptable fresh evidence and safe authority-time evaluation
```

Rules:

- no global implicit default may silently grant an infinite or implementation-defined grace period;
- a product may choose zero stale grace by requiring fresh authority continuously, but it may not omit the decision;
- numeric values remain product/runtime configuration and may differ by product version, but they are mandatory before activation and must be testable;
- increasing a lease/grace value is an authorization-policy change and requires the owning product/version rollout and compatibility review rather than an unnoticed cache configuration change.

### Game-side authority states

Oteryn-v2 must distinguish enough state to enforce the bound without collapsing network health into commercial truth. Classification precedence is deterministic: a known `REVOKED` lifecycle decision wins first; otherwise a known commercial expiry or elapsed accepted authority cutoff is `EXPIRED`; only then may current/stale/unavailable authority availability be evaluated.

- `CURRENT_AUTHORITY` — latest accepted ordered active representation is usable, remains before its conservatively evaluated finite cutoff and refresh is not known to be overdue;
- `STALE_WITHIN_BOUND` — refresh is due/failed or Platform authority is temporarily unreachable, but the accepted active representation remains before its conservatively evaluated finite cutoff and the product policy permits bounded stale use;
- `AUTHORITY_UNAVAILABLE` — Platform authority cannot provide acceptable evidence and there is no accepted representation or authoritative expiry/revocation fact sufficient to classify the decision; this is not the same fact as commercial expiry or revocation;
- `EXPIRED` — a known commercial effective interval/accepted expiry state has ended **or an accepted active representation has reached/passed its conservatively evaluated `authority_valid_until` cutoff**;
- `REVOKED` — a newer accepted lifecycle state explicitly revoked the entitlement.

An accepted representation whose finite authority cutoff has elapsed is therefore always `EXPIRED`, never `AUTHORITY_UNAVAILABLE`. A transport timeout/error may lead to `STALE_WITHIN_BOUND` or `AUTHORITY_UNAVAILABLE` only after the higher-precedence revoked/expired conditions have been ruled out. Transport state must never manufacture `ACTIVE`, `EXPIRED` or `REVOKED` commercial truth.

### Revision, replay and rollback fencing

The game-side consumer must preserve a durable high-water fence sufficient to enforce the accepted ordering:

- a higher `lifecycle_revision` always supersedes every lower lifecycle revision;
- within one lifecycle revision, a higher `authority_revision` supersedes every lower authority revision;
- the same ordered revision with contradictory state, interval, target, product version or cutoff fails closed as conflicting authority evidence;
- once a newer revoke/expiry/lifecycle revision has been observed, delayed active evidence from an older revision is rejected after reconnect, process restart, cache replay, projection rollback or transport reordering;
- restart cannot reset the lease duration; cached evidence retains its original absolute `authority_valid_until` and ordering fence;
- restoring an older Platform/game projection cannot lower the accepted high-water fence by implication; explicit disaster-recovery semantics must preserve or safely re-establish monotonic authority before Profile-B enforcement resumes.

Storage and transport implementation are deferred, but this durable anti-resurrection property is mandatory.

### Clock and skew boundary

Authority time is server-side security input. **All known time uncertainty shortens usable authority; it never creates grace.**

- browser/client/device clocks are never used to extend entitlement authority;
- `authority_issued_at`, `authority_valid_until` and commercial interval timestamps come from Platform entitlement authority;
- Oteryn-v2 evaluates the absolute cutoff using a trusted server time source and the product/runtime-declared finite `max_clock_skew` (or an equivalently strong monotonic authority-time mechanism);
- when trusted current time is known only within an uncertainty interval, the consumer evaluates authorization against the **latest plausible current time / conservative upper bound**, equivalently shortening the locally usable deadline by the known uncertainty; it must never subtract uncertainty from current time or add uncertainty to `authority_valid_until`;
- even uncertainty within `max_clock_skew` therefore reduces the remaining usable lease and cannot authorize past the Platform-issued absolute cutoff;
- if trusted-time uncertainty exceeds `max_clock_skew`, the consumer fails closed for new/continued Profile-B benefit. **Receiving a fresh entitlement representation alone does not restore authority while time remains unsafe.** Benefit may resume only after trusted-time uncertainty is again within the declared bound, or when the fresh authority exchange itself establishes an equivalently strong bounded authority-time/monotonic anchor that permits conservative evaluation of that representation's absolute cutoff;
- any authority-time/monotonic anchor used to recover from unsafe clock state must be bound to the accepted authority exchange/revision, must itself have finite uncertainty/validity, and must not turn message receipt time into a new lease origin;
- a delayed message does not receive a fresh full lease from receipt time; only the conservatively evaluated remainder of the authority-issued interval is usable;
- process restart, VM snapshot restore or wall-clock rollback must not extend an already accepted cutoff; implementations must combine persisted absolute authority time with monotonic/fencing behavior sufficient to prevent rollback-based extension.

### Admission, reconnect and running sessions

The entitlement authority boundary is distinct from session termination policy:

- new admission, reconnect or re-enablement of a Profile-B gameplay benefit requires `CURRENT_AUTHORITY` or explicitly permitted `STALE_WITHIN_BOUND` evidence that has not reached its conservatively evaluated `authority_valid_until` cutoff or commercial expiry **and whose authority-time basis is safely evaluable**;
- after the conservatively evaluated cutoff, stale `active` evidence is `EXPIRED` and cannot authorize a new admission/reconnect or continued Profile-B benefit;
- an already-running gameplay session does not have to be forcibly disconnected merely because entitlement authority expires; forced-disconnect policy remains owned by the runtime/session contract;
- however, session continuity cannot be used as an implicit entitlement grace: ongoing Premium/VIP/Profile-B benefits must transition to the product-defined non-entitled/degraded behavior at or before the conservative finite cutoff unless fresh acceptable evidence **and safe authority-time evaluation** are obtained;
- durable gameplay value already authoritatively granted under another delivery profile is not retroactively reclassified as Profile-B session benefit.

### Outage behavior

During Platform authority outage:

1. before the conservative finite cutoff, a product may allow only its explicitly declared `STALE_WITHIN_BOUND` behavior while the authority-time basis remains safely evaluable;
2. at/after that cutoff, accepted stale evidence is `EXPIRED` and game-consumed entitlement benefit fails closed until acceptable fresh authority evidence arrives **and its cutoff can be conservatively evaluated**;
3. if no accepted evidence or known expiry/revocation fact exists and current authority cannot be obtained, classify `AUTHORITY_UNAVAILABLE` and fail closed for the entitlement benefit;
4. a later refresh that proves the entitlement still active may restore benefit only if trusted time is safe again or that refresh establishes an equivalently bounded authority-time/monotonic anchor; fresh entitlement state alone is insufficient when time uncertainty remains unsafe;
5. a later refresh that proves expiry/revocation fences every older active representation immediately according to revision ordering;
6. outage duration, clock uncertainty, reconnect and restart never increase or reset the authority cutoff.

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
- Profile-B authority refresh age, conservative remaining lease, `STALE_WITHIN_BOUND` duration and cutoff expirations;
- rejected stale/out-of-order authority revisions and clock/skew fail-closed/recovery events;
- premium/VIP propagation lag;
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
12. for every Profile-B product/version, finite authority-lease, refresh, stale-use, conservative clock-skew/cutoff behavior plus durable revision-fencing compatibility and a safe authority-time recovery mechanism.

Rollback may redirect **new** deliveries to a proven compatibility path only when safe. An already submitted native delivery must first reach a terminal/reconciled or explicitly fenced state.

Profile-B rollback must not restore an older accepted entitlement snapshot, lower the durable revision high-water mark or restart an expired authority lease. If a rollback target cannot consume the current authority evidence/fence safely, Profile-B activation stays disabled until a compatible state is re-established.

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
- Platform outage before Profile-B lease expiry yields only the declared `STALE_WITHIN_BOUND` behavior and never extends the conservative cutoff;
- Platform outage at/after Profile-B authority cutoff classifies accepted stale evidence as `EXPIRED`, denies entitlement benefit and remains distinguishable from explicit revocation;
- `AUTHORITY_UNAVAILABLE` is used only when current authority cannot be obtained and no accepted evidence/known expiry/revocation fact already yields a more specific state;
- delayed stale `active` after a newer revoke/expiry is rejected by lifecycle/authority revision fencing;
- commercial expiry during outage cannot be masked by a longer authority lease;
- reconnect/new admission with expired or unavailable Profile-B authority fails closed for the entitlement benefit;
- process restart with cached active evidence preserves the original absolute cutoff and revision high-water fence;
- out-of-order authority refreshes and rollback to an older entitlement snapshot cannot resurrect or extend stale active authority;
- a slow trusted clock even **within** declared skew cannot authorize past the Platform-issued cutoff because all known uncertainty shortens the locally usable lease;
- clock rollback, VM snapshot restore or trusted-time uncertainty beyond the declared skew bound cannot extend Profile-B authority;
- after trusted-time uncertainty exceeds `max_clock_skew`, a fresh entitlement representation without a newly established bounded trusted-time/monotonic anchor still fails closed;
- already-running session handling may avoid forced disconnect but cannot continue Profile-B benefit past the conservative finite authority cutoff by implication;
- refund/chargeback never silently performs uncontracted irreversible game mutation;
- logs/audit redact secrets/voucher/provider/private target data;
- exact-head cross-domain E2E exists before product activation.

### Profile-B validation matrix

| Scenario | Required classification | Required result |
| --- | --- | --- |
| Platform unavailable; accepted active evidence remains before conservative cutoff | `STALE_WITHIN_BOUND` only if product policy permits | Bounded benefit only; cutoff never extended |
| Platform unavailable; accepted active evidence reaches/passes conservative cutoff | `EXPIRED` | Deny new and continued Profile-B benefit until acceptable fresh authority with safe cutoff evaluation |
| Platform unavailable; no accepted evidence and no known expiry/revocation fact | `AUTHORITY_UNAVAILABLE` | Deny Profile-B benefit; do not invent commercial revocation |
| Newer revoke observed; older active later arrives | `REVOKED` plus older evidence rejected by revision fence | Never resurrect older benefit |
| Commercial effective interval ends before authority lease | `EXPIRED` | Commercial expiry wins immediately |
| Reconnect with stale evidence inside conservative bound | `STALE_WITHIN_BOUND` only if explicitly allowed | Permit only bounded declared behavior while time basis is safe |
| Reconnect after conservative bound | `EXPIRED` | Deny entitlement benefit until fresh acceptable authority with safe cutoff evaluation |
| Restart/rollback with cached older active evidence | Preserve durable revision/authority high-water fence and original cutoff | No lease reset or authority resurrection |
| Trusted time may be slow within `max_clock_skew` | Evaluate latest plausible current time | Shorten usable lease; never authorize beyond absolute cutoff |
| Trusted-time uncertainty exceeds `max_clock_skew` and only fresh entitlement state arrives | authority time remains unsafe | Continue failing closed; fresh state alone is insufficient |
| Trusted-time uncertainty exceeds `max_clock_skew`, then trusted time is restored or fresh exchange establishes bounded authority-time/monotonic anchor | Re-evaluate ordered fresh evidence conservatively | Benefit may resume only within the provable remaining finite cutoff |

## Deferred details

Still intentionally `UNKNOWN` / owned elsewhere:

- real payment provider and provider contract;
- exact ProductId/EntitlementId/delivery-operation encoding;
- Platform ProductsEntitlements schema and worker/job design;
- exact Oteryn-v2 entitlement/grant service, storage and enforcement architecture;
- event/query/command transport and serialization;
- exact premium/VIP benefits and numeric per-product Profile-B lease/refresh/skew values; those values are mandatory before activation even though this generic contract does not select them;
- exact trusted-time/authority-time/monotonic recovery mechanism; whichever mechanism is selected must preserve the finite cutoff and cannot use fresh message receipt as a new lease origin;
- exact forced-disconnect and UI transition behavior for an already-running session after authority expiry; this deferral does not permit entitlement benefit beyond the finite cutoff;
- exact durable gameplay-grant catalogue;
- exact refund/chargeback policy per product;
- exact legal/tax/currency policy;
- staging/production rollout values.

These unknowns do not permit direct game-table writes, unbounded stale entitlement authority or collapsing payment, entitlement and game-delivery truth.

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