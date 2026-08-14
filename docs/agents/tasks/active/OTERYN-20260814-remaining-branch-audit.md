---
task_id: OTERYN-20260814-remaining-branch-audit
issue: 1068
status: investigating
project_lane: oteryn-platform-core
phase: investigate
execution_mode: github_connector
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
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
search_first:
  - Issue #1050 and merged PRs #1056, #1064, #1066
  - current live branches and open PRs
  - current active tasks
optional_reads: []
---

# OTERYN-20260814-remaining-branch-audit

## Goal

Audit every repository branch left ambiguous after Issue #1050, preserve live/recovery work, and exact-SHA delete only the historical refs proven disposable from immutable live evidence.

## Acceptance criteria

- [ ] Fresh live inventory accounts for every branch on current protected `main`.
- [ ] Every branch receives an evidence-backed disposition: `OPEN_PR`, `PROTECTED`, `RETAIN`, `RECOVERY`, or `DELETE`.
- [ ] No branch is deleted by age or prefix alone.
- [ ] Open PR, active-task, protected, unique rollback/recovery/backup and materially ambiguous refs remain fail-closed.
- [ ] Every `DELETE` candidate is bound to its exact immutable branch SHA and deterministic recovery provenance.
- [ ] A reviewed candidate digest is validated immediately before apply with no branch/SHA/liveness drift.
- [ ] Post-merge apply deletes exactly the reviewed candidates and verifies absence without REST read-after-delete races.
- [ ] Final inventory has zero unreviewed deletion candidates under the accepted audit policy.
- [ ] One-time approval state is removed, task is archived, Issue #1068 closes completed, and the task source branch is reconciled.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/tasks/archive/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/historical_branch_audit.py
  - tools/agents/test_historical_branch_audit.py
  - .github/workflows/historical-branch-audit.yml
modules:
  - repository-governance
  - branch-lifecycle
dependencies:
  - Issue #1050 terminal closeout
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T20:49:00Z
head: ef156b16286d531b08feb9477b5e0d72f177d5ae
branch: repair/issue-1068-remaining-branch-audit
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/tasks/archive/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/historical_branch_audit.py
  - tools/agents/test_historical_branch_audit.py
  - .github/workflows/historical-branch-audit.yml
proven:
  - Issue #1050 closed completed after deleting 113 exact-reviewed historical refs and installing terminal branch lifecycle governance.
  - Protected main at task start is ef156b16286d531b08feb9477b5e0d72f177d5ae.
  - The prior approval-free inventory immediately before PR #1066 merge had 80 refs: 9 OPEN_PR, 1 PROTECTED, 60 UNKNOWN and 10 UNMERGED_ORPHAN; PR #1066 then merged and its source ref was auto-deleted.
  - Current live repository has 8 open PRs and two active task records whose checkpoints own no branch.
  - No Codex/OpenAI/owner-funded AI use is authorized for Issue #1068.
derived:
  - The remaining historical set requires a one-time evidence-rich audit because current terminal lifecycle intentionally fails closed on moved/no-PR refs.
  - Exact ancestry/divergence and PR history can distinguish redundant historical refs from unique recovery evidence without merging abandoned work.
unknown:
  - Exact disposition of each remaining UNKNOWN and UNMERGED_ORPHAN ref.
  - Exact reviewed deletion candidate count after live ancestry/PR/task audit.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Delete all old-looking refs by prefix or date; forbidden because naming/age is not terminal evidence.
  - Merge abandoned branches merely to make them deletable; forbidden by repository lifecycle policy.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
validation:
  - command: live Issue #1050 / PR #1066 / branch inventory reconciliation
    result: PASS
    evidence: #1050 is closed completed, #1066 merged, its source branch is absent, and current open PR/active-task state was re-read at task start.
  - command: audit E2E
    result: NOT_APPLICABLE
    evidence: repository Git-ref governance has no user/browser/runtime journey; exact live GitHub inventory and destructive-ref recovery verification are the applicable end-to-end evidence.
blockers:
  - none
next_action: Implement a fail-closed historical branch audit that resolves exact branch SHA, main ancestry/divergence, PR history, active ownership and recovery-purpose evidence, then run it through GitHub Actions on the task branch before approving any deletion.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active on a dedicated same-repository repair branch
source_branch_evidence: pending
```

## Notes

This task is a bounded historical reconciliation only. Future task/PR branch cleanup remains governed by the lifecycle installed under Issue #1050. No production, staging, external-repository, secret or owner-funded AI operation is in scope.