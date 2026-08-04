---
task_id: OTERYN-20260804-native-protocol-producer
project_lane: oteryn-platform-auth
status: validating
branch: feat/OTERYN-20260804-native-protocol-producer
base_branch: main
created: 2026-08-04
updated: 2026-08-04
related_pr: "523"
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
search_first:
  - OTS-20260804-native-protocol-selection
  - gameplay_offer
  - game session v2
optional_reads:
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
---

# OTERYN-20260804-native-protocol-producer

## Goal

Implement the disabled-by-default Platform and Game Gateway producer side for bounded gameplay offers, deterministic World Registry selection and opaque Game Session contract version 2 while preserving the existing Gateway API v1 and Canary-compatible path.

## Delivery classification

```yaml
feature_scope:
  type: contract_producer
  user_facing: false
  backend_required: true
  frontend_required: false
  integration_required: true
  e2e_required: true
  completion_claim: partial_producer
implementation_status: validating
user_facing_feature_complete: false
missing_consumers:
  - Otheryn Game Session v2/native admission and listener
  - Rust protocol-oteryn and automatic selection
```

## Acceptance criteria

- [x] Legacy `{protocol_version:1, game_login_ticket}` behavior and security properties remain unchanged.
- [x] Optional `gameplay_offer` is bounded and invalid syntax is rejected before ticket redeem.
- [x] World Registry owns ordered candidate policy, revision, endpoint identity and disabled-by-default rollout state.
- [x] Gateway ignores client order and selects the first exact authoritative intersection.
- [x] Canonical capability digest and exact native schema SHA-256 are enforced.
- [x] No match returns `unsupported_gameplay_pair` after redeem; no fallback or second issuance occurs.
- [x] Response versions and selected gameplay identity remain distinct.
- [x] Game Session v2 binds account generation, login attempt, world/channel, policy, endpoint, audience and exact tuple/digest.
- [x] Contradictory readiness fails closed before issuance.
- [x] Native is not seeded, enabled or advertised by default.
- [x] Synthetic fixtures, focused tests and producer-boundary E2E cover success, negative and boundary cases.
- [x] Operations, World Registry, Canary coexistence and Gateway documentation report producer-only completion and rollback.
- [ ] Exact-final-head CI, ready-state CI, merge and archival complete.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-producer.md
  - app/GameAuth/Context/**
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/*native_gameplay_protocol_policy*
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
modules:
  - auth-identity
  - api
  - database
  - canary-integration
  - security
  - testing
dependencies:
  - canonical contract merge 9035ae987db67c062a8778721a2c8e686ce76750
  - canonical schema blob adc5e906fa612fdfa00e050687bdfd5697f695cf
  - Otheryn correspondence merge 1807b6210375f6a18afabc817a01ccdfee80ddce
  - Rust correspondence merge bda9e749e5fefaa89180ede08e355028a4263fc0
blockers:
  - none
cross_repository_tasks:
  - Otheryn implementation remains separate
  - Rust implementation remains separate
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-04T21:20:00Z
invocation_started_at: 2026-08-04T20:47:00Z
last_progress_at: 2026-08-04T21:20:00Z
head: PENDING_THIS_CHECKPOINT_COMMIT
branch: feat/OTERYN-20260804-native-protocol-producer
pr: 523
status: validating
phase: validate
session_id: agent-20260804-native-protocol-producer-02
session_role: implementer
execution_mode: github
execution_reason: GitHub connector mutation with repository Actions for focused, integration and exact-head validation
project_lane: oteryn-platform-auth
task_kind: implementation
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive security-sensitive producer package across Platform policy, Gateway selection and issuer compatibility
validation_level: full
heavy_validation_runs: 2
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 1
ci_checks_for_current_head: 0
ci_check_generation: draft-final
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 0
stall_warnings: 0
context_routes:
  - auth-identity
  - api
  - database
  - security
  - canary-integration
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-producer.md
  - app/GameAuth/Context/**
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/*native_gameplay_protocol_policy*
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
proven:
  - Gateway API version 1 and legacy Game Session contract version 1 remain operational when gameplay_offer is absent.
  - Extended request validation is limited to 16 KiB, rejects duplicate keys, unknown fields, invalid grammar, duplicate tuples and noncanonical capability lists before redeem.
  - World Registry candidate rows default disabled and no candidate is seeded.
  - Exact canonical native schema SHA-256 c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8 is pinned from exact IDL bytes and enforced by Platform and Gateway.
  - Selection follows authoritative policy order and performs one readiness check plus at most one v2 issue request.
  - V2 claims include authoritative account generation, world/channel, policy, endpoint, audience, selection digest, bind-on-first-admission and single-admission intent.
  - Login projection is bounded to one world and at most 100 characters.
  - Synthetic producer E2E exercises Platform redeem/context through Gateway selection, strict readiness projection, v2 issuance and public response without exposing secrets.
  - Rollout/rollback documentation truthfully preserves missing Otheryn and Rust consumers.
  - Temporary diagnostic/formatting/documentation workflows removed themselves and are absent from the final diff.
derived:
  - The producer can be merged safely while disabled because no native route is advertised and all consumer readiness remains a later gate.
unknown: []
conflicts: []
first_failure:
  marker: initial Game Gateway formatting and strict-readiness fixture failures
  evidence: repaired multiline Go syntax, applied gofmt, narrowed readiness response and replaced the diagnostic hash test with a stable exact-byte assertion
rejected_hypotheses:
  - client-ordered preference
  - post-redeem client selection
  - gameplay-byte sniffing or fallback
  - enabling native during producer delivery
  - accepting arbitrary schema hash under oteryn.native.v1
changed_paths:
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/2026_08_04_170000_add_native_gameplay_protocol_policy.php
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-producer.md
validation:
  - command: Game Gateway CI on pre-final coherent implementation head f9aef6c73b4a32da12783baaad468c356a352d2e
    result: PASS
    evidence: run 30951172466 passed gofmt, all Go tests including producer E2E, vet and build
  - command: fresh independent security/consistency audit
    result: PASS
    evidence: exact schema identity, strict readiness projection, bounded characters and documentation contradictions were identified and repaired; zero open material findings remain
  - command: exact-final-head repository workflows
    result: NOT_RUN
    evidence: this checkpoint commit starts the authoritative final draft generation
blockers:
  - none
next_action: verify all exact-head workflows, mark PR ready, verify ready-state generation, then merge and archive
```

## Notes

This is producer-only completion. Native gameplay remains unavailable until separately authorized Otheryn and Rust consumer packages and integrated gameplay E2E are complete.
