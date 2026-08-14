---
task_id: OTERYN-20260814-branch-lifecycle-closeout
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
search_first:
  - issue #658 and PR #666
  - issue #1050
  - latest Branch Lifecycle dry-run artifact
optional_reads: []
---

# OTERYN-20260814-branch-lifecycle-closeout

## Goal

Safely reconcile stale repository branches and extend branch lifecycle so terminal agent tasks do not leave unexplained source refs.

## Acceptance criteria

- [ ] Fresh current-main inventory classifies every branch fail-closed.
- [ ] Proven terminal deletion candidates are removed with exact-head evidence.
- [ ] Open PR, active task/claim, protected, recovery, rollback, release and retained refs survive.
- [ ] Closed-unmerged PR refs can be distinguished from ambiguous or modified orphan refs without deletion by age/prefix alone.
- [ ] Agent/task closeout requires explicit source-branch disposition before `completed`.
- [ ] Ordinary merged PR branches continue to use repository automatic deletion.
- [ ] Focused tests and exact-head required CI pass.
- [ ] One-time approval state is removed after cleanup and final inventory has zero policy-authorized pending deletions.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_TEMPLATE.md
modules:
  - repository-governance
  - branch-lifecycle
dependencies:
  - issue #1050
  - accepted ADR 0024
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T13:19:30Z
head: 166561fe066b12310fb534172542e60b51484c46
branch: repair/issue-1050
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_TEMPLATE.md
proven:
  - Protected main at task start is 166561fe066b12310fb534172542e60b51484c46.
  - Repository has delete_branch_on_merge enabled.
  - Existing policy only authorizes TERMINAL_MERGED deletion.
  - 2026-08-13 retained dry-run had 185 refs: 115 UNMERGED_ORPHAN, 55 UNKNOWN, 8 TERMINAL_MERGED, 6 OPEN_PR, 1 PROTECTED.
  - Issue 658 previously removed 354 proven terminal merged refs.
derived:
  - Current branch clutter is dominated by work that never reached the merged-PR automatic-delete path.
unknown:
  - Current exact classification of all refs after main advanced on 2026-08-14.
  - Which closed-unmerged refs have exact terminal PR-head identity and no live ownership.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not yet complete
blockers:
  - none
next_action: Open the task PR, then produce a fresh live branch inventory and implement the smallest fail-closed lifecycle extension required by issue 1050.
```

## Notes

Issue #1050 is the single remediation owner. No Codex or owner-funded AI use is authorized.
