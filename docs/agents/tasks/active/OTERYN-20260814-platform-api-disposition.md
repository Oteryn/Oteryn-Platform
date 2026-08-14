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
- [x] Existing `MODULE_CATALOG.md`, `PORTAL_COMPLETENESS_ARCHITECTURE.md` and `SECURITY_ARCHITECTURE.md` were rechecked and require no content change because their concrete-consumer/service-reuse/security statements already agree with Option A; module `PLANNED` is explicitly an implementation-availability label rather than launch disposition.
- [x] Issue #490 already contains the decision-ready question; terminal PlatformAPI disposition will be recorded after the accepted package merges.
- [ ] Exact final-head documentation/governance CI and PR hygiene pass.
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
updated_at: 2026-08-14T10:20:00+02:00
material_head: af3d355f126f8f31261c3e704fa3df44f4d16fd0
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
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
proven:
  - Trusted main at task start is e0d9f28abad3a30c547d53f40cccf4ea713cf197.
  - `routes/api.php` exposes only POST `/v1/game-auth/tickets`, protected by Passport-backed `auth:api` and a dedicated throttle.
  - `routes/internal.php` exposes only private game-auth ticket redemption and login-context routes guarded by the Gateway service credential and dedicated throttles.
  - Existing specialized game-auth/internal endpoints are not a general PlatformAPI surface.
  - Repository owner explicitly selected ARCH-DEC-0005 Option A in the controlling conversation on 2026-08-14.
  - ADR 0036 is accepted with Option A and `PLATFORM_API_ARCHITECTURE.md` defines a fail-closed named-consumer activation checklist.
  - ARCH-DEC-0005 has been removed from the active decision backlog; the backlog serializes exactly to current main bytes and therefore is not a final PR changed path.
  - Portal work allocation marks Platform API DEFERRED and creates no speculative implementation handoff.
  - `MODULE_CATALOG.md` already requires a concrete consumer and excludes specialized endpoints from general API completeness; its PLANNED status is implementation availability only.
  - `PORTAL_COMPLETENESS_ARCHITECTURE.md` already states that first-party API is justified by concrete consumers and must reuse module services/authorization/version/freshness semantics.
  - `SECURITY_ARCHITECTURE.md` already supplies the applicable generic API trust/rate/privacy/logging requirements; Option A creates no new runtime attack surface.
  - ADR README contains the unique 0036 inventory entry allocated by this PR.
  - Exact compare after accepted reconciliation reports `behind_by=0` and seven final changed paths.
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
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260814-platform-api-disposition.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PLATFORM_API_ARCHITECTURE.md
  - docs/architecture/adr/0036-platform-api-activation-and-first-surface-policy.md
  - docs/architecture/adr/README.md
validation:
  - command: source/ownership inspection
    result: PASS
    evidence: exact main routes, auth guard, module catalogue, portal completeness, security architecture, work allocation, backlog, active tasks and open PRs inspected
  - command: accepted decision reconciliation
    result: PASS
    evidence: Option A recorded in accepted ADR 0036, focused authority, authority index and portal work allocation; decision backlog empty
  - command: final diff inventory
    result: PASS
    evidence: compare against current main is behind_by=0 with exactly seven final documentation/governance paths; backlog reconciliation has zero net diff
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only explicit deferral; no executable behavior changes
  - command: exact-final-head CI and PR hygiene
    result: PENDING
    evidence: run on the unchanged accepted-decision head after this checkpoint commit
blockers: []
next_action: Validate exact final PR 1044 head and full diff/review hygiene, mark ready, squash-merge, record the PlatformAPI slice terminal in Issue 490, then archive this task and return the architecture programme to ready rotation.
```

## Notes

No Codex, owner-funded OpenAI/API token, protected environment, production secret, live-system mutation or external-repository access is authorized or used by this task.
