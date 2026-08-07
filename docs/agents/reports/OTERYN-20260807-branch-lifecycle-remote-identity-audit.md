# Audit report — Branch Lifecycle remote repository identity

## Scope

Repository-only audit of the destructive Branch Lifecycle boundary after the Issue #793 / PR #796 atomic-delete repair. The review focused on whether GitHub API safety evidence and the final force-with-lease operation are guaranteed to address the same authorized repository and working tree.

No destructive operation, implementation change, production/staging mutation or external-repository write was performed.

## Verdict

`FINDING — HIGH / P1 / PROVEN`

Finding: `OPA-GOV-0024` / Issue #815.

## Path reviewed

```text
configured --repo / GitHubClient.repo
  -> live branch/ref/PR safety reads
  -> configured --root for policy/tasks/evidence
  -> _delete_ref_with_lease()
  -> local git remote (default origin) + process CWD
  -> force-with-lease branch deletion
  -> GitHub API post-delete verification
```

## Primary evidence

1. Live safety reads are explicitly scoped to `GitHubClient.repo`.
2. `_delete_ref_with_lease()` performs the destructive operation using a local git remote that defaults to `origin`.
3. The local remote URL is not resolved or compared with `GitHubClient.repo` before the push.
4. The CLI resolves `--root`, but the destructive `subprocess.run()` does not use that root as `cwd` and the client is not bound to it.
5. Thus the repository selected for the destructive push can differ from the repository whose state was audited.
6. A same-SHA branch in a fork, mirror or different checkout can satisfy the force-with-lease and be deleted before post-delete verification against the intended repository notices the mismatch.
7. Existing tests prove SHA lease atomicity but not repository/worktree identity.

## Expected invariant

All evidence used to authorize a destructive branch mutation and the mutation itself must be bound to one explicit repository identity. A local checkout or remote mismatch must fail closed before any push.

## Actual behavior

GitHub API evidence is bound to `self.repo`; the destructive force-with-lease operation is bound implicitly to the caller's local git repository and selected remote.

## Impact

A misconfigured runner, nested invocation or manual execution with `--root` different from the current working directory can delete a branch in a repository outside the audited target. Post-delete verification can detect the inconsistency only after that unauthorized mutation has already occurred.

Because Branch Lifecycle is intentionally destructive and autonomous external-repository mutation is forbidden, this remains HIGH / P1 despite requiring a runner/configuration mismatch to trigger.

## Duplicate analysis

Issue #793 is related but distinct: it introduced server-enforced expected-SHA deletion using force-with-lease. It did not establish local remote or worktree identity. No separate actionable duplicate was found.

## Remediation handoff

Issue #815 owns remediation. The repair should bind the git subprocess to the configured root, normalize and validate the selected GitHub remote against `GitHubClient.repo`, fail closed before push on any mismatch, and add deterministic wrong-CWD/foreign-origin fixtures while preserving the expected-SHA lease behavior from Issue #793.

## Delivery completeness

Audit records and remediation handoff are complete for this bounded package. Runtime/product E2E is `NOT_APPLICABLE` because the audit delivery changes documentation/evidence only.
