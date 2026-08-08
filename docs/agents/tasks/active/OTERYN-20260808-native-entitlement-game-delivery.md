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

- [ ] Platform-only, game-consumed account entitlement, durable gameplay grant, character-service entitlement, Oteryn Coin package and voucher/redeem profiles are classified.
- [ ] Payment/order, Platform entitlement and game-delivery truth remain separate.
- [ ] Canonical identity, stable delivery operation identity, idempotency, typed outcome and reconciliation semantics are explicit.
- [ ] Single-use character-service reserve -> authoritative operation -> consume ordering is explicit.
- [ ] Premium/VIP expiry/revocation semantics preserve Platform entitlement authority and game enforcement boundaries without inventing forced-session behavior.
- [ ] Refund/chargeback/revocation policy distinguishes reversible delivery, compensation, deny-future-use and manual reconciliation.
- [ ] Character-service mutations reuse the accepted Character Authority command contract.
- [ ] Oteryn Coin packages route through Platform Wallet mutators, not game persistence.
- [ ] Canary numeric IDs/direct SQL remain Legacy Canary Compatibility only.
- [ ] Exact-head self-review, Agent Governance and repository-selected CI pass.
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
forbidden_paths:
  - app/**
  - database/**
  - .github/workflows/**
  - deploy/**
  - external repositories
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T20:22:00+02:00
head: 1e7086e3749ae8dfa5bdea31897f10dbee7e73b3
branch: docs/OTERYN-20260808-native-entitlement-game-delivery
pr: none
status: implementing
phase: design
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
  - Issue #924 is the focused P1 owner for this architecture boundary.
  - Issue #322 owns Products/Entitlements/Vouchers implementation and already requires payment truth to remain separate from entitlement delivery truth.
  - Issue #321 owns provider/payment truth and explicitly forbids browser return from granting value.
  - MODULE_CATALOG classifies ProductsEntitlements as Platform-owned catalogue/entitlement/fulfilment and forbids provider settlement, Wallet gameplay policy and undocumented Canary premium mutations.
  - ADR 0031 keeps gameplay/world mutation authority in Oteryn-v2 and Platform business workflows in Platform.
  - Character-service game mutations now have the accepted OTERY​N_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT on main.
derived:
  - Native entitlement architecture needs distinct commercial-entitlement and game-delivery states plus per-profile fulfilment ordering.
  - Character-service entitlements should reserve Platform entitlement capacity until the authoritative Character Authority operation reaches a terminal result.
unknown:
  - exact Oteryn-v2 entitlement/grant transport and enforcement storage
  - exact product catalogue and entitlement persistence schema
  - exact premium/VIP in-session expiry behavior
  - exact reversible/compensating policy for future durable gameplay grants
conflicts: []
first_failure:
  marker: none
  evidence: overlap preflight found no current owner for this focused native boundary
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-entitlement-game-delivery.md
validation:
  - command: overlap and authority preflight
    result: PASS
    evidence: Issue #924 is focused and no open entitlement/game-delivery PR or branch was found
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task
blockers: []
next_action: Draft the focused entitlement/game-delivery contract and architecture review, then perform exact-head full-diff self-review before ready-state CI.
```

## Notes

No provider/payment implementation, real charge, wallet mutation, entitlement database, Oteryn-v2/Canary write, deployment or production activation is authorized.