# OTERYN-20260807 branch lifecycle atomic-delete audit

## Verdict

`PROVEN_MATERIAL_FINDING`

Finding: `OPA-GOV-0022` / Issue #793 — HIGH / P1.

## Audited boundary

Current main: `993b3561feb75644d4a07f3e3377020be051eed6`.

Inspected:
- PR #789 and its merged Issue #780 repair;
- `tools/agents/branch_lifecycle.py`;
- `tools/agents/test_branch_lifecycle.py`;
- `.github/workflows/branch-lifecycle.yml`;
- ADR 0024;
- current GitHub REST delete-reference contract.

## Finding

The Issue #780 repair correctly adds per-entry live revalidation and a second expected-SHA read immediately before destructive deletion. However, `GitHubClient.delete_branch()` still implements the final guard as two independent operations:

1. GET the ref and compare its SHA;
2. DELETE the ref by name.

The expected SHA is not carried into the destructive request. GitHub's documented REST Delete a reference endpoint has no expected-old-SHA body or path precondition. A branch can therefore advance after the last GET returns and before the DELETE is processed, and the DELETE can remove the newly advanced ref.

The existing deterministic race test mutates the fake ref before the expected-SHA comparison, so it proves the client-side reread but not a server-enforced compare-and-delete boundary.

## Expected behavior

The remote ref mutation itself must fail when the branch no longer equals the reviewed expected SHA. An equivalent server-enforced expected-old-ref primitive is acceptable; the audit does not prescribe one implementation.

## Impact

The remaining race window is much narrower than OPA-GOV-0019, but the consequence remains destructive: newly pushed unmerged work can be removed in the final read-to-delete interval. This violates ADR 0024's fail-closed no-active-work-deletion invariant.

## Duplicate result

No open or closed actionable Issue was found for server-enforced atomic branch deletion after Issue #780. #780 addressed stale inventory and immediate client-side revalidation; #793 owns the residual remote atomicity boundary.

## Delivery boundary

Audit-only. No workflow, runtime, branch-deletion implementation, production, staging or external-repository mutation is included.

Runtime/product E2E: `NOT_APPLICABLE` for this documentation/evidence package.
