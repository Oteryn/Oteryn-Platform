---
task_id: OTERYN-20260814-portal-selector-reconciliation
mode: architecture
issue: 1057
status: implementing
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: implement
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
search_first:
  - live main, active tasks, open PRs, open remediation/product Issues, ownership and overlap
---

# OTERYN-20260814-portal-selector-reconciliation

## Goal

Reconcile `OTERYN_PORTAL_COMPLETION` against protected `main` so canonical selection is deterministic, historical queue examples cannot become live truth, and currently eligible Platform-only work remains reachable without creating a second scheduler.

## Acceptance criteria

- [ ] Historical remediation Issues #948/#944/#941 are classified from live state and removed from current-queue wording.
- [ ] Canonical selector states are exactly `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY` and every skipped earlier entry has exact evidence.
- [ ] Work Allocation `ARCHITECTURE_READY` is explicitly non-promotional; canonical `READY` is derived only from live eligibility.
- [ ] LiveOps architecture #1046 is terminal, `MODULE_CATALOG.md` remains truthful, WorldStatus/Maintenance runtime promotion requires exact producer evidence, and ServerSave remains unavailable until its source is proven.
- [ ] Client Distribution / Issue #1039 is explicitly reachable from canonical selection order.
- [ ] `ACTIVE_WORK.md` / `PROJECT_STATE.md` cannot override current selector state when their routing snapshot is stale.
- [ ] A compact selection proof records selected base SHA, skipped entries, authority/ownership, overlap and final task/PR/CI state.
- [ ] Documentation/link/governance validation and exact-head required CI pass.
- [ ] Runtime/browser E2E is `NOT_APPLICABLE` because this task changes documentation/governance only and introduces no executable route, API, persistence or frontend behavior.
- [ ] PR merges, Issue closes completed, task archives, ownership releases and selector is rerun on new protected `main`.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/reports/OTERYN-20260814-portal-completion-selection-proof.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
modules:
  - portal completion control plane
dependencies: []
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T13:23:00Z
head: task-record-commit
material_head: task-record-commit
branch: docs/issue-1057-portal-selector-reconcile
pr: none
status: implementing
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/reports/OTERYN-20260814-portal-completion-selection-proof.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
proven:
  - Protected main at selection preflight was 166561fe066b12310fb534172542e60b51484c46.
  - Issue #1046 is closed completed; PR #1047 merged its focused LiveOps architecture and PR #1048 merged its archive closeout.
  - Issues #948, #944 and #941 are closed completed.
  - Live open high-risk remediation search returned no `risk:high` Issue.
  - Issues #317 and #319 are open `state:blocked` on accepted Oteryn-v2 Character Authority command/result semantics; #320 is also blocked and requires an explicit product decision.
  - Issue #1039 is open with `agent:ready`, accepted ADR 0035 dependency and Platform-only implementation scope.
  - PublicPortal Today Issue #1049 is already owned by active PR #1055; its changed paths are task record, Architecture Authority and focused Today architecture only, with no overlap with this task.
  - Open PR #338 remains a Game Catalog consumer compatibility hold; #1006/#988 are separate research and #1019/#1020 dependency PRs.
derived:
  - Canonical selection-order drift is the first unowned READY item because the earlier current-portal task is owned by PR #1055 and historical repair examples are terminal.
  - LiveOps runtime is not selector-READY until exact producer evidence for delivered runtime facts is proven; ServerSave is separately unavailable by canonical LiveOps architecture.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
validation:
  - command: live selector preflight
    result: PASS
    evidence: current protected main, tasks, Issues and PR ownership reconciled without external-repository access
blockers: []
next_action: Open the early draft PR, then reconcile the selector/allocation/delivery routing and persist the exact selection proof.
```

## Notes

No external/server repository was accessed. No production/protected environment, credential, Cloudflare, signer, payment or owner-funded AI operation is authorized or performed.
