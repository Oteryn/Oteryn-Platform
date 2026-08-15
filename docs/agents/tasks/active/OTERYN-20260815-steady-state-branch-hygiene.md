---
task_id: OTERYN-20260815-steady-state-branch-hygiene
issue: 1089
status: implementing
project_lane: oteryn-platform-core
phase: implement
execution_mode: github_connector
task_kind: implementation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
search_first:
  - Issue #1089 and terminal Issue #1072 evidence
  - current live branches, active tasks and open same-repository PRs
  - existing Historical Branch Audit implementation and tests
  - overlapping CI/governance PR ownership
optional_reads:
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
---

# OTERYN-20260815-steady-state-branch-hygiene

## Goal

Extend the existing post-#1072 repository governance so new unexplained remote branch debt is detected continuously without reintroducing a second cleanup programme or unnecessary CI load.

## Acceptance criteria

- [ ] Hard steady-state invariant is `NEW_UNEXPLAINED_BRANCHES = 0`; raw branch count remains informational and has no fixed cap.
- [ ] Protected `main` is the only ordinary long-lived branch; other ordinary remote refs require a live same-repository PR, active task claim, or managed-recovery contract.
- [ ] New active ordinary refs using top-level `tmp`, `backup`, `archive`, `recovery`, or `rollback` fail closed; the terminal #1072 registry is provenance, not a future exemption.
- [ ] Multiple open same-repository PRs on one branch or multiple active task claims on one branch fail closed.
- [ ] Human/agent naming `<type>/issue-<number>-<slug>` is advisory only; bot/system branches are exempt and naming never authorizes deletion.
- [ ] Repository setting drift from `main`, squash-only delivery and `delete_branch_on_merge=true` is reported deterministically and never auto-mutated.
- [ ] Existing Historical Branch Audit gains read-only trusted PR-lifecycle plus scheduled steady-state execution; destructive apply remains trusted-main push only.
- [ ] Applied historical registry no longer blocks safe validator/workflow maintenance merely because the original destructive implementation blob/count changed after deletion completed.
- [ ] Focused negative-path tests cover the new invariants and current live repository state passes.
- [ ] Exact-head CI, self-review, PR hygiene and terminal task/branch closeout are complete.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - .github/workflows/historical-branch-audit.yml
  - tools/agents/historical_work_reconciliation.py
  - tools/agents/test_historical_work_reconciliation.py
modules:
  - repository-governance
  - branch-lifecycle
  - historical-work-reconciliation
dependencies:
  - Issue #1072 terminal historical reconciliation
  - ADR 0037
  - ADR 0039
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T08:38:00Z
head: UNKNOWN
branch: repair/issue-1089-steady-state-branch-hygiene
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260815-steady-state-branch-hygiene.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - .github/workflows/historical-branch-audit.yml
  - tools/agents/historical_work_reconciliation.py
  - tools/agents/test_historical_work_reconciliation.py
proven:
  - Issue #1072 is terminal completed; 37 reviewed historical refs are absent and the final approval-free audit reports 11 fully accounted live refs with zero unexplained refs.
  - Repository settings currently use default main, squash merge enabled, merge/rebase merge disabled and delete_branch_on_merge=true.
  - Current Historical Branch Audit is path-triggered plus workflow_dispatch; it has no scheduled or trusted PR-lifecycle steady-state trigger.
  - Current live_state already detects ownerless unregistered refs but does not reject duplicate PR/task ownership, forbidden active namespaces, or report naming advisories/repository setting drift.
  - PR #1086 explicitly does not own .github/workflows/historical-branch-audit.yml.
derived:
  - The smallest correct implementation extends existing Historical Branch Audit and historical_work_reconciliation.py rather than creating another workflow/programme.
  - Destructive reviewed-implementation blob/count binding is only safety-critical while registry_phase is reviewed_for_deletion; after applied with no managed recovery it must not freeze future read-only maintenance.
unknown:
  - Exact final head after coherent implementation commit.
  - Exact required CI generation and review state after draft PR creation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - impose a fixed maximum raw branch count
  - delete branches by name or age
  - create a second branch-hygiene workflow/programme
  - auto-mutate repository merge settings from the audit
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-steady-state-branch-hygiene.md
validation:
  - command: live repository and overlap preflight
    result: PASS
    evidence: terminal #1072 evidence, 11 live refs, current repo settings and open PR ownership were read from GitHub before task creation
blockers:
  - none
next_action: Implement the steady-state validator, focused tests, ADR update and read-only scheduled/PR-lifecycle audit in one coherent change.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: github-20260815-0838
  session_started_at: 2026-08-15T08:33:00Z
  checkpointed_at: 2026-08-15T08:38:00Z
  last_progress_at: 2026-08-15T08:38:00Z
  phase: implement
  exact_head: UNKNOWN
  pull_request: none
  active_operation: none
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: dedicated task branch remains owned by Issue #1089
  next_action: Implement the steady-state branch hygiene change and open/update the draft PR.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1089 implementation is active on repair/issue-1089-steady-state-branch-hygiene
source_branch_evidence: pending
```

## Notes

No production, staging, external-repository, credential or owner-funded AI operation is in scope. Runtime/browser E2E is expected `NOT_APPLICABLE`; the real integration path is the read-only GitHub branch/PR/task inventory audit.