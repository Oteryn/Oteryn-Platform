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
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
search_first:
  - issue #658 and PR #666
  - issue #1050 and PR #1056
  - latest Branch Lifecycle and Terminal Branch Lifecycle dry-run artifacts
optional_reads: []
---

# OTERYN-20260814-branch-lifecycle-closeout

## Goal

Safely reconcile stale repository branches and extend branch lifecycle so terminal agent tasks do not leave unexplained source refs.

## Acceptance criteria

- [x] Fresh current-main inventory classifies every branch fail-closed.
- [ ] Proven terminal deletion candidates are removed with exact-head evidence.
- [ ] Open PR, active task/claim, protected, recovery, rollback, release and retained refs survive post-apply verification.
- [x] Closed-unmerged PR refs can be distinguished from ambiguous or modified orphan refs without deletion by age/prefix alone.
- [x] Agent/task closeout requires explicit source-branch disposition before `completed` and the rule is machine-enforced for completed active/newly archived task records.
- [x] Ordinary merged PR branches continue to use repository automatic deletion.
- [ ] Focused tests and exact-head required CI pass on the final material head.
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
  - tools/agents/terminal_branch_cleanup_legacy.py
  - tools/agents/terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_guarded.py
  - tools/agents/test_terminal_branch_approval.py
  - tools/agents/source_branch_closeout.py
  - tools/agents/test_source_branch_closeout.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - .github/workflows/agent-governance.yml
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/README.md
modules:
  - repository-governance
  - branch-lifecycle
dependencies:
  - issue #1050
  - accepted ADR 0024
  - accepted ADR 0037
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T15:08:00Z
head: 548280e1fab3f986f02db1dd8238ba9673984f33
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
  - tools/agents/terminal_branch_cleanup_legacy.py
  - tools/agents/terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_guarded.py
  - tools/agents/test_terminal_branch_approval.py
  - tools/agents/source_branch_closeout.py
  - tools/agents/test_source_branch_closeout.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - .github/workflows/agent-governance.yml
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/README.md
proven:
  - Protected main remains f61c7b229cbf251be6e5eae0be5db55aac722242 and PR #1056 is synchronized with that base.
  - Repository has delete_branch_on_merge enabled and protected main requires classify-changes and test.
  - Exact-head 1a06f3d3296e0912a6f426ee0b267cf2bce782dd passed CI 31807580870, Agent Governance 31807580829, Branch Lifecycle 31807580831, Terminal Branch Lifecycle 31807580891 and all other associated workflows returned by GitHub before Codex review.
  - The repository owner explicitly authorized one Codex review for PR #1056; review 4938107909 consumed that authorization and no further owner-funded AI invocation is authorized.
  - Codex review 4938107909 identified two P2 defects: explicit retain metadata could be bypassed by historical reconciliation, and source-branch closeout was documentation-only.
  - Agent Governance 31812781404 on repair head 548280e1fab3f986f02db1dd8238ba9673984f33 passed the new source-closeout unit tests and the live Validate terminal source branch closeout gate.
  - Self-review of the first retain fix found a bypass risk when invoking terminal_branch_cleanup.py directly, so the guarded implementation is being moved under the canonical tool name while the prior implementation is preserved only as an internal legacy module.
derived:
  - Every canonical invocation path must apply retain-aware historical classification; a workflow-only wrapper is insufficient.
  - Historical exact closed-unmerged PRs with valid retain metadata must never enter deletion manifests; malformed branch-disposition metadata must fail closed.
  - Migration-safe closeout enforcement validates every completed active task and only newly added/modified archive records relative to the current PR/push base.
unknown:
  - Exact live terminal candidate count/digest after canonical retain-aware classification.
  - Exact-final-head CI and complete full-diff self-review result after the canonical-facade refinement.
  - Post-merge deletion evidence and final retained branch inventory.
conflicts: []
first_failure:
  marker: codex-review-retention-and-closeout-enforcement
  evidence: Codex review 4938107909 on exact head 1a06f3d3296e0912a6f426ee0b267cf2bce782dd produced two P2 findings; both remain merge-blocking until the canonical repair head is green and the corresponding review threads are resolved.
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
  - Closed-unmerged refs can be deleted by age or prefix; exact PR/ref identity and liveness must be proven.
  - A workflow-only retain guard is sufficient; direct canonical tool invocation would bypass it, so the guard must own terminal_branch_cleanup.py itself.
  - A documented source-branch closeout rule is sufficient without machine validation; completed/archive tasks could otherwise bypass it while CI remained green.
changed_paths:
  - .github/workflows/agent-governance.yml
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/README.md
  - tools/agents/source_branch_closeout.py
  - tools/agents/terminal_branch_approval.py
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/terminal_branch_cleanup_legacy.py
  - tools/agents/test_source_branch_closeout.py
  - tools/agents/test_terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_guarded.py
validation:
  - command: exact-head workflow set on 1a06f3d3296e0912a6f426ee0b267cf2bce782dd before Codex review
    result: PASS
    evidence: CI 31807580870, Agent Governance 31807580829, Branch Lifecycle 31807580831, Terminal Branch Lifecycle 31807580891 and all associated exact-head workflows succeeded; superseded for merge by review-driven repairs.
  - command: owner-authorized Codex review 4938107909
    result: BLOCKED
    evidence: two P2 findings require repair before merge; no second Codex invocation is authorized.
  - command: Agent Governance 31812781404 on 548280e1fab3f986f02db1dd8238ba9673984f33
    result: PASS
    evidence: source-closeout tests, live terminal source-closeout gate, checkpoints, task liveness, Control Room, policy and prompt gates all passed.
blockers: []
next_action: Validate the canonical terminal_branch_cleanup.py retain guard and exact reviewed candidate digest on the next PR head, then repair any CI failure before resolving the two existing Codex review threads.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository remediation PR; repository delete_branch_on_merge is enabled
source_branch_evidence: pending merge and post-merge ref verification
```

## Notes

Issue #1050 is the single remediation owner. One owner-authorized Codex review for PR #1056 was consumed; no additional Codex or owner-funded AI use is authorized.
