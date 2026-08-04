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

Publish the canonical, implementation-ready cross-repository contract for native Oteryn gameplay protocol selection and bind it to the existing Identity, Game Login Ticket, Game Gateway, World Registry and Game Session chain without changing runtime behavior.

## Acceptance criteria

- [ ] One canonical contract resolves negotiation, ownership, session binding, framing, serialization, versioning, capabilities, action lifecycle, state synchronization, downgrade resistance and rollout/rollback.
- [ ] A review-only protobuf IDL defines exact public negotiation and native gameplay envelopes without runtime wiring.
- [ ] Producer/consumer responsibilities and current-versus-target status are explicit for Platform, Gateway, Otheryn and the Rust client.
- [ ] Four separate implementation prompts are ready for later bounded tasks.
- [ ] Linked correspondence records exist in `blakinio/Otheryn` and `blakinio/otclient` under coordination ID `OTS-20260804-native-protocol-selection`.
- [ ] Documentation validation, independent consistency/security review and exact-head required CI pass.

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
  - none
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
updated_at: 2026-08-04T14:50:00Z
head: UNKNOWN
branch: docs/OTS-20260804-native-protocol-contract
pr: none
status: implementing
context_routes:
  - coordination:OTS-20260804-native-protocol-selection
  - repo:blakinio/Oteryn-Platform
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
  - Gateway API protocol_version 1 is distinct from gameplay profile/version.
  - Current Gateway redeems one-time ticket before issuing a Game Session and currently returns one world with a session credential.
  - Current task is contract and architecture only; runtime, dependencies, migrations and deployment are forbidden.
derived:
  - Platform is the appropriate canonical contract repository because it owns World Registry policy and Game Gateway selection/session issuance boundaries.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-contract.md
validation:
  - command: repository documentation/governance validation
    result: NOT_RUN
    evidence: contract files not yet complete
blockers: []
next_action: create the canonical contract, review-only IDL, ADR, security/rollout documents and implementation prompts
```

## Notes

This task cannot authorize or claim runtime behavior in another repository. All target behavior remains disabled and unimplemented until later linked producer/consumer packages pass exact integrated staging evidence.
