---
task_id: OTERYN-20260815-historical-work-reconciliation
issue: 1072
status: blocked
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

- [x] Reconcile against fresh protected `main`; seed counts are evidence only.
- [x] Account for every live branch and every open same-repository PR before mutation design.
- [x] Audit every historical `RETAIN` / `RECOVERY` ref for commits, changed paths, content value, provenance, reachability, current-main relevance and recovery purpose.
- [x] Replace non-terminal `RETAIN` / unmanaged `RECOVERY` classifications with terminal decisions.
- [x] Do not blindly merge stale historical branches.
- [x] Preserve selected historical evidence and exact source SHA/provenance before deletion.
- [x] Prove no long-lived managed recovery ref is required for the reviewed legacy set.
- [x] Inspect repository workflows before considering custom/tag recovery refs; introduce none when no recovery requirement exists.
- [x] Implement exact-SHA, protection, open-PR, active-claim, provenance, preservation and main-drift guards before deletion.
- [x] Implement authoritative post-delete Git ref absence verification, non-candidate exact-SHA preservation and a real create/delete/recreate/delete restore probe.
- [x] Preserve protected/open/live owned work.
- [x] Add durable validator coverage preventing unexplained live historical refs and incomplete managed recovery contracts.
- [ ] Merge the reviewed implementation to protected `main`.
- [ ] Execute trusted-main reviewed mutation and prove post-mutation inventory plus real Git lifecycle E2E.
- [ ] Move this task to `docs/agents/tasks/archive/`, close Issue #1072 completed, remove temporary state, and verify source branch deletion.

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
  - Issue #1068 historical exact-SHA cleanup and final zero-candidate audit
  - ADR 0037 terminal source-branch lifecycle
blockers:
  - PR #1074 is intentionally draft and its accepted authorization explicitly forbids marking Ready or invoking Codex/OpenAI/API/owner-funded AI review without explicit permission for that exact use.
cross_repository_tasks:
  - none
```

## Accepted owner decision

ADR 0039 remains authoritative: branches are execution resources, `RETAIN` is transitional, historical value belongs on current `main`, focused durable provenance, or a real managed recovery mechanism, and branch deletion remains exact-head/fail-closed under ADR 0037.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T07:38:20Z
head: LIVE_PR_1074_HEAD
branch: repair/issue-1072-historical-work-reconciliation
pr: 1074
status: blocked
context_routes:
  - agent-governance
  - architecture
  - testing
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
proven:
  - Issue #1068 is closed completed after its exact-reviewed historical cleanup and approval removal.
  - Discovery run 31870802443 produced artifact 9243364594 with digest sha256:8923531b9a224f1cfaa3d341abf553ad054f1b31d84f168b5b63ae1dc5eb5177 and content/provenance/reachability evidence for the historical candidates.
  - The reviewed historical set contains 37 refs with terminal decisions: 6 DELETE, 20 DOCUMENT_ARCHIVE and 11 PR_PROVENANCE_DELETE; MANAGED_RECOVERY and CANONICALIZE_TO_MAIN are both zero.
  - Every one of the 15 legacy RECOVERY refs was individually reconciled; none retains an independent exact Git-history recovery requirement.
  - The live branch test/OTERYN-20260815-e2e-486-completion moved into owned PR #1077 after discovery and was excluded from historical deletion before review.
  - Current protected main was rechecked at 841793be68d4c52aff14bfc51d6f291a24b74477; the intervening merged PR #1076 changed only three unrelated prompt-hardening paths admitted to the reviewed main-drift allowlist.
  - PR #1074 current implementation head before this checkpoint commit was 5b1c3e586c368e9f1843003fe147fb06d75f7dbe, mergeable, draft, and changed exactly the eight Issue #1072-owned implementation paths.
  - PR #1074 has no submitted reviews, no inline review threads and no PR discussion comments at the latest review-hygiene check.
  - Historical Branch Audit run 31872416348 validate job passed focused historical-governance tests on head 5b1c3e586c368e9f1843003fe147fb06d75f7dbe.
  - The previous exact implementation head f5f45d284ee27d86c8f485784cb216b8e8a14a21 passed all automatically triggered workflows including CI, Agent Governance and Historical Branch Audit before protected main advanced.
  - Agent Governance run 31872416374 on the current synthetic merge failed only because the unrelated merged PR #1076 left task OTERYN-20260815-portal-completion-prompt-hardening active with stale terminal next action; its dedicated closeout PR #1084 exists separately and is outside Issue #1072 ownership.
  - PR #1074 body preserves the explicit authorization boundary that it must remain draft unless exact Ready/owner-funded-review permission is granted.
derived:
  - Historical deletion must not run until the reviewed implementation is merged and the trusted-main apply job rechecks all live guards.
  - No custom/tag recovery ref should be created for the reviewed legacy set because there is no surviving exact-history recovery requirement.
  - The implementation itself is ready for terminal merge gating once the explicit draft authorization blocker is removed and exact-head validation is refreshed against then-live main.
unknown:
  - Protected main and active/open-work drift at the future instant authorization is granted.
  - Exact final merge SHA and trusted-main apply run because merge is not authorized while PR #1074 remains draft.
conflicts:
  - Current Agent Governance synthetic-merge validation is red because of unrelated Issue #1075 lifecycle debt on protected main, not because of an Issue #1072-owned validation failure.
first_failure:
  marker: PR_1074_DRAFT_AUTHORIZATION_BOUNDARY
  evidence: PR #1074 is draft and explicitly says not to mark Ready or invoke Codex/OpenAI/API/owner-funded AI review without explicit permission for that exact use.
rejected_hypotheses:
  - keep historical RETAIN or RECOVERY branches indefinitely
  - delete branches by name prefix age inactivity or similarity
  - blindly merge stale historical branches
  - treat a textual SHA as managed Git recovery
  - mark PR #1074 Ready despite the explicit authorization boundary
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
  - command: discovery content/provenance/reachability inventory
    result: PASS
    evidence: Historical Branch Audit run 31870802443 artifact 9243364594 digest sha256:8923531b9a224f1cfaa3d341abf553ad054f1b31d84f168b5b63ae1dc5eb5177
  - command: focused historical governance tests
    result: PASS
    evidence: run 31872416348 validate job passed on implementation head 5b1c3e586c368e9f1843003fe147fb06d75f7dbe
  - command: exact-head Agent Governance after protected-main advance
    result: FAIL
    evidence: run 31872416374 failed live-task liveness only for unrelated merged PR #1076 task; Issue #1072-owned validator/test steps passed
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Issue #1072 changes repository governance and Git-ref lifecycle tooling; mandatory E2E is the real Git create/delete/restore/apply lifecycle after merge, not browser/runtime behavior
blockers:
  - Explicit owner authorization is absent for marking draft PR #1074 Ready for review or invoking any Ready-triggered owner-funded AI review.
next_action: Owner explicitly authorizes marking PR #1074 Ready for review for Issue #1072, including any Ready-triggered owner-funded review required by repository policy.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Issue #1072 remains blocked before merge on repair/issue-1072-historical-work-reconciliation and draft PR #1074
source_branch_evidence: live PR #1074 owns the source branch; deletion is required only after terminal merge and closeout
```

## Notes

All destructive work remains fail-closed. No historical ref has been deleted by this task yet, no managed recovery ref has been introduced, and no paid/owner-funded AI/Codex/OpenAI API service has been invoked.
