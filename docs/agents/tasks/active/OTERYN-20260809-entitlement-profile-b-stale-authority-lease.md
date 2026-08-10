---
task_id: OTERYN-20260809-entitlement-profile-b-stale-authority-lease
mode: implementation
issue: 944
branch: repair/issue-944
status: implementing
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

- [ ] Profile-B entitlement evidence carries an implementable finite authority bound rather than an indefinite stale `active` state.
- [ ] Every product/version selecting Profile B must declare finite stale/offline policy before activation; no implicit infinite or implementation-defined grace is allowed.
- [ ] Oteryn-v2 can distinguish current authority, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, expired and revoked evidence without treating transport status as commercial truth.
- [ ] Lifecycle revision/effective interval ordering prevents delayed, replayed, restarted or rolled-back stale `active` evidence from resurrecting a newer expiry/revocation.
- [ ] New admission/reconnect is distinguished from already-running-session behavior; session-disconnect policy may remain deferred but entitlement benefit may not continue beyond the finite authority bound by implication.
- [ ] Clock/skew semantics or an equivalently strong authority-issued validity mechanism prevent local-clock ambiguity from extending authority.
- [ ] Validation requirements cover outage before/after lease expiry, delayed stale active after revoke, expiry during outage, reconnect with stale evidence, restart with cached active evidence, out-of-order revisions and rollback.
- [ ] Premium/VIP runtime, actual numeric product values and production activation remain explicitly deferred/not claimed.
- [ ] No runtime/schema/routes/tests/workflow/deployment/credential/production/external-repository mutation occurs.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass; runtime/browser E2E is `NOT_APPLICABLE` for this contract-only repair.

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
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:39:00+02:00
head: 7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654
branch: repair/issue-944
pr: none
status: implementing
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
  - Issue #944 is open, P1/high risk, implementation-authorized, `agent:ready`, rollout-order independent before Profile-B runtime activation, and bounded to the entitlement/game-delivery architecture contract.
  - No deterministic repair/issue-944 branch or open #944 PR existed immediately before the atomic branch claim.
  - The current contract says stale/unavailable entitlement evidence must not silently extend commercial authority forever but explicitly defers exact offline grace/cache TTL/current-session behavior and contains no mandatory finite authority-expiry datum.
  - Current entitlement identity includes lifecycle revision and grant/activation/expiry/revocation semantics but no mandatory `valid_until`, lease expiry, refresh deadline or equivalent finite game-consumption authority bound.
  - Existing revision language only guarantees that delayed stale active cannot override a newer revocation once revision order is known; it does not bound how long an unseen newer revocation can be masked by outage.
  - No current production Premium/VIP runtime defect is claimed; this is a pre-runtime architecture-contract repair.
derived:
  - A finite authority lease can close the gap without selecting transport, storage or runtime implementation by requiring each Profile-B product/version to declare a bounded authority-validity policy and by carrying an authority-issued cutoff in consumed evidence.
  - The contract must treat transport availability and entitlement commercial truth as separate axes while still refusing new/continued gameplay benefit after the finite evidence bound expires.
  - Durable revision high-water fencing is required so process restart, stale cache replay, delayed delivery or projection rollback cannot resurrect an older active representation after a newer lifecycle revision is known.
unknown:
  - exact future Premium/VIP benefits
  - exact per-product numeric lease duration, refresh lead and permitted clock-skew values
  - exact future transport/IDL/storage implementation
conflicts: []
first_failure:
  marker: profile-b-active-authority-has-no-finite-cutoff
  evidence: The accepted contract requires stale authority not to last forever while simultaneously deferring all finite stale/offline values and omitting a mandatory authority-valid-until mechanism.
rejected_hypotheses:
  - Lifecycle revision alone bounds an unseen future revocation during an authority outage; it only orders revisions after the newer revision becomes observable.
  - Commercial `effective_until` alone is sufficient for all outage cases; revocation can occur before commercial expiry, so a separately bounded refresh/authority lease is required.
  - Transport failure itself means entitlement revoked; commercial truth remains Platform-owned and unavailable authority is distinct from revocation.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
validation: []
blockers: []
next_action: Update the entitlement/game-delivery contract with finite Profile-B authority evidence, product policy, state, revision-fencing, clock/skew, admission/session, rollout and validation requirements; then run exact-head documentation/governance validation and whole-diff self-review.
```
