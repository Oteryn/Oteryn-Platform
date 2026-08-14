---
task_id: OTERYN-20260814-platform-api-disposition
mode: architecture
issue: 490
status: validating
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: validate
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
search_first:
  - Issue #490, open PR ownership, current API/internal routes and architecture decision backlog
---

# OTERYN-20260814-platform-api-disposition

## Goal

Resolve the remaining PlatformAPI architecture/product-disposition gap from Issue #490 without misclassifying bounded game-auth/internal endpoints as a general API and without implementing a speculative public/authenticated surface before an approved named consumer exists.

## Acceptance criteria

- [x] Current API and internal route surfaces are classified from exact `main` source.
- [x] Open PR/task ownership is checked and no PlatformAPI architecture owner overlaps this package.
- [x] ADR 0036 records concrete options, trade-offs, activation invariants and rejected shortcuts.
- [x] Repository owner selected Option A on 2026-08-14: explicitly defer general Platform API until a named consumer/use case exists.
- [x] ADR 0036 is `Accepted` and `PLATFORM_API_ARCHITECTURE.md` is the focused canonical owner.
- [x] ARCH-DEC-0005 is removed from the active architecture decision backlog and programme projection is empty.
- [x] `ARCHITECTURE_AUTHORITY.md` routes PlatformAPI to ADR 0036 / focused architecture.
- [x] Portal work allocation marks Platform API `DEFERRED` with no implementation handoff.
- [x] `MODULE_CATALOG.md` reconciles its stale Issue #490 decision-gap statement while retaining `PLANNED` strictly as implementation availability.
- [x] `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` removes PlatformAPI from Issue #490's unresolved residual list while preserving PublicEdge/direct-production evidence as open.
- [x] `PORTAL_COMPLETENESS_ARCHITECTURE.md` and `SECURITY_ARCHITECTURE.md` were rechecked and require no content change because their concrete-consumer/service-reuse/security statements already agree with Option A.
- [x] PR #1044 P2 review finding about stale PlatformAPI decision references is remediated in the exact canonical owner documents and ADR rationale; its thread is resolved.
- [x] Concurrent PlayerCompanion merge #1028 is preserved through an exact current-main three-way merge without losing either PlayerCompanion or PlatformAPI canonical state.
- [x] Issue #490 already contains the decision-ready question; terminal PlatformAPI disposition will be recorded after the accepted package merges.
- [ ] Exact final-head documentation/governance CI and PR hygiene pass on the synchronized head.
- [ ] Accepted package is merged, PlatformAPI slice is recorded terminal in Issue #490, task is archived and programme returns to ready rotation.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this package changes architecture/governance only and intentionally creates no general API endpoint.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
modules:
  - PlatformAPI
dependencies:
  - Issue #490 shared audit owner
  - current module/service authority boundaries
  - repository-owner Option A decision
blockers: []
cross_repository_tasks:
  - none
```

## Context pressure

```yaml
policy_version: 2
task_kind: architecture
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive Platform-only product/API activation decision with no runtime implementation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T11:02:00+02:00
head: 8024689319a8b133f3d446a1389af6157d338eab
material_head: 1f1d4824f854594641a6fbd05331c3d3e4f1aeb9
branch: docs/OTERYN-20260814-platform-api-disposition
pr: 1044
status: validating
context_routes:
  - architecture
  - security
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
proven:
  - Trusted main at task start was e0d9f28abad3a30c547d53f40cccf4ea713cf197.
  - `routes/api.php` exposes only POST `/v1/game-auth/tickets`, protected by Passport-backed `auth:api` and a dedicated throttle.
  - `routes/internal.php` exposes only private game-auth ticket redemption and login-context routes guarded by the Gateway service credential and dedicated throttles.
  - Existing specialized game-auth/internal endpoints are not a general PlatformAPI surface.
  - Repository owner explicitly selected ARCH-DEC-0005 Option A in the controlling conversation on 2026-08-14.
  - ADR 0036 is accepted with Option A and `PLATFORM_API_ARCHITECTURE.md` defines a fail-closed named-consumer activation checklist.
  - ARCH-DEC-0005 has been removed from the active decision backlog; the backlog serializes exactly to current main bytes and therefore is not a final PR changed path.
  - Portal work allocation marks Platform API DEFERRED and creates no speculative implementation handoff.
  - `MODULE_CATALOG.md` explicitly separates PLANNED implementation availability from ADR 0036 DEFERRED product disposition and records PlatformAPI as terminal for Issue #490.
  - `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` records OperationsObservability and PlatformAPI as terminal Issue #490 slices and leaves only PublicEdge/direct-production evidence residual.
  - `PORTAL_COMPLETENESS_ARCHITECTURE.md` already states that first-party API is justified by concrete consumers and must reuse module services/authorization/version/freshness semantics.
  - `SECURITY_ARCHITECTURE.md` already supplies the applicable generic API trust/rate/privacy/logging requirements; Option A creates no new runtime attack surface.
  - ADR README contains the unique 0036 inventory entry allocated by this PR.
  - Agent Governance run 31783580860 on pre-repair head 6d1a3df750f13c50c8e6332ebe6e0ff4e30b655a passed checkpoint-validator tests, liveness tests, Control Room tests, 95 policy-consistency tests and prompt-contract validation before failing the live checkpoint command only because `head` was absent and validation result `PENDING` was unsupported.
  - Exact head 688ff0b70d0bbdeaa607f2f80b4d311b9d535543 then passed Agent Governance run 31783716186 completely, including active checkpoint and live ownership validation.
  - Repository-configured automated Codex PR review was triggered by the earlier ready-for-review transition and reviewed older head f087717af1ab256d7580530b329568188ca1b7c4; the agent did not request `@codex review` or another explicit Codex invocation.
  - That automated review produced PR #1044 thread PRRT_kwDOTcsYjs6ZNajT with one P2 stale-reference finding; exact net patches for MODULE_CATALOG and OPERATIONS_OBSERVABILITY_ARCHITECTURE reconcile those references without unrelated file drift, and the thread is resolved/outdated.
  - Exact pre-sync head b2b6d2ab316c36c704f3a9de28ea49a22c434536 reached seven successful exact-head workflows before concurrent PR #1028 advanced main; those results are retained as historical evidence only and are not used as the final merge gate.
  - PR #1028 advanced main to dfd7acc29f16252a8d83d9de398f915875d36aab and overlapped this package only in MODULE_CATALOG.md.
  - GitHub computed signed three-way merge 8024689319a8b133f3d446a1389af6157d338eab with parents main@dfd7acc29f16252a8d83d9de398f915875d36aab and branch refresh 11e8760d93f559bc5c052774b6ca367d987aadde; the feature branch was fast-forwarded to this commit without force.
  - The synchronized MODULE_CATALOG tree preserves PlayerCompanion AVAILABLE plus Hunt Session Analyzer v1 from #1028 and the accepted PlatformAPI PLANNED-implementation/DEFERRED-product distinction and terminal #490 wording.
