---
task_id: OTERYN-20260805-native-protocol-single-version-completion
coordination_id: OTS-20260804-native-protocol-selection
status: implementing
agent: ChatGPT
branch: agents/ots-native-selection-platform-correction-20260804
base_branch: main
created: 2026-08-05T12:41:00+02:00
updated: 2026-08-05T12:41:00+02:00
risk: high
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
execution_mode: github-only
execution_budget_minutes: 120
implementation_authorized: true
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - live main heads in all authorized repositories
  - active tasks, ownership and shared-path leases
  - related open and superseded pull requests
  - exact current protocol contracts, IDL and runtime producers/consumers
optional_reads: []
---

# OTERYN-20260805-native-protocol-single-version-completion

## Goal

Complete the coordinated native Oteryn protocol programme across Platform, Otheryn and the Rust client with exactly one native tuple: `family = oteryn`, `native_protocol_version = 1`, and no native `profile` dimension or placeholder. Keep production activation disabled.

## Acceptance criteria

- [ ] Correct and merge the canonical Platform contract, IDL, migration/rollout documents and cross-repository correspondence.
- [ ] Correct and merge Otheryn correspondence.
- [ ] Correct and merge Rust-client correspondence.
- [ ] Remove the transitional native `profile` dimension from the disabled Platform/Game Gateway producer and safely deliver API, persistence, World Registry, readiness and Game Session v2 changes.
- [ ] Implement and merge the authoritative Otheryn Game Session v2 and native TLS/ASIO protocol runtime.
- [ ] Implement and merge independent Rust `protocol-oteryn` using the existing Tokio transport and automatic family selection without native profile selection.
- [ ] Pass security, parser, replay, downgrade, snapshot, delta, action-cycle and `protocol-canary` regression validation.
- [ ] Pass bounded staging E2E and rollback through the real authorized chain without enabling production.
- [ ] Pass exact-head CI and independent audits in every repository.
- [ ] Merge all related PRs, archive all programme tasks and release ownership/leases.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-completion.md
  - docs/architecture/adr/0010-native-gameplay-protocol-selection.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md
  - docs/contracts/oteryn_native_gameplay_v1.proto
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_MIGRATION.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/contracts/correspondence/otheryn/**
  - docs/contracts/correspondence/otclient-rust/**
  - docs/contracts/fixtures/oteryn-native-v1/**
modules:
  - native-gameplay-protocol-contract
  - cross-repository-correspondence
dependencies:
  - canonical completion prompt on Platform main at task start
  - merged single-version decision ADR/amendment
blockers:
  - none for the Platform contract-correction phase
cross_repository_tasks:
  - OTH-20260805-native-protocol-single-version-completion
  - OTC2-20260805-native-protocol-single-version-completion
```

## Programme sequence

1. Platform contract/IDL correction.
2. Otheryn correspondence.
3. Rust-client correspondence.
4. Platform/Game Gateway producer.
5. Otheryn runtime.
6. Rust runtime.
7. Bounded staging E2E and rollback.
8. Final audits, exact-head CI, merges, archives and ownership release.

The Platform producer phase will explicitly supersede or reconcile stale ownership of `services/game-gateway/**` before any runtime edit. Canary compatibility profiles remain unchanged and isolated.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T12:41:00+02:00
head: b56c2b27ceae5c7798632631b1386d726f2425f7
branch: agents/ots-native-selection-platform-correction-20260804
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - canary-integration
  - api
  - security
  - testing
owned_paths:
  - Platform correction paths listed in Ownership
proven:
  - The canonical completion prompt was read completely from current Platform main.
  - The owner decision requires exactly family oteryn and native protocol version 1 with no native profile field, alias, placeholder or selection mechanism.
  - No open PR using coordination ID OTS-20260804-native-protocol-selection existed in the three authorized repositories at preflight.
  - Platform contract and disabled producer work from the earlier programme are merged but still contain the transitional profile dimension.
  - Otheryn has no active native-protocol owner; its active PRS coordinator owns only its own task record.
  - Rust protocol-canary ownership is isolated to protocol-canary paths and declares no shared-path lease; protocol-oteryn paths are free.
  - Production activation is not authorized.
derived:
  - This programme is a correction and completion of merged transitional work, not a greenfield contract.
  - Contract and correspondence must merge before runtime producers and consumers.
unknown:
  - Exact final staging environment availability and protected secret readiness for the bounded E2E phase.
conflicts:
  - Stale Platform native-auth task ownership still lists services/game-gateway/** and must be explicitly reconciled before the producer phase.
  - Otheryn PR #339 touches legacy protocolgame.cpp; native runtime changes must remain isolated or wait for terminal state.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Preserve an empty native profile field for future variants.
  - Use oteryn.native.v1 as a native profile.
  - Reuse Canary compatibility profile machinery for the native protocol.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-completion.md
validation:
  - command: live repository, task, ownership, lease and open-PR preflight
    result: PASS
    evidence: three current repositories and their relevant active records and open diffs were inspected before branch creation
blockers:
  - none
next_action: Correct the Platform contract, IDL, migration/rollout documents, correspondence and fixtures on the canonical correction branch.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: OTS-20260805T1241+0200-platform-contract-correction
  session_started_at: 2026-08-05T12:41:00+02:00
  checkpointed_at: 2026-08-05T12:41:00+02:00
  last_progress_at: 2026-08-05T12:41:00+02:00
  phase: platform-contract-correction
  exact_head: b56c2b27ceae5c7798632631b1386d726f2425f7
  pull_request: none
  active_operation: correct Platform contract and IDL
  external_run_ids: []
  operation_started_at: 2026-08-05T12:41:00+02:00
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: canonical correction branch exists and no conflicting owner has claimed the correction paths
  next_action: Correct the Platform contract, IDL, migration/rollout documents, correspondence and fixtures.
```

## Notes

This task authorizes repository changes and isolated validation only. It does not authorize production deployment, protected-environment approval, production secret mutation or live-data mutation.
