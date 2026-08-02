# GitHub-Only Execution Contract

```yaml
github_only_execution_policy_version: 1
```

## Purpose

The absence of Codex or a local terminal is not, by itself, a task blocker. When repository access is available, use the GitHub connection for repository reads and writes and GitHub Actions as the remote execution and validation environment.

This contract never weakens repository safety, ownership, scope, production, credential, secret, deployment, merge, database, payment, authentication, protocol, asset, capital, or cross-repository restrictions. It is subordinate to `ANTI_STALL_AND_EXECUTION_BUDGET.md`; GitHub-only execution must remain bounded.

## Required execution pattern

1. Inspect live repository state before editing: the relevant code and structure, active task and checkpoint, related issue, related or overlapping pull requests, workflows, and recent material CI results.
2. Work on a dedicated branch. Never write task changes directly to the default branch.
3. Change only the layers required for a complete result. Depending on the task, this may include backend, frontend or client, persistence and migrations, integration, tests, configuration, and documentation. Do not touch every layer merely to appear complete.
4. Prefer existing GitHub Actions for remote execution and validation. Select checks according to impact, including dependency installation, lint, type checking, build, unit and integration tests, application startup, temporary-database migrations, E2E such as Playwright, Docker, or helper services when relevant.
5. Use the smallest validation capable of proving the change. Do not rerun full or heavy CI when a focused test, one failed job, or an allowed immediate-parent result is sufficient.
6. On failure, inspect the full failed-job log, identify the first actionable error, record a causal hypothesis, make one targeted repair, and run the smallest proving validation. Do not repeat an identical operation without new evidence or a new hypothesis.
7. When existing workflows cannot validate the task, a minimal temporary validation workflow may be added only on the working branch. It must not deploy to production, weaken project protections, or persist into the final merge unless retaining it is an explicitly justified deliverable.
8. Preserve reports, logs, screenshots, traces, and other evidence as GitHub Actions artifacts when they are needed for audit or cannot be represented reliably in job logs.
9. Inspect and reconcile only pull requests related to the current task or directly blocking it. Do not clean the repository-wide PR queue without a separately authorized scope.
10. A related pull request may be closed as stale or superseded only after documenting why it must not merge, what replaces it, and whether any unique changes must be preserved.
11. Continue through safe executable steps instead of stopping at a plan or status report. Stop only at a real stop condition from this contract, repository safety rules, or the anti-stall contract.
12. Do not claim that Codex or a local terminal is required until a concrete unavailable operation has been identified and shown not to be achievable through the available GitHub connection and GitHub Actions.

## Remote validation rules

- Do not create a workflow merely because local execution is unavailable; first prove that existing workflows and rerun capabilities are insufficient.
- Do not use GitHub Actions to bypass branch protection, required review, environment protection, secret controls, or production approval.
- Do not expose secrets in logs, artifacts, screenshots, traces, pull-request comments, or task records.
- Temporary databases and services must be isolated from production and disposable.
- A passing remote workflow is evidence only for the exact commit and configuration it tested.
- Waiting for GitHub Actions is not active work. Follow the exact-head check and unchanged-state limits in `ANTI_STALL_AND_EXECUTION_BUDGET.md`.

## Valid stop conditions

A GitHub-only task may stop only when at least one of these conditions is real and evidenced:

- required GitHub or GitHub Actions permission is unavailable;
- the GitHub connection does not expose an operation required by the task;
- a required secret, key, certificate, protected environment, or external-infrastructure access is unavailable;
- the task requires a physical device or system unavailable to any permitted runner;
- a business decision is required and repository evidence provides no decision criteria;
- an anti-stall runtime, no-progress, retry, repair-cycle, context-reconstruction, or exact-head check limit is exhausted;
- the next operation would require an unauthorized merge, auto-merge, production deployment, or protected configuration change.

When stopping, report exactly:

- the unavailable operation;
- the tool, action, endpoint, or workflow used;
- the received error or observed restriction;
- the missing permission, secret, resource, device, or decision;
- the attempts and evidence already collected;
- the nearest safe alternative;
- the current branch, pull request, exact head, validation state, and one next action.

## Merge and production authority

Do not merge, enable auto-merge, deploy to production, or modify protected production configuration without either explicit owner authorization or durable repository authorization that unambiguously covers the exact repository and operation.

Lack of merge authority does not block preparing a complete pull request, running permitted validation, resolving review findings, and presenting a merge-ready result.

## Completion report

A terminal report must include:

- changes and affected layers;
- changed files;
- validation and audit results;
- related pull-request states;
- active-task and ownership state;
- remaining real restrictions;
- final status: `DONE`, `WAITING`, `BLOCKED`, or `ROTATE`.
