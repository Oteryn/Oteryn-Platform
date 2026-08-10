---
task_id: OTERYN-20260809-today-private-cache-isolation-contract
mode: implementation
issue: 941
branch: repair/issue-941
status: implementing
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

- [ ] Any `Today` representation containing owner-private PlayerCompanion routines, goals, tracking preferences or derived signals is explicitly private/personalized even when mixed with public cards.
- [ ] Owner-private/mixed Today output cannot enter shared/public page, CDN/proxy or anonymous fragment caches; safe bypass/private/no-store or equivalently strong behavior is required.
- [ ] Guest and authenticated variants cannot share a cache identity by route/query/world/profile alone; anonymous responses cannot inherit authenticated private fragments.
- [ ] Any future private server-side representation cache binds authenticated owner identity plus authorization/privacy/applicability revisions required to fence cross-user or stale-authority reuse.
- [ ] Logout, session replacement, account/character ownership change, tracking deletion and privacy/authorization tightening invalidate/fence prior personalized representations.
- [ ] Public sub-fragments may be cached independently only when private inputs cannot be captured in or influence their cacheable representation/key/eligibility.
- [ ] Negative-path validation covers two-user cross-cache isolation, auth↔guest transitions, logout/session replacement, stale private fragment, CDN/proxy simulation and privacy/authorization tightening.
- [ ] No current runtime confidentiality leak is claimed; Today remains architecture/planned only.
- [ ] No runtime/schema/routes/tests/workflow/deployment/credential/production/external-repository mutation occurs.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass; runtime/browser E2E is `NOT_APPLICABLE` for this architecture-only repair.

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
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:29:00+02:00
head: 93f462101f73737ca8587fd7c6b053f5eb134372
branch: repair/issue-941
pr: none
status: implementing
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
  - Issue #941 is open, P1/high risk, implementation-authorized, agent-ready, unblocked and explicitly architecture/documentation-only.
  - No deterministic repair/issue-941 branch or open #941 PR existed immediately before claim; the branch was created from protected main 93f462101f73737ca8587fd7c6b053f5eb134372.
  - ADR 0032 allows PublicPortal Today to compose public sources with authenticated PlayerCompanion owner-private routines/goals/tracked signals and correctly preserves source privacy/freshness semantics.
  - PlayerCompanion defines tracking preferences, routines, goals and derived signal history as owner-private by default and says a later Today view may consume them without moving authority to PublicPortal.
  - PORTAL_COMPLETENESS_ARCHITECTURE says personalized Today cards are owner-private and omitted for guests, but the three current architecture records do not define post-composition cache isolation, guest/auth cache identity, private representation cache fencing or transition invalidation.
  - Issue #941 explicitly distinguishes this future confidentiality risk from Issue #938 federated-public-search publication/revocation semantics and does not claim a current production leak.
derived:
  - Privacy classification must propagate from any private input to the complete mixed representation unless public and private sub-fragments are proven compositionally isolated.
  - Shared-cache prevention must be defined independently from authorization checks because cache reuse can bypass recomposition and owner checks after an initially valid render.
  - A future owner-private server representation cache requires security-context identity/revision fencing rather than route/query/world/profile identity alone.
unknown:
  - exact future Today route/view implementation
  - exact future CDN/reverse-proxy/cache middleware product
  - exact private-cache storage technology and TTL values
conflicts: []
first_failure:
  marker: owner-private-today-representation-has-no-cache-isolation-contract
  evidence: Current architecture protects source privacy at composition but does not prohibit reuse of the resulting personalized representation through shared/public cache identities.
rejected_hypotheses:
  - Authorization at initial composition is sufficient even if the resulting personalized representation is later reused from cache; cache reuse can bypass the owner check.
  - Identical public cards make guest and authenticated page variants share-cacheable; authenticated variants may contain owner-private fragments even when public inputs match.
  - A public source fact makes the user's decision to track it public; tracking preferences and private derived history remain PlayerCompanion owner-private state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-isolation-contract.md
validation: []
blockers: []
next_action: Reconcile the three Issue-authorized architecture documents with a single private-representation classification/cache-isolation contract, transition fencing and negative-path validation matrix; then perform exact-head whole-diff review and repository validation.
```
