---
task_id: OTERYN-20260815-historical-work-reconciliation
issue: 1072
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
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
search_first:
  - Issue #1072 and Issue #1068 terminal evidence
  - current live branches and every open PR
  - current active tasks and path ownership
  - release/tag/ref-triggered workflows before choosing a managed recovery ref mechanism
optional_reads:
  - docs/agents/reports/OTERYN-20260814-remaining-branch-audit.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
---

# OTERYN-20260815-historical-work-reconciliation

## Goal

Reconcile every historical branch that survived Issue #1068 as `RETAIN` or `RECOVERY`, preserve useful work in canonical durable state, and remove obsolete execution refs under exact-head fail-closed controls so ordinary branches are not historical archives.

## Acceptance criteria

- [x] Fresh live branch/PR/task accounting performed from protected main.
- [x] All 37 historical candidates received individual content/provenance/reachability review and terminal dispositions.
- [x] Terminal matrix contains 6 `DELETE`, 20 `DOCUMENT_ARCHIVE`, 11 `PR_PROVENANCE_DELETE`, 0 `MANAGED_RECOVERY`, 0 `CANONICALIZE_TO_MAIN`.
- [x] All 15 legacy `RECOVERY` refs were individually reconciled; no independent exact-Git-history retention requirement remains.
- [x] No historical branch was blindly merged and no deletion decision was based on name, age, prefix or inactivity.
- [x] Exact-SHA, protection, open-PR, active-claim, provenance, preservation and main-drift guards are implemented.
- [x] Authoritative Git ref absence checks, non-candidate exact-SHA preservation and create/delete/recreate/delete restore probe are implemented.
- [x] Owner explicitly authorized marking PR #1074 Ready and any repository-required Ready-triggered owner-funded review for this exact Issue.
- [x] Protected main drift through #1076/#1084 was re-read; the #1084 lifecycle archive change was accepted by refreshing reviewed main rather than weakening drift policy.
- [ ] Exact current-head focused validation, repository-required CI and Ready-triggered review state are terminal green.
- [ ] PR #1074 is merged with expected-head protection after final current-main/mergeability recheck.
- [ ] Trusted-main reconciliation apply deletes the exact reviewed historical refs and proves real Git lifecycle E2E.
- [ ] Approval-free post-mutation inventory has zero unexplained `RETAIN` and zero unmanaged `RECOVERY`.
- [ ] Task is archived, Issue #1072 is closed completed, temporary state is absent and source branch deletion is verified.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
  - tools/agents/historical_branch_audit.py
  - tools/agents/historical_branch_policy.py
  - tools/agents/test_historical_branch_audit.py
  - tools/agents/test_historical_branch_policy.py
  - tools/agents/*historical*reconciliation*.py
  - .github/workflows/historical-branch-audit.yml
modules:
  - repository-governance
  - branch-lifecycle
  - historical-work-reconciliation
dependencies:
  - Issue #1050 terminal source-branch lifecycle
  - Issue #1068 historical exact-SHA cleanup
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
updated_at: 2026-08-15T07:47:00Z
head: LIVE_PR_1074_HEAD
branch: repair/issue-1072-historical-work-reconciliation
pr: 1074
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
  - tools/agents/historical_work_reconciliation.py
  - tools/agents/test_historical_work_reconciliation.py
  - .github/workflows/historical-branch-audit.yml
proven:
  - Discovery and content/provenance inventory run 31870802443 produced artifact 9243364594.
  - Later full-accounting run 31872078706 produced artifact 9243721593 with digest sha256:562fe26572947031da3b1d92228de78a8fbb045002c1cd95dffd140241f27b64.
  - The reviewed historical set is exactly 37 refs: 6 DELETE, 20 DOCUMENT_ARCHIVE, 11 PR_PROVENANCE_DELETE.
  - No managed recovery ref is required for the reviewed legacy set.
  - PR #1074 was changed from Draft to Ready after explicit owner authorization in this conversation.
  - Current protected main was re-read as 88b3dce7b822ed27d9f61c412493c57ba6608a38 after merged closeout PR #1084.
  - Historical Branch Audit run 31872549491 failed only because reviewed-main drift correctly detected the newly archived Issue #1075 task path.
  - The registry now binds reviewed_main_sha to 88b3dce7b822ed27d9f61c412493c57ba6608a38 instead of allowlisting that unrelated drift.
derived:
  - A fresh exact-head validation generation is required because the registry and task checkpoint changed after owner authorization and main refresh.
unknown:
  - Ready-triggered review terminal verdict on the resulting exact PR head.
  - Exact final merge SHA and trusted-main apply run ID.
conflicts: []
first_failure:
  marker: none-open
  evidence: the prior draft authorization blocker was explicitly removed by the owner; the prior reviewed-main drift failure was repaired fail-closed.
rejected_hypotheses:
  - weaken reviewed-main drift validation to ignore arbitrary unrelated changes
  - keep legacy RETAIN or RECOVERY branches indefinitely
  - create a managed recovery ref without a real exact-history recovery requirement
  - delete before trusted-main exact-head revalidation
changed_paths:
  - .github/workflows/historical-branch-audit.yml
  - docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/tasks/active/OTERYN-20260815-historical-work-reconciliation.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
  - docs/architecture/adr/README.md
  - tools/agents/historical_work_reconciliation.py
  - tools/agents/test_historical_work_reconciliation.py
validation:
  - command: historical content/provenance/reachability discovery
    result: PASS
    evidence: run 31870802443 artifact 9243364594
  - command: full live accounting before owner authorization
    result: PASS
    evidence: run 31872078706 artifact 9243721593
  - command: current-main drift gate
    result: PASS
    evidence: run 31872549491 rejected #1084 archive drift before mutation; registry was then refreshed to exact current protected main 88b3dce7b822ed27d9f61c412493c57ba6608a38
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Issue #1072 is repository Git-ref governance; mandatory E2E is the real Git lifecycle apply/restore proof after merge
blockers: []
next_action: Validate the resulting exact PR #1074 head against current protected main, inspect Ready-triggered review and all required checks, then merge only if the head is unchanged and every gate is terminal green.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: source branch remains required until PR #1074 merge, trusted-main apply and terminal archive closeout complete
source_branch_evidence: PR #1074 currently owns repair/issue-1072-historical-work-reconciliation; final closeout must verify ref absence after merge
```

## Notes

No historical ref has been deleted yet. No production, staging, protected-environment deployment, external-repository mutation or unapproved owner-funded AI operation is performed by this task.
