# Prompting Coordinator Handover

## Purpose

This document tells a continuation or coordinator agent how to apply the prompting standard to the repository owner's current request and how to execute short programme invocations.

The authoritative contracts are:

```text
docs/agents/PROMPTING_STANDARD.md
docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
docs/agents/EXECUTION_PROTOCOL.md
docs/agents/CONTEXT_HANDOFF.md
```

`PROMPTING_STANDARD.md` defines prompt construction and run scope. `AUTONOMOUS_PROGRAM_CONTINUATION.md` defines the long foreground coordinator loop. This handover defines how to resolve the owner's words into those contracts.

## Mandatory startup

Before advising the owner, writing a worker prompt, or executing a programme command:

1. read `PROMPTING_STANDARD.md`;
2. read `AUTONOMOUS_PROGRAM_CONTINUATION.md` when a programme, coordinator, rollout, short command, or autonomous continuation is involved;
3. identify the repository and correct project lane;
4. inspect live active/ready tasks, checkpoints, branches, PRs, required CI, ownership, barriers, and relevant contracts;
5. classify the current bounded work as `discovery`, `audit`, `e2e`, `implementation`, `validation`, `integration`, `recovery`, or `close`;
6. select `single`, `phased`, `discovery_first`, or `split` using `EXECUTION_PROTOCOL.md`;
7. select the cheapest capable mode: Chat, Codex, Work, or a fresh validator session;
8. resolve unsafe, stale, or overly broad assumptions from live state before acting;
9. never ask the owner for information that Git, task records, PRs, CI, registries, or repository documentation can resolve.

## Advisory requests

When the owner asks only for a plan, recommendation, or prompt, use:

```text
Rekomendacja: <mode and task shape>
Dlaczego: <compact reason>
Prompt dla agenta:
<one ready-to-paste prompt>
```

Do not offer several nearly identical prompts. Do not fabricate repository state.

## Short owner invocation

The owner may start or continue a durable programme with a short natural-language command instead of pasting a generated worker prompt.

Recognize forms such as:

```text
Uruchom <program> autonomicznie.
Kontynuuj <program> autonomicznie.
Uruchom <program> <task>.
Kontynuuj <program> <task>.
Zweryfikuj <program> <task>.
Pokaż stan <program>.
Zamknij <program>.
```

When the command is resolvable:

1. locate the repository-owned short-invocation registry when present;
2. locate the programme/coordinator task and current wave/barrier;
3. read live task checkpoints, exact branches/heads, PRs, CI, leases, ownership, blockers, and `next_action` values;
4. generate the bounded worker instruction internally from live state, using `resume.py` when available or constructing the equivalent contract;
5. set:

```yaml
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

6. execute the current autonomous coordinator loop instead of returning the generated prompt to the owner;
7. continue across ready phases and tasks until a real stop condition is reached;
8. persist every material decision and state transition in Git, the task record, PR, or programme documentation.

Do not ask the owner to paste, reconstruct, or maintain the long prompt.

For WickHunter, when the registry exists, use:

```text
docs/agents/prompts/WICKHUNTER_SHORT_INVOCATIONS.md
```

The generic command `Kontynuuj WickHunter autonomicznie` resolves through the live rollout coordinator, current wave, barriers, and task checkpoints. It does not authorize protected-holdout access, credentials, orders, live capital, or any other action forbidden by repository policy.

Other programmes may define equivalent registries. Discover them from live repository state rather than inventing static paths.

## Continuous execution semantics

A short autonomous invocation is not satisfied by producing a plan, opening one PR, completing one phase, or archiving one task.

During the same owner invocation, the coordinator should:

1. execute the highest-priority safe ready phase;
2. checkpoint material progress;
3. continue the same task when its next phase is ready;
4. finalize and archive the task when terminal;
5. update programme state and release ownership;
6. review the synchronization barrier;
7. select and execute the next ready task;
8. repeat until a real stop condition applies.

A worker session may end or rotate while the owner invocation continues. Checkpoint cadence must not become owner-interaction cadence.

## Waiting behavior

Do not keep a worker active merely to wait for CI, another task, deployment, an external observation window, a scheduled run, or a user response.

Persist the waiting task, release its worker session, and continue another independent ready task. Return only when all authorized paths are waiting/blocked or another real stop condition applies.

Do not repeatedly poll unchanged state.

## Task completion and archival

When a task reaches its terminal gate:

- verify exact head, changed paths, reviews, required CI, and acceptance;
- write the terminal checkpoint and exact evidence;
- merge, close, or leave ready only as repository policy permits;
- archive or move the task record according to repository convention;
- release lease and owned paths;
- update the programme/coordinator task and barrier state;
- continue with the next ready task without routine owner confirmation.

Do not leave a completed task falsely active merely because the programme continues.

## Low-noise owner experience

For autonomous execution:

- do not narrate routine reads, searches, tool calls, commands, or unchanged checks;
- do not send the full internal prompt unless the owner explicitly asks for it;
- do not ask questions answerable from live state;
- send compact updates only for material milestones, real blockers, required decisions, or material safety/scope changes;
- avoid walls of text and chronological diaries;
- provide one compact final summary when the autonomous run actually stops.

## Real stop conditions

Return to the owner only when:

- all currently authorized work is complete;
- no safe ready task remains;
- remaining work is genuinely waiting or blocked;
- a material owner decision or additional authorization is required;
- ownership or repository safety rules prevent continuation;
- context/tool/environment limits make continued work unsafe;
- the heavy-attempt limit requires defect isolation in a new bounded session.

Do not stop merely because a checkpoint, commit, PR, green CI result, merge, phase completion, worker-session end, or task archive occurred.

## Durable-state rule

Previous chat history is context, not authority. Live Git, active task checkpoints, programme state, PR/CI, ownership, and durable evidence control all generated prompts and execution decisions.

No material decision or execution state may remain only in chat.

## Conflict order

When instructions overlap:

1. repository safety, security, authorization, production, and cross-repository rules;
2. active task ownership and live Git/PR/CI state;
3. `EXECUTION_PROTOCOL.md` and `CONTEXT_HANDOFF.md`;
4. `PROMPTING_STANDARD.md`;
5. `AUTONOMOUS_PROGRAM_CONTINUATION.md`;
6. this handover;
7. stale conversational context.
