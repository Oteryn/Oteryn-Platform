# Agent Governance branch/PR reconciliation audit

## Target

- repository: `blakinio/Oteryn-Platform`
- audited main: `a1b3690c85fe4fb585d5725474769a8aced2e686`
- subject: live active-task liveness delivered for Issue #558 / PR #779
- mode: `AUDIT ONLY`

## Verdict

`FINDINGS`

One material finding is confirmed with high confidence:

### OPA-GOV-0021 — HIGH — Branch-only tasks bypass live PR terminal reconciliation

When a task declares `pr: none` and a branch, `evaluate_task()` checks only whether the branch ref exists. If it does, the task is classified `BRANCH_ONLY` and `ownership_active = True`.

The live-state abstraction exposes `get_pull_request(number)` and `get_branch(branch)` only. It cannot discover PRs associated with a branch when the task omitted the PR number. Numeric-PR tasks receive open/draft/terminal reconciliation; branch-only tasks do not.

This leaves two material blind spots:

1. after a PR is opened, an unchanged `pr: none` task can still pass as branch-only instead of being reconciled with the live open PR;
2. after that PR becomes merged/closed, a retained branch can continue to claim active ownership because terminal PR state remains invisible.

## Focused regression evidence

`tools/agents/test_task_liveness.py::test_branch_only` explicitly accepts `pr: none` with an existing branch as live-valid `BRANCH_ONLY`.

The terminal fixtures (`test_terminal_archive_pending`, `test_terminal_stale_task`) use numeric `pr: 12`. There is no fixture for:

- omitted PR + matching open PR;
- omitted PR + retained merged PR branch;
- omitted PR + retained closed-unmerged PR branch;
- branch reuse after a terminal PR;
- ambiguous matching PR history.

## Live integration evidence

The prior continuous-audit task for PR #784 retained `pr: none` after PR #784 existed. Agent Governance passed exact-head validation because branch-only liveness only proved that the ref existed. The task was later archived correctly, so no stale ownership remained, but the real repository lifecycle demonstrated the omitted-PR blind spot.

## Expected contract

Issue #558 required live branch/PR/task reconciliation and explicit handling of retained terminal branches without treating branch existence as active ownership. The implemented numeric-PR path satisfies that shape; the branch-only path can bypass it.

## Impact

A stale or incomplete task record can again reserve ownership after a PR exists or becomes terminal, causing false collision prevention, invalid continuation decisions and retained obsolete ownership. This is the same systemic risk class that motivated Issue #558, reachable through omitted PR identity.

## Remediation handoff

Issue #788 owns remediation. Acceptance must preserve legitimate pre-PR branch-only work, reconcile matching PRs against exact live branch/head identity, handle branch reuse and ambiguity fail-closed, and add deterministic regression fixtures.

No implementation, workflow, test or runtime path is modified by this audit.

## Audit-delivery E2E

`NOT_APPLICABLE`: this report and its companion audit records change no executable behavior.
