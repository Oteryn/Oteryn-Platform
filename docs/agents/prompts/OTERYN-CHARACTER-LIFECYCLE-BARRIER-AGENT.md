# Oteryn Character Lifecycle Barrier Agent

```yaml
prompt_contract:
  version: 2.0
  objective: Convert the shared #317/#319/#320 Character Authority blocker into the smallest truthful Platform-side executable handoff without inventing game semantics.
  baseline_version: 1.0
  eval_suite: docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  rollback_version: 1.0
  required_invariants:
    - separate_feature_ownership
    - unknown_producer_semantics_block_runtime
    - external_repository_requires_exact_authority
owner_alias: OTERYN-CHARACTER-LIFECYCLE-BARRIER
```

## Outcome

Resolve the shared native Character Authority barrier for Issues #317, #319 and #320 before runtime implementation. Produce one evidence-backed Platform result stating what is accepted, what Platform can implement, and which game-owned command/result semantics or product decisions remain missing for deletion/restore, rename and world/channel transfer.

Shared prerequisite work may be one task. Each later user-facing mutation remains a separate Issue/task/branch owner; do not combine all three features into one PR.

## Scope and domain constraints

Writes are limited to `Oteryn/Oteryn-Platform`. Do not access another server/game repository without separate exact authority. No production or live character mutation, direct game-table SQL, credentials, protected deployment or payment activation is authorized.

Read ADRs 0029/0030/0031, `NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md`, current Issues #317/#319/#320, relevant Bazaar conflict policy and only the contracts/source required by the evidence.

Do not invent command names, wire formats, locking, canonical game state, ownership results, transfer semantics or server behavior. Legacy Canary evidence does not establish native authority. Preserve `UNKNOWN` while producer semantics are unavailable.

## Acceptance delta

- The three Issues are classified independently from current evidence.
- Platform/game responsibilities and canonical identifiers are explicit.
- Each operation lists accepted and missing game-owned command/result semantics.
- Mutual exclusion with Bazaar and sibling character mutations is explicit.
- #320 remains blocked until accepted authority supports player-selectable native world/channel transfer.
- No runtime implementation starts while a material game-owned contract remains unknown.
- The result is the smallest architecture, contract or task handoff with no duplicate owner.

Build a three-operation dependency matrix covering authority, responsibilities, missing evidence, conflict policy and rollout gate. Search existing Platform architecture/contracts before proposing a decision. Route a genuinely missing durable decision through `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`. Implement a Platform-only prerequisite only when it is fully specified and independently mergeable.

## Stop delta

Stop for separately authorized server access, missing game semantics, a product-owner transfer decision or unresolved ownership. Documentation/contract-only runtime E2E is `NOT_APPLICABLE` because it delivers no executable character mutation. Record the exact blocker and one next action.
