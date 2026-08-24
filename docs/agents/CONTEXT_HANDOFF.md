# Oteryn Platform Agent Context Handoff

## Principle

Chat history is disposable and never authoritative. The governing live GitHub Issue/task is canonical for task lifecycle state. The live GitHub PR is authoritative for PR head/base/check/review/merge state.

Repository task/context records and deterministic evidence are durable context, evidence, ownership, handoff, next-action and historical records, not an independent competing mutable lifecycle authority. Stale record fields do not override newer live Issue/PR state. Local filesystem/worktree state is an execution/cache plane and does not override GitHub control-plane state.

A continuation agent must be able to resume without reading the previous conversation and must reconcile durable records against live GitHub before acting.

## Governance contract version

The machine-readable marker is `docs/agents/GOVERNANCE_CONTRACT.json`.

Current shared checkpoint structure: **version 1**. Current policy revision: **4**.

Policy revision 4 is backward-compatible with checkpoint structure version 1. Revision 2 added accepted task statuses `waiting` and `completed`, validation result `NOT_APPLICABLE`, and separated checkpoint task status from terminal invocation result. Revision 3 added fail-closed live PR/branch/archive liveness. Revision 4 adds a required active-task `governing_issue` frontmatter identity plus fail-closed live governing-Issue validation. Existing valid version 1 checkpoints remain structurally valid and do not require bulk migration.

The shared contract covers required checkpoint fields, supported task statuses, evidence-state fields (`PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT`), supported validation results, one top-level concrete `next_action`, and the rule that the same normalized fact cannot occupy multiple evidence-state lists. Repository-specific live-state policy additionally requires every active task to name a numeric governing Issue whose live state is open.

Checkpoint task statuses are:

```text
investigating | implementing | validating | ready | waiting | blocked | completed
```

Terminal invocation results are:

```text
DONE | WAITING | BLOCKED | ROTATE
```

`ROTATE` must never be written as checkpoint task status. Before returning `ROTATE`, persist `ready`, `waiting`, or `blocked` with one concrete `next_action`.

`blakinio/canary` is a read-only compatibility reference for shared checkpoint semantics. Oteryn Platform allowlists, delivery policy, architecture routing, Laravel, database, security, and other repository-specific governance remain independent and must not be mechanically synchronized.

A structural version upgrade is required only for an incompatible checkpoint change, such as adding or removing a required field, changing a field type, removing an accepted value, or invalidating an existing valid checkpoint. Backward-compatible additive values and repository-specific live-state invariants use a synchronized policy revision instead.

## When to checkpoint

Update the active task record when:

- a root cause or blocker is proven;
- a hypothesis is rejected by evidence;
- files are materially modified;
- validation or CI changes task state;
- branch, head, PR, or governing Issue state changes;
- review feedback changes required work;
- context becomes large, repetitive, or unreliable;
- before session replacement or context exhaustion.

## Context pressure protocol

1. Stop broad exploration.
2. Verify the governing live GitHub Issue lifecycle state and live branch, head and PR state, then verify the working-tree state.
3. Update the active task `## Context checkpoint` as durable context/evidence without allowing stale lifecycle or PR fields to override GitHub.
4. Preserve only coherent work; otherwise record uncommitted paths.
5. Record exact validation evidence.
6. Leave exactly one concrete `next_action`.

## Checkpoint schema

Every substantial active task contains a checkpoint whose `checkpoint_version` matches `shared_checkpoint_contract.version`:

