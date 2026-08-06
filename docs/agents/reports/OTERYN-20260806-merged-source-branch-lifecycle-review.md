# Merged source-branch lifecycle review

Date: 2026-08-06
Repository: `blakinio/Oteryn-Platform`
Issue: #586
Decision: `ARCH-DEC-0001`
Accepted ADR: 0024
Implementation handoff: #658

## Result

The original Issue #586 setting evidence is stale: live repository metadata reports:

- `delete_branch_on_merge=true`;
- `allow_squash_merge=true`;
- `allow_merge_commit=false`;
- `allow_rebase_merge=false`;
- `allow_auto_merge=true`.

The repository owner explicitly selected Option A on 2026-08-06. ADR 0024 therefore establishes automatic deletion of ordinary merged PR branches with protected, documented and reviewable exceptions.

## Inventory signal

A complete branch listing returned 498 branch refs, including `main` and the decision branch. The inventory contains large historical families under `task/`, `docs/`, `chore/`, `audit/`, `feat/`, `fix/`, `ops/`, `test/`, `archive/`, `backup/`, `repair/`, `trigger/`, `sync/` and other prefixes.

This count proves material accumulated ambiguity but does not prove that any individual branch is safe to delete. Classification against open PRs, active task claims, protection/rulesets, terminal merge state and recovery dependencies belongs to the separate implementation package in Issue #658.

## Accepted policy

1. Keep automatic merged-head deletion enabled.
2. Keep squash as the sole merge method.
3. Treat ordinary PR source branches as ephemeral.
4. Require repository-rule or branch-protection enforcement for every approved long-lived exception.
5. Require owner, reason and review/expiry metadata for every exception.
6. Use PR/commit history and branch restoration as ordinary recovery.
7. Perform a deterministic one-time inventory and cleanup.
8. Delete only branches proven `TERMINAL_MERGED` and unnecessary for recovery.
9. Fail closed for open PRs, active claims, release/rollback/recovery roles, protected branches, unmerged work and `UNKNOWN` state.

## Implementation boundary

No branch was deleted and no repository setting was changed by the decision package.

Issue #658 owns implementation, dry-run classification, retention metadata, cleanup evidence and recovery proof. `ARCH-DEC-0001` is removed from the active decision backlog because the policy question is resolved; implementation completion remains operational work rather than an unresolved architecture decision.
