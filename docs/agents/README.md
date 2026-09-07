# Agent documentation

Root `AGENTS.md`, `PLATFORM_AGENT_BOOTSTRAP.md` and `META_AGENT_POLICY_BINDING.json` form the Platform instruction entry point. The nearest nested `AGENTS.md` applies when a task changes this directory.

Load other files on demand:

- `CONTEXT_ROUTING.md` selects domain context when the governing task and paths do not already make it clear.
- `PROMPTING_STANDARD.md`, `PROMPTING_HANDOVER.md` and `PROMPT_EVAL_STANDARD.md` are Platform-specific deltas used only for prompt work.
- `GOVERNANCE_CONTRACT.json`, `TASK_TEMPLATE.md` and the live Issue/PR govern task-record changes.
- Specialist documents apply only when their named subject is material.

Do not treat this directory as a mandatory reading list. Historical plans, prompts, reports and archived tasks preserve evidence; they are not current execution authority.
