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
- [x] Oteryn-v2 can distinguish current authority, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, expired and revoked evidence without treating transport status as commercial truth.
- [x] Lifecycle and authority revision ordering plus durable high-water fencing prevent delayed, replayed, restarted or rolled-back stale `active` evidence from resurrecting a newer expiry/revocation or older lease.
- [x] New admission/reconnect is distinguished from already-running-session behavior; session-disconnect policy may remain deferred but entitlement benefit cannot continue beyond the finite authority bound by implication.
- [x] Clock/skew semantics require all known uncertainty to shorten usable authority and prevent slow/local-clock ambiguity from extending the Platform-issued cutoff.
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
    exact_head: 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0
    evidence:
      - Whole-diff review against main@7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654 confirms only the Issue-authorized entitlement contract plus this task packet are changed.
      - Profile-B active evidence has mandatory finite authority_valid_until and lifecycle plus authority_revision monotonic ordering.
      - Every Profile-B product/version must explicitly declare finite max_authority_lease, refresh_before and max_clock_skew plus stale/expired behavior before activation.
      - State precedence is deterministic: known REVOKED first, then known commercial or authority-cutoff EXPIRED; AUTHORITY_UNAVAILABLE is reserved for no accepted/authoritative fact sufficient to classify a more specific state.
      - Accepted evidence that reaches/passes the finite cutoff is always EXPIRED, eliminating implementation-dependent EXPIRED-versus-AUTHORITY_UNAVAILABLE behavior.
      - All known trusted-time uncertainty shortens usable authority by evaluating latest plausible current time; bounded skew can never add grace or authorize beyond the Platform-issued absolute cutoff.
      - Durable lifecycle/authority high-water fencing covers delayed delivery, cache replay, reconnect, restart, transport reordering and projection/storage rollback.
      - Running-session continuity does not imply entitlement continuity; Profile-B benefit stops at/before the conservative finite cutoff unless fresh acceptable authority is obtained.
      - Rollback cannot lower revision fences, restore older authority or restart an expired lease; incompatible rollback keeps Profile B disabled.
      - The validation matrix covers outage, expiry/unavailable precedence, revoke replay, commercial expiry, reconnect, restart/rollback and clock uncertainty.
      - No Premium/VIP activation, runtime/schema/test/workflow/deployment/production or external-repository mutation is authorized or claimed.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:52:00+02:00
head: 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0
branch: repair/issue-944
pr: 968
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
  - The repair branch was atomically claimed from protected main 7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654 and PR #968 is the single authoritative delivery PR.
  - The pre-repair contract expressed that stale authority must not last forever but omitted a mandatory finite authority cutoff and implementable degraded-state boundary.
  - Exact reviewed implementation head 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0 defines finite evidence, lifecycle/authority ordering, explicit product policy, deterministic state precedence, conservative clock/skew, durable fencing, admission/session behavior, outage behavior, rollback gates and validation matrix.
  - Codex review 4900827318 identified conservative-clock, state-precedence and exact-final-review findings; the two semantic findings are addressed in implementation head 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0 and this checkpoint records the renewed whole-diff review.
  - The diff from protected main contains only docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md and this task packet; no forbidden path changed.
  - Numeric product lease/refresh/skew values remain intentionally unknown but are mandatory activation inputs rather than implicit defaults.
  - Agent Governance run 31430515990 validated checkpoint schema/ownership and failed only because the task had not yet persisted PR #968; subsequent checkpoint persisted the PR identity.
derived:
  - The finite lease bounds unseen revocation/expiry during partition, while durable revision fencing separately prevents resurrection after a newer lifecycle/authority representation has become known.
  - Conservative clock evaluation makes uncertainty fail safer by consuming lease budget instead of manufacturing additional authorization time.
  - Separating EXPIRED from AUTHORITY_UNAVAILABLE yields machine-testable telemetry and enforcement without treating transport failure as commercial truth.
  - Forced disconnect can remain an owning runtime/session-policy decision because entitlement benefit itself is independently forbidden beyond the finite authority cutoff.
unknown:
  - exact future Premium/VIP benefits
  - exact per-product numeric lease duration, refresh lead and permitted clock-skew values
  - exact future transport/IDL/storage implementation
conflicts: []
first_failure:
  marker: profile-b-active-authority-has-no-finite-cutoff
  evidence: Resolved at exact reviewed implementation head 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0 by mandatory finite authority_valid_until semantics and activation-blocking finite product policy.
rejected_hypotheses:
  - Lifecycle revision alone bounds an unseen future revocation during an authority outage; it only orders revisions after the newer revision becomes observable.
  - Commercial effective_until alone is sufficient for all outage cases; revocation can occur before commercial expiry, so a separately bounded refresh/authority lease is required.
  - Transport failure itself means entitlement revoked; commercial truth remains Platform-owned and unavailable authority is distinct from revocation.
  - Bounded clock skew is harmless for lease expiry; disproven because a slow clock inside the bound could over-authorize unless uncertainty shortens the usable deadline.
  - Deferring forced-disconnect mechanics permits continued commercial benefit after lease expiry; the repaired contract explicitly forbids that interpretation.
changed_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
validation:
  - command: whole-diff architectural/security self-review against Issue #944 acceptance and Codex findings
    result: PASS
    evidence: Exact reviewed implementation head 817e0565044b92aacb9f6a1ebd93b5c077b9b6c0 resolves the semantic findings and satisfies Issue acceptance without widening into forbidden runtime or rollout scope.
  - command: Agent Governance run 31430515990
    result: FAIL
    evidence: Schema/ownership validation passed; live liveness reported only branch_pr_identity_omitted before PR #968 was persisted in the task checkpoint.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Contract-only documentation repair with no executable behavior, route, schema, test harness or deployment mutation.
blockers: []
next_action: Validate the resulting task-only final package head on PR #968, record exact-final-head whole-diff review externally without changing the tree, obtain Agent Governance/repository CI/Codex clearance, then merge and archive/release ownership.
```
