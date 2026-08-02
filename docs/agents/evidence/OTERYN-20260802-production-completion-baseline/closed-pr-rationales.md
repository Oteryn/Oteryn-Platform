# Closed PR rationales

The following closures intentionally preserve the PR history while removing stale queue entries:

- **#116:** request-only blocked task record. The original scheduled times passed; issue #114 remains open and must be handled by a fresh run-history inspection rather than merging stale ACTIVE_WORK state.
- **#182:** obsolete request to trigger an unchanged historical Liquid20 retry.
- **#189:** obsolete historical attempt record and retry authorization for the same superseded observation programme.
- **#328:** request-only rename task/index setup. It did not deliver the required ADR/contract; issue #324 remains open.
- **#335:** superseded implementation. Current `main` already has `restart: always` and a runner-executed, fail-closed autostart repair workflow.
- **#387:** superseded public-domain validation. Later production-gate evidence (#405) contains the relevant observations and current blockers, while public-edge repair/audit work is already on `main`.

Closing these PRs does not assert that their parent product/evidence issues are complete.
