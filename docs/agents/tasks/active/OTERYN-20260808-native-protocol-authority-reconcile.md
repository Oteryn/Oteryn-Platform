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

- [x] The historical Platform native protocol contract no longer identifies itself as current normative authority for Oteryn-v2 gameplay protocol or admitted-session semantics.
- [x] The historical schema and PR #542 producer evidence remain discoverable and explicitly classified as transitional/reconciliation-only.
- [x] The producer operations document no longer directs target rollout toward Otheryn and cannot be read as Oteryn-v2 activation guidance.
- [ ] The architecture review programme records this domain and its terminal disposition after the delivery PR merges.
- [ ] Exact-head Agent Governance and repository-selected CI pass.
- [ ] Full changed-file review has zero unresolved material findings.
- [x] Runtime E2E is `NOT_APPLICABLE` because the task changes architecture/governance documentation only.

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
updated_at: 2026-08-08T06:20:00Z
head: b39a3acde290ea99f9437df3fc443e514c200934
branch: docs/OTERYN-20260808-native-protocol-authority-reconcile
pr: 875
status: validating
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
proven:
  - ADR 0031 is accepted and assigns final native gameplay admission, admitted-session lease/fencing and protocol-oteryn semantics to the Oteryn-v2 game/native authority.
  - The pre-repair Platform contract declared itself NORMATIVE and canonical across Platform, Game Gateway, Otheryn and the Rust client.
  - Candidate PR 875 reclassifies that contract as historical transitional reconciliation input and preserves the historical schema digest plus validator markers.
  - Candidate PR 875 reclassifies the producer operations guide and removes Otheryn rollout as current target guidance.
  - The native-auth production verification task already classifies Platform PR 542 and historical Otheryn/OTClient correspondence as compatibility/reconciliation evidence rather than final Oteryn-v2 conformance.
derived:
  - The accepted ADR already resolves the durable ownership question; this task requires no new owner decision or ADR.
unknown:
  - Exact future Oteryn-v2 FND admission/session/lease contract bytes and transport details remain outside Platform authority and must not be invented here.
conflicts: []
first_failure:
  marker: stale-platform-native-protocol-normative-authority
  evidence: Resolved in PR 875 candidate by removing the current normative claim and routing exact future semantics to Oteryn-v2 authority.
rejected_hypotheses:
  - Treat PR 542 producer semantics as final Oteryn-v2 protocol/admission authority.
  - Rewrite Oteryn-v2 runtime contracts from Platform without accepted external authority.
  - Delete historical schema/fixture evidence instead of preserving it for reconciliation.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-protocol-authority-reconcile.md
  - docs/agents/reports/OTERYN-20260808-native-protocol-authority-reconcile.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no open Oteryn-Platform Issue or PR owned this exact authority-drift repair; Issue 874 and PR 875 now own the bounded scope.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance documentation only; no executable behavior, deployment or environment changed.
  - command: exact-head GitHub Actions and native-protocol artifact validators
    result: NOT_RUN
    evidence: final candidate checkpoint commit must be pushed before exact-head workflow evidence is evaluated.
blockers:
  - none
next_action: Verify the exact PR 875 head checks, inspect the full diff and review threads, then mark the PR ready and squash-merge only if every required gate passes.
```

## Notes

No runtime code, schema, producer enablement, external-repository write, deployment or production activation is authorized by this task.