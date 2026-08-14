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
- [x] Closed-unmerged PR refs can be distinguished from ambiguous or modified orphan refs without deletion by age/prefix alone.
- [x] Agent/task closeout requires explicit source-branch disposition before `completed`.
- [x] Ordinary merged PR branches continue to use repository automatic deletion.
- [ ] Focused tests and exact-head required CI pass.
- [ ] One-time approval state is removed after cleanup and final inventory has zero policy-authorized pending deletions.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_cleanup.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
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
updated_at: 2026-08-14T13:26:30Z
head: ee927072569d1a600e81ad69c3b1f2d5b3c65f71
branch: repair/issue-1050
pr: 1056
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_cleanup.py
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_TEMPLATE.md
proven:
  - Protected main at task start is 166561fe066b12310fb534172542e60b51484c46.
  - Repository has delete_branch_on_merge enabled.
  - Existing policy only authorizes TERMINAL_MERGED deletion.
  - 2026-08-13 retained dry-run had 185 refs: 115 UNMERGED_ORPHAN, 55 UNKNOWN, 8 TERMINAL_MERGED, 6 OPEN_PR, 1 PROTECTED.
  - Issue 658 previously removed 354 proven terminal merged refs.
  - Existing exact-manifest cleanup is approved only for the eight previously reviewed TERMINAL_MERGED refs and will fail closed on live candidate drift.
  - New terminal classifier promotes only same-repository closed-unmerged PR branches whose terminal PR head SHA exactly equals the current ref, after the existing open-PR, active-task, repair-Issue, protection, retention and recovery-sensitive-name gates.
  - Future closed-unmerged automatic deletion requires explicit Branch-Disposition: delete plus a non-empty Branch-Disposition-Reason in the PR body; retain or unspecified disposition does not delete.
derived:
  - Current branch clutter is dominated by work that never reached the merged-PR automatic-delete path.
  - Historical closed-unmerged candidates still require one-time human/repository review before a deletion approval is persisted.
unknown:
  - Fresh current exact classification and terminal closed-unmerged candidate list from PR 1056 live inventory.
  - Whether the eight 2026-08-13 TERMINAL_MERGED candidates remain the exact current candidate set.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_cleanup.py
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TASK_TEMPLATE.md
validation:
  - command: PR 1056 exact-head GitHub Actions
    result: NOT_RUN
    evidence: exact-head runs have started and are not yet terminal at this checkpoint
blockers:
  - none
next_action: Inspect the first terminal PR 1056 workflow results and fresh branch-lifecycle artifacts, repair any failures, then review the exact historical terminal closed-unmerged deletion candidates before creating any one-time approval.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository remediation PR; repository delete_branch_on_merge is enabled
source_branch_evidence: pending merge and post-merge ref verification
```

## Notes

Issue #1050 is the single remediation owner. No Codex or owner-funded AI use is authorized.