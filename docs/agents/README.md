# Agent Coordination Entry Point

Use durable repository state instead of previous chat history.

Before advising the repository owner or writing a prompt for another agent, read `docs/agents/PROMPTING_HANDOVER.md`. It defines the required Polish recommendation, execution-mode routing, task-shape decision, ready-to-paste worker prompt, validation contract, stop conditions, and prompt quality gate.

Before resuming substantial work, read `docs/agents/CONTEXT_HANDOFF.md` and the active task checkpoint.

For decomposition, context pressure, session rotation, evidence externalization, and staged validation, read `docs/agents/EXECUTION_PROTOCOL.md`.

When instructions conflict, live Git/PR/CI state and the active durable task record override stale conversational context.
