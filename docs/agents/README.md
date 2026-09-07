# Agent Coordination Entry Point

Use durable repository state instead of previous chat history.

Before advising the repository owner or writing a prompt for another agent:

1. read `docs/agents/PROMPTING_HANDOVER.md` for the coordinator workflow and live-state inspection order;
2. read `docs/agents/PROMPTING_STANDARD.md` for the normative prompt structure, mode routing, task-shape rules, validation contract, templates, stop conditions, and quality gate.

Before resuming substantial work, read `docs/agents/CONTEXT_HANDOFF.md` and the active task checkpoint.

For decomposition, context pressure, session rotation, evidence externalization, and staged validation, read `docs/agents/EXECUTION_PROTOCOL.md`.

When instructions conflict, live Git/PR/CI state and the active durable task record override stale conversational context.