derived:
  - Explicit deferral is the smallest secure architecture disposition: it closes audit ambiguity without creating an unused compatibility and attack surface.
  - A future API package is implementation-authorized only after a named consumer trigger and a new bounded architecture/security package satisfy the accepted activation checklist.
unknown:
  - Future named API consumer, exact resource inventory, scopes/token lifecycle, rate budgets and compatibility windows; these are intentionally deferred until an activation trigger exists.
conflicts: []
first_failure:
  marker: architecture-decision-programme-projection-format
  evidence: Initial ARCH-DEC-0005 programme projection used a YAML block list; direct validator inspection proved only an inline JSON array is accepted, and the projection was repaired before acceptance. After decision resolution the projection is `[]`.
rejected_hypotheses:
  - Existing game-auth `/api/v1` proves a general Platform API already exists.
  - Internal Gateway routes may be reclassified as public/first-party PlatformAPI merely to close Issue #490.
  - Passport availability authorizes a blanket general authenticated API.
  - Option A requires changing module implementation status from PLANNED; module status and product/programme launch disposition are separate dimensions.
  - Option A requires new runtime security controls; no general endpoint is being activated.
  - The first accepted draft could leave stale canonical #490 references because higher-level authority would override them; canonical routing must instead be internally reconciled so future agents do not reopen terminal work.
  - Pre-sync exact-head CI may be used after main advances; current-main synchronization requires a fresh exact-head generation.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
validation:
  - command: source/ownership inspection
    result: PASS
    evidence: exact routes, auth guard, module catalogue, portal completeness, security architecture, work allocation, backlog, active tasks and open PRs inspected
  - command: accepted decision reconciliation
    result: PASS
    evidence: Option A recorded in accepted ADR 0036, focused authority, authority index, module lifecycle references, OperationsObservability residuals and portal work allocation; decision backlog empty
  - command: review-remediation net patch inspection
    result: PASS
    evidence: MODULE_CATALOG PlatformAPI patch and OPERATIONS_OBSERVABILITY_ARCHITECTURE Issue 490 wording contain no unrelated replacement drift
  - command: review hygiene
    result: PASS
    evidence: sole P2 review thread is resolved and outdated; review submission is COMMENTED rather than REQUEST_CHANGES
  - command: current-main synchronization
    result: PASS
    evidence: signed three-way merge 8024689319a8b133f3d446a1389af6157d338eab preserves exact main@dfd7acc plus the nine-path PlatformAPI package; merged MODULE_CATALOG was directly inspected for both PlayerCompanion and PlatformAPI state
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only explicit deferral; no executable behavior changes
  - command: exact-final-head CI
    result: NOT_RUN
    evidence: fresh exact-head generation is required after this synchronized checkpoint commit
blockers: []
next_action: Validate the new exact PR 1044 head with all exact-head workflows, current-base mergeability and zero unresolved material reviews; squash-merge, record the PlatformAPI slice terminal in Issue 490, then archive this task and return the architecture programme to ready rotation.
```

## Notes

No explicit owner-funded Codex/OpenAI/API invocation was authorized or requested by the agent. A repository-configured automated Codex review triggered when PR #1044 was marked ready before that behavior was observed; it reviewed an older head, produced the remediated P2 finding above and no further Codex action is being requested. No protected-environment operation, production secret, live-system mutation or external-repository access is used by this task.
