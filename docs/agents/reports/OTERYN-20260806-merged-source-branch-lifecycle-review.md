# Merged source-branch lifecycle review

Date: 2026-08-06
Repository: `blakinio/Oteryn-Platform`
Issue: #586
Decision: `ARCH-DEC-0001`

## Result

The original Issue #586 setting evidence is stale: live repository metadata now reports:

- `delete_branch_on_merge=true`;
- `allow_squash_merge=true`;
- `allow_merge_commit=false`;
- `allow_rebase_merge=false`;
- `allow_auto_merge=true`.

The operational gap remains because no accepted policy defines retained-branch exceptions, recovery, expiry, one-time cleanup or fail-closed classification.

## Inventory signal

A complete branch listing returned 498 branch refs, including `main` and the current decision branch. The inventory contains large historical families under `task/`, `docs/`, `chore/`, `audit/`, `feat/`, `fix/`, `ops/`, `test/`, `archive/`, `backup/`, `repair/`, `trigger/`, `sync/` and other prefixes.

This count proves material accumulated ambiguity but does not prove that any individual branch is safe to delete. Classification against open PRs, active task claims, protection/rulesets, terminal merge state and recovery dependencies belongs to a separate post-decision cleanup package.

## Platform behavior

Official GitHub documentation confirms:

- repository administrators can enable automatic deletion of pull-request head branches after merge;
- branch protection rules and repository rules can prevent an otherwise automatic deletion;
- protected branches prevent deletion by default unless branch deletion is explicitly enabled;
- a deleted branch associated with a pull request can be restored when GitHub still exposes the restore action.

These capabilities support Option A without custom workflow automation: ordinary PR branches remain disposable, while approved long-lived exceptions are protected before merge.

## Option comparison

| Option | Safety | Ongoing cost | Determinism | Main risk |
|---|---|---:|---:|---|
| A — automatic deletion with protected exceptions | High when exceptions fail closed | Low | High | Requires precise protection and retention metadata |
| B — manual deletion during every closeout | Medium | High | Low | Human/agent omission recreates branch accumulation |
| C — prefix-scoped hybrid | Medium | Medium/high | Medium | Naming drift and extra automation create ambiguity |

## Recommendation

Adopt Option A:

1. keep automatic deletion enabled;
2. keep squash as the sole merge method;
3. declare ordinary PR source branches ephemeral;
4. require repository-rule or branch-protection enforcement for every approved long-lived exception;
5. require owner, reason and review/expiry metadata for exceptions;
6. use PR/commit history and branch restoration as ordinary recovery;
7. perform a separate deterministic one-time inventory and cleanup;
8. delete only branches proven `TERMINAL_MERGED` and unnecessary for recovery;
9. fail closed for open PRs, active claims, release/rollback/recovery roles, protected branches, unmerged work and `UNKNOWN` state.

## Decision boundary

No branch was deleted and no repository setting was changed by this review.

The associated ADR 0024 is `Proposed`. The repository owner must explicitly choose A, B or C before implementation, cleanup or closure of Issue #586.
