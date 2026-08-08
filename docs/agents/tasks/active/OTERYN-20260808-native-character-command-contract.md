---
task_id: OTERYN-20260808-native-character-command-contract
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
issue: 919
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
search_first:
  - Issue #919
  - Issues #317 #319 #320
  - open PRs and active branches for native character command/result work
optional_reads:
  - docs/contracts/CHARACTER_DELETION_CONTRACT.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
---

# OTERYN-20260808 native Character Authority command contract

## Goal

Define one reusable Platform-side semantic command/result contract for native Oteryn-v2 Character Authority mutations so create, rename, deletion/restore/finalization, optional world transfer and Bazaar/account ownership transfer share consistent authority, idempotency, concurrency, typed outcome and reconciliation rules without freezing Oteryn-v2 wire implementation.

## Acceptance criteria

- [x] Common command envelope and result/receipt semantics are explicit.
- [x] Canonical AccountId/CharacterId and applicable WorldId/ChannelId are used; Canary numeric IDs remain compatibility-only.
- [x] Stable operation identity, idempotency and conflicting-reuse behavior are defined.
- [x] Typed terminal/non-terminal/ambiguous outcomes and reconciliation semantics are defined.
- [x] Oteryn-v2 revalidation and Platform orchestration authority remain separated.
- [x] Cross-command/session/Bazaar concurrency and mutual-exclusion semantics are explicit without distributed ACID.
- [x] Per-command profiles cover create, rename, schedule deletion, restore/cancel, finalize deletion, optional world transfer and Bazaar/account ownership transfer.
- [x] Public projection effects route to the accepted PublicGameData projection/privacy contract rather than creating a second authority.
- [x] Existing lifecycle routing guide points to the new focused contract.
- [x] Bazaar ownership transfer preserves authoritative game transfer before wallet settlement.
- [x] Retryable producer failure remains non-terminal/reconcilable under the same operation identity; terminal rejection permits only a later new semantic attempt after fresh Platform gates.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass on the post-review-repair release candidate.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: architecture/documentation-only change with no executable producer, consumer, schema, route or product activation.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
modules:
  - Characters
  - Accounts
  - PublicGameData
  - architecture-governance
dependencies:
  - Issue #919
  - ADR 0030
  - ADR 0031
  - Issues #317 #319 #320
blockers:
  - none
cross_repository_tasks:
  - none
```

## Exact-content self-review after review repairs

```yaml
self_review:
  result: PASS
  exact_head: dd8e35400bf882f79a30f9ff8cecd572673e1fd0
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - Exactly four intended documentation/task/report paths are changed.
    - P1 review finding is repaired: funds remain reserved until authoritative Oteryn-v2 ownership transfer COMPLETED, then wallet settlement executes under existing wallet idempotency semantics.
    - P2 review finding is repaired: terminal REJECTED is distinct from ACCEPTED_PENDING/RETRYABLE_PENDING/AMBIGUOUS, and uncertain operations retain the same operation identity.
    - Shared operation identity, idempotency, conflicting reuse, terminal/non-terminal result and reconciliation semantics are explicit.
    - Oteryn-v2 remains authoritative for ownership/lifecycle/game-state mutation while Platform remains orchestration/business authority.
    - Public projection/privacy routing points to OTERY​N_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT rather than creating a second read authority.
    - Canary direct SQL/numeric identifiers remain Legacy Canary Compatibility only; ambiguous native operations cannot fall back blindly to a second authority.
    - No runtime/schema/workflow/deployment/payment/production/external-repository path changed.
