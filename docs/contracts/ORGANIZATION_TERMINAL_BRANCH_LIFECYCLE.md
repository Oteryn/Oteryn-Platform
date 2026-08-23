# Organization Terminal Branch Lifecycle Contract

Status: active after merge of Issue #1230 implementation.

## Scope

This contract exposes the proven Oteryn Platform branch-lifecycle implementation to Oteryn repositories without sharing a cross-repository write credential.

Canonical implementation paths:

- `.github/workflows/terminal-branch-lifecycle-read-reusable.yml` — read-only inventory and approval validation;
- `.github/workflows/terminal-branch-lifecycle-reusable.yml` — write-capable `close` / `apply` operations only;
- `tools/agents/branch_lifecycle.py`;
- `tools/agents/terminal_branch_cleanup.py`;
- `tools/agents/terminal_branch_approval.py`;
- `docs/architecture/adr/0037-terminal-source-branch-lifecycle.md`.

## Caller requirements

A caller repository MUST:

1. pin both reusable workflow `uses:` references to one full 40-character merged Oteryn-Platform commit SHA;
2. pass that same SHA as `platform_ref`, so workflow definitions and checked-out lifecycle implementation cannot drift independently;
3. use the read-only reusable workflow for `pull_request`, `schedule`, and manual inventory;
4. use the write-capable reusable workflow only for explicit `close` and `apply` calls;
5. keep `docs/agents/BRANCH_LIFECYCLE_POLICY.json` and its accepted ADR in the caller repository;
6. use only the caller repository `GITHUB_TOKEN` for live inventory and deletion;
7. grant `contents: write` only to explicit `close` and `apply` calls;
8. trigger `close` only from `pull_request_target` `closed` events and rely on reusable-workflow event guards;
9. trigger `apply` only from protected `main` push after a reviewed historical approval is merged;
10. keep scheduled/manual inventory strictly read-only.

## Required operations

`read` is implemented by `.github/workflows/terminal-branch-lifecycle-read-reusable.yml`. It validates caller policy and produces fail-closed live inventory plus a generated historical-candidate manifest. The workflow contains no `contents: write` permission and no deletion operation.

`close` is implemented by `.github/workflows/terminal-branch-lifecycle-reusable.yml` and consumes a closed same-repository PR event. `Branch-Disposition: delete` is effective only after the implementation revalidates exact PR identity, current branch SHA, no open PR, no active task/open repair Issue, no protection/retention exception, and no reserved recovery-sensitive name. `retain` or unspecified disposition never deletes.

`apply` is implemented by the same write-capable reusable workflow and is for separately reviewed historical cleanup only. The approval binds candidate count, canonical entries digest and policy digest. The live set is rebuilt on `main`; any drift fails closed before deletion.

## Security boundary

GitHub reusable-workflow `GITHUB_TOKEN` permissions can only be maintained or reduced through a caller/called chain. The read path is therefore physically separated from write-capable jobs instead of asking a read-only caller to invoke a workflow definition that also declares write jobs.

Both reusable workflows check out the caller repository first, then check out Oteryn-Platform tooling into `.oteryn-branch-lifecycle` with `persist-credentials: false`. The caller checkout therefore remains the only authenticated git remote used by exact-head deletion.

Write operations never execute source code from an untrusted PR. Close-event cleanup checks out caller `main`; PR metadata is data, not executable authority.

The workflow never deletes by branch age, prefix or inactivity alone. Protected/default, open-PR, active-claim, release, rollback, recovery, backup-sensitive, retained, moved or ambiguous refs remain untouched.

## Upgrade procedure

A caller may move to a newer Platform implementation only by a normal PR that updates both reusable `uses:` SHAs and every `platform_ref` to the same reviewed merged Platform commit. Caller CI and read-only lifecycle inventory must pass before merge.

## Historical cleanup

Initial adoption does not imply permission to delete pre-existing orphan refs. Each repository must first review its generated inventory and, when desired, merge an exact `TERMINAL_BRANCH_DELETION_APPROVAL.json` bound to the reviewed candidate set. Ambiguous refs remain a separate reconciliation task.
