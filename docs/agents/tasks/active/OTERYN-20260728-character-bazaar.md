---
task_id: OTERYN-20260728-character-bazaar
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
search_first:
  - docs/agents/tasks/active/** for marketplace, character, account, wallet or payment ownership
  - open pull requests for overlapping application, database, route, view and acceptance paths
  - existing account binding, character creation, public game-data, audit and RBAC implementations
optional_reads:
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
---

# OTERYN-20260728-character-bazaar

## Goal

Deliver a secure, responsive Character Bazaar that lets authenticated users escrow eligible owned characters, list timed auctions, watch and bid with Platform-owned Oteryn Coins, and settle ownership to the winning bound Canary account through an explicit recoverable transfer contract.

## Acceptance criteria

- [ ] Public current-auction catalogue supports bounded filters, deterministic sorting, pagination and explicit empty/unavailable states.
- [ ] Public auction detail presents an immutable listing snapshot, current price, buy-now state, remaining time and bounded bid history without private account data.
- [ ] Authenticated users can manage watchlist, bids, owned auctions and completed/cancelled history.
- [ ] A seller can list only an active, offline character owned through the ready immutable Identity-to-Canary binding and not already controlled by another marketplace operation.
- [ ] Listing moves the character into a configured non-login Canary escrow account through an operation-specific least-privilege adapter and recoverable saga.
- [ ] Direct ascending bids reserve Oteryn Coins transactionally, release an outbid reservation deterministically, reject self-bids and enforce minimum increments.
- [ ] Optional buy-now performs the same authorization, reservation and settlement invariants without duplicate settlement.
- [ ] Auction close transfers the escrowed character to the winner, settles seller proceeds and commission exactly once, or remains recoverable without inventing success.
- [ ] Cancellation returns the escrowed character to the seller and releases every reservation exactly once under documented allowed states.
- [ ] Wallet balances use a Platform-owned append-oriented ledger plus transactionally maintained available/reserved balances; the unresolved Canary tournament-coin field is never used.
- [ ] Administrator wallet adjustments and marketplace recovery actions require auth, confirmed MFA, exact permission and bounded audit metadata.
- [ ] UI is English/Polish, Oteryn-native rather than a visual clone, responsive on desktop/tablet/mobile and keyboard accessible.
- [ ] Focused unit, feature, authorization, real-database concurrency/locking, contract and browser acceptance coverage exists for sensitive paths.
- [ ] ADR, module catalog, data ownership, Canary transfer contract, deployment prerequisites and reconciliation runbook remain current.
- [ ] Exact final head passes required repository CI and relevant marketplace acceptance checks before merge.

## Ownership

```yaml
owned_paths:
  - app/Marketplace/**
  - app/Wallet/**
  - app/Console/Commands/Marketplace/**
  - database/migrations/*character_auction*
  - database/migrations/*wallet*
  - database/factories/Marketplace/**
  - database/factories/Wallet/**
  - routes/modules/marketplace.php
  - resources/views/marketplace/**
  - resources/views/admin/marketplace/**
  - resources/navigation/public/marketplace.php
  - resources/css/**
  - lang/en/marketplace.php
  - lang/pl/marketplace.php
  - config/marketplace.php
  - config/database.php
  - .env.example
  - tests/Unit/Marketplace/**
  - tests/Feature/Marketplace/**
  - tests/Integration/Marketplace/**
  - scripts/acceptance/**marketplace**
  - .github/workflows/*marketplace*
  - docs/architecture/adr/*character-bazaar*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/contracts/*CHARACTER*TRANSFER*
  - docs/operations/*MARKETPLACE*
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-character-bazaar.md
modules:
  - Marketplace
  - Wallet
  - Accounts
  - Characters
  - Integration
  - Admin
  - Audit
dependencies:
  - ready immutable IdentityCanaryAccount binding
  - Canary players/account/cluster-session schema evidence
  - existing Admin RBAC and audit recorder
blockers:
  - none
cross_repository_tasks:
  - blakinio/canary is evidence-only; no write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T06:51:00Z
head: 583cae5f430998b2bbdf5e60b59d93f09ec6f4c8
branch: feat/OTERYN-20260728-character-bazaar
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - accounts-characters
  - canary-integration
  - database
  - security
  - payments
  - web-cms
  - admin-rbac
  - testing
owned_paths:
  - app/Marketplace/**
  - app/Wallet/**
  - database/migrations/*character_auction*
  - database/migrations/*wallet*
  - routes/modules/marketplace.php
  - resources/views/marketplace/**
  - resources/views/admin/marketplace/**
  - config/marketplace.php
  - tests/**/Marketplace/**
  - docs/architecture/adr/*character-bazaar*
  - docs/contracts/*CHARACTER*TRANSFER*
  - docs/operations/*MARKETPLACE*
  - docs/agents/tasks/active/OTERYN-20260728-character-bazaar.md
proven:
  - Oteryn Platform owns web UI, Platform persistence, Identity binding, RBAC and audit.
  - Generic Canary access is read-only and only account provisioning plus character creation are currently approved shared writes.
  - players.account_id identifies character ownership and active characters use deletion = 0.
  - fresh cluster_sessions rows with status ONLINE and future expires_at are the implemented authoritative online-character read boundary.
  - Canary tournament-coin schema/code naming is unresolved and cannot be used.
  - No active task or open PR overlaps Character Bazaar intent.
derived:
  - A safe sale requires character escrow before bidding so the seller cannot continue modifying the advertised character.
  - Cross-database settlement requires an explicit recoverable saga and deterministic reconciliation rather than pretending to be atomically committed.
  - Oteryn Coins must be Platform-owned append-oriented financial data and must not imply a payment-provider integration.
unknown:
  - Exact Canary runtime/cache side effects, row-lock order and safe account_id transfer evidence for listing, cancellation and settlement.
  - Deployment-owned creation and protection of the non-login marketplace escrow account.
  - Final administrator funding workflow and launch balance policy.
conflicts: []
first_failure:
  marker: sandbox-network-unavailable
  evidence: direct git clone could not resolve github.com; repository work must use the GitHub connector and CI evidence
rejected_hypotheses:
  - Reuse Canary tournament coins; rejected because the contract records an unresolved field-name conflict and no approved coin write boundary.
  - Leave a listed character on the seller account; rejected because the seller could log in and materially change the auctioned asset.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-character-bazaar.md
validation:
  - command: targeted governance preflight and overlap search through GitHub connector
    result: PASS
    evidence: active-work index reports none; no open marketplace/auction PR or issue overlap was found before issue 269
blockers:
  - none
next_action: Verify exact Canary player-transfer and online-state behavior, then record the accepted escrow/settlement ADR and operation-specific contract.
```

## Notes

Product references supplied by the owner are interaction references only. The Oteryn implementation must use the existing responsive shell, localization and accessibility conventions rather than copying third-party visual assets or markup.