```yaml
checkpoint_version: 1
updated_at: YYYY-MM-DDTHH:MM:SSZ
head: <commit-sha-or-UNKNOWN>
branch: <branch>
pr: <number-or-none>
status: investigating|implementing|validating|ready|waiting|blocked|completed
context_routes:
  - <route>
owned_paths:
  - <path/glob>
proven:
  - <fact backed by source/tool/test evidence>
derived:
  - <explicit conclusion derived from proven facts>
unknown:
  - <unresolved fact>
conflicts:
  - <authoritative evidence conflict>
first_failure:
  marker: <first unmet invariant/check or none>
  evidence: <artifact/log/test/source reference>
rejected_hypotheses:
  - <hypothesis>: <disproving evidence>
changed_paths:
  - <path>
validation:
  - command: <command/workflow/job>
    result: PASS|FAIL|BLOCKED|NOT_RUN|NOT_APPLICABLE
    evidence: <short reference; required reason for NOT_APPLICABLE>
blockers:
  - <blocker or none>
next_action: <one concrete next step>
```

Use `waiting` when an external event is pending and no worker should remain active. Use `blocked` for a real decision, permission, safety, resource, or exhausted-repair barrier. Use `ready` when another session can execute `next_action`. Use `completed` only after repository closeout gates are satisfied. Checkpoint `status`, `head`, `branch` and `pr` fields are durable mirrors for continuation; reconcile them to the governing live GitHub Issue/PR whenever live state is newer.

The active task frontmatter must also contain:

```yaml
governing_issue: <positive GitHub Issue number>
```

The governing Issue must be live and open while the packet remains under `docs/agents/tasks/active/`. Closing the Issue requires the packet to leave the active directory in the same bounded closeout flow.

Omit irrelevant historical detail. Preserve only what a new agent needs to continue correctly.

Validate one checkpoint locally with:

```sh
python tools/agents/checkpoint.py docs/agents/tasks/active/<task>.md --require-checkpoint
```

Validate all active task records with:

```sh
python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
```

Run validator tests with:

```sh
python tools/agents/test_checkpoint.py
python tools/agents/test_task_liveness.py
python tools/agents/test_task_issue_liveness.py
```

In Agent Governance, `task_liveness.py` validates live PR/branch/archive identity and `task_issue_liveness.py` validates the live governing Issue. Both fail closed when their required GitHub state cannot be established.

The checkpoint validator checks deterministic structure only. It does not verify that head, branch, PR state, CI status, evidence references, or repository state are currently true; agents must perform live verification.

## Starting a continuation agent

Resolve the governing live GitHub Issue/task and live PR first, then generate the compact continuation prompt from durable task context:

```sh
python tools/agents/resume.py --task docs/agents/tasks/active/<task>.md
```

The continuation agent reconciles any stale lifecycle/PR fields, verifies only live state that can invalidate the next action, does not rediscover `PROVEN` facts unless evidence changed, and does not reconstruct state from previous chat.

If no checkpoint exists, `resume.py` should produce a checkpoint-recovery action before substantive implementation.

## Evidence states

- `PROVEN`: directly supported by source, deterministic tool output, tests, logs, artifacts, or live GitHub state.
- `DERIVED`: conclusion that follows from listed proven facts.
- `UNKNOWN`: not established. Never replace with a guess.
- `CONFLICT`: authoritative evidence disagrees and requires resolution.

## Security-specific handoff requirements

For auth, admin, database, secrets, payment, protocol, asset, protected-data, or live-capital work, state:

- the affected trust boundary;
- authentication or authorization invariant affected;
- schema, session, protocol, or cross-repository compatibility impact;
- rollback requirement;
- whether secrets, production configuration, live data, or protected operations are involved.

Never copy secrets into task records, PRs, logs, or handoffs.

## Handoff quality gate

A handoff is incomplete if the next agent cannot answer:

- What is the governing live GitHub Issue lifecycle state, and which branch, PR, and head are current?
- What is proven versus derived, unknown, or conflicting?
- What failed first, if anything?
- Which files changed?
- What validation ran and what was the result?
- What task status applies?
- What blocker remains?
- What is the single next action?

## Anti-bloat

Do not paste full logs, full diffs, whole source files, database dumps, long chat summaries, or unrelated documentation into checkpoints. Store exact references instead.
