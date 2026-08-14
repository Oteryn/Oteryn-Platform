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
  - tools/agents/terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_approval.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
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
blockers:
  - exact-final-head GitHub Actions must be terminal and green before readiness
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T14:03:00Z
head: 098cc3439f7cdad6295e79c2c1066d9bd6979cdd
branch: repair/issue-1050
pr: 1056
status: waiting
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_approval.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/README.md
proven:
  - Protected main is f61c7b229cbf251be6e5eae0be5db55aac722242 and PR #1056 is synchronized with that base.
  - Repository has delete_branch_on_merge enabled and protected main requires classify-changes and test.
  - Issue #658 previously removed 354 proven TERMINAL_MERGED refs; the existing classifier still has eight reviewed TERMINAL_MERGED candidates whose exact manifest approval fails closed on drift.
  - Terminal Branch Lifecycle run 31807011706 against current main f61c7b229cbf251be6e5eae0be5db55aac722242 classified 105 TERMINAL_CLOSED_UNMERGED candidates and preserved UNKNOWN, moved/ambiguous, open, active, protected and recovery-sensitive refs.
  - The reviewed 105-entry historical candidate set is bound to entries SHA-256 77bcff4f3cbc3535a8e4097927f1f7a360e4bd704186214e220ffad35d2de6fe and policy SHA-256 3d646fbd53bd0ae38572dcf80201159d7e05d5562919e55c75f99a83a4031c20; any live candidate, SHA, PR or policy drift invalidates apply.
  - New terminal classification requires exactly one same-repository closed-unmerged PR whose terminal PR head SHA equals the current ref, after open-PR, active-task/open-repair-Issue, protection, retention and recovery-sensitive-name gates.
  - Future closed-unmerged automatic deletion requires explicit Branch-Disposition: delete plus a non-empty Branch-Disposition-Reason; retain or unspecified disposition does not delete.
  - ADR registry failure on superseded head 84a9ed23f6f12e9274147d0f69ecf2a73a74c49e was traced to a conflicting ADR 0025 allocation; the decision is now correctly allocated and registered as ADR 0037.
  - Agent Governance run 31807162607 and Synology Container Hygiene run 31807162706 succeeded on material head 098cc3439f7cdad6295e79c2c1066d9bd6979cdd.
derived:
  - Branch clutter is dominated by work that never reached the merged-PR automatic-delete path; merging abandoned/superseded/diagnostic work merely for cleanup would be incorrect.
  - If both reviewed candidate sets remain unchanged at protected-main apply, 113 historical refs are eligible for exact-head deletion; the task source branch is additionally expected to auto-delete only after a successful merge.
unknown:
  - Final terminal result of all exact-head required/relevant GitHub Actions for the current material head.
  - Post-merge deletion evidence and final retained branch inventory.
conflicts: []
first_failure:
  marker: adr-registry-allocation-conflict
  evidence: CI run 31805959152 failed AdrRegistryValidationTest because a new decision had been incorrectly allocated as 0025 while main already contained 0025 through 0036; repaired by ADR 0037 plus README inventory registration.
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
  - Closed-unmerged refs can be deleted by age or prefix; rejected because exact PR/ref identity and liveness must be proven.
changed_paths:
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/TERMINAL_BRANCH_DELETION_APPROVAL.json
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260814-branch-lifecycle-closeout.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/README.md
  - tools/agents/terminal_branch_approval.py
  - tools/agents/terminal_branch_cleanup.py
  - tools/agents/test_terminal_branch_approval.py
  - tools/agents/test_terminal_branch_cleanup.py
validation:
  - command: Terminal Branch Lifecycle run 31807011706
    result: PASS
    evidence: live dry-run on current main f61c7b229cbf251be6e5eae0be5db55aac722242; reviewed 105-entry candidate digest validated
  - command: Agent Governance run 31807162607 on material head 098cc3439f7cdad6295e79c2c1066d9bd6979cdd
    result: PASS
    evidence: GitHub Actions conclusion success
  - command: Synology Container Hygiene run 31807162706 on material head 098cc3439f7cdad6295e79c2c1066d9bd6979cdd
    result: PASS
    evidence: GitHub Actions conclusion success
  - command: exact-final-head CI / Branch Lifecycle / Terminal Branch Lifecycle
    result: BLOCKED
    evidence: latest runs are queued/pending in GitHub Actions; no failure is currently reported
blockers:
  - exact-final-head GitHub Actions are queued/pending and must become terminal before readiness
  - marking draft PR #1056 ready may invoke the repository-configured owner-funded Codex review and therefore requires explicit owner authorization for that exact action
next_action: Recheck exact-head GitHub Actions; if green and main/head are unchanged, complete full-diff self-review and request explicit owner authorization before marking PR #1056 ready.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository remediation PR; repository delete_branch_on_merge is enabled
source_branch_evidence: pending merge and post-merge ref verification
```

## Notes

Issue #1050 is the single remediation owner. No Codex or owner-funded AI use is authorized.