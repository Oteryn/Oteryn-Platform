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
updated_at: 2026-08-13T20:45:00+02:00
head: 7a7c88de1e11f6d99b4a620dfa2bc974c084bb46
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
  - Draft PR 1030 contains exactly eight declared documentation paths on head 7a7c88de1e11f6d99b4a620dfa2bc974c084bb46.
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
blockers: []
next_action: Run exact-head CI and full-diff self-review for PR 1030, repair findings, then merge and archive the task when every gate passes.
```
