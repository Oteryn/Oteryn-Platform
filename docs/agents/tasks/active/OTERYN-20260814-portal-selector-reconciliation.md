---
task_id: OTERYN-20260814-portal-selector-reconciliation
mode: architecture
issue: 1057
status: validating
programme: OTERYN_PORTAL_COMPLETION
project_lane: oteryn-platform-core
phase: validate
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
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

- [x] Historical remediation Issues #948/#944/#941 are classified from live state and removed from current-queue wording.
- [x] Canonical selector states are exactly `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY`; mixed entries classify exact candidates first and use deterministic roll-up that cannot hide a READY sibling.
- [x] Work Allocation `ARCHITECTURE_READY` is explicitly non-promotional; canonical `READY` is derived only from live eligibility.
- [x] The canonical execution prompt delegates ordering to `OTERYN_PORTAL_COMPLETION.md` and contains no stale second queue.
- [x] LiveOps architecture #1046 is terminal, `MODULE_CATALOG.md` remains truthful, WorldStatus/Maintenance runtime promotion requires exact producer evidence, and ServerSave remains unavailable until its source is proven.
- [x] Client Distribution / Issue #1039 is explicitly reachable from canonical selection order.
- [x] `ACTIVE_WORK.md` / `PROJECT_STATE.md` cannot override current selector state when their routing snapshot is stale.
- [x] A compact selection proof records selected base SHA, skipped entries, candidate evidence, authority/ownership and overlap; final PR/CI state is completed at closeout.
- [ ] Documentation/link/governance validation and exact-head required CI pass on the final repaired head.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task changes documentation/governance only and introduces no executable route, API, persistence or frontend behavior.
- [ ] PR merges, Issue closes completed, task archives, ownership releases and selector is rerun on new protected `main`.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
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
updated_at: 2026-08-14T13:47:00Z
head: b3d7acea0cfe03e4eb821a8535785e6e4fb71ddf
material_head: b3d7acea0cfe03e4eb821a8535785e6e4fb71ddf
branch: docs/issue-1057-portal-selector-reconcile
pr: 1058
status: validating
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/reports/OTERYN-20260814-portal-completion-selection-proof.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
proven:
  - Protected main at selection preflight was 166561fe066b12310fb534172542e60b51484c46 and remained unchanged through the review-repair checkpoint.
  - Issue #1046 is closed completed; PR #1047 merged its focused LiveOps architecture and PR #1048 merged its archive closeout.
  - Historical Issues #948, #944 and #941 are closed completed and no live `risk:high` Issue was returned by the selector query.
  - Shared audit Issue #486 still contains HIGH findings, but its concrete character/achievement owners #317/#319/#323 are blocked; optional badge/status #325 remains triage.
  - Issues #317 and #319 are open `state:blocked` on accepted native Character Authority command/result semantics; #320 is also blocked and requires an explicit product decision.
  - Issue #1039 is open with `agent:ready`, accepted ADR 0035 dependency and a Platform-only implementation boundary.
  - PublicPortal Today Issue #1049 is owned by active PR #1055; its changed paths do not overlap this task.
  - PR #1056 owns branch-lifecycle workflow/governance paths and does not overlap this task.
  - Wiki audit #488 is closed completed; PR #338 retains the active/stalled Game Catalog schema 1.3 consumer compatibility hold.
  - PlayerCompanion Session Analyzer v1 is terminal through merged PR #1028; no exact open Hunt Finder/Equipment Explorer/Build Planner/Quest-Access Issue was found.
  - Exact-head 48a931c5d89d4cf87f279171b54b7d16e2737932 passed CI run 31805351755 and Agent Governance run 31805351631 before review-driven selector/prompt repairs superseded that head.
  - PR review on the pre-repair diff identified two material routing defects: the canonical execution prompt duplicated a stale queue, and combined selection entries lacked deterministic candidate roll-up. Both are corrected in programme version 3 and prompt version 1.1.
derived:
  - Selector drift was the first unowned READY item because entry 1 was owned by PR #1055.
  - LiveOps runtime is not selector-READY until exact authoritative source evidence for delivered WorldStatus facts is proven; ServerSave is separately unavailable until its own source semantics are proven.
  - Candidate-first classification with READY-first roll-up is required so an owned/blocked sibling cannot make an independent ready candidate unreachable.
  - Federated search dependency cleanup and Client Distribution #1039 remain reachable later candidates subject to a fresh ownership/eligibility rerun.
unknown: []
conflicts: []
rejected_hypotheses:
  - A combined workstream entry can be assigned one selector state without first classifying all exact sibling candidates; rejected because it can hide an independent READY candidate behind OWNED/BLOCKED state.
  - The canonical execution prompt can safely duplicate the programme queue; rejected because prompt 1.0 retained terminal repair examples and omitted Client Distribution #1039.
first_failure:
  marker: checkpoint-contract-missing-rejected-hypotheses
  evidence: Exact-head CI classify-changes job 94782179072 failed only at Validate active task checkpoint contract because the new task checkpoint omitted required field rejected_hypotheses; the field is now present and passed on superseded head 48a931c5d89d4cf87f279171b54b7d16e2737932.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-portal-selector-reconciliation.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/reports/OTERYN-20260814-portal-completion-selection-proof.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
validation:
  - command: live selector preflight and overlap reconciliation
    result: PASS
    evidence: current protected main, active tasks, Issues and PR changed-path ownership reconciled without external-repository access
  - command: runtime/browser E2E applicability review
    result: NOT_APPLICABLE
    evidence: documentation/governance-only diff; no route, API, persistence, frontend or runtime behavior is changed
  - command: exact-head CI classify-changes / active task checkpoint contract on c84ab815dee97a50f7991e626492d66a283b7064
    result: FAIL
    evidence: job 94782179072 failed at checkpoint validation because rejected_hypotheses was missing; deterministic correction applied
  - command: exact-head CI and Agent Governance on 48a931c5d89d4cf87f279171b54b7d16e2737932
    result: PASS
    evidence: CI run 31805351755 and Agent Governance run 31805351631 passed; review findings then required programme/prompt repairs, so this is superseded rather than final-head evidence
  - command: Agent Governance on b3d7acea0cfe03e4eb821a8535785e6e4fb71ddf
    result: FAIL
    evidence: run 31806121359 job 94785347388 failed only at Validate active task checkpoints because PASS_SUPERSEDED is not an allowed validation result; deterministic vocabulary correction is applied in this commit
blockers: []
next_action: Run exact-final-head CI and Agent Governance on the corrected checkpoint, re-review the complete eight-file diff, verify review threads remain resolved, then merge/archive only if terminal.
```

## Notes

No external/server repository was accessed. No production/protected environment, credential, Cloudflare, signer, payment or owner-funded AI invocation was performed by this task.
