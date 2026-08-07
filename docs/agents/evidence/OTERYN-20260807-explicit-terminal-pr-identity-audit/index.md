# Evidence — OTERYN-20260807 explicit terminal PR identity audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `ae716e3b955808916cb203bb97b59df0b44070cf`
- Finding: `OPA-GOV-0023` / Issue #811
- Risk / priority: HIGH / P1

## Live-state refresh

At audit selection time, the ready audit-remediation queue contained Issues #797, #801 and #804; no blocked audit-remediation Issue was returned. Open PRs were #541, #338, #391 and #405, all in unrelated ownership domains. The delta since the previous merged audit head `d823e335...` contained the marketplace audit closeout, the Issue #788 liveness repair and its lifecycle closeout.

## Implementation evidence

`tools/agents/task_liveness.py::_pull_state()` returns `state`, `merged`, `draft`, PR `head.ref` and head repository identity.

In `evaluate_task()` for a numeric PR:

1. the PR is fetched and `head_ref` is available;
2. a foreign head repository emits `foreign_pr_head`;
3. when the PR is terminal (`merged` or `closed`), the function immediately sets `live_state = TERMINAL_ARCHIVE_PENDING` and `ownership_active = False` and validates archive-pending metadata;
4. when a task branch is present, the terminal path only queries whether that declared branch is retained and can emit the advisory `terminal_branch_retained`;
5. the equality check `task.branch != head_ref` exists only in the non-terminal open/draft branch, where it emits `branch_pr_mismatch`.

The terminal path therefore does not establish that the PR whose terminal state releases ownership is actually the PR for the task's declared branch.

## Regression evidence

`tools/agents/test_task_liveness.py::test_branch_pr_mismatch` covers branch/head disagreement for an open numeric PR.

`test_terminal_archive_pending` covers a terminal numeric PR with the ordinary matching branch.

No inspected fixture supplies a terminal numeric PR whose `head.ref` differs from `task.branch`, so the asymmetric identity check is not regression-protected.

## Expected invariant

A PR may change task ownership state only after exact task→branch→PR identity is established. A terminal PR belonging to different work must fail closed rather than release ownership.

## Falsified invariant

A task may declare branch A and numeric PR B whose same-repository head is branch C. If PR B is merged/closed and task metadata satisfies the archive-pending contract, the validator can classify the task terminal and `ownership_active = false` without emitting `branch_pr_mismatch`.

## Duplicate search

Open and closed Issue searches for terminal branch/PR mismatch, `branch_pr_mismatch`, `head.ref`, retained terminal branch and Issue #558/#788 follow-up behavior found no independent actionable owner. Issue #558 is the parent exact-identity repair and Issue #788 repaired omitted-PR branch-history discovery; neither records this explicit terminal numeric-PR gap.

## Safety

No governance runtime, product runtime, workflow, production/staging environment or external repository was mutated by this audit.
