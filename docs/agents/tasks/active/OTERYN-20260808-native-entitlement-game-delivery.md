---
task_id: OTERYN-20260808-native-entitlement-game-delivery
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
issue: 924
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - Issue #924
  - Issues #321 #322 #278
  - open PRs/branches for entitlement or game-grant work
---

# OTERYN-20260808 native entitlement game-delivery boundary

## Goal

Define the Platform-side native boundary between commercial product/order truth, Platform entitlement lifecycle and any Oteryn-v2 gameplay enforcement/mutation required to fulfil an entitlement, without choosing payment provider, runtime transport, persistence implementation or activating products.

## Acceptance criteria

- [x] Platform-only, game-consumed account entitlement, durable gameplay grant, character-service entitlement, Oteryn Coin package and voucher/redeem profiles are classified.
- [x] Payment/order, Platform entitlement and game-delivery truth remain separate.
- [x] Canonical identity, stable delivery operation identity, idempotency, typed outcome and reconciliation semantics are explicit.
- [x] Single-use character-service reserve -> authoritative operation -> consume ordering is explicit.
- [x] Premium/VIP expiry/revocation semantics preserve Platform entitlement authority and game enforcement boundaries without inventing forced-session behavior.
- [x] Refund/chargeback/revocation policy distinguishes reversible delivery, compensation, deny-future-use and manual reconciliation.
- [x] Character-service mutations reuse the accepted Character Authority command contract.
- [x] Oteryn Coin packages route through Platform Wallet mutators, not game persistence.
- [x] Canary numeric IDs/direct SQL remain Legacy Canary Compatibility only.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass on the final checkpoint head.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: documentation/architecture only; no executable payment, entitlement, wallet or game-delivery behavior changes.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-entitlement-game-delivery.md
  - docs/agents/tasks/active/OTERYN-20260808-native-entitlement-game-delivery.md
modules:
  - ProductsEntitlements
  - Integration
  - Wallet
  - Characters
  - architecture-governance
dependencies:
  - Issue #924
  - Issue #322
  - Issue #321 for paid-order authorization only
  - ADR 0031
  - OTERY​N_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT
blockers: []
cross_repository_tasks: []
```

## Exact-content self-review

```yaml
self_review:
  result: PASS
  exact_head: cc03e597da453c1725a69600ce7f39e1e0433eda
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - Exactly three intended documentation/task/report paths are changed.
    - Payment/order, Platform entitlement and game-delivery/enforcement truth are separate authorities.
    - Six explicit delivery profiles prevent Platform-only, game-consumed, durable grant, character service, Wallet coin and voucher paths from silently sharing mutation authority.
    - Character-service ordering reserves entitlement before game submission and consumes only after terminal authoritative Character Authority completion.
    - Premium/VIP expiry/revocation preserves revision ordering while leaving in-session disconnect/grace policy explicitly deferred.
    - Refund/chargeback handling requires reversible, compensating, deny-future-use or manual policy and forbids silent destructive game/Wallet edits.
    - Stable delivery identity/idempotency/reconciliation prevents duplicate gameplay grants after timeout/replay.
    - Oteryn Coins stay on approved Platform Wallet mutator/ledger authority.
    - Canary direct SQL/numeric IDs remain Legacy Canary Compatibility only.
    - No runtime/schema/workflow/provider/wallet/deployment/production/external-repository path changed.
```

The next checkpoint-only commit establishes the final validation head without changing contract/report semantics.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T20:27:00+02:00
head: cc03e597da453c1725a69600ce7f39e1e0433eda
branch: docs/OTERYN-20260808-native-entitlement-game-delivery
pr: 925
status: validating
phase: validate
execution_mode: github_only
context_routes:
  - architecture
  - commerce
  - data-ownership
  - cross-repository-contract
owned_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-entitlement-game-delivery.md
  - docs/agents/tasks/active/OTERYN-20260808-native-entitlement-game-delivery.md
proven:
  - Issue #924 and PR #925 are the focused owner/delivery for this architecture boundary.
  - No overlapping open PR or branch owns native entitlement/game-delivery semantics.
  - Issue #322 remains implementation owner for Products/Entitlements/Vouchers/customer histories.
  - Issue #321 remains provider/payment truth owner and browser/provider return cannot directly grant value.
  - MODULE_CATALOG keeps ProductsEntitlements, Wallet and Payments responsibilities distinct.
  - ADR 0031 keeps gameplay mutation/enforcement authority in Oteryn-v2.
  - Character-service mutation reuses the accepted native Character Authority command/result contract.
  - Contract and report define stable delivery identity, distinct entitlement/delivery states, reconciliation, product profiles and rollback/reversal rules.
  - Branch is behind_by=0 against main at semantic self-review.
derived:
  - #322 can implement Platform entitlement lifecycle without inventing direct game writes or conflating payment success with gameplay delivery.
  - Future paid character services require both entitlement reservation/consumption and the authoritative Character Authority operation.
unknown:
  - exact Oteryn-v2 entitlement/grant transport and enforcement storage
  - exact product catalogue/entitlement persistence schema and worker design
  - exact premium/VIP in-session expiry/grace policy
  - exact reversible/compensating policy for each future durable gameplay grant
  - selected real payment provider and legal/tax product policy
conflicts: []
first_failure:
  marker: none
  evidence: full three-path architecture diff review found zero material findings
rejected_hypotheses:
  - payment success can directly prove gameplay delivery
  - active entitlement means game effect is already applied
  - voucher redemption may bypass entitlement delivery profile
  - Oteryn Coin packages should write game coin fields directly
  - character-service entitlement consumption may precede authoritative game completion
  - chargeback may silently reverse arbitrary irreversible gameplay state
  - ambiguous native delivery may safely fall back to direct Canary/native SQL
changed_paths:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/agents/reports/OTERYN-20260808-native-entitlement-game-delivery.md
  - docs/agents/tasks/active/OTERYN-20260808-native-entitlement-game-delivery.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: no competing native entitlement/game-delivery owner found
  - command: exact content-head full-diff architecture self-review
    result: PASS
    evidence: cc03e597da453c1725a69600ce7f39e1e0433eda satisfies semantic acceptance with zero material findings
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task with no executable provider, entitlement, Wallet or game-delivery behavior
blockers: []
next_action: Mark PR #925 ready, observe repository-selected exact-head CI on the checkpoint release candidate, and merge only if all required checks and review hygiene pass.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-20260808T1948+0200-issue-924
  session_started_at: 2026-08-08T20:19:00+02:00
  checkpointed_at: 2026-08-08T20:27:00+02:00
  last_progress_at: 2026-08-08T20:27:00+02:00
  phase: validate
  exact_head: cc03e597da453c1725a69600ce7f39e1e0433eda
  pull_request: 925
  active_operation: final exact-head CI
  external_run_ids: []
  operation_started_at: 2026-08-08T20:27:00+02:00
  wait_deadline_at: 2026-08-08T20:42:00+02:00
  check_generation: ready
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: repository-selected PR checks exist for the final checkpoint head
  next_action: Observe one aggregate exact-head workflow snapshot and merge only after required checks plus review hygiene pass.
```

## Notes

No provider/payment implementation, real charge, wallet mutation, entitlement database, Oteryn-v2/Canary write, deployment or production activation is authorized.