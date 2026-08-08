---
task_id: OTERYN-20260808-native-runtime-status-boundary
repository: blakinio/Oteryn-Platform
issue: 880
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
search_first:
  - open Issues and PRs for runtime status, World Registry readiness, LiveOps world status and readiness projections
optional_reads:
  - read-only Oteryn-v2 ADR-0009 GameNode execution/capacity/deployment/recovery baseline
---

# OTERYN-20260808-native-runtime-status-boundary

## Goal

Define the Platform-side native World/Channel runtime-status projection semantics required by World Registry, Game Gateway and LiveOps, using existing accepted ownership/identity decisions while leaving unfinished Oteryn-v2 producer transport and orchestration details explicitly external and read-only.

## Acceptance criteria

- [x] A focused native runtime-status projection contract separates configured Platform policy/lifecycle from observed Oteryn-v2 runtime facts.
- [x] Canonical WorldId/ChannelId identity, producer authority, observation/revision/freshness, stale/unavailable behavior and admission fail-closed rules are explicit.
- [x] Public LiveOps status cannot fabricate `offline`, zero or maintenance from stale/unavailable evidence.
- [x] Gateway readiness cannot be inferred solely from configured `status=online` or `login_enabled=true`.
- [x] Existing World Registry and focused v2 architecture documents route to the new semantic boundary without claiming runtime implementation.
- [x] Oteryn-v2 remains read-only and exact deferred OPS-CHANNEL-01/FND transport bytes are not invented.
- [ ] Exact-head Agent Governance and repository-selected CI pass; full diff review has zero unresolved material findings.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task is architecture/documentation only.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260808-native-runtime-status-boundary.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260808-native-runtime-status-boundary.md
modules:
  - Integration
  - LiveOps
  - architecture-governance
dependencies:
  - Issue #880
  - ADR 0029
  - ADR 0031
  - Oteryn-v2 ADR-0009 read-only evidence
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T06:35:00Z
head: b6fc4f7a49b9acb2887d631010ced46d5cba2649
branch: docs/OTERYN-20260808-native-runtime-status-boundary
pr: 881
status: validating
context_routes:
  - architecture
  - api
  - operations
owned_paths:
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260808-native-runtime-status-boundary.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260808-native-runtime-status-boundary.md
proven:
  - ADR 0029 assigns Platform World Registry canonical WorldId/ChannelId topology identity and control-plane policy while separating GameNode/runtime ownership and readiness.
  - ADR 0031 assigns gameplay/runtime source facts to Oteryn-v2 and Platform projection/control-plane consumption to explicit contracts.
  - WORLD_REGISTRY_CONTRACT.md current compatibility status/login policy did not define native runtime readiness.
  - MODULE_CATALOG.md assigns authoritative time-sensitive world/service status plus freshness to planned LiveOps and forbids fabricated offline/zero state.
  - Read-only Oteryn-v2 ADR-0009 defines separate GameNode health/readiness/capacity, Channel lifecycle and fail-closed routing after unhealthy/suspected runtime state while deferring exact OPS-CHANNEL-01 producer details.
  - Candidate PR 881 adds an accepted Platform consumer semantic boundary and reconciles World Registry plus the focused v2 architecture without implementing or inventing the external producer.
  - Candidate self-review removed a stale implication that the historical Platform Game Session v2/native producer remained current native authority; it is now explicitly historical/disabled reconciliation evidence.
derived:
  - The P1 Platform-side runtime-status architecture gap is resolved by existing authority; exact Oteryn-v2 producer transport/health/cadence details remain external implementation contracts.
  - Native new-admission readiness is an intersection of Platform configured policy and fresh, applicable, current-owner Oteryn-v2 runtime evidence.
unknown:
  - exact Oteryn-v2 OPS-CHANNEL-01 message schema, transport, reporting cadence, TTL values, health algorithm, ownership-generation encoding and implementation revision.
conflicts: []
first_failure:
  marker: native-runtime-status-projection-undefined
  evidence: Resolved in PR 881 candidate by defining the Platform consumer semantic contract and removing the item from the deferred P1 list.
rejected_hypotheses:
  - Treat persisted status=online plus login_enabled=true as sufficient native runtime readiness.
  - Treat stale or unavailable observations as authoritative offline state.
  - Copy Oteryn-v2 deferred OPS-CHANNEL-01 transport details into Platform by assumption.
  - Make a Platform cache/read model the game-runtime source of truth.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-runtime-status-boundary.md
  - docs/agents/reports/OTERYN-20260808-native-runtime-status-boundary.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no open bounded Issue or PR owned this exact runtime-status projection architecture gap; Issue 880 and PR 881 now own the bounded scope.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation only; no executable behavior, schema, workflow, deployment or environment changed.
  - command: exact-head GitHub Actions and full changed-file review
    result: NOT_RUN
    evidence: final candidate checkpoint commit is the last intended mutation before exact-head workflow and diff evaluation.
blockers:
  - none
next_action: Verify exact PR 881 head workflows, inspect the full five-file diff and all review threads/comments, then mark ready and squash-merge only if every required gate passes.
```

## Notes

No runtime code, schema, workflow, deployment, production activation or Oteryn-v2 write is authorized by this task.
