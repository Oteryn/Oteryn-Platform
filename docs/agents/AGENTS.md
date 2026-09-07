# Agent documentation instructions

These instructions govern `docs/agents/**` and supplement the root bootstrap.

## Authoring and routing

Use `META_AGENT_POLICY_BINDING.json` to resolve the immutable organization prompting or evaluation standard when authoring a material prompt or harness. `PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md` contain only Platform-specific routing and compatibility notes.

Every retained prompt and handoff must be classified in `DOCUMENTATION_IA_CATALOG.json`. Active reusable prompts are task-specific deltas over current instructions. Historical prompts remain provenance only and must not appear dispatchable.

For a new or resumed task packet:

1. read `EXECUTION_PROTOCOL.md` and `PROJECT_LANES.json`;
2. preserve the correct project lane and live Issue/PR identity;
3. use `TASK_TEMPLATE.md` and the checkpoint schema from `GOVERNANCE_CONTRACT.json`;
4. store one concrete `next_action` and keep detailed evidence outside the checkpoint;
5. leave `tasks/active/` when the governing Issue becomes terminal.

Load these Platform procedures only when triggered:

- `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md` for substantial implementation, validation or closeout;
- `REMEDIATION_AUDIT_RISK_GATE.md` for remediation validation intensity;
- `ANTI_STALL_AND_EXECUTION_BUDGET.md` for long-running, retry, CI-wait or continuation work;
- `SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md` before a delayed recheck, long-running operation or recovery;
- `TERMINAL_ONLY_COMMUNICATION.md` for autonomous or scheduled work;
- `GITHUB_ONLY_EXECUTION.md` only when local execution is unavailable;
- `AUTONOMOUS_PROGRAM_CONTINUATION.md` for programme start or continuation.

Do not restate organization execution, concurrency, Remote Desktop, AI-review, retry or merge procedures in a task, prompt or handoff. Keep task-specific scope, domain constraints, acceptance evidence and exceptional stop conditions.

## Validation

Changes in this directory must pass the affected checkpoint, liveness, Documentation IA, prompt and central-policy consumer checks. Product runtime E2E is `NOT_APPLICABLE` only when no executable product behavior changed and the task records that reason.
