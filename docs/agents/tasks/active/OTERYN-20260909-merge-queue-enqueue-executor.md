---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live Issue #1363 state and repository-native queue-executor bootstrap evidence
  - current protected main and required platform-gate
optional_reads: []
---

# OTERYN-20260909-merge-queue-enqueue-executor

## Goal

Governing GitHub Issue: #1363 — add one repository-native, fail-closed queue enqueue executor so qualified PRs do not depend on an online maintainer workstation.

## Acceptance criteria

- [x] Executor only performs GraphQL `enqueuePullRequest` with exact expected-head fencing.
- [x] Wrong repository/base/head/state/draft/origin or non-success exact-head `platform-gate` fails before mutation by deterministic code paths.
- [x] GitHub App token is short-lived, repository-scoped, minimum-permission, and no credential value is committed.
- [x] External GitHub Actions references are immutable full SHAs.
- [x] Focused tests and repository-required exact-head checks pass.
- [x] Integration itself used the normal protected queue with no direct endpoint or protection bypass.
- [ ] One-time GitHub App configuration is proven and one qualified PR is successfully queued by the repository-native executor from `main`.

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
  - dedicated repository-scoped GitHub App bootstrap described in docs/operations/MERGE_QUEUE_EXECUTOR.md
blockers:
  - one-time App variable/secret configuration and one successful repository-native enqueue proof are not yet independently observable from the connected GitHub surface
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T07:20:00Z
head: 4a81631f38e172921ddc42a940426f37e05914ac
branch: ci/1363-merge-queue-enqueue-executor
pr: 1371
status: validating
terminal_pr_policy: archive_pending
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
  - Lifecycle-only PR #1372 reconciled Issue #1362 and integrated first as protected commit 7902564e8251c55f273cba277ce138d8a9b99bc8.
  - PR #1371 then integrated through the configured protected queue as current protected main 4a81631f38e172921ddc42a940426f37e05914ac.
  - main remains protected and still requires platform-gate.
  - PR #1371 source branch ci/1363-merge-queue-enqueue-executor is absent after integration.
  - The delivered manual-only workflow exists on protected main and uses pinned actions/create-github-app-token with checks:read, pull-requests:read and merge-queues:write, while GITHUB_TOKEN remains contents:read.
  - The enqueue script performs only GraphQL enqueuePullRequest with expectedHeadOid after exact repository, PR state/base/origin/head and exact-head platform-gate checks.
  - Focused test module passed fifteen deterministic fail-closed cases on the implementation candidate.
  - The current resulting-main Agent Governance failure is lifecycle bookkeeping only: run 34323008921 reports terminal_pr_stale_next_action and terminal_pr_active_task for this task after PR #1371 became terminal.
derived:
  - Product and integration-control implementation is protected-main released; only bootstrap proof and terminal archival remain.
  - Keeping the task active without archive-pending state is no longer valid after terminal PR #1371.
unknown:
  - Whether OTERYN_MQ_APP_CLIENT_ID and OTERYN_MQ_APP_PRIVATE_KEY are already configured with the documented dedicated App.
  - Whether the first protected-main executor dispatch can mint the scoped App token and queue a qualified PR successfully.
conflicts: []
first_failure:
  marker: post-integration-active-task-liveness
  evidence: resulting-main Agent Governance run 34323008921 reports stale terminal next action and terminal PR active-task findings for PR #1371
rejected_hypotheses:
  - PR #1371 is still awaiting protected integration.
  - The resulting-main governance red state is an implementation or platform-gate failure.
  - A direct endpoint should be used to prove the executor.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: protected integration readback for PR #1371
    result: PASS
    evidence: PR #1371 is merged as current protected main 4a81631f38e172921ddc42a940426f37e05914ac and its source branch is absent
  - command: resulting-main branch protection readback
    result: PASS
    evidence: main remains protected and requires platform-gate
  - command: resulting-main Agent Governance
    result: FAIL
    evidence: run 34323008921 fails only on terminal task lifecycle for this task; deterministic policy, checkpoint, source-branch, prompt and governing-Issue validation steps pass
  - command: repository-native executor bootstrap proof
    result: NOT_RUN
    evidence: connected GitHub automation cannot inspect credential values or dispatch this workflow; bootstrap remains fail-closed until an observable executor run exists
blockers:
  - one-time App configuration and first successful repository-native enqueue proof
next_action: Archive this task after the dedicated GitHub App is configured on Oteryn/Oteryn-Platform and one qualified PR is successfully queued by the repository-native executor from main.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: original implementation ownership ended when PR 1371 reached protected main
source_branch_evidence: branch ci/1363-merge-queue-enqueue-executor is absent after protected integration as main 4a81631f38e172921ddc42a940426f37e05914ac
```

## Notes

Implementation is released. This task intentionally remains active only as `archive_pending` until the documented one-time App bootstrap and one real repository-native enqueue are proven. No direct endpoint, protection bypass, force-push, secret value, or credential disclosure is authorized.