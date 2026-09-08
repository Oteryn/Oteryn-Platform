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

- [ ] Executor only performs GraphQL `enqueuePullRequest` with exact expected-head fencing.
- [ ] Wrong repository/base/head/state/draft/origin or non-success exact-head `platform-gate` fails before mutation.
- [ ] GitHub App token is short-lived, repository-scoped, minimum-permission, and no credential value is committed.
- [ ] External GitHub Actions references are immutable full SHAs.
- [ ] Focused tests and repository-required exact-head checks pass.
- [ ] Integration itself uses normal Merge Queue with no direct-merge or protection bypass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
modules:
  - repository integration control plane
dependencies:
  - Issue 1362 post-merge reconciliation remains separate and owns only its exact current paths
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T23:23:49Z
head: 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce
branch: ci/1363-merge-queue-enqueue-executor
pr: none
status: implementing
context_routes:
  - ci-repair
  - execution-resources
owned_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
proven:
  - Protected main is 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce and requires platform-gate.
  - PR #1367 merged Issue #1362 repair; the remaining #1362 active ownership is narrowed to exact paths and no longer owns all .github/workflows or tools.
  - No existing branch, PR, workflow, or script implementing enqueuePullRequest was found before this task branch was created.
  - Issue #1363 remains open and unowned.
derived:
  - The #1363 implementation can proceed on the exact owned paths without overlapping #1362.
unknown:
  - Exact final CI result and first legitimate Merge Queue bootstrap path for this executor PR.
conflicts: []
first_failure:
  marker: missing-repository-native-merge-queue-enqueue-route
  evidence: connected GitHub automation exposes no enqueuePullRequest mutation and existing fallback depends on an external host
rejected_hypotheses:
  - immediate REST merge is an acceptable substitute for Merge Queue
  - ordinary auto-merge can replace the repository Merge Queue path
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: LIVE overlap and protected-main inspection
    result: PASS
    evidence: #1362 ownership is exact and disjoint from the planned #1363 paths; current main and required platform-gate were re-read before branch creation
blockers:
  - none
next_action: implement the fail-closed enqueue script, workflow, focused tests, and one-time configuration documentation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository integration-control PR path
source_branch_evidence: pending protected Merge Queue integration
```

## Notes

No production operation, protected-main bypass, force-push, secret value, or direct merge is authorized.