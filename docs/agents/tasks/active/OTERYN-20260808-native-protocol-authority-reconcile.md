---
task_id: OTERYN-20260808-native-protocol-authority-reconcile
repository: blakinio/Oteryn-Platform
issue: 874
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md
search_first:
  - open Issues and PRs for native admission, Game Session, lease, reconnect and OTERY_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT
optional_reads:
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
---

# OTERYN-20260808-native-protocol-authority-reconcile

## Goal

Remove the stale pre-cutover Platform/Otheryn native-protocol authority claim from current documentation, preserve the disabled producer/schema as historical transitional evidence, and route future native admission/session/protocol work to accepted ADR 0031 and Oteryn-v2 authority without inventing unfinished cross-repository semantics.

## Acceptance criteria

- [ ] The historical Platform native protocol contract no longer identifies itself as current normative authority for Oteryn-v2 gameplay protocol or admitted-session semantics.
- [ ] The historical schema and PR #542 producer evidence remain discoverable and explicitly classified as transitional/reconciliation-only.
- [ ] The producer operations document no longer directs target rollout toward Otheryn and cannot be read as Oteryn-v2 activation guidance.
- [ ] The architecture review programme records this domain and its terminal disposition.
- [ ] Exact-head Agent Governance and repository-selected CI pass.
- [ ] Full changed-file review has zero unresolved material findings.
- [ ] Runtime E2E is recorded `NOT_APPLICABLE` because the task changes architecture/governance documentation only.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260808-native-protocol-authority-reconcile.md
  - docs/agents/tasks/active/OTERYN-20260808-native-protocol-authority-reconcile.md
modules:
  - Integration
  - architecture-governance
dependencies:
  - Issue #874
  - ADR 0031
  - current Oteryn-v2 protocol/admission authority remains read-only
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
task_kind: architecture
implementation_authorized: false
phase: investigate
execution_mode: github-only
updated_at: 2026-08-08T06:13:00Z
head: 0582b0e853d1b5e983f664452268e7777c886904
branch: docs/OTERYN-20260808-native-protocol-authority-reconcile
pr: none
status: investigating
context_routes:
  - architecture
  - auth-identity
  - api
owned_paths:
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260808-native-protocol-authority-reconcile.md
  - docs/agents/tasks/active/OTERYN-20260808-native-protocol-authority-reconcile.md
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one documentation-only authority reconciliation with one accepted ADR and no runtime implementation
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
proven:
  - ADR 0031 is accepted and assigns final native gameplay admission, admitted-session lease/fencing and protocol-oteryn semantics to the Oteryn-v2 game/native authority.
  - OTERY_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md still labels itself NORMATIVE and claims canonical authority across Platform, Otheryn and the Rust client.
  - OTERY_NATIVE_PROTOCOL_PRODUCER.md still describes Otheryn Game Session v2/native consumers as the target remaining package.
  - The native-auth production verification task already classifies Platform PR #542 and historical Otheryn/OTClient correspondence as compatibility/reconciliation evidence rather than final Oteryn-v2 conformance.
derived:
  - Current documentation contains a lower-level authority drift that can misroute future native admission/protocol work despite ADR 0031.
unknown:
  - exact future Oteryn-v2 FND admission/session/lease contract bytes and transport details; this task must not invent them.
conflicts:
  - The historical Platform protocol contract's self-declared current normative authority conflicts with accepted ADR 0031 target ownership.
first_failure:
  marker: stale-platform-native-protocol-normative-authority
  evidence: docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md starts with Status NORMATIVE and states that it governs Platform, Game Gateway, Otheryn and the Rust client.
rejected_hypotheses:
  - Treat PR #542 producer semantics as final Oteryn-v2 protocol/admission authority.
  - Rewrite Oteryn-v2 runtime contracts from Platform without accepted external authority.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-protocol-authority-reconcile.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no open Oteryn-Platform Issue or PR owns this exact authority-drift repair; Issue #874 created for this bounded scope.
blockers:
  - none
next_action: Reclassify the stale native protocol contract and producer operations guide as historical transitional evidence, then update the architecture programme and validate the exact branch head.
```

## Notes

No runtime code, schema, producer enablement, external-repository write, deployment or production activation is authorized by this task.