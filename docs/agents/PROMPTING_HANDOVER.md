# Prompting Coordinator Handover

## Purpose

This document tells a continuation or coordinator agent how to apply `PROMPTING_STANDARD.md` to the repository owner's current idea or request. It is not the normative prompt-writing specification.

The authoritative construction rules, execution-mode routing, task-shape policy, validation ladder, templates, and quality gate are in:

```text
docs/agents/PROMPTING_STANDARD.md
```

## Mandatory flow

Before advising the owner or writing a worker prompt:

1. Read `PROMPTING_STANDARD.md`.
2. Identify the repository and correct project lane.
3. Inspect live active tasks, checkpoints, branches, PRs, required CI, ownership, and relevant contracts.
4. Classify the requested work as `discovery`, `audit`, `e2e`, `implementation`, `validation`, `integration`, `recovery`, or `close`.
5. Select `single`, `phased`, `discovery_first`, or `split` using `EXECUTION_PROTOCOL.md`.
6. Select the cheapest capable mode: Chat, Codex, Work, or a fresh validator session.
7. Correct an unsafe or overly broad request before turning it into a worker prompt.
8. Return one direct recommendation in Polish, one compact reason, and one ready-to-paste prompt compliant with `PROMPTING_STANDARD.md`.

Use this owner-facing shape:

```text
Rekomendacja: <mode and task shape>
Dlaczego: <compact reason>
Prompt dla agenta:
<one ready-to-paste prompt>
```

Do not ask the owner for information that live Git, task records, PRs, CI, or repository documentation can resolve. Do not offer several nearly identical prompts.

## Durable-state rule

Previous chat history is context, not authority. Live Git, the active task checkpoint, PR/CI state, ownership, and durable evidence control the generated prompt.

No material decision or execution state may remain only in chat. Persist it in the active task, PR, or repository documentation when applicable.

## Conflict order

When instructions overlap:

1. repository safety and security rules;
2. active task ownership and live Git/PR/CI state;
3. `EXECUTION_PROTOCOL.md` and `CONTEXT_HANDOFF.md`;
4. `PROMPTING_STANDARD.md`;
5. this handover;
6. stale conversational context.
