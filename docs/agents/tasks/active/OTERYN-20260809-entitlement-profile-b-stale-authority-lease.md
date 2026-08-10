---
task_id: OTERYN-20260809-entitlement-profile-b-stale-authority-lease
mode: implementation
issue: 944
branch: repair/issue-944
status: validating
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260809-entitlement-profile-b-stale-authority-lease

## Goal

Repair Issue #944 by making Profile-B game-consumed entitlement authority finitely valid and machine-testable during Platform authority outages, without claiming Premium/VIP runtime exists or authorizing any runtime, payment, deployment, production or external-repository work.

## Feature scope

```yaml
feature_scope:
  type: architecture_documentation
  complete_user_facing_feature: false
  backend_required: false
  frontend_required: false
  runtime_required: false
  integration_contract_required: true
  e2e_required: false
```

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
restricted_paths:
  - app/**
  - routes/**
  - resources/**
  - database/**
  - tests/**
  - deploy/**
  - .github/workflows/**
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: entitlement:profile-b-stale-authority-lease
```

## Acceptance inventory

- [x] Profile-B entitlement evidence carries an implementable finite authority bound rather than an indefinite stale `active` state.
- [x] Every product/version selecting Profile B must declare finite stale/offline policy before activation; no implicit infinite or implementation-defined grace is allowed.
- [x] Oteryn-v2 can distinguish current authority, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, expired, revoked and superseded evidence without treating transport status as commercial truth.
- [x] Lifecycle revision/effective interval ordering plus durable high-water fencing prevents delayed, replayed, restarted or rolled-back stale `active` evidence from resurrecting a newer expiry/revocation.
- [x] New admission/reconnect is distinguished from already-running-session behavior; session-disconnect policy may remain deferred but entitlement benefit cannot continue beyond the finite authority bound by implication.
- [x] Clock/skew semantics prevent client/local-clock ambiguity from extending authority and require fail-closed refresh when safe time evaluation cannot be proven.
- [x] Validation requirements cover outage before/after lease expiry, delayed stale active after revoke, expiry during outage, reconnect with stale evidence, restart with cached active evidence, out-of-order revisions, rollback and local-clock uncertainty.
- [x] Premium/VIP runtime, actual numeric product values and production activation remain explicitly deferred/not claimed.
- [x] No runtime/schema/routes/tests/workflow/deployment/credential/production/external-repository mutation occurs.
- [ ] Exact-head Agent Governance and repository-selected CI pass; runtime/browser E2E remains `NOT_APPLICABLE` for this contract-only repair.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2239
  classified_at: 2026-08-10T22:39:00+02:00
  risk: high
  triggers:
    - paid or commercially controlled gameplay authority
    - entitlement expiry and revocation
    - network partition and authority outage
    - stale cache and reconnect
    - time-bound authorization
    - durable cross-module architecture contract
  unknown_or_conflict: []
  rationale: A future Profile-B paid entitlement must not survive an unbounded Platform authority outage merely because the last accepted state was active.
  self_review:
    result: PASS
    exact_head: 082c9f8a19306a79a45dcc3a878589e0d9c0f35a
    evidence:
      - Whole-diff review against main@7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654 shows exactly the task packet plus the single Issue-authorized entitlement/game-delivery contract path.
      - The contract now requires authority-issued finite authority_valid_until evidence for every Profile-B representation capable of authorizing gameplay benefit.
      - Commercial expiry/revocation always bounds or supersedes the lease; active-without-commercial-expiry still requires a finite periodic authorization lease.
      - Every activated product/version must declare finite max-stale/lease, refresh and bounded skew policy; missing numeric policy blocks activation instead of falling back to implementation-defined grace.
      - Current, STALE_WITHIN_BOUND, AUTHORITY_UNAVAILABLE, EXPIRED, REVOKED and SUPERSEDED states are explicitly distinguished without treating transport reachability as commercial truth.
      - Durable lifecycle high-water fencing covers delayed delivery, cache replay, reconnect, process restart and projection/storage rollback.
      - Admission/reconnect and continued-benefit behavior are bounded by the existing finite cutoff while forced-disconnect mechanics remain deliberately deferred.
      - The contract supplies an explicit negative-path validation matrix for outage, revocation, expiry, reconnect, restart, out-of-order revisions, rollback and clock uncertainty.
      - No Premium/VIP activation, runtime/schema/test/workflow/deployment/production or external-repository mutation is authorized or claimed.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:47:00+02:00
head: 082c9f8a19306a79a45dcc3a878589e0d9c0f35a
branch: repair/issue-944
pr: none
status: validating
context_routes:
  - agent-governance
  - architecture
  - security
  - commerce-entitlements
  - oteryn-v2-integration
owned_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
proven:
  - Issue #944 is open, P1/high risk, implementation-authorized and bounded to the entitlement/game-delivery architecture contract with runtime/deployment/production/external writes forbidden.
  - The repair branch was atomically claimed from protected main 7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654 and no competing #944 PR existed at claim time.
  - The pre-repair contract expressed that stale authority must not last forever but omitted any mandatory finite authority cutoff and deferred all exact stale/offline behavior.
  - Implementation head 082c9f8a19306a79a45dcc3a878589e0d9c0f35a adds finite authority evidence, product activation policy, authority states, revision fencing, clock/skew, admission/session boundaries and a validation matrix.
  - The diff from protected main contains only docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md and this task packet; no forbidden path changed.
  - Numeric product lease/max-stale/refresh/skew values remain intentionally unknown and must be explicitly selected before any future Profile-B product/version activation.
derived:
  - The finite lease bounds unseen revocation/expiry during partition, while the durable revision high-water mark separately prevents resurrection after a newer lifecycle decision has ever become known.
  - Separating transport reachability from commercial truth permits bounded degraded operation without turning network failure into entitlement revocation or infinite grace.
  - Forced disconnect can remain an owning runtime/session-policy decision because entitlement benefit itself is now independently forbidden beyond the finite authority cutoff.
unknown:
  - exact future Premium/VIP benefits
  - exact per-product numeric lease duration, refresh lead and permitted clock-skew values
  - exact future transport/IDL/storage implementation
conflicts: []
first_failure:
  marker: profile-b-active-authority-has-no-finite-cutoff
  evidence: Resolved at implementation head 082c9f8a19306a79a45dcc3a878589e0d9c0f35a by mandatory finite authority_valid_until semantics and activation-blocking finite product policy.
rejected_hypotheses:
  - Lifecycle revision alone bounds an unseen future revocation during an authority outage; it only orders revisions after the newer revision becomes observable.
  - Commercial effective_until alone is sufficient for all outage cases; revocation can occur before commercial expiry, so a separately bounded refresh/authority lease is required.
  - Transport failure itself means entitlement revoked; commercial truth remains Platform-owned and unavailable authority is distinct from revocation.
  - Deferring forced-disconnect mechanics permits continued commercial benefit after lease expiry; the repaired contract explicitly forbids that interpretation.
changed_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
validation:
  - command: whole-diff architectural/security self-review against Issue #944 acceptance
    result: PASS
    evidence: Exact implementation head 082c9f8a19306a79a45dcc3a878589e0d9c0f35a satisfies every contract acceptance item without widening into forbidden runtime or rollout scope.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Contract-only documentation repair with no executable behavior, route, schema, test harness or deployment mutation.
blockers: []
next_action: Open the authoritative #944 delivery PR, run exact-head Agent Governance and repository-selected CI plus Codex review, repair any material finding on the same PR, then merge and archive/release ownership.
```