```

This checkpoint-only commit will become the final validation head without changing the reviewed architecture semantics.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T20:06:00+02:00
head: dd8e35400bf882f79a30f9ff8cecd572673e1fd0
branch: docs/OTERYN-20260808-native-character-command-contract
pr: 920
status: validating
context_routes:
  - architecture
  - character-lifecycle
  - cross-repository-contract
  - security
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
proven:
  - Issue #919 and PR #920 are the single focused owner/delivery for the shared native Character Authority command/result boundary.
  - No other open PR or active branch owns this exact shared contract.
  - Initial release candidate 95700bbf02316f13891bbadb1a2b35e07c851d5e passed all eight selected workflows but was not merged because review then identified one P1 and one P2 semantic finding.
  - P1 Bazaar ordering finding is repaired in 585704c3b38dee8eaa28cd1cc12e5780698083f9 and routing follow-up f00001e633c2b14222ccb9a0ae9eb8bc68f4981d.
  - P2 retryable-failure operation-identity finding is repaired in 585704c3b38dee8eaa28cd1cc12e5780698083f9.
  - Architecture report records both findings and their dispositions in dd8e35400bf882f79a30f9ff8cecd572673e1fd0.
  - Both review threads are resolved after verifying the repaired text.
  - Branch remains behind_by=0 against main at post-repair review.
derived:
  - #317/#319 can consume one architecture baseline instead of inventing independent cross-system mutation semantics.
  - #320 receives reusable transport-independent semantics without approving the world-transfer product capability.
unknown:
  - exact Oteryn-v2 transport/IDL/encoding/endpoint
  - exact durable result/reconciliation mechanism
  - exact game-internal transaction, locking and command state-machine implementation
  - exact native world-transfer product adoption and transfer rules
conflicts: []
first_failure:
  marker: pr-review-semantic-findings
  evidence: PR #920 review identified P1 Bazaar transfer-before-wallet-settlement ordering and P2 retryable failure operation-identity terminality ambiguity; both are repaired and threads resolved
rejected_hypotheses:
  - each lifecycle issue should independently invent idempotency and result semantics
  - current Canary SQL contracts should define the native command boundary
  - Platform operation rows or public projections can prove current character ownership or mutation completion
  - operation_id can substitute for service authentication or replay protection
  - ambiguous native mutations may safely fall back to direct Canary/native SQL
  - wallet settlement may precede authoritative game ownership transfer
  - retryable_internal_failure can be a terminal rejection while still retrying the same operation as non-terminal
changed_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/agents/reports/OTERYN-20260808-native-character-command-contract.md
  - docs/agents/tasks/active/OTERYN-20260808-native-character-command-contract.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no overlapping owner found; Issue #919 and PR #920 are the focused owner/delivery
  - command: initial exact-head workflow generation on 95700bbf02316f13891bbadb1a2b35e07c851d5e
    result: PASS
    evidence: Agent Governance 31270752002, Native protocol contract 31270752040, Native protocol contract audits 31270752018, Game Auth Ticket Concurrency 31270752043, Edge Security Emulation 31270752027, Platform DB Outage Validation 31270752016, CI 31270752009 and Phase 7 Production-Like Validation 31270752010 all passed; later semantic review repairs invalidate this as final merge evidence
  - command: post-review exact content-head full-diff architecture self-review
    result: PASS
    evidence: dd8e35400bf882f79a30f9ff8cecd572673e1fd0 resolves P1/P2 findings with zero remaining material findings in the four-path diff
  - command: PR #920 review-thread hygiene after repair
    result: PASS
    evidence: both P1/P2 review threads resolved after repaired text verification
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task with no executable producer, consumer, schema, route or product activation
blockers:
  - none
next_action: Observe repository-selected CI on the new checkpoint release-candidate head, repair any exact-head failure, and merge only after all required checks pass and review hygiene remains clean.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260808T1948+0200-issue-919
  session_started_at: 2026-08-08T19:48:00+02:00
  checkpointed_at: 2026-08-08T20:06:00+02:00
  last_progress_at: 2026-08-08T20:06:00+02:00
  phase: validate
  exact_head: dd8e35400bf882f79a30f9ff8cecd572673e1fd0
  pull_request: 920
  active_operation: final exact-head CI after review repairs
  external_run_ids: []
  operation_started_at: 2026-08-08T20:06:00+02:00
  wait_deadline_at: 2026-08-08T20:51:00+02:00
  check_generation: post-review-repair-ready
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: GitHub Actions emits repository-selected checks for the post-review-repair release candidate
  next_action: Observe one aggregate PR/head workflow snapshot and proceed to merge only if the final unchanged head is fully green and review hygiene remains clean.
```

## Notes

Oteryn-v2 and Canary remain read-only. This task does not authorize runtime/schema/workflow/deployment/payment/production changes or product activation.