---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - live PR #1384 and Issue #1363
  - protected META policy and current Platform main
optional_reads: []
---

# OTERYN-20260909 merge-async bridge retirement

## Goal

Retire the Platform-specific Merge Queue bridge after a legitimate authenticated Codex execution surface proved direct native REST `merge-async` end to end on Oteryn/Oteryn PR #193. Platform must not create or require a custom GitHub App or a repository PAT merely to submit a qualified pull request to Merge Queue.

The organization-owned positive route remains native REST `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge-async` with exact qualified `sha` and explicit `merge_action=merge_queue`, governed by protected META policy. Direct merge, generic auto-merge, GraphQL enqueue, default/direct merge actions, bypass, force and protection weakening remain forbidden substitutes.

## Scope

Retire the redundant repository-local bridge and its dedicated implementation surfaces:

- `.github/workflows/merge-queue-enqueue.yml`;
- `scripts/github/merge_queue_enqueue.py`;
- `tests/ci/test_merge_queue_enqueue.py`;
- `docs/operations/MERGE_QUEUE_EXECUTOR.md`.

Update `docs/agents/CI_WORKFLOW_LIFECYCLE.json` so the workflow is retired and the active workflow budget reflects the smaller inventory. Keep this task active only until #1384 protected integration and final organization/provider closeout evidence are reconciled.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T21:54:00Z
head: 08af62d1cd2a0a0cf17a080ac23b9af681ce666a
branch: fix/187-native-merge-async-no-app-auth
pr: 1384
status: validating
context_routes:
  - ci-repair
  - integration-control-plane
owned_paths:
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
proven:
  - META PR #193 used exactly one native merge-async PUT with exact qualified SHA and explicit merge_queue action, entered the real Merge Queue, passed merge_group META CI run 34409271210, and integrated as protected main 3b39e0be05aef008f1bd442821daefa898a201dd.
  - No new custom Oteryn GitHub App was required for the successful native submission.
  - The former Platform bridge is redundant once the authorized direct native execution surface is used organization-wide.
derived:
  - Removing the redundant bridge eliminates its App and PAT bootstrap surface instead of replacing one repository credential dependency with another.
unknown:
  - Fresh exact-head Platform CI and independent review result for the final retirement diff.
  - Final provider-repin and real provider canary receipt/readback evidence required by Oteryn/Oteryn#187.
conflicts: []
first_failure:
  marker: terminal-native-proof-pending
  evidence: provider rollout and receipt/readback proof remain after this bridge-retirement candidate qualifies and integrates
rejected_hypotheses:
  - A custom Oteryn GitHub App is required for native merge-async.
  - A repository PAT bridge is required when an already-authorized direct native execution surface can invoke merge-async.
  - Direct merge or generic auto-merge is an acceptable fallback.
changed_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: protected META PR #193 native merge-async integration proof
    result: PASS
    evidence: added_to_merge_queue 2026-09-09T21:52:10Z; merge_group META CI 34409271210 SUCCESS; protected main 3b39e0be05aef008f1bd442821daefa898a201dd
  - command: final Platform #1384 exact-head repository CI
    result: NOT_RUN
    evidence: candidate changed after bridge retirement and requires fresh exact-head GitHub Actions
blockers:
  - final Platform exact-head CI and independent review are required before queue submission
next_action: Run fresh exact-head Platform CI for the six-path retirement diff, repair any real finding, obtain fresh independent review, then submit #1384 through native merge-async only.
```

## Notes

Historical #1382/#1383 evidence remains in Git and GitHub history. The failed App-token canary is retained as provenance showing why the repository-local bridge was retired; it is not an active execution dependency.
