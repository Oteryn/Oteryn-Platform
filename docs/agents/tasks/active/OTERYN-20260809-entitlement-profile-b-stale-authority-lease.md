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
- [x] Oteryn-v2 can distinguish `NOT_YET_EFFECTIVE`, current authority, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, expired and revoked evidence without treating transport status as commercial truth.
- [x] Profile-B active evidence cannot authorize before commercial `effective_from`; not-before uses earliest plausible current time and fails closed while any plausible trusted time remains before start.
- [x] Lifecycle and authority revision ordering plus durable high-water fencing prevent delayed, replayed, restarted or rolled-back stale `active` evidence from resurrecting a newer expiry/revocation, older lease or earlier commercial start.
- [x] New admission/reconnect is distinguished from already-running-session behavior; session-disconnect policy may remain deferred but entitlement benefit cannot exist outside the conservatively proven commercial/authority interval.
- [x] Clock/skew semantics require all known uncertainty to narrow usable authority: earliest plausible time gates `effective_from`, latest plausible time gates end/cutoff.
- [x] Unsafe trusted-time state cannot be cleared by fresh entitlement state alone; benefit resumes only after safe time is restored or an equivalently bounded authority-time/monotonic anchor is established by the authority exchange.
- [x] Validation requirements cover pre-issued active evidence, uncertainty straddling commercial start, outage before/after lease expiry, delayed stale active after revoke, expiry during outage, reconnect, restart, out-of-order revisions, rollback and clock/time recovery uncertainty.
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
  rationale: A future Profile-B paid entitlement must not start early or survive an unbounded Platform authority outage merely because an active representation exists.
  self_review:
    result: PASS
    exact_head: 70de53206859c5464cd74743ecac5f5157afa49c
    evidence:
      - Whole-diff semantic review against main@7d4f5c88e6f0e67fd8f74bf82b45d7deec0ff654 confirms only the Issue-authorized entitlement contract plus this task packet are changed.
      - Profile-B active evidence has mandatory authority-issued effective_from/effective_until and finite authority_valid_until plus lifecycle/authority_revision monotonic ordering.
      - Every Profile-B product/version must explicitly declare finite max_authority_lease, refresh_before and max_clock_skew plus stale/expired behavior before activation.
      - State precedence is deterministic: known REVOKED first, then known commercial/authority-cutoff EXPIRED, then NOT_YET_EFFECTIVE when commercial start is not conservatively reached; only then current/stale/unavailable states are evaluated.
      - Not-before uses earliest plausible current time so bounded uncertainty can only delay commercial activation; not-after uses latest plausible current time so bounded uncertainty can only shorten authority.
      - If trusted-time uncertainty exceeds max_clock_skew, fresh entitlement state alone remains non-authorizing; benefit may resume only after trusted time becomes safe or the accepted authority exchange establishes an equivalently bounded finite authority-time/monotonic anchor tied to that exchange/revision.
      - The recovery anchor cannot use message receipt as a new commercial start or lease origin and must preserve authority-issued boundaries.
      - Durable lifecycle/authority high-water fencing covers delayed delivery, cache replay, reconnect, restart, transport reordering and projection/storage rollback without moving start earlier or cutoff later.
      - Running-session continuity does not imply entitlement continuity; Profile-B benefit exists only inside the conservatively proven commercial/authority interval.
      - Rollback cannot lower revision fences, restore older authority, move commercial start earlier or restart an expired lease; incompatible rollback keeps Profile B disabled.
      - No Premium/VIP activation, runtime/schema/test/workflow/deployment/production or external-repository mutation is authorized or claimed.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:19:00+02:00
