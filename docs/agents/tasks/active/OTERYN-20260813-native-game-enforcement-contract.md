---
task_id: OTERYN-20260813-native-game-enforcement-contract
mode: architecture
issue: 1029
status: validating
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
---

# OTERYN-20260813-native-game-enforcement-contract

## Goal

Define the Platform-side semantic boundary for support/moderation requests that require authoritative native Oteryn-v2 sanctions or runtime enforcement, without runtime implementation, deployment, production activation or external-repository access.

## Acceptance criteria

- [x] Authority and non-authority boundaries are explicit.
- [x] Stable operation identity, lifecycle, ordering, idempotency and ambiguous-result reconciliation are defined.
- [x] Typed target, scope, result, revoke/expire/appeal and privacy/audit semantics are defined.
- [x] Legacy Canary Compatibility, rollout and rollback remain explicit and fail closed.
- [x] Focused architecture, module/data/security ownership and programme state are reconciled.
- [ ] Offline architecture validation and exact-head full-diff self-review pass.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-native-game-enforcement-contract.md
modules:
  - Support
  - Integration
dependencies:
  - ADR 0031
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T20:55:00+02:00
head: 8d892e74a55bf1d5b9975e6d6bc1625851fc9b00
branch: docs/OTERYN-20260813-native-game-enforcement-contract
pr: 1030
status: validating
context_routes:
  - architecture
  - support-moderation
  - security
  - native-integration
owned_paths:
  - docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-native-game-enforcement-contract.md
proven:
  - Current main is 638df04f616c93d80e33e1abf3f2cf0198163e7a and the architecture programme is ready with support/moderation game enforcement as its first P1 unresolved question.
  - Existing Platform enforcement_records are workflow and communication records only and do not mutate or prove game sanctions.
  - Issue 1029 owns this bounded architecture-only package and no open PR or active task overlaps its declared paths.
  - Draft PR 1030 contains exactly eight declared documentation paths; semantic contract package head 8d623f850da88bfc814618bba84673738dc3c57e includes the validation checkpoint.
derived:
  - The missing boundary can be resolved as a Platform consumer/orchestration contract subordinate to ADR 0031 without selecting external transport or implementation.
unknown:
  - Exact Oteryn-v2 transport, IDL, sanction storage, runtime enforcement mechanism and production rollout remain external and deferred.
conflicts: []
first_failure:
  marker: sanction-stream-identity-missing
  evidence: PR 1030 review thread PRRT_kwDOTcsYjs6ZC-FJ proved operation_id cannot identify a sanction lineage across distinct replace/revoke/expire decisions.
rejected_hypotheses:
  - Platform enforcement_records can serve as authoritative native game sanctions; DATA_OWNERSHIP.md explicitly limits them to communication and workflow truth.
  - A dispatched command or accepted appeal can be presented as completed game enforcement; only an authoritative applicable game result proves effect.
  - Timestamps can safely order apply/revoke/expire decisions; a stable sanction stream and monotonic decision revision are required.
  - Target plus profile/scope can always derive the sanction stream; replace can change profile/scope and one target can carry multiple independent restrictions.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-native-game-enforcement-contract.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md
validation:
  - command: python tools/agents/checkpoint.py task --require-checkpoint
    result: PASS
    evidence: task conforms to checkpoint contract v1.
  - command: ADR registry and architecture decision backlog validators/tests
    result: PASS
    evidence: 41 ADRs validated; 10 backlog tests and empty canonical backlog validation passed.
  - command: python3 tools/agents/control_room.py --format markdown and git diff --check
    result: PASS
    evidence: programme/task reconciliation rendered and diff whitespace validation passed.
  - command: real runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only semantic contract changes no executable user or integration journey.
  - command: PR 1030 exact-head CI generation for 8d892e74a55bf1d5b9975e6d6bc1625851fc9b00
    result: PASS
    evidence: all eight returned workflows completed successfully; subsequent review finding requires a new repaired exact-head generation.
blockers: []
next_action: Validate the sanction_stream_id repair on a new exact head, resolve the review thread, rerun full-diff self-review and required CI, then merge and archive when every gate passes.
```
