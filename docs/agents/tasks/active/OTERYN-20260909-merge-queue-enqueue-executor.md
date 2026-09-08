---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live Issue #1362 ownership and Issue #1363 state
  - current Merge Queue rules and required platform-gate
optional_reads: []
---

# OTERYN-20260909-merge-queue-enqueue-executor

## Goal

Governing GitHub Issue: #1363 — add one repository-native, fail-closed Merge Queue enqueue executor so qualified PRs do not depend on an online maintainer workstation.

## Acceptance criteria

- [x] Executor only performs GraphQL `enqueuePullRequest` with exact expected-head fencing.
- [x] Wrong repository/base/head/state/draft/origin or non-success exact-head `platform-gate` fails before mutation by deterministic code paths.
- [x] GitHub App token is short-lived, repository-scoped, minimum-permission, and no credential value is committed.
- [x] External GitHub Actions references are immutable full SHAs.
- [ ] Focused tests and repository-required exact-head checks pass.
- [ ] Integration itself uses normal Merge Queue with no direct-merge or protection bypass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
modules:
  - repository integration control plane
dependencies:
  - Issue 1362 post-merge reconciliation remains separate and owns only its exact current paths
blockers:
  - Issue 1362 terminal PR #1367 is still represented by an active task without archive-pending lifecycle state, causing Agent Governance to fail independently of this diff
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T23:40:47Z
head: df2211f95b3bd8cab6609494909f00fa1e6ea61b
branch: ci/1363-merge-queue-enqueue-executor
pr: 1371
status: validating
context_routes:
  - ci-repair
  - execution-resources
owned_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
proven:
  - Protected main is 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce and requires platform-gate.
  - PR #1367 merged Issue #1362 repair; the remaining #1362 active ownership is narrowed to exact paths and no longer owns all .github/workflows or tools.
  - No existing branch, PR, workflow, or script implementing enqueuePullRequest was found before this task branch was created.
  - Issue #1363 remains open and PR #1371 is the canonical implementation PR.
  - GitHub GraphQL documents enqueuePullRequest with pullRequestId and expectedHeadOid; MergeQueueEntry exposes id, position, and state.
  - actions/create-github-app-token v3.2.0 resolves to immutable commit bcd2ba49218906704ab6c1aa796996da409d3eb1; its action contract exposes separate checks, pull-requests, and merge-queues permission inputs.
  - The workflow requests only checks:read, pull-requests:read and merge-queues:write from the short-lived App token; the built-in GITHUB_TOKEN remains contents:read.
  - The new manual-only workflow is explicitly registered in the workflow lifecycle policy with a documented review/retirement condition.
  - The focused test module now exercises fifteen fail-closed qualification, enqueue and workflow-permission-contract cases and passes against the exact script/workflow/test bytes at the current implementation generation.
  - PR #1371 historical head 75e681f9c4578453a01749936a1409538fe37705 passed CI workflow inventory, action-pinning, active-checkpoint and change-classification validation before later permission-hardening commits superseded that head.
  - PR #1371 Agent Governance failure on the historical head is caused by the pre-existing #1362 task lifecycle defect: terminal PR #1367 remains represented as active without archive-pending state.
derived:
  - The #1363 implementation proceeds on exact owned paths without overlapping #1362.
  - #1371 cannot claim complete governance qualification until the #1362 owner reconciles its own terminal task lifecycle; editing that #1362-owned task here would violate ownership separation.
unknown:
  - Repository-required exact-head CI result for the final PR #1371 head after this checkpoint commit.
  - First legitimate Merge Queue bootstrap path for PR #1371; all authorized Remote Desktop hosts were offline at the latest check and the connected GitHub surface has no enqueuePullRequest mutation.
conflicts: []
first_failure:
  marker: missing-repository-native-merge-queue-enqueue-route
  evidence: connected GitHub automation exposes no enqueuePullRequest mutation and existing fallback depends on an external host
rejected_hypotheses:
  - immediate REST merge is an acceptable substitute for Merge Queue
  - ordinary auto-merge can replace the repository Merge Queue path
  - pull-requests:write is the minimum App permission for queue mutation; the current App-token action exposes dedicated merge-queues permission and the workflow now uses merge-queues:write plus pull-requests:read
changed_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: LIVE overlap and protected-main inspection
    result: PASS
    evidence: #1362 ownership is exact and disjoint from the #1363 paths; current main and required platform-gate were re-read before branch creation
  - command: python tests/ci/test_merge_queue_enqueue.py
    result: PASS
    evidence: 15 deterministic tests passed, including wrong repository/base/head/state/draft/origin, missing/failed exact-head platform-gate, latest-run selection, expectedHeadOid mutation fencing, missing queue-entry rejection and minimum App permission/pinned-action workflow contract
  - command: PR #1371 historical-head CI classify-changes validation
    result: PASS
    evidence: workflow inventory/lifecycle, immutable action references, active checkpoint contract and change classification all passed before later exact-head-hardening commits superseded that run
  - command: PR #1371 historical-head Agent Governance
    result: FAIL
    evidence: task_liveness reports terminal_pr_active_task for #1362 because terminal PR #1367 is still represented as active without archive-pending transition; no #1363 task-liveness finding was reported
  - command: implementation self-review against Issue #1363 mutation boundary
    result: PASS
    evidence: the script has no REST merge call and its only GraphQL mutation is enqueuePullRequest with expectedHeadOid
blockers:
  - Issue 1362 terminal PR #1367 active-task lifecycle reconciliation by the #1362 owner
next_action: qualify the final exact PR #1371 head after #1362 lifecycle reconciliation, then enqueue only through a legitimate normal Merge Queue route
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository integration-control PR path
source_branch_evidence: pending protected Merge Queue integration
```

## Notes

No production operation, protected-main bypass, force-push, secret value, or direct merge is authorized.