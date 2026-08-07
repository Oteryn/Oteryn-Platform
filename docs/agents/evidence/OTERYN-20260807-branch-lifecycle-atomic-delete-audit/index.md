# Evidence — OTERYN-20260807 branch lifecycle atomic-delete audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `993b3561feb75644d4a07f3e3377020be051eed6`
- Completed parent repair: Issue #780 / PR #789 / merge `ec04606c657756ae36261cecc1c075d9099e8ba9`

## Primary repository evidence

- `tools/agents/branch_lifecycle.py::GitHubClient.delete_branch()` performs a live `get_ref()` expected-SHA comparison followed by a separate REST DELETE addressed only by branch ref name.
- `tools/agents/branch_lifecycle.py::revalidate_delete_entry()` performs live SHA, protection, open-PR, active-task, remediation-Issue, policy and default-branch checks and a final SHA read before calling `delete_branch()`.
- `tools/agents/test_branch_lifecycle.py::test_delete_call_rechecks_expected_sha` mutates before the fake delete client's expected-SHA comparison.
- `tools/agents/test_branch_lifecycle.py::test_github_client_delete_branch_guards_expected_sha` proves the client performs the guard before sending DELETE but does not model a remote ref change after GET and before server-side deletion.
- `.github/workflows/branch-lifecycle.yml` retains the reviewed-manifest, live inventory, policy and recovery boundaries; no reviewed deletion approval was required or activated by this audit.

## External contract evidence

GitHub's official REST **Delete a reference** documentation lists the ref path identity and no request body or expected-old-SHA conditional parameter. This supports the narrow conclusion that the current REST DELETE request does not carry the reviewed expected SHA into a server-enforced compare-and-delete operation.

## Finding

`OPA-GOV-0022` / Issue #793 — HIGH / P1 / PROVEN.

The final check and destructive operation remain separate remote requests. A branch update that lands after the final GET response and before the DELETE is processed is not represented in the DELETE request.

## Safety

No branch deletion, reviewed approval activation, production/staging mutation or external-repository write was performed to prove this finding.