head: 70de53206859c5464cd74743ecac5f5157afa49c
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
  - Semantic head 70de53206859c5464cd74743ecac5f5157afa49c defines finite evidence, conservative commercial not-before/not-after evaluation, lifecycle/authority ordering, explicit product policy, deterministic state precedence, trusted-time recovery fencing, durable revision fencing, admission/session behavior, outage behavior, rollback gates and validation matrix.
  - Codex reviews 4900827318, 4900905868 and 4900961038 identified conservative-clock/state-precedence/exact-review, unsafe-time recovery and missing effective_from enforcement respectively; all semantic findings are addressed in the semantic head.
  - Pre-issued active evidence is now non-authorizing until earliest plausible trusted current time reaches effective_from; uncertainty that straddles start remains fail closed as NOT_YET_EFFECTIVE.
  - Fresh authority representation cannot restore Profile-B benefit while trusted-time uncertainty remains unsafe unless the authority exchange also establishes an equivalently bounded finite time/monotonic anchor supporting conservative start/end/cutoff evaluation.
  - The diff from protected main contains only docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md and this task packet; no forbidden path changed.
  - Numeric product lease/refresh/skew values and the exact trusted-time recovery mechanism remain intentionally unknown but are mandatory safe activation/implementation inputs rather than implicit defaults.
derived:
  - The finite lease bounds unseen revocation/expiry during partition, while conservative not-before prevents pre-issued authority from starting a paid benefit early.
  - Conservative time evaluation narrows the usable interval from both directions: lower-bound time proves start, upper-bound time proves remaining authority.
  - Durable revision fencing separately prevents resurrection after a newer lifecycle/authority representation has become known.
  - Separating NOT_YET_EFFECTIVE, EXPIRED and AUTHORITY_UNAVAILABLE yields machine-testable enforcement without treating transport state as commercial truth.
  - Forced disconnect can remain an owning runtime/session-policy decision because entitlement benefit itself is independently constrained to the finite commercial/authority interval.
unknown:
  - exact future Premium/VIP benefits
  - exact per-product numeric lease duration, refresh lead and permitted clock-skew values
  - exact future transport/IDL/storage implementation
  - exact trusted-time/authority-time/monotonic recovery mechanism satisfying the bounded-anchor contract
conflicts: []
first_failure:
  marker: profile-b-active-authority-has-no-finite-cutoff-or-conservative-start
  evidence: Resolved at semantic head 70de53206859c5464cd74743ecac5f5157afa49c by mandatory finite authority_valid_until, conservative effective_from enforcement, activation-blocking finite product policy and safe authority-time recovery requirements.
rejected_hypotheses:
  - Lifecycle revision alone bounds an unseen future revocation during an authority outage; it only orders revisions after the newer revision becomes observable.
  - Commercial effective_until alone is sufficient for all outage cases; revocation can occur before commercial expiry, so a separately bounded refresh/authority lease is required.
  - Transport failure itself means entitlement revoked; commercial truth remains Platform-owned and unavailable authority is distinct from revocation.
  - Bounded clock skew is harmless for lease expiry; a slow clock inside the bound could over-authorize unless uncertainty shortens the usable deadline.
  - Fresh entitlement state alone can recover authorization while local trusted time is unsafe; disproven because absolute boundaries cannot be conservatively evaluated without safe time or an equivalently bounded authority-time anchor.
  - An active representation can authorize as soon as received; disproven because pre-issued evidence before effective_from would enable paid benefit early unless the conservative not-before boundary is enforced.
  - Deferring forced-disconnect mechanics permits continued commercial benefit outside the allowed interval; the repaired contract explicitly forbids that interpretation.
changed_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260809-entitlement-profile-b-stale-authority-lease.md
validation:
  - command: whole-diff architectural/security self-review against Issue #944 and Codex reviews 4900827318 / 4900905868 / 4900961038
    result: PASS
    evidence: Exact semantic head 70de53206859c5464cd74743ecac5f5157afa49c resolves all current material semantic findings without widening into forbidden runtime or rollout scope.
  - command: previous exact-package Agent Governance/CI on f0a8e3e2d3f7a1095d2613389608ecd9addebf98
    result: NOT_RUN
    evidence: Prior package results are intentionally not used as final evidence after the effective_from semantic fix moved PR head; that package also exposed an invalid local checkpoint enum which this checkpoint corrects.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Contract-only documentation repair with no executable behavior, route, schema, test harness or deployment mutation.
blockers: []
next_action: Validate the resulting task-only final package head on PR #968, record an exact-final whole-diff self-review without mutating that tree, obtain fresh Agent Governance/repository CI/Codex clearance on that exact head, then merge and archive/release ownership.
```
