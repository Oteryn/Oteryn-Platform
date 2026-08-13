---
task_id: OTERYN-20260813-native-game-enforcement-contract
mode: architecture
issue: 1029
status: implementing
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
---

# OTERYN-20260813-native-game-enforcement-contract

## Goal

Define the Platform-side semantic boundary for support/moderation requests that require authoritative native Oteryn-v2 sanctions or runtime enforcement, without runtime implementation, deployment, production activation or external-repository access.

## Acceptance criteria

- [ ] Authority and non-authority boundaries are explicit.
- [ ] Stable operation identity, lifecycle, ordering, idempotency and ambiguous-result reconciliation are defined.
- [ ] Typed target, scope, result, revoke/expire/appeal and privacy/audit semantics are defined.
- [ ] Legacy Canary Compatibility, rollout and rollback remain explicit and fail closed.
- [ ] Focused architecture, module/data/security ownership and programme state are reconciled.
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
updated_at: 2026-08-13T20:36:00+02:00
head: 638df04f616c93d80e33e1abf3f2cf0198163e7a
branch: docs/OTERYN-20260813-native-game-enforcement-contract
pr: none
status: implementing
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
derived:
  - The missing boundary can be resolved as a Platform consumer/orchestration contract subordinate to ADR 0031 without selecting external transport or implementation.
unknown:
  - Exact Oteryn-v2 transport, IDL, sanction storage, runtime enforcement mechanism and production rollout remain external and deferred.
conflicts: []
first_failure:
  marker: native-support-game-enforcement-semantic-contract-missing
  evidence: OTERYN_V2_INTEGRATION_ARCHITECTURE.md deferred P1 item 1 and programme next_action.
rejected_hypotheses:
  - Platform enforcement_records can serve as authoritative native game sanctions; DATA_OWNERSHIP.md explicitly limits them to communication and workflow truth.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260813-native-game-enforcement-contract.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: contract package is being implemented.
blockers: []
next_action: Write the bounded native game-enforcement command/result contract and reconcile its canonical architecture owners.
```

