# Evidence — OTERYN-20260807 branch lifecycle remote identity audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `7ae96633871e1d970f22d8de69499adb3d1e6d37`
- Finding: `OPA-GOV-0024` / Issue #815
- Risk / priority: HIGH / P1

## Destructive-boundary evidence

`GitHubClient` binds live branch, PR and ref checks to `self.repo` through GitHub API requests.

The destructive deletion path is different. `_delete_ref_with_lease()` selects a local remote using `getattr(self, "git_remote", "origin")` and executes `git push --porcelain --force-with-lease=refs/heads/<branch>:<expected_sha> <remote> :refs/heads/<branch>`.

Before that push, the method does not resolve the selected remote URL or prove that it points to `self.repo`.

The CLI accepts `--root` and resolves it for policy, task and evidence paths, but the destructive subprocess is not executed with `cwd` bound to that root and the `GitHubClient` receives no root/worktree identity. The git repository and `origin` used by the destructive command therefore depend on the process current working directory rather than the configured audit root.

## Falsified repository-identity invariant

Safety reads can validate repository A while the destructive force-with-lease operation is sent to repository B selected by the local git checkout/remote.

If repository B contains the same branch at the reviewed expected SHA, the force-with-lease deletion can succeed there. The later API verification against repository A detects that the intended deletion did not occur only after repository B may already have been changed.

## Regression evidence

`test_github_client_atomic_delete_uses_exact_remote_lease_without_token` proves expected-SHA force-with-lease command construction and explicitly expects `origin`. `test_github_client_atomic_delete_rejects_last_instruction_race` proves a remote SHA advance rejects deletion.

No inspected fixture proves that `origin` belongs to `GitHubClient.repo`, that `--root` controls the destructive git working tree, or that foreign/missing/ambiguous remote identity fails before the push.

## Duplicate search

Issue #793 / `OPA-GOV-0022` repaired remote expected-SHA atomicity. It did not bind the local destructive remote to the audited GitHub repository identity. Searches for Branch Lifecycle remote/origin mismatch, wrong-repository deletion and root/CWD mismatch found no independent actionable owner; Issue #815 now owns remediation.

## Safety

No branch deletion, external-repository write, production/staging mutation or executable implementation change was performed by this audit.
