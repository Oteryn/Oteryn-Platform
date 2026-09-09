---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live Issue #1363 and Draft PR #1382 state
  - current protected main and exact-head platform-gate
optional_reads: []
---

# OTERYN-20260909-merge-queue-enqueue-executor

## Goal

Migrate the protected Platform executor to the native META 3.1 `merge-async` contract on Draft PR #1382, branch `fix/1363-native-merge-async-executor`, without expanding the six-path Issue #1363 scope.

## Acceptance criteria

- [x] The only positive mutation is REST `PUT .../merge-async` with an exact qualified `sha` and explicit `merge_action: merge_queue`.
- [x] Fresh preflight rejects wrong repository/PR/base/head/state/draft/origin and any latest exact-head `platform-gate` result other than `completed/success`.
- [x] A valid `202` records a server UUID and positive executor-owned sequence, followed immediately by same-UUID async readback and fresh target readback with a strictly greater executor sequence.
- [x] Missing/malformed/mismatched UUID, invalid causal sequence, retarget, head change, and stale or incomplete evidence fail closed; there is no automatic dequeue.
- [x] `200`/`409` return non-terminal reconciliation results without fabricated acceptance; `400`/`422` reject precisely and `403`/`404` classify `BLOCKED_CAPABILITY_UNAVAILABLE`.
- [x] `workflow_dispatch` is retained and the exact `/oteryn-mq-enqueue <40-hex-head>` PR-comment command is gated to `OWNER`, `MEMBER`, or `COLLABORATOR`.
- [x] The dedicated repository-scoped App token requests Contents: Write, Checks: Read, and Pull requests: Read only (plus implicit metadata); top-level `GITHUB_TOKEN` remains read-only.
- [x] External GitHub Actions references remain pinned to immutable full SHAs.
- [ ] Draft PR #1382 exact-head required CI is green and protected-main freshness is reconciled.
- [ ] After protected integration, a separate qualified provider PR proves the remaining real canary end to end.

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
  - protected META 3.1 at Oteryn/Oteryn@ed6c8c98605a7fbfea858e0ef616f89baa617262
  - dedicated repository-scoped GitHub App bootstrap in docs/operations/MERGE_QUEUE_EXECUTOR.md
blockers:
  - real native canary is intentionally deferred until this repaired executor reaches protected main
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T00:00:00Z
head: exact GREEN head is recorded in live Draft PR #1382 after publication
branch: fix/1363-native-merge-async-executor
pr: 1382
status: validating
terminal_pr_policy: keep_active_until_protected_integration_and_real_canary
meta_policy: OTERYN_ORGANIZATION_AGENT_POLICY 3.1.0 @ ed6c8c98605a7fbfea858e0ef616f89baa617262
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
  - RED was consumed at d0b6e38042f06f9504b8ea00cfb4d2a8c8a6b249 with 15 passing and 3 expected failing tests.
  - Native request body, response classification, receipt/readback causal ordering, target reread and connector authorization have deterministic coverage.
  - No credential values, product/runtime paths, protection settings, rulesets, required checks or deployment behavior changed.
unknown:
  - Exact-head GitHub CI result for the final published GREEN candidate.
  - Real native merge-async App-token canary receipt and later merge_group/protected-main proof.
derived:
  - The executor repair is governance/CI control-plane work, so product runtime E2E is not applicable.
  - A native acceptance receipt remains non-terminal until merge_group and protected-main evidence exist.
conflicts: []
first_failure:
  marker: awaiting-exact-head-ci
  evidence: local GREEN validation is in progress and exact-head GitHub CI requires publication
rejected_hypotheses:
  - A 202 response alone proves integration.
  - Existing 200 or 409 state permits fabricating a fresh acceptance receipt.
  - Capability denial permits a GraphQL, default-action or direct-merge fallback.
changed_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: python -m pytest tests/ci/test_merge_queue_enqueue.py -q
    result: PASS
    evidence: 25 tests and 20 subtests passed locally on the six-path GREEN worktree
  - command: repository policy/lifecycle/workflow validation
    result: PASS
    evidence: focused repository policy tests, governance unit suites, workflow YAML parsing and lifecycle JSON parsing passed locally
  - command: required exact-head CI
    result: NOT_RUN
    evidence: exact-head CI can start only after the committed GREEN candidate is published to Draft PR #1382
blockers:
  - real-canary acceptance remains after protected integration
next_action: Keep PR #1382 Draft, publish the six-path GREEN candidate, and wait for required exact-head CI before requesting stable independent review.
```

## Notes

Queue admission is not integration proof. Terminal proof remains a later real `merge_group` `platform-gate` plus protected-main readback. No direct/default merge, alternate GraphQL enqueue, bypass, force/rebase/reset, no-op retrigger, automated dequeue, credential inspection, or premature closeout is authorized.
