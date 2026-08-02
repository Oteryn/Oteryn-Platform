# Mandatory Codex Bootstrap

This root bootstrap is automatically loaded by Codex. It supplements and never weakens system, developer, owner, repository-allowlist, safety, production, credential, database, payment, authentication, deployment, merge, or cross-repository restrictions.

Before planning, editing, creating or resuming a task, creating a branch or PR, or claiming completion:

1. Read the root `AGENTS.md` completely, even when Codex selected this `AGENTS.override.md` as the automatic instruction file.
2. Read `docs/agents/AGENTS.md` and follow the nearest additional `AGENTS.md` governing every path you may touch.
3. Read `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` for delivery classification, vertical-slice completeness, outcome verification, independent audit, E2E, exact-head CI, PR hygiene, archival, and ownership release.
4. Read `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md` before autonomous, long-running, retry-prone, CI-waiting, repair, continuation, or multi-task work. Its budgets and stop conditions override any instruction to continue indefinitely.
5. Read `docs/agents/GITHUB_ONLY_EXECUTION.md` whenever Codex or a local terminal is unavailable, unsuitable, or would otherwise be treated as a blocker. Continue through the GitHub connection and GitHub Actions when permitted; tool absence alone is not a blocker.
6. For any start, resume, continuation, autonomous-programme, or multi-task request, read `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md` before acting.
7. Inspect the authoritative active task checkpoint, live branch/head, related PRs, reviews, CI, ownership, dependencies, and current repository state. Do not reconstruct available state from chat history or ask the owner to repeat it.
8. If a required bootstrap document is missing or materially conflicts with live repository safety, stop and report the exact conflict instead of silently ignoring the document.

## Short-command contract

`Uruchom <program> autonomicznie.` and `Kontynuuj <program> autonomicznie.` are sufficient owner commands when the programme can be resolved from repository state.

Interpret such a command as authorization to execute the foreground coordinator loop until a real stop condition defined by the repository contracts. Continue through bounded phases, checkpoints, implementation, validation, audit, E2E, exact-head CI, PR closeout, task archival, ownership release, barrier review, and the next safe `READY` task without requesting a routine follow-up prompt.

A worker-session end, commit, PR creation, green CI, merge, audit, E2E result, PR cleanup, or task archive is a milestone, not by itself a reason to stop the owner invocation. No work continues after the final response; this instruction does not authorize hidden background execution.

## Anti-stall baseline

Autonomous continuation is always budgeted. Default to 60 minutes per foreground invocation, allow 120 minutes only when the task explicitly declares and justifies a large budget, and stop after 15 minutes without measurable progress. Check CI or unchanged external state at most twice per exact head in one invocation. Do not repeat an identical failure without a new hypothesis, and stop after three repair cycles for one gate.

Budget exhaustion, the no-progress limit, retry-limit exhaustion, or unchanged pending CI are real stop conditions. Persist exact durable state and return `WAITING`, `BLOCKED`, or `ROTATE`; never keep polling or running overnight merely because the owner requested autonomous continuation.

## GitHub-only execution baseline

Do not stop, return only a plan, or ask the owner to switch tools merely because Codex or a local terminal is unavailable. Use the GitHub connection for repository operations and GitHub Actions for remote execution and validation on a dedicated branch, within the anti-stall budget. Prefer existing workflows and the smallest proving checks; add a temporary validation workflow only when existing workflows cannot prove the task, and remove it before final merge unless retention is an explicitly justified deliverable.

Do not merge, enable auto-merge, deploy, expose secrets, bypass protections, or modify protected production, database, payment, or authentication configuration without the required authority. A genuine GitHub-only blocker must identify the exact unavailable operation, attempted tool or workflow, received error, missing permission, secret or resource, collected evidence, nearest safe alternative, current branch, PR and exact head, and one next action.

## Completion baseline

Do not call user-facing work complete while any required persistence, backend/server, API/protocol, frontend/client, integration, observable state, test, or E2E layer is missing. Partial producer-only work must be labelled partial and linked to concrete consumer work.

Before `completed`, require verified resulting state, independent audit with no open material findings, required real E2E `PASS` or a legitimate repository-approved `NOT_APPLICABLE_WITH_REASON`, required CI on the exact final head, zero unresolved review threads, every related or superseded PR in an intentional terminal state, a terminal task record, released ownership/leases, and reconciled stale task branches according to repository policy.
