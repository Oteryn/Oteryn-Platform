# Resilient Multi-Agent Execution Protocol

## Purpose

This protocol keeps high parallel throughput without making chat sessions, Codex sessions, or the repository owner the durable coordination layer.

The durable unit is the task record and Git state. A worker session is disposable and may stop, lose context, exhaust limits, or be replaced at any time.

Policy version `2` adds context-pressure assessment, minimum-task decomposition, explicit audit and E2E profiles, session rotation, evidence externalization, and staged heavy validation.

## Operating model

Coordination has three levels:

1. **Portfolio control** chooses priorities across repositories.
2. **Repository control room** groups active work into project lanes and synchronization waves.
3. **Workers** execute one coherent task phase and persist the result.

Many workers may run in parallel. Only material exceptions are surfaced to the repository owner.

## Mandatory session contract

Every substantial worker session must:

1. Read the active task record, current branch/head, live PR, and relevant CI before broad discovery.
2. Select one phase: `investigate`, `design`, `implement`, `validate`, `integrate`, or `close`.
3. Assess task shape and context pressure before broad work and reassess after material discovery.
4. Record `phase`, `execution_mode`, `updated_at`, `status`, and one concrete `next_action` in the task checkpoint.
5. Persist a coherent commit and checkpoint after a material milestone and before a long-running or failure-prone operation.
6. Avoid routine narration, full logs, repeated architecture summaries, and repeated full preflight.
7. End or rotate the session after the coherent phase is complete, a real blocker is persisted, or context pressure becomes unsafe.

A session must not remain open merely to wait for CI, another task, an external observation window, a deployment, or a user reply.

## Minimum-task decomposition policy

The default objective is the smallest safe number of task records.

Start with `decomposition_decision: single` unless evidence shows that another shape is safer. Wall-clock duration, a slow build, a large file count, or Codex use are not split triggers by themselves.

Use these decisions:

- `single` — one cohesive task and normally one worker session;
- `phased` — one task, branch, and PR completed through multiple bounded phases or replacement sessions;
- `split` — multiple independent tasks are required because domains, ownership, acceptance criteria, or durable outputs are genuinely separable;
- `discovery_first` — scope is too uncertain to decide safely before a bounded discovery phase.

Before splitting, prefer this order:

1. externalize logs and evidence;
2. reduce unrelated scope;
3. write a compact checkpoint;
4. finish the current coherent phase;
5. rotate to a fresh session on the same task;
6. split only when safe completion remains unlikely or independent ownership requires it.

A session rotation never creates a new task, branch, or PR.

## Context-pressure assessment

Workers do not claim an exact remaining-token count unless a tool provides one. They estimate risk using five dimensions scored from `0` to `3`:

- `scope_breadth` — repositories, modules, and independent surfaces;
- `evidence_volume` — logs, documents, reports, screenshots, and artifacts;
- `history_dependency` — prior PRs, decisions, and compatibility constraints;
- `iteration_uncertainty` — likely test/fix/retest cycles;
- `parallel_hypotheses` — unrelated explanations that must remain active at once.

Suggested pressure bands:

- `low`: total `0-5`;
- `medium`: total `6-9`;
- `high`: total `10-12`;
- `unbounded`: total `13-15` or scope cannot yet be bounded.

Record only the compact result in the task checkpoint:

```yaml
policy_version: 2
task_kind: e2e
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: medium
decomposition_decision: single
decomposition_reason: one sequential scenario with shared state
```

Reassess after discovery, after the first full execution, after the second failed repair cycle, or when another repository or independent implementation enters scope.

When pressure becomes high, first externalize evidence and rotate the session while keeping the same task. Split only when the work itself is independent, not merely because the current conversation is long.

## Task profiles

### Discovery

Use `task_kind: discovery` when the worker must first determine boundaries, dependencies, feasibility, or decomposition.

Discovery may inspect and run bounded probes, but it must not silently expand into implementation. Its durable result is `single`, `phased`, `split`, or `blocked`, with evidence and the smallest next work package.

### Audit

Use `task_kind: audit` with `implementation_authorized: false` by default.

An audit reads code, configuration, history, and evidence; may run safe checks; and records findings with severity, confidence, evidence, impact, and recommendation. It does not repair findings unless the task explicitly authorizes a tightly bounded fix. Unrelated remediation becomes a recommendation or a separate task after the audit closes.

Prefer one audit task when the subject is cohesive. Split only independent audit domains and use one aggregation phase to deduplicate findings, resolve contradictions, and normalize severity.

### E2E

Use `task_kind: e2e` and keep one end-to-end flow in one task when steps share state or must remain sequential.

Split scenarios only when fixtures, ownership, acceptance criteria, and durable outputs are independent. Platform repair and feature-specific scenarios remain separate when they change different owned paths.

E2E logs, screenshots, SQL snapshots, traces, and binaries belong in artifacts or an evidence index, not in the active checkpoint.

### Implementation and validation

Use `task_kind: implementation` for product changes and `task_kind: validation` for independent exact-head verification. A separate validator session is preferred for heavy final validation, but it continues the same task unless the validation itself is an independent deliverable.

