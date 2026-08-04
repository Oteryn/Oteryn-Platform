---
task_id: OTERYN-20260804-native-protocol-contract
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/adr/0009-oteryn-game-authentication-architecture.md
  - docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - OTS-20260804-native-protocol-selection
optional_reads:
  - services/game-gateway/internal/gateway/types.go
  - services/game-gateway/internal/gateway/service.go
  - services/game-gateway/internal/httpapi/server.go
---

# OTERYN-20260804-native-protocol-contract

## Goal

Publish the canonical implementation-ready cross-repository contract for native Oteryn gameplay protocol selection while preserving the existing Identity, Game Login Ticket, Game Gateway, World Registry and Game Session chain and changing no runtime behavior.

## Acceptance criteria

- [x] Canonical contract resolves negotiation, ownership, exact session binding, framing, serialization, versioning, capabilities, action lifecycle, synchronization, downgrade resistance and rollout/rollback.
- [x] Review-only protobuf IDL defines native bootstrap, commands, results, snapshots and deltas without runtime wiring.
- [x] Producer/consumer responsibilities and current-versus-target truth are explicit.
- [x] Four separate implementation prompts are ready for later bounded packages.
- [x] Linked correspondence PRs exist in `blakinio/Otheryn` and `blakinio/otclient`.
- [x] Independent consistency/security review has zero remaining material findings.
- [x] Required Platform workflows passed on validated content head `50d6c79c206391b211249ffb3d30b836303d7c65`.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-contract.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_RUST_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
modules:
  - game-auth
  - game-gateway
  - cross-repository contracts
dependencies:
  - OTS-20260804-native-protocol-selection
  - blakinio/otclient#263
blockers:
  - exact-head CI for the final checkpoint commit
cross_repository_tasks:
  - OTH-20260804-native-protocol-contract
  - OTC2-20260804-native-protocol-contract
```

## Delivery classification

```yaml
feature_scope:
  type: documentation
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: false
implementation_status: contract_only
user_facing_feature_complete: false
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-04T15:30:00Z
head: 50d6c79c206391b211249ffb3d30b836303d7c65
branch: docs/OTS-20260804-native-protocol-contract
pr: blakinio/Oteryn-Platform#519
status: validating
context_routes:
  - coordination:OTS-20260804-native-protocol-selection
  - producer:blakinio/Oteryn-Platform#519
  - producer-correspondence:blakinio/Otheryn#356
  - consumer-correspondence:blakinio/otclient#265
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-contract.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_RUST_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
proven:
  - Gateway API protocol_version 1 remains distinct from gameplay and Game Session versions.
  - Current Gateway redeems one ticket and currently routes exactly one world/session.
  - Gateway is the sole target selector from a bounded client-supported set and World Registry order.
  - Game Session v2 is an opaque hashed-at-rest reference with exact server-side account, generation, world, channel, endpoint, profile, schema and capability bindings.
  - Native v1 is separate TLS 1.3 plus BE32 plus protobuf and never falls back or translates through Canary.
  - Runtime, dependencies, migrations, deployment and production activation remain unchanged.
derived:
  - Platform is canonical because it owns World Registry policy and Gateway selection/session issuance boundaries.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - post-redeem client selection
  - game-listener protocol sniffing
  - native-over-Canary packet translation
changed_paths:
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_RUST_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-contract.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
validation:
  - command: Agent Governance run 30924131410
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: CI run 30924129969
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: Edge Security Emulation run 30924129879
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: Game Auth Ticket Concurrency run 30924129947
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: Phase 7 Production-Like Validation run 30924129919
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: Platform DB Outage Validation run 30924130377
    result: PASS
    evidence: exact content head 50d6c79c206391b211249ffb3d30b836303d7c65
  - command: independent contract/security review
    result: PASS
    evidence: exact claims, request/error semantics, deterministic digests, duplicate policy and normal/emergency rollback were resolved with zero remaining material findings
blockers:
  - exact-head required workflows for the checkpoint commit
next_action: verify exact-head workflows, mark PR ready and merge canonical contract first
```

## Notes

This task cannot authorize runtime behavior in another repository. All target behavior remains disabled and unimplemented until later linked producer/consumer packages pass exact integrated staging evidence.
