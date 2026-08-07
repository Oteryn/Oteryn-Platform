---
task_id: OTERYN-20260805-native-protocol-single-version-completion
coordination_id: OTS-20260804-native-protocol-selection
status: validating
terminal_pr_policy: archive_pending
agent: ChatGPT
branch: agents/ots-native-selection-platform-correction-20260804
base_branch: main
created: 2026-08-05T12:41:00+02:00
updated: 2026-08-07T11:31:00Z
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
updated_at: 2026-08-07T11:31:00Z
head: eaed70477258e0e1dfb5b03c1e74002913e919dc
branch: agents/ots-native-selection-platform-correction-20260804
pr: 540
status: validating
terminal_pr_policy: archive_pending
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
  - Platform contract correction PR #540 merged from branch agents/ots-native-selection-platform-correction-20260804 at exact source head eaed70477258e0e1dfb5b03c1e74002913e919dc as c0b8703d326a04b43ae8e06f6192b0cb91c859b7.
  - Later Platform producer work was delivered through separate phase-specific task ownership, including merged PR #542 and its archived producer task; this umbrella record must not keep ownership alive through the retained PR #540 source branch.
  - Platform contract and disabled producer work from the earlier programme are merged; production activation remains unauthorized.
  - Otheryn and Rust continuation state must be reconciled from their own current repositories before any claim of whole-programme completion.
derived:
  - The PR #540 branch phase is terminal and this task record is archive-pending for that ownership identity.
  - This archive-pending transition does not claim that the full cross-repository programme is complete; any remaining work must continue through current phase-specific task ownership after fresh live reconciliation.
unknown:
  - Exact current terminal/completion state of every Otheryn and Rust phase-specific task.
  - Exact final staging environment availability and protected secret readiness for the bounded E2E phase.
conflicts: []
first_failure:
  marker: stale-branch-only-ownership
  evidence: Issue #788 validation proved this active task still treated retained terminal PR #540 source branch as BRANCH_ONLY ownership because pr was omitted.
rejected_hypotheses:
  - Preserve an empty native profile field for future variants.
  - Use oteryn.native.v1 as a native profile.
  - Reuse Canary compatibility profile machinery for the native protocol.
  - Treat retained PR #540 source-branch existence as current implementation ownership.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-completion.md
validation:
  - command: live GitHub reconciliation of PR #540 and retained source branch under Issue #788 liveness semantics
    result: PASS
    evidence: PR #540 is closed merged, exact source branch/ref/head identity is known, and ownership is explicitly terminal/archive-pending rather than branch-only.
blockers: []
next_action: Archive this terminal contract-phase ownership record; continue any remaining native-protocol programme work only through freshly reconciled phase-specific tasks without reusing PR #540 branch ownership.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: OTS-20260805T1241+0200-platform-contract-correction
  session_started_at: 2026-08-05T12:41:00+02:00
  checkpointed_at: 2026-08-07T11:31:00Z
  last_progress_at: 2026-08-07T11:31:00Z
  phase: platform-contract-terminal-reconciliation
  exact_head: eaed70477258e0e1dfb5b03c1e74002913e919dc
  pull_request: 540
  active_operation: archive terminal contract-phase ownership identity
  external_run_ids: []
  operation_started_at: 2026-08-07T11:31:00Z
  wait_deadline_at: null
  check_generation: terminal
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #540 is terminal and the retained branch must not be used as active ownership
  next_action: Archive this terminal contract-phase ownership record, then reconcile any remaining phase-specific native-protocol tasks from live repository state.
```

## Notes

This task authorizes repository changes and isolated validation only. It does not authorize production deployment, protected-environment approval, production secret mutation or live-data mutation. The 2026-08-07 reconciliation changes only durable task ownership truth; it does not claim full cross-repository programme completion.
