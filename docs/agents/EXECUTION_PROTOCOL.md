# Resilient Multi-Agent Execution Protocol

## Purpose

This protocol keeps high parallel throughput without making chat sessions, Codex sessions, or the repository owner the durable coordination layer.

The durable unit is the task record and Git state. A worker session is disposable and may stop, lose context, exhaust limits, or be replaced at any time.

## Operating model

Coordination has three levels:

1. **Portfolio control** chooses priorities across repositories.
2. **Repository control room** groups active work into project lanes and synchronization waves.
3. **Workers** execute one bounded phase and persist the result.

Many workers may run in parallel. Only material exceptions are surfaced to the repository owner.

## Mandatory session contract

Every substantial worker session must:

1. Read the active task record, current branch/head, live PR and relevant CI before broad discovery.
2. Select one phase: `investigate`, `design`, `implement`, `validate`, `integrate`, or `close`.
3. Record `phase`, `execution_mode`, `updated_at`, `status`, and one concrete `next_action` in the task checkpoint.
4. Persist a coherent commit and checkpoint after a material milestone and before a long-running or failure-prone operation.
5. Avoid routine narration, full logs, repeated architecture summaries, and repeated full preflight.
6. End the session after the bounded phase is complete or a real blocker is persisted.

A session must not remain open merely to wait for CI, another task, an external observation window, a deployment, or a user reply.

## Checkpoint cadence and lease

The repository configuration in `docs/agents/PROJECT_LANES.json` defines:

- `checkpoint_interval_minutes`: target maximum interval between durable progress checkpoints;
- `lease_minutes`: how long one session may assume exclusive ownership without renewing its checkpoint;
- `stale_after_minutes`: when an active task is reported as `STALE`.

Recommended optional checkpoint fields:

```yaml
phase: implement
session_id: agent-20260731-001
execution_mode: codex
execution_reason: full checkout, multi-file edit and focused test loop required
lease_expires_at: 2026-07-31T01:45:00+02:00
last_completed_step: added deterministic router and focused tests
```

The lease is advisory, not a distributed lock. Before takeover, a replacement worker must verify live Git, the PR head, active task ownership and uncommitted work. Two workers must never write to the same branch or worktree concurrently.

## Status model

Task files may use the repository's existing detailed statuses. The Control Room normalizes them to:

- `RUNNING` — active checkpoint is fresh;
- `READY` — next phase may start;
- `BLOCKED` — a real dependency or decision prevents progress;
- `WAITING` — no worker should remain active while an external event is pending;
- `STALE` — an active checkpoint exceeded the configured threshold;
- `DONE` — implementation or closure is complete;
- `UNKNOWN` — task state cannot be classified safely.

`STALE` is derived by the Control Room and does not rewrite task files automatically.

## Non-interactive blocker rule

A worker that reaches a blocker must not keep the session alive waiting for instructions.

It must:

1. preserve coherent work in Git where safe;
2. update the task checkpoint;
3. set `status: blocked` or `status: waiting`;
4. record the exact blocker and evidence;
5. leave one concrete `next_action`;
6. release the lease and end the session.

Questions requiring the repository owner are collected at the next synchronization barrier. Routine technical choices remain autonomous.

## Wave and barrier coordination

Parallel work is dispatched in waves.

A worker completion, blocker or failure does not require an immediate manual context switch. Results accumulate in durable task records until the repository control room performs a barrier review.

At a barrier the coordinator:

1. runs `python tools/agents/control_room.py --format markdown`;
2. verifies live PR and CI state for changed or stale tasks;
3. resolves task dependencies and path ownership;
4. selects the next ready work packages;
5. escalates only decisions that genuinely require the repository owner.

A task waiting for another task or external event has no active worker session.

## Codex and execution-mode decision

The task worker chooses the execution mode without asking the repository owner for routine approval.

Use Codex when the phase requires a full checkout, multi-file edits, terminal commands, build/test loops, generated files, or iterative code repair.

Prefer Chat plus the GitHub connector for repository state inspection, coordination, task records, PR metadata, narrow documentation changes and evidence review.

Do not spend Codex capacity on waiting, status polling, repeated preflight, prompt generation, or general coordination.

Record the choice in `execution_mode` and the short reason in `execution_reason`.

## Failure and takeover

Before any operation that may run for a long time or fail silently, checkpoint first and use a bounded timeout when the tool supports it.

When a task is reported as `STALE`:

1. inspect the live branch, PR head, commits, checks and task record;
2. determine whether the previous worker is still writing;
3. do not reuse a worktree or branch concurrently;
4. continue from the last coherent commit in a new session;
5. update `session_id`, `updated_at`, lease and `next_action`.

No important decision or work state may exist only in chat.

## Low-noise user communication

Workers report to the user only for:

- a material milestone;
- a real blocker requiring a decision;
- a material scope or safety change;
- final completion.

Routine file reads, searches, commands, unchanged checks and full logs belong in tooling or durable evidence, not chat.

## Control Room commands

Repository summary:

```bash
python tools/agents/control_room.py --format markdown
```

Machine-readable output:

```bash
python tools/agents/control_room.py --format json
```

Detect stale work in automation:

```bash
python tools/agents/control_room.py --fail-on-stale
```

Select one project lane:

```bash
python tools/agents/control_room.py --lane <lane-id>
```

The lane identifiers and task discovery paths are defined in `docs/agents/PROJECT_LANES.json`.
