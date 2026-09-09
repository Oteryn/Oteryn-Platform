---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live Issue #1363, PR #1383 canary, and current repair PR state
  - current protected main and exact-head platform-gate
optional_reads: []
---

# OTERYN-20260909-merge-queue-enqueue-executor

## Goal

Finish the repository-native exact-head `merge-async` executor without creating or depending on a custom GitHub App. Preserve the already-protected native endpoint/receipt semantics from #1382 while replacing its obsolete App-token bootstrap with a fine-grained PAT mutation credential and read-only built-in `GITHUB_TOKEN` preflight/readback.

## Acceptance criteria

- [x] The only positive mutation is REST `PUT .../merge-async` with an exact qualified `sha` and explicit `merge_action: merge_queue`.
- [x] Fresh preflight rejects wrong repository/PR/base/head/state/draft/origin and any latest exact-head `platform-gate` result other than `completed/success`.
- [x] A valid `202` records a server UUID and positive executor-owned sequence, followed by same-UUID async readback and fresh target readback with a strictly greater executor sequence.
- [x] `200`/`409` are reconciliation-only; `400`/`422` reject precisely; `403`/`404` fail closed.
- [x] The exact `/oteryn-mq-enqueue <40-hex-head>` PR-comment command remains gated to `OWNER`, `MEMBER`, or `COLLABORATOR`.
- [x] No custom GitHub App, App client ID, private key, or `actions/create-github-app-token` dependency remains.
- [x] Built-in `GITHUB_TOKEN` remains read-only and is used only for PR/check reads.
- [x] `OTERYN_MQ_TOKEN` is the sole mutation credential and is used only for native async PUT/status GET; required token permission is fine-grained repository **Contents: Read and write**.
- [ ] Follow-up repair PR exact-head CI and independent review are green.
- [ ] Repair integrates through normal protected Merge Queue.
- [ ] Existing canary PR #1383 is invoked exactly once after protected repair and proves `202 + UUID + strictly-later readback + merge_group + protected-main`.

## Ownership

```yaml
owned_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
modules:
  - repository integration control plane
dependencies:
  - protected native merge-async implementation from PR #1382 at main@57b775a932ad24a7fad5d7c9120f725a4c451374
  - one fine-grained PAT secret named OTERYN_MQ_TOKEN with selected-repository Contents write authority
blockers:
  - secret value cannot be created or inspected by repository code or the connected GitHub surface
cross_repository_tasks:
  - Oteryn/Oteryn#187 organization policy clarification and provider rollout
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-09-09T20:53:08Z
protected_main: 57b775a932ad24a7fad5d7c9120f725a4c451374
branch: fix/187-native-merge-async-no-app-auth
canary_pr: 1383
canary_head: d16d1eee2c11403bdd7be8c0b01146d1eaaf7b50
status: repairing-auth-bootstrap
proven:
  - PR #1382 integrated through real Merge Queue; merge_group run 34403134530 and platform-gate job 102640346898 succeeded; protected main readback is 57b775a932ad24a7fad5d7c9120f725a4c451374.
  - Canary #1383 exact-head platform-gate job 102641164484 succeeded.
  - Canary issue_comment run 34403686876 authorized the exact invocation successfully.
  - The first canary failed before merge-async at job 102641451100 because OTERYN_MQ_APP_CLIENT_ID was empty; no native mutation was attempted.
  - GitHub documents fine-grained PAT support for merge-async with Contents write and documents GITHUB_TOKEN event suppression for most workflow-triggering events.
  - The repair separates read-only GITHUB_TOKEN PR/check traffic from fine-grained PAT merge-async mutation/status traffic.
unknown:
  - Whether OTERYN_MQ_TOKEN is already configured on the repository/organization Actions secret surface.
  - Final real 202 receipt UUID and merge_group proof after the app-free repair reaches protected main.
rejected_hypotheses:
  - A custom GitHub App is required for native merge-async.
  - GITHUB_TOKEN is safe as the queue mutation credential when merge_group workflow execution is required.
  - The failed App-token canary proves merge-async itself is unavailable.
next_action: Qualify and merge the app-free repair normally, then invoke the same #1383 canary once with OTERYN_MQ_TOKEN configured; never create a custom GitHub App.
```

## Notes

Queue admission is non-terminal. No direct/default merge, GraphQL enqueue, bypass, force/rebase/reset, no-op retrigger, automated dequeue, credential inspection, or custom-App bootstrap is authorized.
