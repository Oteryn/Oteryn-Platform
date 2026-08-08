---
task_id: OTERYN-20260808-native-pre-admission-handoff
repository: blakinio/Oteryn-Platform
issue: 888
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
optional_reads:
  - read-only Oteryn-v2 ADR-0003 Platform Identity/Game Gateway/admission boundary
---

# OTERYN-20260808 native pre-admission handoff

## Goal

Define the Platform-side semantic boundary for native pre-admission material handed from Platform/Game Gateway to Oteryn-v2, preserving accepted authority while leaving final game-domain admission/session/lease/fencing implementation external and read-only.

## Acceptance criteria

- [x] Focused contract defines pre-admission purpose, authority, canonical identities, binding, freshness, expiry and replay semantics.
- [x] Pre-admission material is explicitly not canonical `GameSessionId`, gameplay lease or proof of final admission.
- [x] Issuance preconditions compose authoritative ticket redemption, character authorization, World Registry policy and fresh applicable runtime evidence.
- [x] Failure, ambiguity, duplicate/replay, channel-switch and reconnect boundaries fail closed without inventing Oteryn-v2 implementation.
- [x] Focused integration and Gateway/Identity documentation route to the contract without reactivating historical Platform gameplay authority.
- [x] Native account identity is reconciled with ADR 0028: native ticket/redeem authority uses canonical `AccountId`; delivered `canary_account_id` redeem v1 remains compatibility evidence only.
- [x] Oteryn-v2 remains read-only; exact transport/encoding/signing/lease/fencing/GameSessionId wire format remain deferred.
- [ ] Exact-head Agent Governance and repository-selected CI pass; full diff/review has zero unresolved material findings.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task is architecture/documentation only.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260808-native-pre-admission-handoff.md
  - docs/agents/tasks/active/OTERYN-20260808-native-pre-admission-handoff.md
modules:
  - Identity
  - Integration
  - GameGateway
  - architecture-governance
dependencies:
  - Issue #888
  - ADR 0028
  - OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT
  - ADR 0031
  - Oteryn-v2 ADR-0003 read-only evidence
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T10:58:00+02:00
head: e85b9b542b82ac07909f5886f86c85071f96d075
branch: docs/OTERYN-20260808-native-pre-admission-handoff
pr: 900
status: validating
phase: exact-head-validation
execution_mode: github_only
invocation_started_at: 2026-08-08T10:39:00+02:00
last_progress_at: 2026-08-08T10:58:00+02:00
ci_checks_for_current_head: 1
ci_check_generation: repair-1
terminal_ci_wait_started_at: 2026-08-08T10:58:00+02:00
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
context_reconstruction_attempts: 1
stall_warnings: 0
context_routes:
  - architecture
  - security
  - api
  - operations
owned_paths:
  - docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260808-native-pre-admission-handoff.md
  - docs/agents/tasks/active/OTERYN-20260808-native-pre-admission-handoff.md
proven:
  - Platform ADR 0031 owns Identity, Game Login Ticket, World Registry and Gateway pre-admission orchestration while assigning final gameplay admission and authoritative admitted session/lease/fencing to Oteryn-v2.
  - Read-only Oteryn-v2 ADR-0003 makes the same authority distinction and explicitly distinguishes pre-admission material from canonical logical GameSessionId.
  - ADR 0028 and OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT define canonical native AccountId as Platform-issued UUIDv7 and make canary_account_id legacy compatibility only.
  - Delivered GAME_GATEWAY_IDENTITY_CONTRACT redeem v1 still binds/returns canary_account_id and therefore remains a compatibility implementation, not proof of a native AccountId-bearing redeem context.
  - Historical Platform native gameplay/Game Session v2 artifacts are reconciliation evidence only and cannot define current Oteryn-v2 admission/session authority.
  - The orphaned predecessor branch already contained the focused semantic contract, Gateway routing clarification, integration-architecture update and review report.
  - The recovered branch was restacked to current main as one commit with behind_by=0 and five intended changed paths.
derived:
  - A focused Platform semantic contract closes the Platform-side handoff ambiguity without selecting unfinished Oteryn-v2 wire or lease implementation.
  - Native implementation will require a separately authorized versioned Identity/Gateway redeem/login context that yields canonical AccountId before pre-admission issuance can be implemented safely.
unknown:
  - exact native AccountId-bearing redeem/login-context endpoint/version/runtime implementation
  - exact Oteryn-v2 FND-04 admission/session state machine
  - material transport/encoding/signing primitive
  - replay store and atomic consume implementation
  - lease/fencing algorithm
  - canonical GameSessionId wire form
conflicts: []
first_failure:
  marker: checkpoint-status-vocabulary
  evidence: first exact-head Agent Governance and CI classify-changes failed only because checkpoint status used unsupported value active; architecture/native protocol checks otherwise started normally and both native-protocol contract workflows passed
rejected_hypotheses:
  - Platform-issued pre-admission material is a canonical gameplay session
  - historical Platform native Game Session v2 bytes define the Oteryn-v2 target
  - successful legacy Canary redeem v1 is sufficient proof of a native AccountId-bearing login context
  - native AccountId may be reconstructed from canary_account_id as canonical authority
  - first CI failure proves an architecture or runtime defect
changed_paths:
  - docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/agents/reports/OTERYN-20260808-native-pre-admission-handoff.md
  - docs/agents/tasks/active/OTERYN-20260808-native-pre-admission-handoff.md
validation:
  - command: orphaned-session recovery preflight
    result: PASS
    evidence: predecessor branch had no PR and no durable progress after 2026-08-08T09:29:35+02:00; existing branch was reused rather than duplicated
  - command: authority review against ADR 0028, Account Identity contract, ADR 0031 and Oteryn-v2 ADR-0003
    result: PASS
    evidence: native AccountId authority and game-domain final admission authority are explicitly separated from legacy Canary redeem/session semantics
  - command: full changed-file semantic review
    result: PASS
    evidence: one material AccountId/redeem-v1 ambiguity was found and reconciled; zero unresolved material findings remain
  - command: restack against main@3dc9b9a7f21c04aca16d3729dbf951621c800f07
    result: PASS
    evidence: PR 900 head e85b9b542b82ac07909f5886f86c85071f96d075 was one commit ahead, zero behind, mergeable, with exactly five intended paths
  - command: first exact-head CI generation
    result: FAIL
    evidence: checkpoint validator rejected unsupported status active; native protocol contract and native protocol contract audits both passed
  - command: runtime/browser E2E
    result: NOT_RUN
    evidence: architecture/documentation only; no executable runtime or deployment behavior is authorized
blockers: []
next_action: Re-run exact-head required workflows after checkpoint status repair; merge only if required CI is green and final review remains clean.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: github-20260808-1039-issue888-takeover
  session_started_at: 2026-08-08T10:39:00+02:00
  checkpointed_at: 2026-08-08T10:58:00+02:00
  last_progress_at: 2026-08-08T10:58:00+02:00
  phase: exact-head-validation
  exact_head: e85b9b542b82ac07909f5886f86c85071f96d075
  pull_request: 900
  active_operation: required CI after checkpoint-status repair
  external_run_ids:
    - 31249435310
    - 31249435288
  operation_started_at: 2026-08-08T10:58:00+02:00
  wait_deadline_at: null
  check_generation: repair-1
  checks_used: 1
  status: validating
  safe_to_resume: true
  resume_condition: PR remains mergeable and current head workflows complete without new material failures
  next_action: Inspect the next exact-head workflow generation; do not retry unchanged failures.
```
