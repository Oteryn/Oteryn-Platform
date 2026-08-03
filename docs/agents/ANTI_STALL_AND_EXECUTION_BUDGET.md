# Anti-Stall and Execution Budget Contract

```yaml
anti_stall_policy_version: 2.1
```

## Purpose

Autonomous execution must make measurable progress and must not become an unbounded polling, retry, repair, context-reconstruction, PR-creation, or task-selection loop. This contract limits one foreground owner invocation. It does not weaken stricter repository safety, authorization, production, merge, ownership, data, payment, authentication, protocol, asset, live-capital, or validation rules.

`continue_until_real_stop` means continue while safe, useful progress or eligible bounded terminal-CI completion remains inside the budget. Budget exhaustion and verified lack of progress outside the exception are real stop conditions.

## State model

Task state and invocation result are separate:

```yaml
checkpoint_task_statuses:
  - investigating
  - implementing
  - validating
  - ready
  - waiting
  - blocked
  - completed
terminal_invocation_results:
  - DONE
  - WAITING
  - BLOCKED
  - ROTATE
```

Use `waiting` when an external event is pending and no worker should remain active, except for the bounded terminal-CI continuation defined below. Use `blocked` when a decision, permission, safety rule, missing resource, or exhausted repair path prevents progress. Use `ready` when a fresh session can safely execute `next_action`. Use `completed` only for a terminal task that has satisfied repository closeout rules.

`ROTATE` is an invocation result, not a task status. Before returning `ROTATE`, persist a checkpoint with `status: ready`, `waiting`, or `blocked` and exactly one concrete `next_action`.

## Default budget

```yaml
normal_foreground_runtime_minutes: 60
large_foreground_runtime_minutes: 120
large_budget_requires_explicit_task_declaration: true
no_progress_minutes: 15
max_ci_state_checks_per_exact_head: 2
max_unchanged_external_state_checks: 2
terminal_ci_wait_budget_minutes: 45
terminal_ci_minimum_poll_interval_minutes: 3
max_terminal_ci_state_checks_per_check_generation: 12
max_identical_failure_retries_without_new_hypothesis: 1
max_repair_cycles_per_gate: 3
max_context_reconstruction_attempts: 1
max_additional_tasks_after_terminal_entry_task: 1
minimum_remaining_minutes_to_start_additional_task: 30
normal_command_timeout_minutes: 20
heavy_command_timeout_minutes: 45
heavy_timeout_requires_reason: true
```

The **entry task** is the active task at invocation start or, when none is active, the first `READY` task selected by the coordinator. Required post-merge lifecycle closeout for that entry task, including a repository-mandated archive PR, remains part of the same entry task and is not an additional task. After the entry task becomes fully terminal, at most one additional task may be started in the same invocation under `Starting another task` below.

A repository or owner may set a smaller budget. A larger budget requires an explicit task-record field and reason; vague instructions such as “work autonomously” do not enlarge it.

When exact wall-clock information is available, record `invocation_started_at` and `last_progress_at`. When it is unavailable, enforce retry and check counters conservatively rather than assuming unlimited time.

## Measurable progress

At least one of these must occur to reset the ordinary no-progress timer:

- a coherent code, configuration, migration, test, documentation, governance, or task-record change is persisted;
- a new test or validation result provides materially new evidence;
- a specific failure is repaired or isolated with a genuinely new hypothesis;
- a PR, review, CI, deployment, dependency, or external state materially changes;
- a material audit finding is opened, resolved, or reclassified with evidence;
- a task or PR reaches an intentional terminal state.

Reading the same files again, repeating an unchanged command, checking the same pending workflow, rewriting summaries, or creating activity-only commits or PRs is not progress. A permitted terminal-CI wait does not become measurable progress merely because another pending-state check occurred.

## Required checkpoint counters

For autonomous or failure-prone work, persist when applicable:

