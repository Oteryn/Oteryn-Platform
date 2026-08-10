---
task_id: OTERYN-20260809-today-private-cache-isolation-contract
mode: implementation
issue: 941
branch: repair/issue-941
status: validating
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260809-today-private-cache-isolation-contract

## Goal

Repair Issue #941 by making owner-private `Today` / command-centre representation isolation explicit and machine-testable at the architecture boundary, without claiming a current runtime leak or authorizing Today runtime, cache middleware, CDN, route, schema, deployment, production or external-repository work.

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
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-isolation-contract.md
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
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: public-portal:today-private-cache-isolation
```

## Acceptance inventory

- [x] Any `Today` representation containing or influenced by owner-private PlayerCompanion routines, goals, tracking preferences or derived signals is explicitly `PRIVATE_PERSONALIZED` even when mixed with public cards.
- [x] Owner-private/mixed Today output cannot enter shared/public page, CDN/proxy or anonymous fragment caches; shared-cache bypass plus private/non-shareable semantics and `no-store` when no owner-scoped cache exists are defined as safe defaults.
- [x] Guest and authenticated/private variants cannot share a cache identity by route/query/world/profile alone; anonymous responses cannot inherit authenticated private fragments or private-influenced presentation.
- [x] Any future private server-side representation cache binds authenticated owner identity plus session/authentication, authorization, privacy, ownership, private-companion and applicability/schema revisions required to fence cross-user or stale-authority reuse.
- [x] Logout, session replacement, account/character ownership change, tracking/private-state deletion, applicability change and privacy/authorization tightening fence prior personalized representations before reuse.
- [x] Public sub-fragments may be cached independently only when private inputs cannot be captured in or influence bytes, inclusion, ordering, counts, semantic meaning, cache key or eligibility; the combined response remains private.
- [x] Negative-path validation covers two-user cross-cache isolation, auth↔guest transitions, logout/session replacement, ownership changes, stale private fragments, CDN/proxy simulation, privacy/authorization tightening and public+private fragment composition.
- [x] No current runtime confidentiality leak is claimed; Today remains architecture/planned only.
- [x] No runtime/schema/routes/tests/workflow/deployment/credential/production/external-repository mutation occurs.
- [ ] Exact-head Agent Governance and repository-selected CI pass; runtime/browser E2E is `NOT_APPLICABLE` for this architecture-only repair.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2329
  classified_at: 2026-08-10T23:29:00+02:00
  risk: high
  triggers:
    - owner-private personalized content
    - public/private composition on one route
    - reverse-proxy/CDN/shared response caches
    - authentication/session transitions
    - privacy and authorization tightening
    - durable public/private architecture boundary
  unknown_or_conflict: []
  rationale: Authorization can be correct at composition time and still leak owner-private data if the resulting mixed representation is later reused outside the owner-scoped cache boundary.
  self_review:
    result: PASS
    exact_head: 0488a4e03da0cae9b75821df04768ac7e5542d43
    evidence:
      - Whole-diff review against protected main 93f462101f73737ca8587fd7c6b053f5eb134372 contains exactly the three Issue-authorized architecture documents plus this task packet.
      - ADR 0032 is the controlling focused decision: privacy classification propagates to the complete materialized Today representation whenever owner-private input or influence exists.
      - PRIVATE_PERSONALIZED output is explicitly barred from shared/public page, anonymous fragment, CDN and reverse-proxy shared caches; a private/no-store or equally strong owner-scoped delivery policy is required.
      - Guest and authenticated variants cannot alias by route/query/world/profile, generic authenticated bit, role or public-card equality.
      - Future private server-side representation cache identity/fencing includes owner, session/authentication generation, authorization/privacy, ownership/private-state, applicability and representation revisions as applicable.
      - Logout/session replacement, ownership changes, auth/privacy tightening, private-state deletion/change and representation/applicability changes fence old private materializations.
      - Public sub-fragments remain independently cacheable only under composition isolation proving private inputs cannot affect bytes, inclusion, ordering, counts, semantics, key or eligibility; mixed response remains private.
      - PlayerCompanion preserves private representation semantics even when tracked source facts are public and defines required security-context revision handoff to PublicPortal.
      - PORTAL_COMPLETENESS_ARCHITECTURE makes the privacy/cache matrix a release-completion gate before personalized Today implementation can be called complete.
      - No Today runtime/cache middleware/CDN/route/schema/test/deployment/production behavior is introduced or claimed.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:39:00+02:00
head: d6f006d00df8984a699fac07eb0de68b0cf1b873
branch: repair/issue-941
pr: 970
status: validating
context_routes:
  - agent-governance
  - architecture
  - security
  - public-web
  - player-companion
  - privacy
  - cache-cdn
owned_paths:
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-isolation-contract.md
proven:
  - Issue #941 is open, P1/high risk, implementation-authorized, unblocked and explicitly architecture/documentation-only.
  - The deterministic branch was claimed from protected main 93f462101f73737ca8587fd7c6b053f5eb134372 with no competing #941 branch/PR at claim time.
  - PR #970 is the single authoritative delivery PR for the claimed branch repair/issue-941.
  - Before repair, ADR 0032/Portal/PlayerCompanion preserved private-source semantics but did not classify the materialized mixed response, prohibit shared cache reuse, define guest/auth cache identity separation, define owner-scoped private cache fencing or list transition invalidation requirements.
  - Semantic head 0488a4e03da0cae9b75821df04768ac7e5542d43 reconciles all three required architecture documents around one privacy propagation/cache-isolation contract.
  - ADR 0032 now classifies a mixed owner-private Today representation as PRIVATE_PERSONALIZED and defines shared-cache prohibition, owner/security revision private cache identity, transition fencing, public-subfragment isolation and an explicit negative-path matrix.
  - PlayerCompanion now makes the representation handoff private even when underlying tracked facts are public and requires enough owner/security/private-state revision semantics for safe future cache fencing.
  - PORTAL_COMPLETENESS_ARCHITECTURE now makes the same isolation rules and negative-path proof part of the Today delivery/completion gate.
  - The PR changes only the three Issue-authorized architecture paths plus this task packet; no forbidden path changed.
  - No current production/runtime leak is claimed; the finding remains a future implementation confidentiality boundary correction.
derived:
  - Privacy is taint-like for materialized composition: owner-private input or influence makes the complete mixed representation private unless fragments are proven isolated.
  - Cache isolation is an authorization continuation boundary because serving cached private bytes must prove equivalent principal/security context just as recomposition would.
  - Private state deletion or security tightening requires revision/fence invalidation; TTL expiry alone cannot be the sole confidentiality boundary.
  - Public-fragment cacheability must not propagate to a mixed private response.
unknown:
  - exact future Today route/view implementation
  - exact future CDN/reverse-proxy/cache middleware product
  - exact private-cache storage technology, headers and TTL values
conflicts: []
first_failure:
  marker: owner-private-today-representation-has-no-cache-isolation-contract
  evidence: Resolved at semantic head 0488a4e03da0cae9b75821df04768ac7e5542d43 by explicit privacy propagation, shared-cache prohibition, owner/security-revision private cache fencing, transition invalidation and negative-path validation requirements.
rejected_hypotheses:
  - Authorization at initial composition is sufficient even if the resulting personalized representation is later reused from cache; cache reuse can bypass the owner check.
  - Identical public cards make guest and authenticated page variants share-cacheable; authenticated variants may contain or be influenced by owner-private state even when public inputs match.
  - A public source fact makes the user's decision to track it public; tracking preferences, thresholds, routines, goals and private derived history remain owner-private.
  - A generic authenticated bit, role, route/query/world/profile key or Vary header alone proves private cache isolation; it does not bind one owner and all security-context transitions.
  - A public sub-fragment makes the combined response public-cacheable; privacy of the combined materialization is determined independently and remains private when owner-private input participates.
changed_paths:
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-isolation-contract.md
validation:
  - command: whole-diff architecture/security self-review against Issue #941 acceptance and negative paths
    result: PASS
    evidence: Exact semantic head 0488a4e03da0cae9b75821df04768ac7e5542d43 satisfies the documented privacy/cache contract without widening into forbidden runtime/cache/deployment scope.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Architecture/documentation-only repair; no executable route, cache middleware, response, schema, test harness or deployment behavior changes.
blockers: []
next_action: Validate the resulting task-only final package head on PR #970, record exact-final whole-diff self-review without mutating the tree, run Agent Governance and repository-selected CI, and repair any material review finding on the same PR before merge.
```
