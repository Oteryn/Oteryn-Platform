# Task Closeout, Self-Review and E2E

```yaml
task_closeout_policy_version: 4
independent_repair_audit_required: false
```

## Purpose

Implementation completion is not task completion. Terminal status requires complete acceptance, exact-head self-review, risk-proportional validation, real applicable E2E, exact-head required CI, PR hygiene, task archival and ownership release.

No different-agent audit is required for remediation closeout. A separate platform-wide audit programme may discover new Issues, but it is not a merge gate for each repair.

## Lifecycle

```yaml
task_lifecycle:
  - implementing
  - validating
  - self_reviewing
  - e2e_testing
  - final_ci
  - closing_prs
  - ready_to_archive
  - completed
```

Do not move directly from implementation to completed.

## Required terminal evidence

Before `completed`, record:

- exact final head and complete changed-path list;
- acceptance and resulting-state evidence;
- self-review `PASS` for the full exact-head diff;
- risk-proportional validation results;
- real E2E `PASS` or justified `NOT_APPLICABLE`;
- repository-required exact-head CI;
- zero unresolved material findings, requested changes and review threads;
- intentional terminal states for related and superseded PRs;
- merge commit, Issue closure, task archival and ownership release.

## Self-review

The implementation owner must inspect:

- every acceptance criterion;
- the full diff, not only the latest commit;
- negative and failure paths;
- rollback and compatibility;
- data, auth, payment, concurrency, migration, protocol, dependency, deployment and production implications where relevant;
- related PRs, unresolved threads and current CI findings.

Findings are repaired by the same implementation owner before readiness. Self-review must not be labeled independent audit.

## E2E rule

E2E must prove the actual user or integration path when behavior changed. Unit tests, static analysis, screenshots, PR text or worker summaries are not substitutes.

`NOT_APPLICABLE` requires a concrete reason showing that the change has no executable user or integration journey.

## Exact-head CI and Actions economy

Final validation runs after a coherent candidate exists.

- Use focused checks during implementation.
- Batch checkpoint and evidence updates at material boundaries.
- Avoid one commit or push per file or checkpoint field.
- Cancel superseded same-PR workflow runs.
- Checkpoint-only, task-only and agent-governance-only changes must not start unrelated heavy runtime workflows.
- Full runtime, edge, outage, production-like and concurrency tests run only when path policy selects them.

A runtime-affecting head change invalidates prior final runtime evidence. A later docs-only change requires only checks selected by path policy.

## PR hygiene

Before merge:

- resolve all requested changes and material review comments;
- resolve or intentionally close every review thread;
- verify the current head and base;
- inspect all related, duplicate, predecessor, successor and superseded PRs;
- merge or intentionally close each related PR;
- confirm the final implementation and rollback provenance.

## Closeout sequence

1. finish implementation and focused tests;
2. perform exact-head full-diff self-review;
3. complete applicable E2E and required exact-head CI;
4. repair all material findings and requested changes;
5. merge through normal branch protection;
6. verify resulting state on the merged commit;
7. close or reconcile the Issue;
8. archive the task and release ownership;
9. remove temporary execution scaffolding.

Do not create or wait for an audit-only Issue, audit PR, frozen audit generation or different-agent PASS merely to complete a repair.