```yaml
invocation_started_at: <timestamp>
last_progress_at: <timestamp>
ci_checks_for_current_head: 0
ci_check_generation: <draft | ready | merge_queue | current_base | other>
terminal_ci_wait_started_at: <timestamp or null>
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
```

Reset a counter only after the underlying exact head, failure signature, hypothesis, external state, or required-check generation materially changes. A draft-to-ready transition, current-base refresh, or merge-queue admission that creates a genuinely new required-check set on the same exact SHA is a new check generation. It resets only the terminal-CI generation counter, not unrelated retry, repair, unchanged-state, or total terminal-wait budgets.

Eligible terminal-CI observations increment only `terminal_ci_checks_for_current_generation`. They do not consume `ci_checks_for_current_head` or `unchanged_state_checks`; those ordinary counters remain frozen until the terminal exception ends. All ineligible, ordinary, or non-terminal observations continue to use the ordinary counters and limits.

## CI and external waiting

### Ordinary CI and external waiting

For one exact head outside the terminal-CI exception:

1. inspect required CI once after it is expected to exist;
2. perform at most one later state check;
3. if it remains pending and authorized auto-merge or a merge queue is available, configure it once;
4. persist exact head, run IDs, pending checks, `status: waiting`, and one `next_action`;
5. end or rotate the invocation, or execute genuinely independent work already inside the same declared task and remaining budget.

Never perform a third ordinary CI state check for the same exact head in one invocation. Do not keep a worker active merely to wait for reviews, deployment, scheduled jobs, dependencies, observation windows, an owner reply, or non-terminal CI.

### Bounded terminal exact-head CI continuation

The owner invocation may remain active through final required CI, branch-protection completion and the resulting merge only when all are true:

- implementation and all non-CI acceptance work for the entry task are complete;
- fresh audit has no open material finding;
- required E2E passed or is validly `NOT_APPLICABLE` with a concrete reason;
- the PR exact head is final and unchanged;
- the only remaining task gate is required CI, branch protection, auto-merge, merge queue, or the final authorized merge after CI;
- no failing check, requested change, unresolved review thread, ownership conflict, decision, permission, migration hold, or safety blocker exists;
- the execution environment can perform a bounded wait or delayed recheck without inventing background execution;
- foreground runtime remains.

Auto-merge availability is **not** required. When repository auto-merge is disabled or unavailable, the task remains eligible if the agent is authorized to perform the final merge after all exact-head checks pass.

While eligible:

1. start one terminal-CI wait budget capped at `45` minutes or the remaining foreground runtime, whichever is smaller;
2. wait at least `3` minutes between unchanged-state checks;
3. perform no more than `12` checks for one materially new required-check generation;
4. treat draft, ready-state, current-base, and merge-queue checks as separate generations only when GitHub actually creates a new required-check set;
5. do not start unrelated work merely to occupy the interval;
6. do not reset the total wait budget after a new generation on the same head;
7. allow this specific bounded wait to run past the ordinary `15`-minute no-progress limit, but never past the terminal-CI wait budget or foreground runtime;
8. after success, immediately re-read the PR, exact head, required checks, reviews, ownership, and mergeability;
9. when auto-merge is available, observe the protected merge result; when unavailable, perform a direct squash merge only after every required exact-head check passes and every merge gate remains satisfied;
10. after merge, record the merge commit and complete required post-merge task archival, ownership release, Issue update, and programme reconciliation as the same entry task when they fit inside remaining runtime;
11. after a failing check, leave the waiting path and enter the normal evidence-based CI repair loop.

Do not return `WAITING` solely because eligible terminal CI is still pending before the terminal-CI wait budget, per-generation check cap, or foreground runtime is exhausted. When any limit is reached, persist exact head, generation, run IDs, counters, merge mode, and one `next_action`, then return `WAITING` or `ROTATE` accurately.

## Failure and repair limits

