# Platform Prompting Standard

Platform prompt authors must resolve the organization prompting standard selected by `META_AGENT_POLICY_BINDING.json` and treat this file as a repository-specific delta.

## Platform-specific authoring

- Owner-facing recommendations are Polish unless the owner requests another language; worker prompts are concise English by default.
- A worker prompt names the observable Platform outcome, writable paths, governing Issue/PR locators, domain constraints, dependencies, acceptance evidence and exceptional stop condition that are unique to the task.
- Route architecture, security, data, payments, authentication, deployment and cross-repository work through `CONTEXT_ROUTING.md`. Preserve every applicable root product and safety invariant.
- Resolve short programme commands through `SHORT_PROGRAM_INVOCATIONS.md` and current live state. Do not turn a resolvable command into another long meta-prompt.
- Do not ask the owner for facts available through authorized repository reads.

## Prompt lifecycle

Every retained prompt under `docs/agents/prompts/` has one entry in `DOCUMENTATION_IA_CATALOG.json`.

- `active_reusable` prompts contain only a task or programme delta and reconstruct mutable facts at invocation.
- `historical_do_not_run` prompts remain inert provenance and route readers to their recorded supersession target.
- Updating prompt files or their lifecycle requires the catalog, relevant deterministic evaluation and affected documentation validation in the same change.

A prompt may point to a Platform procedure, contract, skill or immutable META source. The reference does not create authority and does not make every linked document mandatory background reading.

## Platform delivery delta

For a user-facing Platform task, define the applicable actor, entry point, backend/domain effect, persistence or external effect, observable output, error/authorization behavior and real E2E evidence. For governance-only or documentation-only work, state the concrete reason product runtime E2E does not apply.

Use `PROMPTING_HANDOVER.md` only for owner advice or a worker handoff. Use `PROMPT_EVAL_STANDARD.md` when the prompt or harness change is material.
