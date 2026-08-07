# Evidence index — Agent Governance branch/PR reconciliation audit

## Scope

Audit-only evidence for `OTERYN-20260807-agent-governance-branch-pr-reconciliation-audit` on `main@a1b3690c85fe4fb585d5725474769a8aced2e686`.

## Primary sources

- `tools/agents/task_liveness.py`
  - `GitHubState` exposes only `get_pull_request(number)` and `get_branch(branch)`.
  - `evaluate_task()` routes `pr: none` + branch to a branch-existence-only `BRANCH_ONLY` path with active ownership.
  - terminal/open/draft reconciliation occurs only after a numeric PR is supplied by task text.
- `tools/agents/test_task_liveness.py`
  - `test_branch_only` accepts `pr: none` plus an existing branch.
  - terminal fixtures declare numeric `pr: 12`.
  - no omitted-PR matching-open/terminal/reused-branch fixture exists.
- `.github/workflows/agent-governance.yml`
  - runs live task-liveness with `contents: read` and `pull-requests: read`.
  - available permissions are sufficient for a repair to query PR state; the current validator simply lacks branch-to-PR discovery.
- Issue #558 / PR #779
  - accepted goal includes branch/PR/task identity reconciliation and retained-terminal-branch classification.
- PR #784 lifecycle
  - its active audit task remained `pr: none` after the PR existed while Agent Governance passed, demonstrating the live omitted-PR condition.

## Finding

- `OPA-GOV-0021`
- durable remediation: Issue #788
- severity: high
- confidence: high
- evidence state: PROVEN

## Negative boundaries checked

- genuine pre-PR branch-only work must remain valid;
- retained terminal branch must not imply active ownership when exact matching PR is terminal;
- branch reuse after terminal PR needs exact branch/head identity comparison rather than name-only inference;
- ambiguous matching PR history must fail closed or require explicit task identity;
- GitHub state failure must remain fail closed.

## Mutation boundary

No governance runtime, workflow, test, production, staging, repository setting or external repository was modified by this audit evidence package.
