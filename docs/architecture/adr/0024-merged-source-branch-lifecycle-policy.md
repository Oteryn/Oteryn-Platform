# ADR 0024: Merged source-branch lifecycle policy

- Status: Proposed
- Date: 2026-08-06
- Decision owner: repository owner
- Decision record: Issue #586 / `ARCH-DEC-0001`

## Context

The repository currently uses squash-only pull-request merges, allows auto-merge and reports `delete_branch_on_merge=true`. The original audit evidence in Issue #586 that automatic deletion was disabled is therefore historical and no longer current.

The remaining problem is policy, not the repository toggle. The repository has 498 branch refs, dominated by old task, documentation, audit, feature, fix, operations and closeout branches. No accepted source defines:

- which merged head branches are disposable;
- which long-lived branches may survive a merge;
- how a retention exception is declared and expires;
- how deleted work is recovered;
- how the existing branch inventory is reconciled without deleting active or ambiguous work.

GitHub documents that automatic head-branch deletion can be prevented by branch protection rules or repository rules. Protected branches are non-deletable by default unless deletion is explicitly allowed. This makes a fail-closed protected exception model compatible with the currently enabled repository setting.

## Options

### Option A — automatic deletion with explicit protected exceptions

Keep `delete_branch_on_merge=true`.

Ordinary task, repair, documentation, audit, feature, fix, test, operations, synchronization and temporary pull-request head branches are ephemeral and should disappear after merge.

A branch may remain long-lived only when all of the following are true before its pull request merges:

1. its exact name or pattern is protected by a repository rule or branch-protection rule that prevents deletion;
2. a canonical task, release or recovery record identifies its owner, purpose and review/expiry condition;
3. it belongs to an approved class such as the default branch, a release line, an active rollback line or an explicitly authorized recovery branch;
4. its continued existence does not conflict with an open pull request, active work claim or newer accepted authority.

Recovery for an ordinary deleted branch uses the pull request, immutable commit SHA and GitHub branch restoration rather than indefinite retention.

A one-time reconciliation classifies each existing branch as `PROTECTED`, `OPEN_PR`, `ACTIVE_CLAIM`, `RELEASE`, `ROLLBACK`, `RECOVERY`, `TERMINAL_MERGED`, `UNMERGED_ORPHAN` or `UNKNOWN`. Only `TERMINAL_MERGED` branches proven unnecessary for recovery may be deleted automatically. `UNKNOWN`, conflicting and unmerged branches fail closed.

### Option B — disable automatic deletion and require manual closeout

Set `delete_branch_on_merge=false`. Every merged branch must be deleted or explicitly retained during task closeout.

This maximizes manual control but recreates the failure mode already evidenced by hundreds of retained historical branches and makes policy compliance depend on every agent completing an extra repository-administration action.

### Option C — prefix-scoped hybrid policy

Keep automatic deletion for selected standardized prefixes while manually retaining and reviewing all other branch classes.

GitHub's repository-level automatic deletion setting does not itself provide arbitrary prefix-specific deletion behavior. This option therefore requires additional rules or automation, depends heavily on naming discipline and leaves nonstandard branches ambiguous.

## Proposed decision

Adopt **Option A**.

The accepted policy would be:

- keep squash merge as the sole merge method;
- keep automatic deletion of merged head branches enabled;
- treat ordinary PR source branches as ephemeral;
- require branch/ruleset protection plus a durable owner, purpose and expiry/review condition for every retention exception;
- use pull-request history, commit SHAs and branch restoration as the normal recovery mechanism;
- execute one separate, reviewable cleanup package that inventories all retained branches and deletes only proven terminal merged branches;
- never delete `main`, a protected branch, an open-PR branch, an active-claim branch, a release/rollback/recovery branch or any `UNKNOWN`/conflicting branch;
- record cleanup evidence and rollback/recovery proof before Issue #586 closes.

## Consequences

### Positive

- New merged task branches stop accumulating automatically.
- Retention becomes explicit, reviewable and fail-closed.
- Recovery remains possible without preserving every historical ref forever.
- Branch discovery and task ownership become less ambiguous.
- One-time cleanup can be separated from the durable policy decision and independently audited.

### Negative

- Legitimate long-lived source branches must be protected before merge.
- The initial 498-branch inventory requires a conservative reconciliation package.
- Exceptional branch retention needs an owner and lifecycle metadata rather than an informal convention.

## Rejected shortcuts

- Bulk-delete branches by age or prefix without resolving live PR, claim, recovery and protection state.
- Treat the already-enabled repository toggle as a complete policy.
- Infer that every old `archive/`, `docs/`, `task/` or `repair/` branch is safe to delete.
- Keep all branches indefinitely as a substitute for immutable commit and PR history.

## Activation boundary

This ADR remains proposed until the repository owner explicitly accepts A, B or C in Issue #586 or the associated pull request.

Acceptance authorizes a separate bounded implementation and cleanup package. It does not authorize branch deletion from this decision PR and does not by itself prove that existing branches are terminal or safe to remove.

## References

- Issue #586
- `docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json` (`ARCH-DEC-0001`)
- `docs/agents/EXECUTION_PROTOCOL.md`
- `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md`
- GitHub Docs: Managing the automatic deletion of branches
- GitHub Docs: About protected branches
- GitHub Docs: Deleting and restoring branches in a pull request
