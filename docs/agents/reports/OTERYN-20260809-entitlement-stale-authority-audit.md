# Continuous Audit Report — Entitlement Profile-B Stale Authority

## Scope

Audited protected `main@88a4c6c844c45f641375fab3b2319496dbef44b1` for the accepted Oteryn-v2 game-consumed entitlement boundary, focusing on whether a previously accepted Platform commercial `active` decision has a finite, enforceable authority lifetime during Platform outage, stale cache, delayed revocation and reconnect.

Primary evidence:

- Issue #924 — native product/entitlement game-delivery architecture;
- PR #925 — merged contract delivery and review history;
- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`;
- Issue #322 — future Products/Entitlements implementation owner;
- live active-task, open-PR and audit-repair ownership state.

The auditor does not implement the finding. Issue #944 owns contract remediation.

## Delivery/security matrix

| Concern | State | Evidence / disposition |
|---|---|---|
| Payment/order vs entitlement vs gameplay truth split | present | Contract keeps the three authorities distinct. |
| Stable entitlement and delivery operation identity | present | Contract requires stable IDs and exact-retry reuse. |
| Duplicate/replay protection | present | Duplicate delivery cannot double-grant and transport replay protection remains mandatory. |
| Newer revocation precedence when observed | present | Newer lifecycle revision supersedes older active state once revision order is known. |
| Stale/unavailable state representation | present | Contract says stale/unavailable evidence must be explicit. |
| Infinite stale authority prohibited as intent | present | Contract says commercial authority must not silently extend forever. |
| Finite stale authority cutoff | absent | No mandatory `valid_until`, lease expiry, max-stale, refresh deadline or equivalent finite proof. |
| Product-specific offline bound | absent | Exact offline grace/cache TTL is deferred without requiring every Profile-B product to choose a finite bound before activation. |
| Outage-after-revocation cutoff | absent | Old active evidence has no contract-defined time at which it becomes unusable while newer revoke is unreachable. |
| Restart/reconnect stale-cache fencing | absent | No mandatory finite authority datum prevents replay of cached active evidence after restart/reconnect. |
| Forced disconnect semantics | intentionally deferred | This is distinct from the finite authority-validity requirement. |
| Runtime Profile-B implementation | not delivered | Contract explicitly defers runtime/transport/product activation. |
| Runtime/browser E2E for this audit | NOT_APPLICABLE | Audit documentation only. |

## Acceptance falsification

Issue #924 required:

> Premium/VIP/expiry/revocation semantics distinguish Platform entitlement authority from game enforcement, including bounded stale/unavailable behavior without inventing forced-session behavior.

The merged contract partially satisfies this by requiring explicit stale/unavailable states and saying stale commercial authority cannot last forever. It then defers exact offline grace/cache TTL/current-session behavior without requiring an enforceable finite validity field or policy bound.

This leaves the acceptance criterion non-testable. A consumer cannot prove compliance with “bounded” behavior when the contract does not require any bound it can evaluate.

## Negative-path falsification

### Platform outage after active grant

1. Platform publishes entitlement revision `R10` = `active`.
2. Oteryn-v2 consumes/caches `R10`.
3. Platform entitlement authority becomes unreachable.
4. The game marks its evidence stale/unavailable as required.
5. No contract datum says when `R10` becomes unauthorized.
6. Gameplay benefit can continue indefinitely while the implementation still claims it is not treating stale state as fresh.

### Revocation during partition

1. Game has active `R10`.
2. Platform records revoked `R11`.
3. Network partition prevents Oteryn-v2 from observing `R11`.
4. Revision precedence cannot help because the newer revision is unknown.
5. Without a finite lease on `R10`, the prior allow remains usable for an unbounded period.

### Restart/reconnect with cached active evidence

1. `R10` was previously accepted as active.
2. Platform is unavailable.
3. Game node restarts or player reconnects.
4. Cached/persisted `R10` is restored.
5. Contract provides no finite authority expiry proof to distinguish “previously valid” from “still authorized now”.

## Finding

### OPA-SEC-0007 — Game-consumed entitlement contract lacks bounded stale-authority lease

- **severity:** high
- **confidence:** high
- **evidence_state:** PROVEN
- **finding Issue:** #944
- **disposition:** open
- **affected future surfaces:** Profile-B Premium/VIP or any Platform-owned commercial entitlement continuously enforced by Oteryn-v2 gameplay

#### Expected architecture

Every Profile-B entitlement representation or product policy gives Oteryn-v2 a finite, testable authorization lifetime. This can be an authority-issued `valid_until`, finite lease, finite product-specific `max_stale`, refresh deadline or equivalent mechanism. Once that bound expires without fresh acceptable authority, new/reconnected/continued benefit follows an explicit fail-closed or strictly bounded degraded policy.

A newer revoke/expiry remains dominant and cannot be resurrected by delayed messages, stale caches, process restart or rollback.

#### Actual architecture

The contract states the intent to avoid indefinite stale authority but defers the only mechanisms that would make the bound enforceable, while not requiring an alternative finite validity proof.

#### Impact

A paid gameplay entitlement can drift from Platform commercial truth during an outage and remain effective beyond expiry/revocation for an unbounded duration. That is a revenue/integrity and authorization-boundary risk before Profile-B implementation.

No current deployed Premium/VIP defect is claimed.

## Duplicate / overlap analysis

Searches covered:

- entitlement stale/unavailable TTL;
- Premium/VIP stale revocation;
- bounded entitlement authority / offline grace;
- `OPA-SEC-0007`.

No existing Issue owns the exact root cause.

Related but distinct:

- **Issue #322:** owns future Products/Entitlements runtime implementation, customer lifecycle and tests; it does not own the canonical Oteryn-v2 stale-authority contract correction.
- **Issue #924 / PR #925:** delivered the current contract and explicitly required bounded stale/unavailable behavior; this audit tests that acceptance boundary.
- **Issue #938:** federated-search publication revocation.
- **Issue #941:** personalized Today private-cache isolation.
- **native-auth production-verification task:** verification-only and owns no contract/runtime path.

## Remediation boundary

Issue #944 exclusively owns:

- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`.

Required repair contract includes:

1. finite Profile-B authority-validity/lease semantics;
2. product/version-specific bounded stale/offline policy before activation;
3. distinguish current, stale-within-bound, authority-unavailable, expired and revoked states;
4. delayed/out-of-order revoke and rollback fencing;
5. restart/reconnect/cached-active behavior;
6. time/skew or equivalent authority-validity semantics;
7. validation matrix for outage before/after bound, revocation during partition, expiry during outage and rollback.

The audit does not authorize runtime, schema, payment, transport, Oteryn-v2 mutation, deployment or production work.

## Validation

- protected-main / ownership preflight: PASS;
- Issue #924 acceptance review: PASS — bounded stale behavior was explicitly required;
- PR #925 review-history inspection: PASS — only voucher/profile-axis material review finding was raised;
- accepted contract negative-path review: PASS — no finite authority cutoff found;
- open/closed duplicate search: PASS;
- deterministic `repair/issue-944` branch availability checked before `agent:ready`: PASS;
- current runtime defect claim: REJECTED;
- runtime/browser E2E: **NOT_APPLICABLE**;
- exact-head audit PR CI/review hygiene: pending PR creation.

## Conclusion

The entitlement/game-delivery contract has strong authority separation, idempotency and reconciliation semantics, but its Profile-B stale-authority rule remains aspirational rather than enforceable. A finite authorization lease or equivalent per-product bound is required before game-consumed Premium/VIP can safely tolerate Platform outages. OPA-SEC-0007 / Issue #944 is the single material finding for this audit package.
