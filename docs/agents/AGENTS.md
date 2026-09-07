# Agent-document instructions

These instructions apply only under `docs/agents/**`.

- Treat root `AGENTS.md` and `docs/agents/META_AGENT_POLICY_BINDING.json` as the instruction entry point. Do not recreate organization-wide execution, review, Remote Desktop, retry, continuation, model or merge policy in this directory.
- Load only the target document and the source that owns the decision being changed. A link is not a recursive read requirement.
- When writing a reusable task prompt, use `PROMPTING_STANDARD.md` as the Platform delta and the prompting standard selected by the META binding. Keep the prompt task-specific.
- When changing a prompt evaluator, keep deterministic contract checks, provider adoption/delivery evidence and runtime/model evidence separate. A text check is not a model trial.
- When changing a task record, use `tasks/TASK_TEMPLATE.md`, `GOVERNANCE_CONTRACT.json` and the live governing Issue/PR. Never leave a terminal PR represented as active without the contract's explicit archive-pending transition.
- Preserve historical evidence as history. Historical prompts, plans and task packets do not become current authority merely because they remain in the repository.
