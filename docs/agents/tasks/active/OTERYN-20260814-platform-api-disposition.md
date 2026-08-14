---
task_id: OTERYN-20260814-platform-api-disposition
mode: architecture
issue: 490
status: investigating
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
- [ ] A Proposed ADR records the concrete options, trade-offs, recommendation and activation invariants.
- [ ] The machine-readable architecture decision backlog records exactly one owner decision obligation with `implementation_authorized=false`.
- [ ] ADR inventory and architecture programme projection remain consistent.
- [ ] Issue #490 receives one decision-ready owner question without claiming acceptance.
- [ ] Exact-head documentation/governance CI and PR hygiene pass for the decision-ready package.
- [ ] After explicit owner selection, accepted authority is reconciled or explicit deferral/rejection is recorded, then the task is merged/archived and ownership released.
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
  - none while analysis is being prepared
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
updated_at: 2026-08-14T09:35:00+02:00
head: UNKNOWN
branch: docs/OTERYN-20260814-platform-api-disposition
pr: none
status: investigating
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
  - `MODULE_CATALOG.md` marks PlatformAPI `PLANNED`, says endpoints exist only for a concrete client/use case, and explicitly excludes bounded internal/game-auth routes from general PlatformAPI classification.
  - Portal work allocation marks Platform API `ARCHITECTURE_READY / DECISION_REQUIRED for first public surface`.
  - The active architecture decision backlog is empty before this task.
  - Current open PR #1028 does not own any path claimed by this architecture package; the other open PRs are unrelated operational/research/GameCatalog work.
derived:
  - The current repository has reusable API authentication infrastructure for the specialized game-auth endpoint, but that is not evidence that a general authenticated Platform API product is selected or safe to expose.
  - A speculative broad API would create a compatibility/security commitment before a named consumer exists, contrary to the current module boundary.
unknown:
  - Whether the repository owner wants the first general surface to be deferred, public read-only, or authenticated first-party account/client API.
  - Exact future resource inventory, token/scopes model, rate budgets and compatibility/deprecation windows depend on that owner selection and implementation design.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Existing game-auth `/api/v1` proves a general Platform API already exists.
  - Internal Gateway routes may be reclassified as public/first-party PlatformAPI merely to close Issue #490.
  - A generic continuation instruction authorizes choosing a material public API product surface.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
validation:
  - command: source/ownership inspection
    result: PASS
    evidence: exact `main` routes, module catalogue, work allocation, backlog, active tasks and open PRs inspected
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only decision preparation
blockers:
  - none
next_action: Create the Proposed ADR 0036 and ARCH-DEC-0005 decision-ready record, then persist the exact owner question in Issue #490 and the architecture programme.
```

## Notes

No Codex, owner-funded OpenAI/API token, protected environment, production secret, live-system mutation or external-repository access is authorized or used by this task.
