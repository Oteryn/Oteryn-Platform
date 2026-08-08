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

- [x] Platform-only, game-consumed account entitlement, durable gameplay grant, character-service entitlement and Oteryn Coin package delivery profiles are classified.
- [x] Voucher/redeem is classified on a separate entitlement-issuance-source axis rather than as a contradictory second delivery profile.
- [x] Payment/order, Platform entitlement and game-delivery truth remain separate.
- [x] Canonical identity, stable delivery operation identity, idempotency, typed outcome and reconciliation semantics are explicit.
- [x] Single-use character-service reserve -> authoritative operation -> consume ordering is explicit.
- [x] Premium/VIP expiry/revocation semantics preserve Platform entitlement authority and game enforcement boundaries without inventing forced-session behavior.
- [x] Refund/chargeback/revocation policy distinguishes reversible delivery, compensation, deny-future-use and manual reconciliation.
- [x] Character-service mutations reuse the accepted Character Authority command contract.
- [x] Oteryn Coin packages route through Platform Wallet mutators, not game persistence.
- [x] Canary numeric IDs/direct SQL remain Legacy Canary Compatibility only.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass on the post-review-repair final head.
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

## Exact-content self-review after review repair

```yaml
self_review:
  result: PASS
  exact_head: 6a1153216d89433db7fd68c6d673c8d80124361d
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
    - Delivery profiles A-E are mutually clear; voucher/redeem is a separate issuance-source axis, removing the review contradiction.
    - Issuance source answers why an entitlement exists; delivery profile answers how value is fulfilled/enforced.
    - Character-service ordering reserves entitlement before game submission and consumes only after terminal authoritative Character Authority completion.
    - Premium/VIP expiry/revocation preserves revision ordering while leaving in-session disconnect/grace policy explicitly deferred.
    - Refund/chargeback handling requires reversible, compensating, deny-future-use or manual policy and forbids silent destructive game/Wallet edits.
    - Stable delivery identity/idempotency/reconciliation prevents duplicate gameplay grants after timeout/replay.
    - Oteryn Coins stay on approved Platform Wallet mutator/ledger authority.
    - Canary direct SQL/numeric IDs remain Legacy Canary Compatibility only.
    - No runtime/schema/workflow/provider/wallet/deployment/production/external-repository path changed.
```

The next checkpoint-only commit establishes the final validation head without changing the reviewed architecture semantics.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T20:31:00+02:00
head: 6a1153216d89433db7fd68c6d673c8d80124361d
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
  - Initial final-candidate head eae164c4391be2fb507bd42bd2849eef51ce495e reached an all-green exact-head workflow generation but review then found one P2 semantic contradiction, so it was not merged.
  - PR #925 P2 found that voucher/redeem could not simultaneously be Profile F and still require an A-E delivery profile.
  - Contract repair 04f8190f85ce4469b0504b20f5ca0c9f3ad5fa9d moves voucher/redeem to a separate issuance-source/provenance axis and keeps exactly one delivery profile A-E per fulfilment unit.
  - Report repair 6a1153216d89433db7fd68c6d673c8d80124361d records the finding and corrected architecture.
  - Character-service mutation reuses the accepted native Character Authority command/result contract.
derived:
  - #322 can model paid, voucher, admin or migration issuance independently from fulfilment profile and therefore reuse one delivery/reconciliation policy per product version.
unknown:
  - exact Oteryn-v2 entitlement/grant transport and enforcement storage
  - exact product catalogue/entitlement persistence schema and worker design
  - exact premium/VIP in-session expiry/grace policy
  - exact reversible/compensating policy for each future durable gameplay grant
  - selected real payment provider and legal/tax product policy
conflicts: []
first_failure:
  marker: pr-review-voucher-profile-contradiction
  evidence: PR #925 P2 review found voucher/redeem incorrectly modeled as Profile F while also requiring A-E; repaired by separate issuance-source axis
rejected_hypotheses:
  - payment success can directly prove gameplay delivery
  - active entitlement means game effect is already applied
  - voucher/redeem should be a sixth primary delivery profile
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
  - command: initial exact-head workflow generation eae164c4391be2fb507bd42bd2849eef51ce495e
    result: PASS
    evidence: all eight selected workflows passed, but this generation is invalid as final merge evidence because later P2 review required semantic repair
  - command: post-review exact content-head full-diff architecture self-review
    result: PASS
    evidence: 6a1153216d89433db7fd68c6d673c8d80124361d resolves voucher contradiction with zero remaining material findings in the three-path semantic diff
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task with no executable provider, entitlement, Wallet or game-delivery behavior
blockers: []
next_action: Resolve the repaired P2 review thread, observe fresh exact-head CI on the checkpoint release candidate, and merge only after all required checks plus review hygiene pass.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260808T1948+0200-issue-924
  session_started_at: 2026-08-08T20:19:00+02:00
  checkpointed_at: 2026-08-08T20:31:00+02:00
  last_progress_at: 2026-08-08T20:31:00+02:00
  phase: validate
  exact_head: 6a1153216d89433db7fd68c6d673c8d80124361d
  pull_request: 925
  active_operation: fresh exact-head CI after review repair
  external_run_ids: []
  operation_started_at: 2026-08-08T20:31:00+02:00
  wait_deadline_at: 2026-08-08T20:46:00+02:00
  check_generation: post-review-repair-ready
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: repository-selected PR checks exist for the post-review-repair checkpoint head
  next_action: Observe one aggregate workflow snapshot and merge only after exact-head checks and review hygiene pass.
```

## Notes

No provider/payment implementation, real charge, wallet mutation, entitlement database, Oteryn-v2/Canary write, deployment or production activation is authorized.