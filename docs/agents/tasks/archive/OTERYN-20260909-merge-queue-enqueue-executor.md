---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
status: completed_on_merge
project_lane: oteryn-platform-core
execution_mode: github_connector
delivery_pull_request: 1378
delivery_branch: docs/issue-1363-merge-queue-executor-closeout
risk: low
validation_intensity: STANDARD
ownership: releases_on_merge
source_branch_disposition: auto_delete_after_merge
---

# OTERYN-20260909 Merge Queue enqueue executor — conditional terminal closeout

## Archive condition

```yaml
archive_state:
  status: completed_on_merge
  effective_when:
    pull_request: 1378
    branch: docs/issue-1363-merge-queue-executor-closeout
    merged: true
  invalidated_by:
    - PR #1378 closes without merge
    - exact PR #1378 head does not pass required platform-gate
    - exact PR #1378 head does not pass Agent Governance
    - first repository-native Merge Queue Enqueue run does not successfully queue this exact qualified PR #1378 head
    - merge occurs through any path other than the configured protected Merge Queue
```

This record is conditional until PR #1378 is successfully queued by `.github/workflows/merge-queue-enqueue.yml` from protected `main` and merges through the configured Merge Queue. It does not claim the bootstrap proof in advance.

## Delivered implementation

- PR #1371 delivered the repository-native manual-only Merge Queue enqueue executor and merged through the configured protected queue as `4a81631f38e172921ddc42a940426f37e05914ac`.
- PR #1373 reconciled the executor task to `archive_pending` and merged through the configured protected queue as `65de7b61c2120dda35f3d49797cb55027885247b`.
- `.github/workflows/merge-queue-enqueue.yml` exists on protected `main`, mints a short-lived repository-scoped GitHub App token, and requests only Checks: Read, Pull requests: Read and Merge queues: Write.
- `scripts/github/merge_queue_enqueue.py` performs exact repository/PR/base/origin/head/platform-gate qualification before its only integration mutation: GraphQL `enqueuePullRequest` with `expectedHeadOid`.
- Focused deterministic tests and exact-head repository checks passed on the implementation delivery.
- Historical branch hygiene was subsequently reconciled; rerun attempt 4 of Historical Branch Audit run `34323397748` passed with `branches=7 new_unexplained=0 findings=0`.

## Remaining proof bound to this closeout

The final acceptance item is intentionally made self-proving: PR #1378 itself is the first qualified same-repository PR that must be queued by the protected-main `Merge Queue Enqueue` workflow. A successful executor run plus normal Merge Queue integration proves the one-time App configuration, short-lived token minting, exact-head qualification, and `enqueuePullRequest(expectedHeadOid)` end to end.

## Final validation gate

Before this conditional archive becomes effective, all of the following must be true:

- exact PR #1378 head is open, non-draft, same-repository and based on `main`;
- exact PR #1378 head `platform-gate` is completed successfully;
- Agent Governance passes on the exact PR #1378 head;
- whole-diff review has zero material findings and zero unresolved review threads;
- a `Merge Queue Enqueue` workflow-dispatch run from `main` targets PR #1378 and its exact qualified head SHA and completes successfully;
- GitHub records PR #1378 entering the configured Merge Queue without direct merge, ordinary auto-merge substitution, protection bypass or force-push;
- the Merge Queue candidate passes required `platform-gate` and integrates to protected `main`;
- resulting-main governance and required CI remain healthy.

## Security boundary

No App private key, token, credential value, repository secret or variable value is recorded in Git, Issue text, PR text or task evidence. Secret configuration remains an external repository setting. The executor remains fail-closed if that configuration is absent or invalid.

## Ownership release

On successful protected Merge Queue integration after the repository-native executor proof, Issue #1363 may be closed as completed and ownership of the executor implementation paths is released.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: this branch is the terminal lifecycle/proof carrier for Issue 1363 and has no continuing ownership purpose after successful executor-driven protected integration
source_branch_evidence: delete only after PR #1378 is queued by the repository-native executor, merges through the configured Merge Queue, and protected-main readback succeeds
```

## Notes

`completed_on_merge` is conditional. If the executor cannot queue this exact PR or PR #1378 closes unmerged, this record does not establish completed work and Issue #1363 remains open.
