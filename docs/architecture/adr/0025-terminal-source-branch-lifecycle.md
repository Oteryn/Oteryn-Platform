# ADR 0025: Terminal source-branch lifecycle

- Status: Accepted
- Date: 2026-08-14
- Decision owner: repository owner
- Decision record: Issue #1050
- Extends: ADR 0024

## Context

ADR 0024 made ordinary merged pull-request source branches ephemeral and kept `delete_branch_on_merge=true`. That solved the merged path, including a one-time cleanup of 354 proven `TERMINAL_MERGED` refs in Issue #658.

A fresh inventory on 2026-08-14 still found 192 refs. The base classifier reported 115 `UNMERGED_ORPHAN` and 60 `UNKNOWN` refs in addition to eight open PRs, eight `TERMINAL_MERGED` refs and protected `main`. A stricter terminal classifier then proved 105 of the 115 unmerged orphans were exact current heads of one same-repository pull request that had already been closed without merge.

The remaining lifecycle gap is therefore not automatic deletion after merge. It is terminal work that intentionally or accidentally exits through a closed-unmerged PR, superseded/restacked attempt, diagnostic branch or other non-merge path and leaves the source ref behind.

Merging such work merely to trigger branch deletion would corrupt `main`: closed-unmerged work may be obsolete, diagnostic, superseded, incomplete or intentionally rejected.

## Decision

Every task source branch is an execution resource and must have an intentional terminal disposition before the task is `completed`.

### Merged PR

For an ordinary same-repository PR that is accepted into `main`:

- merge only because the content is accepted, never for cleanup convenience;
- rely on repository `delete_branch_on_merge=true`;
- verify the source ref disappears after merge;
- if automatic deletion does not occur, reconcile the exact ref through the Branch Lifecycle control.

### Closed PR without merge

Before intentionally closing a same-repository PR without merge, its body must contain exactly one disposition:

- `Branch-Disposition: delete`, or
- `Branch-Disposition: retain`,

and a non-empty `Branch-Disposition-Reason: ...`.

`delete` authorizes cleanup only after trusted-main automation proves all of the following at deletion time:

1. the PR is same-repository, closed and unmerged;
2. exactly one authoritative closed-unmerged PR matches the current source branch and exact head SHA;
3. there is no open PR for the ref;
4. there is no active task/claim or open deterministic repair Issue owning the ref;
5. the ref is not protected and is not an explicit retention exception;
6. the ref is not release, rollback, recovery or backup sensitive by the fail-closed reserved-name policy;
7. the default branch and source branch exact SHAs have not drifted.

The deletion must use exact-head semantics and verify the ref is absent afterwards.

`retain` requires a durable owner, purpose and review/expiry trigger. Retention without that evidence is a closeout defect.

### Historical reconciliation

Historical closed-unmerged refs are not merged merely to clean them. A one-time reviewable cleanup may delete a branch when the same fail-closed liveness checks pass and one exact same-repository closed-unmerged PR matches the current ref SHA. The reviewed candidate set is bound by count, canonical entries digest and policy digest; any live drift fails closed before apply.

PR number and immutable commit SHA remain the recovery authority. The cleanup package must run a bounded branch create/delete/restore/delete recovery test and preserve deletion evidence.

`UNKNOWN`, moved/ambiguous, protected, active, open-PR and recovery-sensitive refs remain untouched until separately reconciled.

## Consequences

### Positive

- Agents can no longer call a task complete while leaving an unexplained source branch.
- Both terminal paths—merged and intentionally closed-unmerged—now have deterministic cleanup.
- Abandoned or diagnostic work does not need to pollute `main` to become disposable.
- Closed PR and exact commit history provide provenance and restoration without indefinite branch retention.
- Future branch accumulation becomes a detectable closeout defect rather than normal repository state.

### Negative

- Intentionally retained branches require explicit lifecycle metadata.
- Historical refs with missing, conflicting or moved PR evidence remain fail-closed and require separate reconciliation.
- Cleanup automation adds a small trusted-main GitHub Actions surface with `contents: write`; destructive execution is therefore exact-head guarded and isolated from untrusted PR code.

## Rejected alternatives

- Merge every old branch into `main` before deletion.
- Delete all closed-PR branches without matching the current ref SHA.
- Delete by age, prefix or branch-name heuristics alone.
- Automatically delete recovery, rollback, release or backup-looking refs without an explicit retention/reconciliation decision.
- Let each agent perform ad-hoc branch deletion without repository-level evidence and exact-head controls.

## Implementation

Issue #1050 and PR #1056 implement this decision through:

- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
- `docs/agents/tasks/TASK_TEMPLATE.md`;
- `tools/agents/terminal_branch_cleanup.py`;
- `tools/agents/terminal_branch_approval.py`;
- `.github/workflows/terminal-branch-lifecycle.yml`.

ADR 0024 remains authoritative for ordinary merged-source branch behavior. This ADR adds the terminal closed-unmerged path and task-closeout requirement.
