# R5 Platform instruction reduction evidence

Issue: #1302  
Admission Platform main: `3557085c20512d25576d8884cc54471665784b00`  
Bound META authority: `Oteryn/Oteryn@5ed3f14400af450b5875c091e443da70f2d67ab9`, policy `3.0.0`

## Structural comparison

Counts use UTF-8 file bytes and newline counts. They measure repository source reduction only; they do not establish loaded-token, API-cost or behavior savings.

| Surface | Before | Candidate | Change |
| --- | ---: | ---: | ---: |
| 12 core Platform instruction/prompt documents | 145,815 B / 2,040 lines | 27,193 B / 305 lines | -118,622 B / -1,735 lines |
| Policy/prompt validator code, tests and active eval suite (5 files) | 116,524 B / 2,238 lines | 59,357 B / 1,345 lines | -57,167 B / -893 lines |
| Ordinary root task mandatory local sources | 3 files / 52,435 B / 582 lines | 3 files / 10,590 B / 113 lines | -41,845 B / -469 lines |
| `docs/agents/**` task mandatory local sources | 4 files / 62,316 B / 671 lines | 4 files / 11,786 B / 123 lines | -50,530 B / -548 lines |

Before ordinary-task sources were root `AGENTS.md`, `PLATFORM_AGENT_BOOTSTRAP.md` and the always-consulted `CONTEXT_ROUTING.md`. Candidate sources are root, the thin bootstrap and `META_AGENT_POLICY_BINDING.json`; context routing is on demand. A `docs/agents/**` task additionally loads the nearest nested `AGENTS.md` in both states.

The core set is root, bootstrap, nested, context routing, prompting handover/standard/eval standard, anti-stall, autonomous continuation, delivery/closeout, GitHub-only and terminal-communication documents. The consumer set is policy consistency code/tests, prompt evaluator code/tests and its selected JSON suite. The candidate selects the v2 suite; the unchanged v1 suite remains as historical evidence.

## Consumer behavior

- Before importing checkout-owned Python, `policy_consistency.py` validates the closed binding identity and full SHA, verifies exact clean checkout identity, and independently authenticates protected-main ancestry through GitHub. A rejected checkout cannot execute its candidate validator.
- The executable derives the bound checkout path under its fixed repository root from the validated full SHA; it exposes no command-line repository or policy-checkout path.
- Agent Governance checks out META with a literal immutable commit pin and fails when the binding differs from that pin, preventing a candidate-controlled checkout from poisoning later workflow steps.
- After that trust bootstrap, the consumer loads META's own `central_agent_policy.py` from the exact Git checkout, validates the complete META bundle and binding against the already authenticated snapshot, applies META's provider-overlay validator, and separately retains bounded Platform invariants.
- Local checkpoint and liveness shapes remain in `GOVERNANCE_CONTRACT.json` and their existing focused validators. Status and budget lists are no longer copied into prose for `policy_consistency.py` to parse.
- The deterministic prompt evaluator checks Platform-owned deltas and reports `model_trials_executed=0`. It does not claim provider delivery or runtime/model behavior.
- The existing Portal Completion scope-manifest contract remains covered unchanged, including non-scheduling, non-self-activation and per-capability disposition semantics.

## Representative execution manifest

```yaml
execution_surface: ChatGPT Work / Codex subagent
repository: Oteryn/Oteryn-Platform
starting_working_directory: /workspace/scratch/3742b8bda0f6
repository_checkout_present_at_start: false
repository_instruction_auto_load_from_starting_directory: not_observed
explicit_repository_source_order:
  - live protected-main AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md for docs/agents paths
  - task-relevant docs/agents/CONTEXT_ROUTING.md
  - bound META policy and prompting/evaluation sources
parent_model: not independently exposed to this child
child_model: gpt-5.6-sol
child_reasoning_effort: high
model_or_effort_hard_ceiling: UNKNOWN
model_trials_executed: 0
loaded_token_count: NOT_MEASURED
api_cost: NOT_MEASURED
```

The repository sources were explicitly read after live GitHub preflight. Their existence alone is not treated as proof that the client loader would have discovered them from the original working directory.
