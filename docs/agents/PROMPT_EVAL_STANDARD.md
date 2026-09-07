# Platform Prompt Evaluation Standard

Platform prompt and harness evaluation must resolve the organization evaluation standard selected by `META_AGENT_POLICY_BINDING.json`. This file supplies the local consumers and evidence locations.

## Local evidence layers

1. Run the central META provider/prompt boundary through `tools/agents/policy_consistency.py` using the exact bound authority.
2. Run `python tools/agents/documentation_ia.py` and its tests for prompt classification and instruction delivery paths.
3. Run `python tools/validation/prompt_eval.py` and its tests for the repository's deterministic prompt-contract suite.
4. For a materially changed reusable prompt, compare the same representative baseline/candidate cases and record actual client/model/effort/tool configuration when observable.

Deterministic checks prove their declared schema, routing and text boundaries. They are not model trials. Record unavailable model or delivery evidence as `NOT_EVALUATED`; do not convert it to PASS.

## Platform case selection

Choose cases from the affected Platform surface. Include authority refusal, stale live locators, prompt injection, incomplete product slices, invalid or unauthorized behavior, exact-candidate validation and recovery when those risks apply. Keep runtime/product acceptance separate from narration quality.

Store compact evaluation records under `docs/agents/evidence/` or the owning task evidence. Retain full logs outside always-loaded instructions, redact sensitive data, preserve the baseline and candidate revisions, and make rollback coherent across binding, overlays and consumers.
