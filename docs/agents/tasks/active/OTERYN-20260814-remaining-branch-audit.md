---
task_id: OTERYN-20260814-remaining-branch-audit
issue: 1068
status: validating
project_lane: oteryn-platform-core
phase: validate
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

- [x] Fresh live inventory accounts for every branch on current protected `main`.
- [x] Every branch receives an evidence-backed disposition: `OPEN_PR`, `PROTECTED`, `RETAIN`, `RECOVERY`, or `DELETE`.
- [x] No branch is classified for deletion by age or prefix alone.
- [x] Open PR, active-task, protected, unique rollback/recovery/backup and materially ambiguous refs remain fail-closed.
- [x] Every reviewed `DELETE` candidate is bound to its exact immutable branch SHA and deterministic non-loss proof.
- [ ] Exact-head PR validation revalidates the one-time approval with no candidate/SHA/implementation drift.
- [ ] Post-merge apply deletes exactly the reviewed candidates and verifies recovery/absence/non-candidate preservation.
- [ ] Final approval-free inventory has zero unprocessed reviewed candidates.
- [ ] One-time approval state is removed, task is archived, Issue #1068 closes completed, and the task source branch is reconciled.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/tasks/archive/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
  - .github/workflows/historical-branch-audit.yml
modules:
  - repository-governance
  - branch-lifecycle
dependencies:
  - Issue #1050 terminal closeout
blockers:
  - PR #1070 must reconcile the already-merged PublicEdge task before repository-global Agent Governance can be green; this is unrelated live ownership debt already owned by its lifecycle closeout.
  - PR #1069 must not be marked Ready while repository configuration would auto-trigger Codex without a new explicit per-use owner authorization.
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T21:14:00Z
head: b06f6a54ad323ee4bce12d1ba0538f2a5dda3f6f
branch: repair/issue-1068-remaining-branch-audit
pr: 1069
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/tasks/archive/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
  - .github/workflows/historical-branch-audit.yml
proven:
  - Issue #1050 closed completed after deleting 113 exact-reviewed historical refs and installing terminal branch lifecycle governance.
  - Protected main for the reviewed audit is 780ad6c8178206b13d001537ba651b6e0bd22219; the task branch was synchronised with that main before authoritative inventory.
  - Historical Branch Audit run 31841162360 artifact 9234290642 digest sha256:a12c3a60be37e413fca897f586ad29e2ffdffefe35f0cad160cb9c7c49354682 fully accounts for 80 live refs.
  - Reviewed dispositions are 33 DELETE, 9 OPEN_PR, 1 PROTECTED, 15 RECOVERY and 22 RETAIN.
  - The 33 deletion candidates comprise 25 exact heads already ancestral to protected main, 4 exact Tibia session heads ancestral to live open-PR branch ops/oteryn-tibia-client-analysis-20260811, and x2-x5 as exact aliases of retained x.
  - One-time approval binds all 33 exact branch/SHA pairs with entries digest 4fd027de83cf893b40ff0e3eb6fb61cec5a5513fd3bd2a852d4491aaec42230c and implementation digest 3cad7d0210d20e88ac1e495d6004bd55c6fd7ed948727fa14589eb8200f96966.
  - Unique unmerged generic history is retained; unique backup/recovery/rollback history is RECOVERY; open PRs and protected main are non-candidates.
  - Apply performs a create/delete recovery probe, rebuilds the reviewed policy once against current protected main, fails on any new unreviewed candidate or approved-candidate drift, and re-verifies each candidate proof/SHA/protection/open-PR state immediately before exact lease-guarded deletion.
  - No Codex/OpenAI/owner-funded AI use has been invoked for Issue #1068.
derived:
  - The reviewed set is materially safer than deleting all old-looking refs: every approved deletion preserves commit reachability through protected main or an explicitly retained live anchor.
  - The remaining unique refs are intentionally not deletable under current evidence; cleaning them further would require a new recovery/archive decision rather than inference from naming.
unknown:
  - Exact final approval-validation/CI generation after report, approval and this checkpoint are committed.
  - Post-merge apply evidence and final branch count.
conflicts: []
first_failure:
  marker: repository-global-live-ownership
  evidence: Agent Governance on PR #1069 fails only because merged PR #1063 remains represented by its separately owned active PublicEdge task; draft closeout PR #1070 exists for that lifecycle debt.
rejected_hypotheses:
  - Delete all old-looking refs by prefix/date; forbidden because naming/age is not terminal evidence.
  - Delete unique generic or recovery history just to reduce the branch count; rejected because no retained reachability proof exists.
  - Merge abandoned branches merely to make them deletable; forbidden by repository lifecycle policy.
  - Reuse Issue #1050 approvals; they were one-time candidate-set approvals and are consumed/removed.
changed_paths:
  - .github/workflows/historical-branch-audit.yml
  - docs/agents/HISTORICAL_BRANCH_DELETION_APPROVAL.json
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - docs/agents/tasks/active/OTERYN-20260814-remaining-branch-audit.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
validation:
  - command: Historical Branch Audit run 31841162360
    result: PASS
    evidence: artifact 9234290642 fully accounts for 80 refs and emits the reviewed 33-entry exact candidate manifest.
  - command: manual candidate review
    result: PASS
    evidence: all 33 DELETE entries use only ANCESTOR_OF_MAIN, REACHABLE_FROM_LIVE_ANCHOR, or DUPLICATE_HEAD_RETAINED_AS non-loss proofs; no RETAIN/RECOVERY/OPEN_PR/PROTECTED ref is approved.
  - command: repository-global Agent Governance
    result: BLOCKED
    evidence: unrelated merged PublicEdge task #1063 is still active on protected main; its dedicated draft lifecycle closeout is PR #1070.
  - command: audit E2E
    result: NOT_APPLICABLE
    evidence: repository Git-ref governance has no user/browser/runtime journey; exact live GitHub inventory, destructive-ref recovery probe and post-delete ref verification are the applicable end-to-end evidence.
blockers:
  - repository-global Agent Governance awaits unrelated PublicEdge lifecycle closeout PR #1070
  - marking PR #1069 Ready requires explicit per-use owner authorization if Codex auto-review remains configured
next_action: Require the exact-head Historical Branch Audit generation to validate the committed 33-entry approval; then self-review the final diff. Once repository-global liveness is green, obtain explicit owner authorization before any Ready transition that would invoke Codex.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active on dedicated same-repository draft PR #1069
source_branch_evidence: pending
```

## Notes

This task is a bounded historical reconciliation only. Future task/PR branch cleanup remains governed by Issue #1050 lifecycle. No production, staging, external-repository, secret or owner-funded AI operation is in scope.