- The first failure permits analysis and one evidence-based repair.
- An identical second failure requires a materially new hypothesis, changed input, added instrumentation, or narrower isolation.
- Repeating the identical failure again without new evidence is forbidden.
- After three repair cycles for one gate, persist evidence and return `BLOCKED` or `ROTATE` unless repository policy explicitly authorizes a fresh isolation task.
- Re-running a failed check after no relevant change does not count as validation and does not reset the budget.

## Command timeouts

Every long-running command, build, test, migration dry-run, log stream, or network operation must use a finite timeout where the tool supports it. Use 20 minutes by default. A timeout up to 45 minutes requires a recorded reason. Never start a command whose maximum duration exceeds the remaining invocation budget.

For local CLI or runner execution, use a process-level watchdog when available. A recommended outer bound is 90 minutes with a graceful interrupt followed by forced termination after five additional minutes. Cloud environments that do not expose such a watchdog must still obey checkpoint and stop rules.

## No-progress and budget exhaustion

When the ordinary no-progress limit, runtime budget, retry limit, repair limit, context-reconstruction limit, or eligible terminal-CI limit is reached:

1. stop polling and stop starting new work;
2. preserve coherent changes and exact branch/head state;
3. write the last measurable progress, unchanged state, attempted hypotheses, counters, check generation, and one `next_action`;
4. release unnecessary workers, leases, worktrees, or ownership where safe;
5. set checkpoint status to `waiting`, `blocked`, or `ready` accurately;
6. return `WAITING`, `BLOCKED`, or `ROTATE` accurately.

Do not create another PR, archive PR, task, or branch solely to keep the invocation active. A repository-mandated post-merge archive PR is allowed only as required terminal cleanup for the entry task, after implementation merge, when it fits inside remaining runtime and does not create a new waiting loop.

## Starting another task

Starting one additional task after the terminal entry task is allowed only when all are true:

- the entry task, including required post-merge archival and ownership release, is fully terminal;
- at least 30 minutes of declared budget remains;
- no stall warning occurred;
- no required check or external event is being waited on;
- ownership and dependency preflight confirms the next task is safe and independent;
- no additional task has already been started in the invocation.

Otherwise persist the programme handoff and stop. A rotated session or required archive closeout on the same task is not a new task.

## Canonical terminal response

Use this shared format. Use `not applicable` where a field genuinely does not apply.

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
RESULT: <observable work completed>
CHANGED_PATHS: <paths or none>
VALIDATION: <focused/component/exact-head results>
AUDIT: <result, validator identity and open material findings>
E2E: <PASS | NOT_APPLICABLE with reason | not run with blocker>
PR_HYGIENE: <related PR terminal states and unresolved threads>
LAST_PROGRESS: <last measurable repository or environment change>
BUDGET: <elapsed/limit or counters used, including terminal-CI generation when applicable>
UNCHANGED_STATE: <what remained unchanged>
DURABLE_STATE: <task, branch, exact head, PR and CI state>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```

## Forbidden patterns

Do not:

- repeatedly poll the same CI run, PR, review, deployment, log, or dependency outside the bounded terminal-CI exception;
- check eligible terminal CI more frequently than the minimum interval or beyond its generation, check, time, or foreground-runtime caps;
- invent a new check generation when GitHub did not create a materially new required-check set;
- repeat an identical failed command without a new hypothesis or changed input;
- reopen already verified files merely to appear active;
- reconstruct context repeatedly when durable state already exists;
- create extra tasks, commits, branches, or PRs solely to extend execution;
- interpret silence, pending status, or waiting as productive work;
- write `ROTATE` as a checkpoint task status;
- claim autonomous execution justifies production, data, payment, authentication, protocol, asset, live-capital, or protected-configuration mutation without authority;
- hide budget exhaustion by resetting counters, check generations, or labels without a material state change;
- force, bypass, override, or merge before required exact-head checks pass.

When this contract conflicts with a general continuation or waiting instruction, this specific bounded terminal-CI contract controls. Safety, authority, production, data, payment, authentication, cross-repository, and branch-protection restrictions remain controlling.
