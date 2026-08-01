# Autonomous Program Continuation Contract

## Purpose

This document defines how one short owner invocation can drive a long, low-noise, multi-task programme run without making the owner manually restart every phase or paste every worker prompt.

It supplements `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md`, `EXECUTION_PROTOCOL.md`, and `CONTEXT_HANDOFF.md`. Repository safety, authorization, ownership, merge, production, and cross-repository rules remain more authoritative when stricter.

## Core distinction

A **worker session** is bounded to one role and phase. It may checkpoint, finish, or rotate.

An **owner invocation** is the whole foreground run started by one command. It may coordinate several worker sessions, complete several phases, merge or close permitted PRs, archive completed tasks, cross synchronization barriers, and start the next ready work before returning.

A worker session ending is not an automatic reason for the owner invocation to end.

No work continues after the assistant has returned its final response. This contract authorizes long autonomous work during the current invocation, not hidden background execution.

## Trigger

Use this contract when any of these is true:

- the owner writes `Uruchom <program> autonomicznie` or `Kontynuuj <program> autonomicznie`;
- a registered short-invocation command resolves to a durable programme or rollout coordinator;
- the owner explicitly asks the agent to keep completing tasks without repeated confirmation;
- the active prompt declares:

```yaml
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

## Startup

At the start of the invocation:

1. read repository instructions and the nearest nested instructions;
2. identify the programme, project lane, coordinator task, short-invocation registry, or equivalent durable entry point;
3. inspect live tasks, checkpoints, branches, PRs, CI, leases, ownership, dependencies, waves, barriers, and safety contracts;
4. repair stale coordinator state only when repository policy authorizes it and evidence is sufficient;
5. select the highest-priority safe `READY` work;
6. do not ask the owner to restate information available in live repository state.

If no durable programme exists, create a bounded programme/coordinator task only when the owner's request clearly authorizes substantial programme work. Otherwise use `single_task` scope.

## Autonomous coordinator loop

Repeat this loop while a safe action is available:

1. **Select** — choose one ready task or a bounded set of independent non-overlapping tasks within the repository concurrency limit.
2. **Route** — choose Chat, Codex, Work, or a fresh validator using the cheapest capable mode.
3. **Execute** — complete the current bounded phase without unrelated expansion.
4. **Validate** — use focused checks first, component checks at a coherent milestone, and the heavy final gate only when ready.
5. **Persist** — update checkpoint, branch/head/PR, validation, evidence, blocker, and one exact `next_action`.
6. **Continue the task** — when the next phase is already safe and ready, begin it without asking the owner.
7. **Finalize the task** — when acceptance and repository close/merge gates are satisfied, complete the terminal lifecycle described below.
8. **Review the barrier** — refresh programme dependencies, ownership, PR/CI state, and ready work.
9. **Continue the programme** — immediately select the next ready task and repeat.

Do not return merely because one iteration of this loop completed.

## Checkpoints are not pauses

A checkpoint exists so work survives context loss, tool failure, session replacement, or takeover.

After writing a checkpoint:

- continue immediately when `next_action` is safe and executable;
- rotate the worker session only when a different mode, fresh validator, or safer context is needed;
- keep the owner invocation active while the coordinator can still perform useful ready work.

Do not turn checkpoint cadence into owner-interaction cadence.

## Task terminal lifecycle

When a task is complete:

1. verify the exact final head, changed paths, acceptance criteria, required CI, reviews, ownership, and repository merge/close gate;
2. record the final result and exact evidence in the task checkpoint;
3. merge, close, or leave ready only as permitted by repository policy and the task authorization;
4. update programme state and dependency barriers;
5. move the task record from active to archive, or use the repository's equivalent terminal location/status;
6. release leases, branches/worktrees, and advisory path ownership as required;
7. remove or reconcile stale active indexes only through repository-approved mechanisms;
8. perform a fresh barrier review;
9. start the next ready task without routine owner confirmation.

A terminal task should not remain falsely active merely because more programme work exists.

## Waiting and external events

A worker must not remain active only to wait for CI, another task, deployment, an observation window, a scheduled run, or an owner reply.

When one task must wait:

1. persist `WAITING` with exact evidence and one `next_action`;
2. release its active worker session and lease where appropriate;
3. select another independent ready task;
4. return only when every authorized path is waiting/blocked or another real stop condition applies.

Bounded CI observation is allowed when it is part of the immediate final gate and tools return a result without indefinite polling. Repeated status polling is not useful work.

## Parallel work

Parallelism is allowed only when:

- tasks have independent owned paths or explicitly serialized shared owners;
- each worker has a separate branch/worktree;
- dependency and rollout order permit simultaneous work;
- the programme or repository concurrency limit is respected;
- one coordinator remains responsible for barrier review and shared-state reconciliation.

Do not increase parallelism merely because more agents are available. Prefer the smallest safe number of concurrent writers.

## Low-noise owner communication

During the autonomous run:

- do not narrate routine reads, searches, commands, unchanged checks, or every commit;
- do not emit a long plan before executing a resolvable short invocation;
- do not ask for approval for routine technical decisions already authorized by task and repository policy;
- send compact updates only for a material milestone, a real blocker, a required owner decision, or a material scope/safety change;
- keep durable detail in Git, task records, PRs, and artifacts rather than chat;
- provide one compact final summary when the run stops.

## Real stop conditions

Stop the owner invocation only when:

- all currently authorized programme work is complete;
- no safe `READY` task remains;
- all remaining work is genuinely `WAITING` or `BLOCKED`;
- an owner decision is materially required;
- ownership conflict or a safety rule prevents continuation;
- production, credentials, protected data, irreversible action, or another explicit gate requires separate authorization;
- context pressure or tool/environment limits make continuation unsafe;
- the first relevant failure must be isolated in a new bounded session after the allowed heavy-attempt limit.

The following are not stop conditions by themselves:

- a phase completed;
- a checkpoint was written;
- a commit was created;
- a PR was opened or updated;
- CI passed;
- a PR merged or closed;
- a task was archived;
- a worker session ended.

## Final response

Return a compact result covering the whole owner invocation:

```text
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <tasks/phases completed and material outcome>
VALIDATION: <exact checks and results>
DURABLE_STATE: <programme/task paths, branches, heads, PRs>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none when fully complete>
```

Do not paste full logs, chronological diaries, or every intermediate checkpoint.

## Anti-patterns

Do not:

- ask the owner to paste the next generated prompt after every phase;
- return immediately after creating a PR when implementation or validation can continue;
- leave completed tasks in the active directory indefinitely;
- confuse a worker's bounded stop with the coordinator's programme stop;
- claim continuous background monitoring after the final response;
- poll indefinitely instead of marking waiting and doing other ready work;
- silently broaden authorization, cross repository boundaries, bypass safety gates, or merge conflicting work.
