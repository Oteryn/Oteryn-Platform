---
task_id: OTERYN-20260804-native-protocol-producer
project_lane: oteryn-platform-auth
status: investigating
branch: feat/OTERYN-20260804-native-protocol-producer
base_branch: main
created: 2026-08-04
updated: 2026-08-04
related_pr: ""
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
implementation_status: implementing
user_facing_feature_complete: false
missing_consumers:
  - Otheryn Game Session v2 and native producer
  - Rust protocol-oteryn and automatic selection
```

## Acceptance criteria

- [ ] Legacy `{protocol_version:1, game_login_ticket}` behavior and security properties remain unchanged.
- [ ] Optional `gameplay_offer` is strictly bounded and rejected before ticket redeem when invalid.
- [ ] World Registry owns ordered candidate policy, policy revision, endpoint identity and disabled-by-default rollout state.
- [ ] Gateway ignores client candidate order and selects the first exact authoritative intersection.
- [ ] Capability canonicalization and SHA-256 digest match the merged contract.
- [ ] No match returns `unsupported_gameplay_pair` after redeem; no same-ticket retry or second candidate occurs.
- [ ] Success response keeps Gateway API version distinct from Game Session/profile/schema fields.
- [ ] Game Session v2 request binds account generation, world/channel, policy, endpoint, audience and exact selected tuple/digest.
- [ ] Candidate readiness mismatch fails closed before session issuance.
- [ ] No native candidate is enabled or advertised by default.
- [ ] Synthetic fixtures and focused Platform/Gateway/session tests cover positive, negative and boundary cases.
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
updated_at: 2026-08-04T16:35:00Z
invocation_started_at: 2026-08-04T16:35:00Z
last_progress_at: 2026-08-04T16:35:00Z
head: UNKNOWN
branch: feat/OTERYN-20260804-native-protocol-producer
pr: none
status: investigating
phase: investigate
session_id: agent-20260804-native-protocol-producer-01
session_role: implementer
execution_mode: github
execution_reason: GitHub connector writes plus repository Actions provide the required multi-file implementation and validation loop
project_lane: oteryn-platform-auth
task_kind: implementation
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: medium
decomposition_decision: phased
decomposition_reason: one cohesive security-sensitive producer package spanning Platform policy, Gateway selection and session issuer compatibility
validation_level: not-run
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
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
  - The canonical contract is merged as 9035ae987db67c062a8778721a2c8e686ce76750.
  - The canonical schema blob is adc5e906fa612fdfa00e050687bdfd5697f695cf.
  - Current Gateway validates legacy JSON before ticket redeem and issues exactly one legacy Game Session.
  - Current Platform World Registry is database-owned, online/login-enabled and empty by default.
  - No open PR overlaps Gateway, World Registry or Game Session producer runtime paths.
derived:
  - A normalized per-world candidate table can preserve current game_worlds routing and keep native disabled without seeding any candidate.
unknown:
  - Exact SHA-256 of the canonical schema bytes must be computed and pinned before runtime implementation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - client-ordered production preference
  - post-redeem second-stage client selection
  - gameplay byte sniffing or fallback
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260804-native-protocol-producer.md
validation:
  - command: live ownership and overlap preflight
    result: PASS
    evidence: open PR inventories show no overlap with declared Gateway, World Registry or Game Session paths
blockers:
  - none
next_action: compute and pin the exact canonical schema SHA-256 using an exact-byte repository test before runtime mutation
```

## Notes

The package is producer-only. Native gameplay remains unavailable until the separately authorized Otheryn and Rust consumer packages and exact integrated E2E are complete.