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
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
search_first:
  - OTS-20260804-native-protocol-selection
  - gameplay_offer
  - game session v2
optional_reads:
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
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
implementation_status: validating
user_facing_feature_complete: false
missing_consumers:
  - Otheryn Game Session v2 and native producer
  - Rust protocol-oteryn and automatic selection
```

## Acceptance criteria

- [x] Legacy `{protocol_version:1, game_login_ticket}` behavior and security properties remain unchanged.
- [x] Optional `gameplay_offer` is strictly bounded and rejected before ticket redeem when invalid.
- [x] World Registry owns ordered candidate policy, policy revision, endpoint identity and disabled-by-default rollout state.
- [x] Gateway ignores client candidate order and selects the first exact authoritative intersection.
- [x] Capability canonicalization and SHA-256 digest match the merged contract.
- [x] No match returns `unsupported_gameplay_pair` after redeem; no same-ticket retry or second candidate occurs.
- [x] Success response keeps Gateway API version distinct from Game Session/profile/schema fields.
- [x] Game Session v2 request binds account generation, world/channel, policy, endpoint, audience and exact selected tuple/digest.
- [x] Candidate readiness mismatch fails closed before session issuance.
- [x] No native candidate is enabled or advertised by default.
- [x] Synthetic fixtures and focused Platform/Gateway/session tests cover positive, negative and boundary cases.
- [ ] Producer-boundary E2E, fresh independent audit, exact-head required CI, merge and archival complete.

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
  - Otheryn native producer remains a later separate task
  - Rust native adapter remains a later separate task
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-04T20:57:00Z
invocation_started_at: 2026-08-04T20:47:00Z
last_progress_at: 2026-08-04T20:57:00Z
head: 9b37865662daab6e066cda8519a01c99ced61b68
branch: feat/OTERYN-20260804-native-protocol-producer
pr: 523
status: validating
phase: validate
session_id: agent-20260804-native-protocol-producer-02
session_role: implementer
execution_mode: github
execution_reason: GitHub connector writes and repository Actions provide the exact implementation and validation loop
project_lane: oteryn-platform-auth
task_kind: implementation
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive security-sensitive producer package spanning Platform policy, Gateway selection and session issuer compatibility
validation_level: focused
heavy_validation_runs: 1
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 1
ci_checks_for_current_head: 11
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
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
  - Canonical contract merge is 9035ae987db67c062a8778721a2c8e686ce76750 and schema blob is adc5e906fa612fdfa00e050687bdfd5697f695cf.
  - Exact canonical schema SHA-256 is c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8 and is pinned by an exact-byte repository test.
  - World Registry persistence has no candidate seed; all protocol candidate rows default enabled=false.
  - Gateway preserves the legacy request/session path when gameplay_offer is absent.
  - Extended requests are bounded to 16 KiB, reject duplicate JSON keys, unknown fields, invalid identifiers, candidate duplication and noncanonical capabilities before ticket redeem.
  - Gateway selects by authoritative policy order, performs one readiness check and invokes one v2 session issuance without fallback.
  - Game Session v2 binds account generation, world, channel, policy revision, endpoint, audience, exact tuple, capabilities and single-admission mode.
  - Producer-boundary synthetic E2E covers Platform redeem/context through Gateway selection/readiness/issuance and public response.
  - Temporary one-shot formatting and digest workflows removed themselves and are absent from the final PR changed-file inventory.
derived:
  - The producer package is safe to merge disabled-by-default before Otheryn and Rust consumers because no native candidate is seeded or advertised.
unknown: []
conflicts: []
first_failure:
  marker: Game Gateway CI formatting gate
  evidence: initial Go multiline syntax and later gofmt differences were repaired; the diagnostic schema test was replaced by a stable exact-byte assertion
rejected_hypotheses:
  - client-ordered production preference
  - post-redeem second-stage client selection
  - gameplay byte sniffing or fallback
  - seeding or enabling the native candidate during producer delivery
changed_paths:
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/2026_08_04_170000_add_native_gameplay_protocol_policy.php
  - services/game-gateway/internal/**
  - tests/Feature/GameAuth/**
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-producer.md
validation:
  - command: exact schema byte digest assertion
    result: PASS
    evidence: pinned digest c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8
  - command: gofmt on all changed Go files
    result: PASS
    evidence: one-shot formatting commit 0e9c207c6052a40944f60b60e71e08d7b4fa1772 removed its workflow from the final diff
  - command: exact-head required CI
    result: NOT_RUN
    evidence: this checkpoint commit intentionally starts a user-token CI generation after bot-created commits were classified action_required
blockers:
  - none
next_action: verify focused tests and every required workflow on the exact checkpoint head, then perform fresh independent audit and finalize PR 523
```

## Notes

The package is producer-only. Native gameplay remains unavailable until the separately authorized Otheryn and Rust consumer packages and exact integrated E2E are complete.
