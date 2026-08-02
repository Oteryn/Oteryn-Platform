# Anti-Stall and Execution Budget Contract

```yaml
anti_stall_policy_version: 1
```

## Purpose

Autonomous execution must make measurable progress and must not become an unbounded polling, retry, repair, context-reconstruction, PR-creation, or task-selection loop. This contract limits one foreground owner invocation. It does not weaken stricter repository safety, authorization, production, merge, ownership, database, payment, authentication, or validation rules.

`continue_until_real_stop` means continue while safe, useful progress is available **within this budget**. Budget exhaustion and verified lack of progress are real stop conditions.

## Default budget

```yaml
normal_foreground_runtime_minutes: 60
large_foreground_runtime_minutes: 120
large_budget_requires_explicit_task_declaration: true
no_progress_minutes: 15
max_ci_state_checks_per_exact_head: 2
max_unchanged_external_state_checks: 2
max_identical_failure_retries_without_new_hypothesis: 1
max_repair_cycles_per_gate: 3
max_context_reconstruction_attempts: 1
max_tasks_started_per_invocation: 1
minimum_remaining_minutes_to_start_next_task: 30
normal_command_timeout_minutes: 20
heavy_command_timeout_minutes: 45
heavy_timeout_requires_reason: true
```

A repository or owner may set a smaller budget. A larger budget requires an explicit task record field and reason; vague instructions such as “work autonomously” do not enlarge it.

When exact wall-clock information is available, record `invocation_started_at` and `last_progress_at`. When it is unavailable, enforce the retry/check counters and stop conservatively rather than assuming unlimited time.

## Measurable progress

At least one of these must occur to reset the no-progress timer:

- a coherent code, configuration, migration, test, documentation, or task-record change is persisted;
- a new test or validation result provides materially new evidence;
- a specific failure is repaired or isolated with a genuinely new hypothesis;
- a PR, review, CI, deployment, dependency, or external state changes;
- a material audit finding is opened, resolved, or reclassified with evidence;
- a task or PR reaches an intentional terminal state.

Reading the same files again, repeating an unchanged command, checking the same pending workflow, rewriting summaries, or creating activity-only commits/PRs is not progress.

## Required checkpoint counters

For autonomous or failure-prone work, persist when applicable:

```yaml
invocation_started_at: <timestamp>
last_progress_at: <timestamp>
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
```

Reset a counter only after the underlying exact head, failure signature, hypothesis, or external state materially changes.

## CI and external waiting

For one exact head:

1. inspect required CI once after it is expected to exist;
2. perform at most one later state check;
3. if it remains pending and authorized auto-merge or merge queue is available, configure it once;
4. persist exact head, run IDs, pending checks, status `WAITING`, and one `next_action`;
5. end the invocation or select only genuinely independent work already within the same declared task and remaining budget.

Never perform a third CI state check for the same exact head in one invocation. Do not keep a worker active merely to wait for CI, reviews, deployment, scheduled jobs, dependencies, observation windows, or an owner reply.

## Failure and repair limits

- The first failure permits analysis and one evidence-based repair.
- An identical second failure requires a materially new hypothesis, changed input, added instrumentation, or narrower isolation.
- Repeating the identical failure again without new evidence is forbidden.
- After three repair cycles for one gate, persist evidence and return `BLOCKED` or `ROTATE` unless repository policy explicitly authorizes a fresh isolation task.
- Re-running a failed check after no relevant change does not count as validation and does not reset the budget.

## Command timeouts

Every long-running command, build, test, migration dry-run, log stream, or network operation must use a finite timeout where the tool supports it. Use 20 minutes by default. A timeout up to 45 minutes requires a recorded reason. Never start a command whose maximum duration exceeds the remaining invocation budget.

For local CLI or runner execution, use a process-level watchdog when available. A recommended outer bound is 90 minutes with a graceful interrupt followed by forced termination after five additional minutes. Cloud environments that do not expose such a watchdog must still obey the checkpoint and stop rules in this contract.

## No-progress and budget exhaustion

When the no-progress limit, runtime budget, retry limit, repair limit, or context-reconstruction limit is reached:

1. stop polling and stop starting new work;
2. preserve coherent changes and exact branch/head state;
3. write the last measurable progress, unchanged state, attempted hypotheses, counters, and one `next_action`;
4. release unnecessary workers, leases, worktrees, or ownership where safe;
5. set `WAITING`, `BLOCKED`, or `ROTATE` accurately;
6. return control to the owner.

Do not create another PR, archive PR, task, or branch solely to keep the invocation active. Required terminal cleanup may be completed only when it fits inside the remaining budget and does not require waiting loops.

## Starting another task

Starting another task in the same invocation is allowed only when all are true:

- the previous task is fully terminal;
- at least 30 minutes of declared budget remains;
- no stall warning occurred;
- no required check or external event is being waited on;
- ownership and dependency preflight confirms the next task is safe and independent.

Otherwise persist the programme handoff and stop.

## Required terminal response

```text
STATUS: DONE | WAITING | BLOCKED | ROTATE
LAST_PROGRESS: <last measurable repository or environment change>
BUDGET: <elapsed/limit or counters used>
UNCHANGED_STATE: <what remained unchanged>
DURABLE_STATE: <task, branch, exact head, PR and CI state>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```

## Forbidden patterns

Do not:

- repeatedly poll the same CI run, PR, review, deployment, log, or dependency;
- repeat an identical failed command without a new hypothesis or changed input;
- reopen already verified files merely to appear active;
- reconstruct context repeatedly when durable state already exists;
- create extra tasks, commits, branches, or PRs solely to extend execution;
- interpret silence, pending status, or waiting as productive work;
- claim autonomous execution justifies production, database, payment, or authentication mutation without authorization or running overnight without a declared budget;
- hide budget exhaustion by resetting counters or changing labels without a material state change.

When this contract conflicts with a continuation instruction, follow this contract and stop safely.