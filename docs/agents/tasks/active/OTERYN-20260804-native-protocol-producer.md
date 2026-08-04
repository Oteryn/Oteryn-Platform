---
task_id: OTERYN-20260804-native-protocol-producer
project_lane: oteryn-platform-auth
status: ready
branch: feat/OTERYN-20260804-native-protocol-producer
base_branch: main
created: 2026-08-04
updated: 2026-08-05
related_pr: "523"
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/agents/prompts/OTS_PLATFORM_GATEWAY_NATIVE_PROTOCOL_IMPLEMENTATION.md
search_first:
  - OTS-20260804-native-protocol-selection
  - gameplay_offer
  - game session v2
optional_reads:
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
---

# OTERYN-20260804-native-protocol-producer

## Goal

Implement the disabled-by-default Platform and Game Gateway producer for bounded gameplay offers, deterministic World Registry selection and opaque Game Session contract version 2 while preserving Gateway API v1 and the legacy Canary-compatible path.

## Acceptance criteria

- [x] Legacy Gateway and Game Session v1 behavior remains unchanged when no offer is present.
- [x] Extended requests are bounded and invalid syntax fails before ticket redeem.
- [x] World Registry owns deterministic policy order, revision and endpoint identity.
- [x] Native profile requires the exact canonical transport, schema hash and base capability set.
- [x] Gateway performs one selection, one readiness check and at most one v2 issuance.
- [x] V2 claims bind account generation, login attempt, world/channel, policy, endpoint, audience and exact tuple/digest.
- [x] No native candidate is seeded, enabled or advertised by default.
- [x] Focused tests, producer E2E, rollout documentation and fresh audit are complete.
- [ ] Final ready-state exact-head CI, merge and archival are complete.

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
blockers:
  - none
cross_repository_tasks:
  - Otheryn native v2 consumer remains separate
  - Rust protocol-oteryn remains separate
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-04T23:00:00Z
invocation_started_at: 2026-08-04T20:47:00Z
last_progress_at: 2026-08-04T23:00:00Z
ready_transition_head: c9fb914c005e643084fb9949b567e3dc413773ad
branch: feat/OTERYN-20260804-native-protocol-producer
pr: 523
status: ready
phase: authoritative-ready-state-validation
session_id: agent-20260804-native-protocol-producer-02
session_role: implementer
execution_mode: github
execution_reason: GitHub connector mutation with repository Actions for exact-head validation
project_lane: oteryn-platform-auth
task_kind: implementation
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive security-sensitive producer package
validation_level: full
heavy_validation_runs: 6
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 1
ci_checks_for_current_head: 0
ci_check_generation: ready-state-authoritative
terminal_ci_wait_started_at: 2026-08-04T23:00:00Z
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 6
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
  - Exact canonical schema SHA-256 is c7665223f09001e3294e9a03ab4784defed66b0ac04450e8679d4778421207f8.
  - Candidate persistence is empty and disabled by default.
  - Gateway validates before redeem and selects by authoritative policy order.
  - Readiness and v2 issuance bind the exact immutable selection.
  - Producer E2E and fresh security/consistency audit pass with zero open material findings.
  - Draft-final exact-head CI, Phase 7 and Deep System Validation pass after synchronizing current main.
  - PR 523 transitioned from draft to ready before this checkpoint commit.
derived:
  - Producer merge is safe while disabled; native gameplay remains unavailable without Otheryn and Rust consumers.
unknown: []
conflicts: []
first_failure:
  marker: exact-head formatting, static-analysis and duplicate candidate fixture findings
  evidence: workflow parser, Pint, PHPStan and duplicate identity fixtures were repaired without weakening validation
rejected_hypotheses:
  - client-ordered preference
  - candidate fallback after redeem
  - arbitrary native schema identity
  - native enablement in this package
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
  - command: Game Gateway CI run 30956877442
    result: PASS
    evidence: formatting, all Go tests including producer E2E, vet and build passed on draft-final head 8248d9f1ab3cae44f7a0cd3caaf1128a15a47d9e
  - command: CI run 30956877312
    result: PASS
    evidence: formatting, PHPStan and complete PHP regression passed on the same exact head
  - command: Phase 7 Production-Like Validation run 30956877290
    result: PASS
    evidence: clean MariaDB deployment, migration, privilege, outage, regression, restore and upgrade lanes passed
  - command: Deep System Validation run 30956877280
    result: PASS
    evidence: compiler, dependency audit, PHP regression, concurrency, zero-retry browser matrix, soak and durable evidence passed
  - command: remaining exact-head workflows
    result: PASS
    evidence: governance, portal audit, acceptance, security, staging images, outage and ticket concurrency all passed
  - command: fresh independent audit
    result: PASS
    evidence: exact identity, readiness, response bounds and documentation were challenged and repaired with zero open material findings
blockers:
  - none
next_action: verify this post-ready checkpoint exact-head generation, merge PR 523 and archive
```

## Notes

Producer-only completion. Native gameplay remains disabled until separately authorized Otheryn and Rust consumer packages and integrated gameplay E2E are complete.

PR #523 was already non-draft before this checkpoint commit; workflow runs for this commit are the authoritative ready-state generation.
