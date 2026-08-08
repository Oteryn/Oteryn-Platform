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
search_first:
  - open Issues and PRs for runtime status, World Registry readiness, LiveOps world status and readiness projections
optional_reads:
  - read-only Oteryn-v2 ADR-0009 GameNode execution/capacity/deployment/recovery baseline
---

# OTERYN-20260808-native-runtime-status-boundary

## Goal

Define the Platform-side native World/Channel runtime-status projection semantics required by World Registry, Game Gateway and LiveOps, using existing accepted ownership/identity decisions while leaving unfinished Oteryn-v2 producer transport and orchestration details explicitly external and read-only.

## Acceptance criteria

- [ ] A focused native runtime-status projection contract separates configured Platform policy/lifecycle from observed Oteryn-v2 runtime facts.
- [ ] Canonical WorldId/ChannelId identity, producer authority, observation/revision/freshness, stale/unavailable behavior and admission fail-closed rules are explicit.
- [ ] Public LiveOps status cannot fabricate `offline`, zero or maintenance from stale/unavailable evidence.
- [ ] Gateway readiness cannot be inferred solely from configured `status=online` or `login_enabled=true`.
- [ ] Existing World Registry and focused v2 architecture documents route to the new semantic boundary without claiming runtime implementation.
- [ ] Oteryn-v2 remains read-only and exact deferred OPS-CHANNEL-01/FND transport bytes are not invented.
- [ ] Exact-head Agent Governance and repository-selected CI pass; full diff review has zero unresolved material findings.
- [ ] Runtime/browser E2E is `NOT_APPLICABLE` because this task is architecture/documentation only.

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
updated_at: 2026-08-08T06:27:00Z
head: 9b84279dbd8a35a6f75ccd524daaf4a29e89b27a
branch: docs/OTERYN-20260808-native-runtime-status-boundary
pr: none
status: investigating
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
  - WORLD_REGISTRY_CONTRACT.md keeps current Canary persisted status/login policy and explicitly leaves runtime health versus status/readiness unresolved.
  - MODULE_CATALOG.md assigns authoritative time-sensitive world/service status plus freshness to planned LiveOps and forbids fabricated offline/zero state.
  - Oteryn-v2 ADR-0009 is accepted read-only evidence defining separate GameNode health/readiness/capacity, Channel lifecycle and fail-closed routing after unhealthy/suspected runtime state while deferring exact OPS-CHANNEL-01 producer details.
derived:
  - A Platform consumer/projection semantic contract can be resolved from existing authority without choosing Oteryn-v2 transport bytes or implementation topology.
unknown:
  - exact Oteryn-v2 OPS-CHANNEL-01 message schema, transport, reporting cadence, TTL values, health algorithm and implementation revision.
conflicts:
  - Current compatibility World Registry status may be read as runtime truth even though current docs explicitly say its future relationship to runtime health/readiness is unresolved.
first_failure:
  marker: native-runtime-status-projection-undefined
  evidence: focused v2 architecture P1 backlog explicitly lists World/channel runtime-status to Platform World Registry/LiveOps contract as deferred.
rejected_hypotheses:
  - Treat persisted status=online plus login_enabled=true as sufficient native runtime readiness.
  - Treat stale or unavailable observations as authoritative offline state.
  - Copy Oteryn-v2 deferred OPS-CHANNEL-01 transport details into Platform by assumption.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-runtime-status-boundary.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no open bounded Issue or PR owns this exact runtime-status projection architecture gap; Issue #880 created for this scope.
blockers:
  - none
next_action: Draft the Platform-side native runtime-status projection semantic contract, reconcile World Registry and focused v2 architecture routing, then validate the exact branch head.
```

## Notes

No runtime code, schema, workflow, deployment, production activation or Oteryn-v2 write is authorized by this task.