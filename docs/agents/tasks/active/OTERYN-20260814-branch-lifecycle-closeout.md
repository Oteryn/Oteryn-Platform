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
- [x] Agent/task closeout requires explicit source-branch disposition before `completed`.
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
  - tools/agents/terminal_branch_guarded.py
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
updated_at: 2026-08-14T14:45:00Z
head: 1a06f3d3296e0912a6f426ee0b267cf2bce782dd
branch: repair/issue-1050
pr: 1056
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/terminal_branch_guarded.py
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
  - Exact-head 1a06f3d3296e0912a6f426ee0b267cf2bce782dd passed CI 31807580870, Agent Governance 31807580829, Branch Lifecycle 31807580831, Terminal Branch Lifecycle 31807580891 and all other associated workflows returned by GitHub.
  - Branch Lifecycle exact artifact validates eight reviewed TERMINAL_MERGED candidates; Terminal Branch Lifecycle exact artifact validates 105 reviewed TERMINAL_CLOSED_UNMERGED candidates on the current base.
  - The repository owner explicitly authorized one Codex review for PR #1056 and that single review was consumed by review 4938107909; no further owner-funded AI invocation is authorized.
  - Codex review 4938107909 identified two material P2 findings: historical reconciliation did not honor explicit Branch-Disposition retain metadata, and the source-branch closeout contract was not machine-enforced for completed/newly archived task records.
  - No open PR overlaps the newly claimed terminal-closeout validator paths or .github/workflows/agent-governance.yml.
derived:
  - Historical classification must treat an exact closed-unmerged PR with valid retain metadata as a non-deletion candidate and malformed disposition metadata must fail closed.
  - Migration-safe closeout enforcement must validate every completed active task while validating only newly added or modified archive records relative to the current PR/push base, so legacy archives are not retroactively invalidated.
unknown:
  - Exact candidate count/digest after the retain-aware guard is exercised against live GitHub data.
  - Exact-final-head CI and full-diff self-review result after both Codex findings are repaired.
  - Post-merge deletion evidence and final retained branch inventory.
conflicts: []
first_failure:
  marker: codex-review-retention-and-closeout-enforcement
  evidence: Codex review 4938107909 on exact head 1a06f3d3296e0912a6f426ee0b267cf2bce782dd produced two P2 findings requiring repair before merge; merge remains blocked until both are fixed, exact-head CI passes and the review threads are resolved.
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
  - Closed-unmerged refs can be deleted by age or prefix; rejected because exact PR/ref identity and liveness must be proven.
  - A documented source-branch closeout rule is sufficient without machine validation; rejected by Codex review because completed/archive tasks could bypass the rule while CI remained green.
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
  - tools/agents/terminal_branch_guarded.py
  - tools/agents/test_source_branch_closeout.py
  - tools/agents/test_terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_guarded.py
validation:
  - command: Exact-head workflow set on 1a06f3d3296e0912a6f426ee0b267cf2bce782dd before Codex review
    result: PASS
    evidence: CI 31807580870, Agent Governance 31807580829, Branch Lifecycle 31807580831, Terminal Branch Lifecycle 31807580891 and all associated exact-head workflows succeeded; this evidence is superseded for merge by the pending repair head.
  - command: owner-authorized Codex review 4938107909
    result: BLOCKED
    evidence: review produced two P2 findings that must be repaired before merge; no second Codex invocation is authorized.
  - command: focused retain-guard and source-closeout validator tests
    result: NOT_APPLICABLE
    evidence: implementation is being committed; exact-head GitHub Actions will run the repository copies immediately after the coherent repair commit.
blockers: []
next_action: Commit the retain-aware historical guard and machine-enforced source-branch closeout validator in PR #1056, then inspect exact-head CI and repair any failure before resolving the two Codex threads.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository remediation PR; repository delete_branch_on_merge is enabled
source_branch_evidence: pending merge and post-merge ref verification
```

## Notes

Issue #1050 is the single remediation owner. One owner-authorized Codex review for PR #1056 was consumed; no additional Codex or owner-funded AI use is authorized.