## Task and session identity

A task may outlive several sessions:

```yaml
task_id: EXAMPLE-001
session_id: agent-20260731-003
session_role: validator
session_rotation_count: 2
```

A replacement session reads only the task record, compact checkpoint, current diff, current PR/CI state, and specifically referenced evidence. It does not replay the full chat history or repeat a whole-repository audit.

## Checkpoint cadence and lease

The repository configuration in `docs/agents/PROJECT_LANES.json` defines:

- `checkpoint_interval_minutes`: target maximum interval between durable progress checkpoints;
- `lease_minutes`: how long one session may assume exclusive ownership without renewing its checkpoint;
- `stale_after_minutes`: when an active task is reported as `STALE`.

Recommended checkpoint fields:

```yaml
policy_version: 2
phase: implement
session_id: agent-20260731-001
session_role: implementer
execution_mode: codex
execution_reason: full checkout, multi-file edit, and focused test loop required
lease_expires_at: 2026-07-31T12:45:00+02:00
context_pressure: medium
context_growth: stable
decomposition_decision: phased
validation_level: focused
last_completed_step: added deterministic router and focused tests
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
```

The lease is advisory, not a distributed lock. Before takeover, a replacement worker must verify live Git, the PR head, active task ownership, and uncommitted work. Two workers must never write to the same branch or worktree concurrently.

## Evidence externalization

Keep checkpoints small. Store full logs, screenshots, traces, reports, generated files, SQL snapshots, and large diffs as repository files, PR artifacts, or workflow artifacts.

The checkpoint should reference an evidence index rather than copy the evidence:

```yaml
evidence_index: docs/agents/evidence/EXAMPLE-001/index.md
first_relevant_error: undefined reference to CatalogRouter
```

Read the smallest relevant evidence slice first: final status, failing job, first material error, and a bounded surrounding excerpt. Full logs remain available but are not loaded into context by default.

## Validation strategy

Validation is staged to minimize heavy runs without postponing all feedback until the end.

### Focused validation

Run cheap, narrow checks during implementation:

- syntax and formatting for changed files;
- unit tests for the changed function or module;
- focused type or contract checks;
- a minimal reproduction of the current failure.

### Component validation

After a coherent milestone, run the relevant package, component build, or bounded integration suite.

### Heavy validation

Run full builds, complete E2E, full regression suites, or version matrices primarily after coherent implementation is complete and before exact-head merge.

The default target is one heavy final run. Run an early heavy feasibility gate only when the primary risk is architectural viability, build-system behavior, database migration, protocol compatibility, generated output, or a cross-repository producer/consumer contract.

Do not run a heavy suite after every small change. After a heavy failure:

1. identify the first relevant failure;
2. reproduce it with the cheapest focused test available;
3. iterate using focused or component validation;
4. rerun the heavy suite only after the failure path is coherently repaired.

A session should normally perform no more than two full heavy attempts. After the second failure, checkpoint and isolate the defect before another full run.

Record:

```yaml
validation_level: full
heavy_validation_runs: 1
heavy_validation_result: failed
first_relevant_error: undefined reference to CatalogRouter
next_action: reproduce with the focused linker target
```

## Scope-expansion guard

A worker must not turn an audit or E2E task into an unbounded remediation program.

- Minor fixes inside declared ownership and acceptance criteria may be made when authorized.
- Unrelated findings are recorded only.
- Product defects discovered by validation become findings or follow-up recommendations unless the current task explicitly owns the repair.
- A material architecture or safety change requires a checkpoint and a new decision before implementation.

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

A worker completion, blocker, or failure does not require an immediate manual context switch. Results accumulate in durable task records until the repository control room performs a barrier review.

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

Prefer Chat plus the GitHub connector for repository state inspection, coordination, task records, PR metadata, narrow documentation changes, and evidence review.

Do not spend Codex capacity on waiting, status polling, repeated preflight, prompt generation, or general coordination.

Record the choice in `execution_mode` and the short reason in `execution_reason`.

## Failure and takeover

Before any operation that may run for a long time or fail silently, checkpoint first and use a bounded timeout when the tool supports it.

When a task is reported as `STALE`:

1. inspect the live branch, PR head, commits, checks, and task record;
2. determine whether the previous worker is still writing;
3. do not reuse a worktree or branch concurrently;
4. continue from the last coherent commit in a new session;
5. update `session_id`, `updated_at`, lease, counters, and `next_action`.

No important decision or work state may exist only in chat.

## Backward-compatible rollout

New task records use policy version `2` fields. Existing active tasks migrate when they are next claimed, resumed, or checkpointed.

During advisory rollout, missing v2 fields produce `policy=legacy` in the Control Room but do not fail CI. Do not create mass migration commits for untouched tasks. Enforcement may become stricter only after real audit, E2E, implementation, validation, and takeover cases have succeeded.

## Low-noise user communication

Workers report to the user only for:

- a material milestone;
- a real blocker requiring a decision;
- a material scope or safety change;
- final completion.

Routine file reads, searches, commands, unchanged checks, and full logs belong in tooling or durable evidence, not chat.

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
