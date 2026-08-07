# Audit report — Explicit terminal PR identity

## Scope

Repository-only audit of Agent Governance active-task liveness after the Issue #788 / PR #808 repair, focused on exact task→branch→PR identity in the explicit numeric-PR terminal path.

No implementation, workflow, product runtime, production/staging or external-repository mutation was performed.

## Verdict

`FINDING — HIGH / P1 / PROVEN`

Finding: `OPA-GOV-0023` / Issue #811.

## Path reviewed

```text
active task checkpoint
  -> numeric pr identity
  -> GitHub PR state + head.ref
  -> terminal/open split
  -> ownership_active decision
  -> collision/ownership consumers
```

## Primary evidence

1. `_pull_state()` exposes the PR head branch identity as `head_ref`.
2. For numeric open/draft PRs, `evaluate_task()` compares `task.branch` against `head_ref` and emits `branch_pr_mismatch` when they differ.
3. For numeric merged/closed PRs, `evaluate_task()` enters `TERMINAL_ARCHIVE_PENDING` and sets `ownership_active = False` before any task-branch/head-ref equality check.
4. The terminal branch existence lookup uses `task.branch`, but branch retention is only an advisory and does not prove that the terminal PR belongs to that branch.
5. The deterministic suite has an open-PR mismatch fixture and a terminal happy-path fixture, but no terminal mismatch fixture.
6. Consequently an unrelated same-repository terminal PR can be used as terminal evidence for a different declared task branch.

## Expected invariant

Exact task→branch→PR identity must be proven before open, draft or terminal PR state is allowed to alter ownership. Terminality is not a substitute for identity.

## Actual behavior

The identity check is asymmetric: numeric open/draft PRs enforce branch/head equality, numeric terminal PRs do not.

## Impact

A malformed or stale task record can falsely release ownership for work whose branch remains active. That weakens path-collision prevention and can allow a second remediation worker to enter work that should still be exclusively owned.

This is a governance/liveness defect rather than a product-runtime defect, but it affects the safety mechanism used to coordinate autonomous repair agents, so it is classified HIGH / P1 consistently with the preceding Issue #788 liveness defect.

## Duplicate analysis

No independently actionable duplicate was found. Issue #558 is the completed parent task/branch/PR identity repair. Issue #788 repaired the separate `pr: none` branch-history bypass. The explicit terminal numeric-PR mismatch remains distinct.

## Remediation handoff

Issue #811 owns remediation. The bounded repair should require matching task branch and PR `head.ref` before terminal state can release ownership and add deterministic merged and closed-unmerged mismatch fixtures, while preserving the valid open/draft and branch-reuse behavior already covered by the liveness suite.

## Delivery completeness

Audit records and remediation handoff are complete for this bounded package. Runtime/product E2E is `NOT_APPLICABLE` because this audit PR changes documentation/evidence only.
