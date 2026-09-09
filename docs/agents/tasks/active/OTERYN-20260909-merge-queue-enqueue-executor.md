---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live Issue #1363, PR #1384, PR #1383 canary, and current protected main
  - current exact-head platform-gate and Merge Queue state
optional_reads: []
---

# OTERYN-20260909-merge-queue-enqueue-executor

## Goal

Finish the repository-native exact-head `merge-async` executor without creating or depending on a custom GitHub App. Preserve the protected native endpoint/receipt semantics from #1382 while replacing its obsolete App-token bootstrap with a fine-grained PAT mutation credential and read-only built-in `GITHUB_TOKEN` preflight/readback.

## Acceptance criteria

- [x] The only positive mutation is REST `PUT .../merge-async` with an exact qualified `sha` and explicit `merge_action: merge_queue`.
- [x] Fresh preflight rejects wrong repository/PR/base/head/state/draft/origin and any latest exact-head `platform-gate` result other than `completed/success`.
- [x] A valid `202` records a server UUID and positive executor-owned sequence, followed by same-UUID async readback and fresh target readback with a strictly greater executor sequence.
- [x] `200`/`409` are reconciliation-only; `400`/`422` reject precisely; `403`/`404` fail closed.
- [x] The exact `/oteryn-mq-enqueue <40-hex-head>` PR-comment command remains gated to `OWNER`, `MEMBER`, or `COLLABORATOR`.
- [x] No custom GitHub App, App client ID, private key, or `actions/create-github-app-token` dependency remains.
- [x] Built-in `GITHUB_TOKEN` remains read-only and is used only for PR/check reads.
- [x] `OTERYN_MQ_TOKEN` is the sole mutation credential and is used only for native async PUT/status GET; required token permission is fine-grained repository **Contents: Read and write**.
- [ ] Follow-up repair PR #1384 exact-head CI and independent review are green.
- [ ] Repair integrates through normal protected Merge Queue.
- [ ] Existing canary PR #1383 is invoked exactly once after protected repair and proves `202 + UUID + strictly-later readback + merge_group + protected-main`.

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
  - protected native merge-async implementation from PR #1382 at main@57b775a932ad24a7fad5d7c9120f725a4c451374
  - one fine-grained PAT secret named OTERYN_MQ_TOKEN with selected-repository Contents write authority
blockers:
  - real native canary requires OTERYN_MQ_TOKEN to exist on the Actions secret surface after this repair is protected
cross_repository_tasks:
  - Oteryn/Oteryn#187 organization policy clarification and provider rollout
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T21:16:05Z
head: c43d452feb0e8ae43ffa7d14d7a2dcd38842ba5c
branch: fix/187-native-merge-async-no-app-auth
pr: 1384
status: implementing
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
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
proven:
  - PR #1382 integrated through real Merge Queue; merge_group run 34403134530 and platform-gate job 102640346898 succeeded; protected main readback is 57b775a932ad24a7fad5d7c9120f725a4c451374.
  - Canary #1383 exact-head platform-gate job 102641164484 succeeded.
  - Canary issue_comment run 34403686876 authorized the exact invocation successfully.
  - First canary failed before merge-async at job 102641451100 because the inherited custom-App client ID was empty; no native mutation occurred.
  - The repair removes custom-App bootstrap and separates read-only GITHUB_TOKEN traffic from fine-grained PAT merge-async mutation/status traffic.
  - Five supporting exact-head workflows on initial #1384 head c43d452f completed successfully; CI and Agent Governance failed on deterministic task-checkpoint contract only.
unknown:
  - Whether OTERYN_MQ_TOKEN is already configured on the repository or organization Actions secret surface.
  - Final real 202 receipt UUID and merge_group proof after the app-free repair reaches protected main.
derived:
  - The initial #1384 required-test-gate failure is downstream of failed change classification, not an independent runtime/auth defect.
  - No unchanged failed workflow should be rerun; this checkpoint repair creates a new exact candidate naturally.
  - A native acceptance receipt remains non-terminal until merge_group and protected-main evidence exist.
conflicts: []
first_failure:
  marker: active-task-checkpoint-contract
  evidence: CI run 34405925984 job 102648744121 rejected checkpoint_version=2, unsupported status, and missing canonical checkpoint fields before change classification.
rejected_hypotheses:
  - A custom GitHub App is required for native merge-async.
  - GITHUB_TOKEN is safe as the queue mutation credential when merge_group workflow execution is required.
  - The first canary failure proves merge-async itself is unavailable.
changed_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: initial exact-head GitHub CI on c43d452feb0e8ae43ffa7d14d7a2dcd38842ba5c
    result: FAIL
    evidence: CI run 34405925984 failed change classification only because this checkpoint did not match the repository schema; required test gate then failed closed from classification=failure.
  - command: supporting exact-head workflows on c43d452feb0e8ae43ffa7d14d7a2dcd38842ba5c
    result: PASS
    evidence: Platform DB Outage Validation, Game Auth Ticket Concurrency, CodeQL, Phase 7 Production-Like Validation, and Edge Security Emulation completed successfully.
blockers:
  - exact-head CI must rerun naturally on the repaired checkpoint commit and prove the five-path candidate.
  - real canary remains pending protected repair integration and OTERYN_MQ_TOKEN availability.
next_action: Validate the new exact head, repair only deterministic findings if any, obtain fresh independent review, integrate #1384 through normal Merge Queue, then retry existing canary #1383 once.
```

## Notes

Queue admission is non-terminal. No direct/default merge, GraphQL enqueue, bypass, force/rebase/reset, no-op retrigger, automated dequeue, credential inspection, or custom-App bootstrap is authorized.
