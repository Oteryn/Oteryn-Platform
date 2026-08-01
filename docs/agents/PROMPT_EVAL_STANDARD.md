# Prompt Evaluation and Regression Standard

## Purpose

Prompts, agent instructions, tool descriptions, routing rules, examples, and coordinator contracts are versioned behavioural code. A change is not accepted because one demonstration looked good. It must preserve or improve measured behaviour on representative evaluations.

This contract supplements `PROMPTING_STANDARD.md`. Repository safety and task-specific acceptance remain authoritative when stricter.

## Prompt-as-code record

Every material prompting or harness change must record:

```yaml
prompt_contract:
  version: <semantic or monotonic version>
  changed_surfaces:
    - system instructions
    - repository instructions
    - worker template
    - tool description
    - routing or continuation rule
  objective: <observable behaviour to improve>
  baseline_version: <previous version>
  eval_suite: <path or durable identifier>
  rollback_version: <known safe version>
```

Keep the reason for the change and the failure mode it addresses. Do not add rules only because they sound prudent.

## Evaluation dataset

A useful eval suite contains:

- normal success cases;
- boundary and refusal cases;
- positive and negative tool-use cases;
- stale or conflicting repository state;
- ambiguous requests that live state can resolve;
- cases where the agent should continue autonomously;
- cases where it must stop for authority or safety;
- adversarial untrusted content and prompt injection;
- full-stack features where one layer is missing;
- closeout cases with stale, duplicate, superseded, or intentionally open PRs.

Balance cases where an action is required with cases where the same action would be wrong. Avoid suites that reward always searching, always splitting, always asking, always merging, or always refusing.

## Repeated trials

Agent behaviour is nondeterministic. For material changes, run multiple trials per case when the evaluator or environment supports it.

```yaml
eval_policy:
  minimum_trials: 3
  deterministic_checks: 1
  pass_threshold: <task-specific>
  maximum_regression: 0 on safety-critical cases
```

One lucky run is not sufficient evidence. A deterministic repository validator does not need artificial repetition.

## Trace and outcome

Evaluate two separate things:

- **trace quality** — whether the agent selected appropriate tools, respected ownership, avoided unnecessary calls, and followed the required process;
- **outcome quality** — whether the final repository, PR, application, database, artifact, or external system actually reached the required state.

Outcome has priority. A convincing final message is never terminal evidence.

```text
A worker completion claim is not proof of completion.
Verify the resulting environment state independently.
```

Examples of outcome evidence include exact file contents, changed paths, persisted records, reachable UI behaviour, exact-head CI, terminal PR state, archived task state, and released ownership.

## Machine-readable acceptance inventory

Substantial programmes should maintain acceptance in a structured inventory when practical:

```json
{
  "id": "FEATURE-001",
  "description": "Observable outcome",
  "verification": ["exact check"],
  "passes": false,
  "evidence": null
}
```

Rules:

- all criteria start false unless existing evidence proves otherwise;
- workers may add evidence and change status only after verification;
- workers must not delete, weaken, merge, or reinterpret criteria to obtain completion;
- changing acceptance meaning requires coordinator or owner authority according to repository policy;
- producer-only work must not mark the complete user-facing feature as passed.

## Evaluator independence

Material, risky, security-sensitive, cross-layer, or user-facing tasks should use a fresh validator with independent context.

```yaml
validator_policy:
  independent_context: true
  objective: falsify_acceptance
  trust_worker_summary: false
  inspect_environment_outcome: true
  implementation_authorized: false
```

The validator should attempt to disprove completion, exercise edge cases, and identify the first material failure. The validator may not dismiss a failed criterion merely because the implementation is otherwise strong.

## Canonical examples

Prefer a small number of diverse, stable examples derived from real failure modes. Include at least one positive example, one boundary example, and one negative example for important routing rules.

Do not grow prompts into exhaustive edge-case encyclopedias. When a new example is added, identify which eval failure it prevents.

## Model and runtime profiles

Prompts are not assumed portable across models or tool runtimes.

```yaml
model_profile:
  family: <model family>
  minimum_capability: <required capability>
  reasoning_effort: <adaptive or fixed>
  verbosity: low
  tool_contract_version: <version>
  compatibility_eval_required: true
```

Changing the model family, reasoning configuration, tool schema, or orchestration harness requires a targeted compatibility eval before broad use.

## Tool descriptions as prompt surfaces

Tool names, descriptions, parameters, returned errors, and side-effect metadata are part of the prompt contract. Evaluate whether tools are unambiguous, non-overlapping, and economical in context.

Write tools should expose, when applicable:

```yaml
tool_effects:
  side_effect: <none|repository_write|external_write|irreversible>
  idempotent: <true|false>
  exact_head_required: <true|false>
  rollback_available: <true|false>
  authorization_class: <class>
```

Errors should state the exact failure and a safe next action. Do not make several tools appear equivalent when their authority or effects differ.

## Efficiency metrics

Measure quality first, then efficiency. Useful metrics include:

- task success and safety pass rate;
- tool-call count and repeated reads;
- first-action latency;
- unnecessary owner questions;
- context loaded versus context used;
- heavy validation attempts;
- stale PRs or active tasks left behind;
- cost and runtime where available.

Never improve efficiency by weakening acceptance, skipping required E2E, hiding failures, or reducing safety checks.

## Ablation and simplification

Periodically remove or disable one rule, example, or scaffold and rerun the same evals. Keep it only when it provides measurable value or protects a clearly documented safety invariant.

Newer models may need less scaffolding. Old rules can conflict, waste context, or cause loops. Prefer the smallest contract that consistently passes the required eval suite.

## Change gate

A material prompt change may merge only when:

- baseline and candidate use the same representative eval set;
- safety-critical cases have no regression;
- outcome verification passes;
- failures and trade-offs are documented;
- rollback is available;
- exact changed prompt/tool surfaces are known;
- the final diff does not silently weaken authority, acceptance, E2E, audit, or closeout.

A prompt or harness update that lacks executable eval infrastructure may use a documented manual scenario matrix, but the absence of automation must be explicit and must not be described as an automated pass.
