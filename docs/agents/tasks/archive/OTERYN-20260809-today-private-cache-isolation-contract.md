---
task_id: OTERYN-20260809-today-private-cache-isolation-contract
mode: implementation
issue: 941
status: completed
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260809-today-private-cache-isolation-contract

## Goal

Repair Issue #941 by making owner-private `Today` / command-centre representation isolation explicit and machine-testable at the architecture boundary, without claiming a current runtime leak or authorizing Today runtime, cache middleware, CDN, route, schema, deployment, production or external-repository work.

## Completion

- [x] PR #970 delivered the architecture/security repair and was squash-merged to protected `main` as `c5229194c56198421d13333901cc8953723603a6`.
- [x] Issue #941 auto-closed from the merge.
- [x] Exact-final whole-diff self-review passed on `5b4bddca7eaed66d9e38e6fd5fa2ac66deb775f0` and is persisted in PR comment `5246343693` without mutating the reviewed tree.
- [x] Exact-final Agent Governance and required CI passed.
- [x] Phase 7, Edge Security, Platform DB Outage, Game Auth Ticket Concurrency and Native protocol contract/audit validation passed on the same final package head.
- [x] No unresolved material review thread remained before merge.
- [x] The repair claim and `public-portal:today-private-cache-isolation` coordination ownership were released on Issue #941 by comment `5246375438`.
- [x] The deterministic repair branch `repair/issue-941` was absent after merge.
- [x] No Today runtime, cache middleware/CDN, route, schema, tests, deployment, credential, production or external-repository mutation occurred.

## Delivered contract

- Any materialized Today representation containing or influenced by owner-private PlayerCompanion state is `PRIVATE_PERSONALIZED`, even when mixed with public cards.
- Private/mixed representations are prohibited from shared/public page caches, anonymous fragment caches, CDN caches and shared reverse-proxy caches.
- Guest and authenticated/private variants cannot alias merely through identical route/query/world/profile/public-card dimensions or generic authentication metadata.
- Any future private server-side representation cache must bind the authenticated owner plus the applicable session/authentication, authorization/privacy, account/character ownership, private PlayerCompanion state, applicability and representation/schema revision fences.
- Logout, session replacement, ownership change, authorization/privacy tightening, private-state deletion/change and applicability/schema changes fence old personalized materializations before reuse.
- Public sub-fragments may be cached independently only when private inputs cannot influence bytes, inclusion, ordering, counts, semantics, cache identity or eligibility; the final mixed response remains private.
- Later Today implementation must prove two-user isolation, auth↔guest transitions, logout/session replacement, ownership transitions, stale private-fragment fencing, privacy/authorization tightening and CDN/proxy negative paths.
- No current runtime confidentiality leak is claimed; this repair closes the architecture boundary before implementation.

## Lifecycle closeout

```yaml
lifecycle_closeout:
  implementation_pr: 970
  implementation_merge_sha: c5229194c56198421d13333901cc8953723603a6
  semantic_implementation_head: 0488a4e03da0cae9b75821df04768ac7e5542d43
  final_package_head: 5b4bddca7eaed66d9e38e6fd5fa2ac66deb775f0
  exact_final_self_review_comment: 5246343693
  issue: 941
  issue_state: closed
  claim_release_comment: 5246375438
  repair_branch_present_after_merge: false
  batching:
    applied: false
    reason: Remaining active records are genuine blocked external/protected verification work rather than compatible terminal lifecycle items; delaying this completed P1 security-contract closeout merely to reach batch size two would violate the anti-stall rule.
  runtime_or_product_change_in_closeout: false
```

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  result: PASS
  final_package_head: 5b4bddca7eaed66d9e38e6fd5fa2ac66deb775f0
  evidence:
    - Agent Governance run 31434810835 passed.
    - CI run 31434810825 passed checkpoint/routing and the required test gate; runtime-tests were correctly skipped for docs-only changes.
    - Phase 7 Production-Like Validation run 31434810811 passed.
    - Edge Security Emulation run 31434810808 passed.
    - Platform DB Outage Validation run 31434810812 passed.
    - Game Auth Ticket Concurrency run 31434810816 passed.
    - Native protocol contract run 31434810836 passed.
    - Native protocol contract audits run 31434810869 passed.
    - Exact-final whole-diff self-review comment 5246343693 records PASS on the same final package head.
    - No unresolved material review thread existed at merge.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:44:00+02:00
head: c5229194c56198421d13333901cc8953723603a6
branch: none
pr: 970
status: completed
context_routes:
  - agent-governance
  - architecture
  - security
  - public-web
  - player-companion
  - privacy
  - cache-cdn
owned_paths: []
proven:
  - Protected main resolved to implementation merge c5229194c56198421d13333901cc8953723603a6 after PR #970 merged.
  - Issue #941 is closed and its implementation/coordination claim is explicitly released by Issue comment 5246375438.
  - The repair branch repair/issue-941 is no longer present.
  - PR #970 changed exactly the three Issue-authorized architecture documents plus the task record; no Issue #941 forbidden path changed.
  - Exact-final package head 5b4bddca7eaed66d9e38e6fd5fa2ac66deb775f0 passed required CI, governance and heightened validation with zero unresolved material review threads before merge.
  - The remaining active records are blocked on external/protected evidence and are not compatible terminal lifecycle batch items.
derived:
  - Issue #941 has no remaining implementation, review, branch, lease or coordination ownership.
  - This archival move is lifecycle metadata only and carries no executable behavior.
unknown: []
conflicts: []
first_failure:
  marker: owner-private-today-representation-has-no-cache-isolation-contract
  evidence: resolved by PR #970 through privacy propagation, shared-cache prohibition, owner/security revision fencing, transition invalidation and explicit negative-path validation requirements.
rejected_hypotheses:
  - Composition-time authorization alone is sufficient after a private representation is cached.
  - Identical public cards make guest and authenticated/private Today variants share-cacheable.
  - Public source facts declassify owner-private tracking intent or derived state.
  - Generic auth bits, roles or route/query/world/profile keys are sufficient private cache identity.
  - Public-fragment cacheability can propagate to the final mixed private response.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260809-today-private-cache-isolation-contract.md
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-isolation-contract.md
validation:
  - command: implementation PR #970 exact-head validation and merge verification
    result: PASS
    evidence: required CI and heightened validation passed before squash merge c5229194c56198421d13333901cc8953723603a6.
  - command: post-merge Issue/branch/claim reconciliation
    result: PASS
    evidence: Issue #941 closed, claim release comment 5246375438 persisted and repair/issue-941 is absent.
  - command: lifecycle-only runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this terminal move changes only task lifecycle metadata and cannot alter executable behavior.
blockers: []
next_action: Re-evaluate the live OTERYN_PORTAL_COMPLETION queue from current protected main and select the next safe implementation-authorized item, if any.
```
