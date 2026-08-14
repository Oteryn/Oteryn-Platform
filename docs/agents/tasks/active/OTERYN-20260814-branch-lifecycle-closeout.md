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
- [ ] Focused tests and exact-head required CI pass on the final PR head.
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
updated_at: 2026-08-14T15:21:00Z
head: 9dfc17349de1662eaedc7f5d57793f858e3dedfe
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
  - Protected main remains f61c7b229cbf251be6e5eae0be5db55aac722242 through the final material repair cycle and PR #1056 is mergeable against that base.
  - Repository has delete_branch_on_merge enabled and protected main requires classify-changes and test.
  - The repository owner explicitly authorized one Codex review for PR #1056; Codex review 4938410706 consumed that authorization and no further owner-funded AI invocation is authorized.
  - Codex review 4938410706 identified two P2 defects: explicit retain metadata could be bypassed by historical reconciliation, and source-branch closeout was documentation-only.
  - Source-branch closeout is now machine-enforced by tools/agents/source_branch_closeout.py in Agent Governance for every completed active task and every newly added/modified archive record, with migration-safe changed-archive selection.
  - Exact-head Agent Governance 31813503452 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4 passed source-closeout tests and the live terminal source-branch closeout gate.
  - The retain-aware guard now owns canonical tools/agents/terminal_branch_cleanup.py; the previous implementation is internalized as terminal_branch_cleanup_legacy.py so direct CLI/import execution cannot bypass retention policy.
  - Exact-head Terminal Branch Lifecycle 31813503494 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4 passed cleanup regression tests, live inventory and approval validation with exactly 105 reviewed TERMINAL_CLOSED_UNMERGED candidates, entries SHA 77bcff4f3cbc3535a8e4097927f1f7a360e4bd704186214e220ffad35d2de6fe and policy SHA 3d646fbd53bd0ae38572dcf80201159d7e05d5562919e55c75f99a83a4031c20.
  - Exact-head CI 31813503490 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4 passed classify-changes, runtime-tests and required test gate.
  - Adversarial self-review then found a final retain race between manifest construction and per-branch deletion; material head 9dfc17349de1662eaedc7f5d57793f858e3dedfe re-fetches the exact closed PR immediately before each delete and fails closed on identity drift, malformed metadata or a newly added retain disposition, with a dedicated regression test.
derived:
  - Historical exact closed-unmerged PRs with valid retain metadata never enter deletion manifests and a late retain change cannot race the destructive step.
  - Historical PRs with no branch-disposition metadata remain reviewable only under the one-time exact candidate digest; future close events require explicit disposition metadata.
  - Migration-safe closeout enforcement prevents new terminal task debt without retroactively invalidating unrelated historical archives.
unknown:
  - Exact-final-head workflow result for the material 9dfc17349de1662eaedc7f5d57793f858e3dedfe repair plus this checkpoint-only commit.
  - Post-merge deletion evidence and final retained branch inventory.
conflicts: []
first_failure:
  marker: codex-review-retention-and-closeout-enforcement
  evidence: Codex review 4938410706 on 1a06f3d3296e0912a6f426ee0b267cf2bce782dd produced two P2 findings. Both implementation fixes are present; their review threads remain unresolved only until final exact-head validation and self-review complete.
rejected_hypotheses:
  - All retained branches should be merged to main before deletion; abandoned and diagnostic refs must not be merged merely for cleanup.
  - Closed-unmerged refs can be deleted by age or prefix; exact PR/ref identity and liveness must be proven.
  - A workflow-only retain guard is sufficient; direct canonical tool invocation would bypass it.
  - Snapshot-time retention validation alone is sufficient; a retain marker can change before destructive apply, so each delete must re-fetch the exact PR immediately before mutation.
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
  - command: owner-authorized Codex review 4938410706
    result: BLOCKED
    evidence: two P2 findings required repair; no second Codex invocation is authorized.
  - command: Agent Governance 31813503452 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4
    result: PASS
    evidence: source-closeout tests, live terminal source-closeout gate, checkpoints, task liveness, Control Room, policy and prompt gates passed.
  - command: Terminal Branch Lifecycle 31813503494 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4
    result: PASS
    evidence: canonical cleanup tests, live fail-closed inventory and exact reviewed digest validation passed; candidate_count=105 and entries_sha256=77bcff4f3cbc3535a8e4097927f1f7a360e4bd704186214e220ffad35d2de6fe.
  - command: CI 31813503490 on 0e69bfbf00138f33a1107ab95a5e4883a10797b4
    result: PASS
    evidence: classify-changes, runtime-tests and required test gate passed.
  - command: exact-head workflows after material race repair 9dfc17349de1662eaedc7f5d57793f858e3dedfe
    result: NOT_RUN
    evidence: workflow generation started and will be superseded only by this checkpoint-only commit; merge remains blocked until required checks pass on the resulting final PR head.
blockers: []
next_action: Wait only for the bounded exact-final-head CI set, then complete full-diff self-review, resolve the two existing Codex threads and squash-merge PR #1056 if every merge gate remains satisfied.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository remediation PR; repository delete_branch_on_merge is enabled
source_branch_evidence: pending merge and post-merge ref verification
```

## Notes

Issue #1050 is the single remediation owner. One owner-authorized Codex review for PR #1056 was consumed; no additional Codex or owner-funded AI use is authorized.
