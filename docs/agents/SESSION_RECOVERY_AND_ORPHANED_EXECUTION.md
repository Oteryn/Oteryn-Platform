# Session Recovery and Orphaned Execution Contract

```yaml
session_recovery_policy_version: 1
```

## Purpose

Autonomous work must survive chat interruption, tool cancellation, context loss, browser disconnect, worker replacement, or an execution process that disappears while the UI still appears busy. This contract does not claim hidden background execution and cannot force a platform process to restart itself. It makes the repository state sufficient for the next invocation to resume the exact safe action without owner reconstruction.

This contract specializes `ANTI_STALL_AND_EXECUTION_BUDGET.md`, `AUTONOMOUS_PROGRAM_CONTINUATION.md`, and the repository handoff rules. It never weakens safety, authority, ownership, merge, production, data, credential, or protected-environment restrictions.

## Durable recovery checkpoint

Before the first sleep, delayed recheck, terminal-CI wait, long-running command, runner job, external observation window, or other operation during which the worker may be interrupted, persist a compact `## Recovery checkpoint` in the authoritative task record.

Use this additive shape:

```yaml
recovery:
  policy_version: 1
  generation: <positive integer>
  session_id: <invocation timestamp or stable identifier>
  session_started_at: <timestamp>
  checkpointed_at: <timestamp>
  last_progress_at: <timestamp>
  phase: <current bounded phase>
  exact_head: <sha or not applicable>
  pull_request: <number or none>
  active_operation: <command, workflow wait, review wait, or none>
  external_run_ids: [<ids>]
  operation_started_at: <timestamp or null>
  wait_deadline_at: <timestamp or null>
  check_generation: <name or null>
  checks_used: <integer>
  status: active | ready | waiting | blocked
  safe_to_resume: true | false
  resume_condition: <observable condition>
  next_action: <one executable imperative action>
```

The checkpoint must be committed and pushed before entering a deliberate wait when repository policy requires durable task records. Do not create a commit before every poll: update the checkpoint only when the head, run IDs, counters, deadline, status, blocker, or next action materially changes.

No autonomous wait is valid without a durable recovery checkpoint. A chat message, UI spinner, worker memory, uncommitted file, or local-only note is not a recovery checkpoint.

## Immediate resume rule

At the start of every continuation or replacement session:

1. read the authoritative task and its latest recovery checkpoint before reconstructing broad context;
2. verify the live branch, exact head, PR, ownership, and the external operation named in the checkpoint;
3. if `safe_to_resume: true` and `next_action` remains valid, execute it immediately without asking the owner to repeat the request or provide another prompt;
4. if the external operation completed during the interruption, reconcile its result and continue to the next phase;
5. if it is still pending and bounded waiting remains eligible, continue using the original `operation_started_at`, `wait_deadline_at`, check generation, and counters;
6. never reset a wait budget, retry counter, or CI generation merely because a new session took over;
7. if the wait or execution budget has expired, persist `waiting`, `ready`, or `blocked` accurately and return the canonical terminal result.

A short owner command such as `kontynuuj` is sufficient to trigger this recovery. Live durable state controls; chat history is optional.

## Orphaned-session takeover

A UI that still says that an agent is thinking is not durable ownership evidence.

Treat the previous execution as orphaned when its process or tool session is unavailable, its declared wait deadline has expired, or live repository evidence shows no worker-controlled progress and the previous session cannot be contacted. Before takeover, verify that no other active agent currently owns or mutates the same branch, paths, PR, runner, deployment, or protected state.

When takeover is safe:

- increment `recovery.generation`;
- assign a new `session_id`;
- preserve all prior counters, deadlines, run IDs, findings, and exact heads;
- record that the previous session was recovered as orphaned;
- continue from the existing `next_action` rather than restarting the task or full preflight.

A stale session must not permanently block the programme. Conflicting live ownership must stop takeover and produce an exact blocker instead.

## CI observation aggregation

One CI state check means one aggregate snapshot of all required checks for one exact head and check generation.

- Query the PR/head-level required-check state once per observation.
- Inspect an individual workflow or job only to explain a failure found in that aggregate snapshot.
- Sequentially querying every pending workflow does not create separate observations and must not be used to bypass the poll interval or check cap.
- The minimum unchanged terminal-CI interval is measured from completion of one aggregate observation to the start of the next.
- Fixed 30-second sleeps followed by repeated workflow queries are forbidden.
- Preserve the original terminal wait start, deadline, generation, and counters across session recovery.

## Controlled interruption and finalization

When cancellation, context pressure, tool loss, timeout, or session rotation is observable before the process ends:

1. stop starting new work;
2. persist the recovery checkpoint and exact durable state;
3. leave exactly one executable `next_action`;
4. return `WAITING`, `BLOCKED`, or `ROTATE` using the canonical terminal response.

An abrupt platform failure may prevent that final response. The next invocation must then recover from the most recent durable checkpoint and live state. Never claim that work continued in the background when no durable evidence proves it.

## Forbidden patterns

Do not:

- enter a wait or long-running operation without first persisting recovery state;
- leave a task dependent on chat history or an unwritten mental plan;
- restart the whole task after interruption when a valid checkpoint exists;
- ask the owner what to do when `next_action` is safe and executable;
- reset wait budgets or counters after session replacement;
- treat a UI spinner as proof that an agent still owns the work;
- poll workflows one by one at short intervals;
- leave the invocation without a terminal result when a controlled stop is possible.
