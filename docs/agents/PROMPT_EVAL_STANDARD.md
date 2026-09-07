# Platform Prompt Evaluation Delta

Resolve the prompt-evaluation standard named by `docs/agents/META_AGENT_POLICY_BINDING.json`. This file adds Platform-specific evidence requirements.

Keep three evidence classes separate:

1. **Structural/static** — schema, source identity, references and deterministic regressions.
2. **Provider adoption/delivery** — the Platform binding and consumer resolve the intended immutable META source, and the actual client receives applicable instructions.
3. **Runtime/model behavior** — observed task outcomes on the recorded client, model and effort.

A deterministic text check is not a model trial. `tools/validation/prompt_eval.py` must report `model_trials_executed: 0`. Missing runtime trials are `NOT_EVALUATED`, not PASS.

For a material prompt or instruction change, compare the same representative Platform paths before and after when evidence is available: a local code fix, an `docs/agents/**` governance change and a domain/security boundary task. Record mandatory sources and bytes, repeated reads, false blockers, handoffs and owner interventions when observable. Structural byte reduction does not prove token, cost or behavior savings.

Safety-critical regression tolerance is zero. Preserve the root WWW boundary, candidate self-authorization denial, Platform product/security/data/payment invariants and required repository gate. Keep a rollback that restores the binding, provider instructions and consumer together.
