---
task_id: OTERYN-20260814-platform-api-disposition
mode: architecture
issue: 490
status: blocked
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: investigate
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

Resolve the remaining PlatformAPI architecture/product-disposition gap from Issue #490 without misclassifying bounded game-auth/internal endpoints as a general API and without implementing a speculative public/authenticated surface before the repository owner chooses the first approved consumer posture.

## Acceptance criteria

- [x] Current API and internal route surfaces are classified from exact `main` source.
- [x] Open PR/task ownership is checked and no PlatformAPI architecture owner overlaps this package.
- [x] Proposed ADR 0036 records concrete options, trade-offs, recommendation, activation invariants and rejected shortcuts without claiming acceptance.
- [x] ARCH-DEC-0005 records exactly one owner decision obligation with `implementation_authorized=false`.
- [x] ADR inventory and architecture programme projection include the proposal/active decision without promoting it to accepted authority.
- [x] Issue #490 comment 5290781071 records one decision-ready owner question without claiming acceptance.
- [ ] Exact final-head documentation/governance CI and PR hygiene pass for the decision-ready package.
- [ ] Repository owner selects A, B or C; then accepted authority or explicit deferral is reconciled, the PlatformAPI slice of Issue #490 is terminally dispositioned, and task ownership is released.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` before a runtime surface is authorized because this task is architecture/governance only.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
modules:
  - PlatformAPI
dependencies:
  - Issue #490 shared audit owner
  - current module/service authority boundaries
  - repository-owner first-surface disposition
blockers:
  - Repository owner must select ARCH-DEC-0005 Option A, B or C.
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
decomposition_reason: one cohesive Platform-only product/API activation decision with no runtime implementation before acceptance
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T09:45:00+02:00
head: a30e655e350c29d81f553ab2d80c94765125f13c
branch: docs/OTERYN-20260814-platform-api-disposition
pr: 1044
status: blocked
context_routes:
  - architecture
  - security
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
proven:
  - Trusted main at task start is e0d9f28abad3a30c547d53f40cccf4ea713cf197.
  - `routes/api.php` exposes only POST `/v1/game-auth/tickets`, protected by Passport-backed `auth:api` and a dedicated throttle.
  - `routes/internal.php` exposes only private game-auth ticket redemption and login-context routes guarded by the Gateway service credential and dedicated throttles.
  - `MODULE_CATALOG.md` marks PlatformAPI `PLANNED`, requires a concrete client/use case, and excludes bounded internal/game-auth routes from general PlatformAPI classification.
  - Portal work allocation marks Platform API `ARCHITECTURE_READY / DECISION_REQUIRED for first public surface`.
  - The architecture decision backlog was empty before this package; ARCH-DEC-0005 is the only active record on this branch.
  - ADR README proves 0035 was the highest allocated prefix before this package and no open architecture PR claimed 0036; Proposed ADR 0036 is therefore the next valid allocation.
  - Open PR #1028 does not own any path claimed by this architecture package; the other open PRs are unrelated operational/research/GameCatalog work.
  - Draft PR #1044 contains the decision package and remains intentionally unmergeable by product policy until explicit owner selection.
  - Issue #490 comment 5290781071 asks for exactly A, B or C and recommends A without treating continuation as acceptance.
derived:
  - Existing Passport-backed API authentication is reusable implementation capability for specialized game-auth, not evidence that a general token/scopes/client product has been selected.
  - Option A best matches the current concrete-consumer-first architecture and minimizes attack/compatibility surface while preserving a machine-auditable future activation trigger.
unknown:
  - Repository-owner selection: A explicit deferral until a named consumer exists, B public read-only v1 first, or C authenticated first-party account/client v1 first.
  - Exact future resource inventory, token/scopes model, rate budgets and compatibility/deprecation windows depend on the selected posture and later implementation design.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Existing game-auth `/api/v1` proves a general Platform API already exists.
  - Internal Gateway routes may be reclassified as public/first-party PlatformAPI merely to close Issue #490.
  - Generic continuation authorizes a material public API product decision.
  - Passport availability authorizes a blanket general authenticated API.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
validation:
  - command: source/ownership inspection
    result: PASS
    evidence: exact main routes, auth guard, module catalogue, work allocation, backlog, active tasks and open PRs inspected
  - command: decision-package structural review
    result: PASS
    evidence: Proposed lifecycle only, unique ADR 0036 allocation, one decision_required ARCH-DEC-0005 record, implementation_authorized=false and exact programme projection
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only decision preparation
  - command: exact-final-head CI and PR hygiene
    result: NOT_RUN
    evidence: run after this durable blocked checkpoint is persisted
blockers:
  - Repository owner must select ARCH-DEC-0005 Option A, B or C.
next_action: Repository owner selects A (recommended explicit deferral), B (public read-only v1 first), or C (authenticated first-party account/client v1 first); then reconcile accepted canonical authority and close the PlatformAPI slice of Issue #490.
```

## Notes

No Codex, owner-funded OpenAI/API token, protected environment, production secret, live-system mutation or external-repository access is authorized or used by this task.
