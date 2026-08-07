---
task_id: OTERYN-20260808-oteryn-v2-integration-baseline
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
search_first:
  - Oteryn-v2
  - native protocol
  - Canary compatibility
  - Game Session
optional_reads:
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
---

# OTERYN-20260808-oteryn-v2-integration-baseline

## Goal

Persist the repository-owner-approved Platform architecture reconciliation that separates the delivered Canary compatibility model from the target native Oteryn-v2 integration model, without changing runtime code, persistence, external repositories, protocol implementation, deployment or production state.

## Acceptance criteria

- [x] Current Canary-compatible implementation is preserved as current compatibility evidence rather than silently deleted.
- [x] Target native Platform ↔ Oteryn-v2 ownership and dependency direction is explicitly defined.
- [x] Accepted ADR 0031 records the native integration / legacy compatibility boundary and supersedes stale native protocol ownership decisions where necessary.
- [x] A focused canonical `OTERYN_V2_INTEGRATION_ARCHITECTURE.md` records current-vs-target topology, ownership, persistence, admission, projections, analytics, compatibility, migration and risks.
- [x] `ARCHITECTURE_AUTHORITY.md` routes this concern to the focused canonical document.
- [x] ADR registry inventory is reconciled.
- [x] The architecture review report preserves open-PR classifications and deferred P1/P2 questions without turning them into implementation authority.
- [ ] Exact-head Agent Governance/CI and architecture validation pass.
- [ ] Full exact-head diff review has zero material findings and zero unresolved review threads.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-oteryn-v2-integration-baseline.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/README.md
modules:
  - Identity
  - Accounts
  - Characters
  - Integration
  - PublicGameData
  - GameGateway
  - WorldRegistry
  - PlayerCompanion
  - Marketplace
  - Support
  - PlatformAPI
dependencies:
  - Issue #863
  - Accepted ADR 0030
  - Oteryn-v2 accepted Character Authority / cross-repository contract evidence, read-only
blockers:
  - none
cross_repository_tasks:
  - none; Oteryn-v2 is evidence-only and receives no writes
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T01:05:30+02:00
head: UNKNOWN
branch: docs/OTERYN-20260808-oteryn-v2-integration-baseline
pr: 866
status: validating
context_routes:
  - architecture
  - canary-integration
  - accounts-characters
  - public-game-data
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-oteryn-v2-integration-baseline.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/README.md
proven:
  - Current Platform documents and runtime adapters retain Canary-compatible SQL, numeric identifiers, Game Session v1 and disabled transitional native-protocol producer assumptions.
  - Accepted ADR 0030 already separates native CharacterId ownership and character mutation authority from Platform presentation/orchestration.
  - Oteryn-v2 accepted cross-repository architecture makes Platform the AccountId/Identity/control-plane owner and the game domain the native CharacterId/gameplay/protocol authority.
  - Issue 863 and draft PR 866 exist for the Platform-only reconciliation package.
  - ADR 0031, the focused Oteryn-v2 integration architecture, authority routing, supersession notes, registry entry and dated review report are present on the task branch.
derived:
  - The Platform needs one explicit native-integration anti-corruption boundary so new consumers do not inherit Canary schema/protocol/session assumptions.
  - A focused canonical document plus a superseding ADR is narrower and safer than rewriting all current Canary compatibility documents as if they no longer existed.
  - Current OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT remains transitional implementation evidence subordinate to ADR 0031 for target ownership; replacing its concrete runtime producer is a later implementation task.
unknown:
  - Exact native transport/API schemas for several future Platform-v2 command/query/event contracts remain deliberately deferred.
  - Exact Canary retirement dates and per-adapter cutover criteria remain future implementation/migration decisions.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Rebuilding the Platform as microservices is not justified by the discovered integration drift.
  - Deleting Canary compatibility now would conflate architecture target state with unproven migration/cutover state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-oteryn-v2-integration-baseline.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/README.md
validation:
  - command: focused source reconciliation
    result: PASS
    evidence: current Platform architecture and read-only Oteryn-v2 authority were compared before mutation.
  - command: full PR diff self-review
    result: NOT_RUN
    evidence: run after this coherent checkpoint commit.
  - command: exact-head repository validation
    result: NOT_RUN
    evidence: required GitHub checks must run on the resulting coherent head.
  - command: user/integration E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task changes no executable journey.
blockers:
  - none
next_action: Review PR 866 full exact-head diff and validate the resulting head with repository-required Agent Governance/CI before merge.
```
