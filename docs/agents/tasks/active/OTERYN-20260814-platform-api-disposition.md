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
- [x] PR #1044 P2 review finding about stale PlatformAPI decision references is remediated and its thread is resolved.
- [x] Concurrent PlayerCompanion merge #1028 is preserved through current-main three-way synchronization.
- [x] The post-merge orphaned #1028 active task that made repository-wide Governance fail is terminally moved active → archive in this package, and PlayerCompanion foundation work allocation is reconciled from `IN_PROGRESS` to `DONE`.
- [x] Issue #490 already contains the decision-ready question; terminal PlatformAPI disposition will be recorded after the accepted package merges.
- [ ] Exact final-head documentation/governance CI and PR hygiene pass after the liveness repair.
- [ ] Accepted package is merged, PlatformAPI slice is recorded terminal in Issue #490, task is archived and programme returns to ready rotation.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this package changes architecture/governance only and intentionally creates no general API endpoint.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/agents/tasks/archive/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
modules:
  - PlatformAPI
  - PlayerCompanion.SessionAnalysis lifecycle repair only
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
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: cohesive Platform architecture decision plus a bounded current-main governance blocker repair caused by a concurrently merged terminal task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T11:18:00+02:00
head: ca6eec1fa915f931811a371d72b19c0e9dc4f2bf
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
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/agents/tasks/archive/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
proven:
  - Trusted main at task start was e0d9f28abad3a30c547d53f40cccf4ea713cf197.
  - `routes/api.php` exposes only POST `/v1/game-auth/tickets`; `routes/internal.php` exposes private Gateway game-auth contracts; neither is a general PlatformAPI surface.
  - Repository owner selected ARCH-DEC-0005 Option A on 2026-08-14.
  - ADR 0036 is accepted with Option A and `PLATFORM_API_ARCHITECTURE.md` defines a fail-closed named-consumer activation checklist.
  - ARCH-DEC-0005 is removed from the active decision backlog; no speculative PlatformAPI implementation handoff exists.
  - Portal work allocation marks Platform API DEFERRED.
  - `MODULE_CATALOG.md` separates PLANNED implementation availability from ADR 0036 DEFERRED product disposition and records PlatformAPI as terminal for Issue #490.
  - `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` leaves only PublicEdge/direct-production evidence as residual Issue #490 concerns.
  - Repository-configured automated Codex review on older head f087717af1ab256d7580530b329568188ca1b7c4 produced one P2 stale-reference finding; it is repaired and the thread is resolved/outdated. No explicit Codex invocation was requested by the agent.
  - PR #1028 advanced main to dfd7acc29f16252a8d83d9de398f915875d36aab; GitHub's signed three-way synchronization preserved both #1028 PlayerCompanion state and this PlatformAPI package.
  - Exact final #1028 head de8742d1062ddbbfda263c4d3c3975bd11e16b36 emitted 24 workflows and all 24 completed SUCCESS; its three material review threads are resolved; PR #1028 merged as dfd7acc29f16252a8d83d9de398f915875d36aab.
  - Fresh #1044 Governance run 31786345949 on synchronized head 7501b8c574105079d0089ca57c1827b33de44824 passed all repository policy/checkpoint tests but failed live task liveness because merged PR #1028 still had an active task with stale next action `merge` and no archive-pending transition.
  - No independent open lifecycle PR owned the #1028 task, so this package performs the bounded blocker repair: active task removed, terminal archive added, and the already-owned portal work-allocation row changed PlayerCompanion foundation IN_PROGRESS → DONE.
  - The PlayerCompanion repair changes no implementation/runtime/schema/workflow behavior and merely records already-proven terminal evidence.
derived:
  - Explicit PlatformAPI deferral is the smallest secure disposition: it closes audit ambiguity without creating an unused compatibility or attack surface.
  - The #1028 lifecycle repair is required current-main governance reconciliation, not scope expansion into PlayerCompanion implementation.
unknown:
  - Future named API consumer, exact resource inventory, scopes/token lifecycle, rate budgets and compatibility windows; intentionally deferred until an activation trigger exists.
conflicts: []
first_failure:
  marker: current-main-terminal-task-liveness-after-concurrent-merge
  evidence: Agent Governance 31786345949 failed only after all static checkpoint/policy tests passed because merged PR #1028 remained represented by a stale active task; exact errors were `terminal_pr_stale_next_action` and `terminal_pr_active_task`.
rejected_hypotheses:
  - Existing game-auth `/api/v1` proves a general Platform API already exists.
  - Internal Gateway routes may be reclassified as general PlatformAPI to close Issue #490.
  - Passport availability authorizes a blanket general authenticated API.
  - Option A requires changing module implementation status from PLANNED; implementation availability and product disposition are separate dimensions.
  - Pre-sync exact-head CI may be used after main advances; current-main synchronization requires a fresh generation.
  - The #1028 liveness failure may be ignored as unrelated; repository-wide Governance correctly blocks all PRs until terminal active ownership is reconciled.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/tasks/active/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/agents/tasks/archive/OTERYN-20260813-player-companion-session-analyzer-v1.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OPERATIONS_OBSERVABILITY_ARCHITECTURE.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
validation:
  - command: source/ownership and architecture reconciliation
    result: PASS
    evidence: Option A accepted; focused authority/routing/module/operations/work-allocation state reconciled
  - command: PlatformAPI review remediation
    result: PASS
    evidence: sole P2 thread resolved/outdated; no REQUEST_CHANGES review remains
  - command: concurrent main synchronization
    result: PASS
    evidence: exact three-way merge preserved #1028 PlayerCompanion and #1044 PlatformAPI state
  - command: PlayerCompanion terminal evidence
    result: PASS
    evidence: exact final #1028 head has 24/24 emitted workflows success and all three review threads resolved before merge dfd7acc29f16252a8d83d9de398f915875d36aab
  - command: runtime/browser E2E for this architecture/lifecycle delta
    result: NOT_APPLICABLE
    evidence: no executable behavior changes; PlayerCompanion implementation E2E was already proven on its exact delivery head
  - command: exact-final-head CI
    result: NOT_RUN
    evidence: fresh generation required after bounded terminal-task liveness repair and checkpoint update
blockers: []
next_action: Validate the exact new PR #1044 head with all emitted workflows, current-base mergeability and zero unresolved material reviews; squash-merge, record PlatformAPI terminal in Issue #490, then archive this task and return the architecture programme to ready rotation.
```

## Notes

No explicit owner-funded Codex/OpenAI/API invocation was authorized or requested by the agent. A repository-configured automated Codex review triggered earlier when PR #1044 was marked ready; it reviewed an older head, produced the remediated P2 finding and no further Codex action is requested. No protected-environment operation, production secret, live-system mutation or external-repository access is used by this task.
