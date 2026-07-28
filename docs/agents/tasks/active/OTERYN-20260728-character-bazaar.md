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

- [x] Public current-auction catalogue supports bounded filters, deterministic sorting, pagination and explicit empty/unavailable states.
- [x] Public auction detail presents an immutable listing snapshot, current price, buy-now state, remaining time and bounded bid history without private account data.
- [x] Authenticated users can manage watchlist, bids, owned auctions and completed/cancelled history.
- [x] A seller can list only an active, offline character owned through the ready immutable Identity-to-Canary binding and not already controlled by another marketplace operation.
- [x] Listing moves the character into a configured non-login Canary escrow account through an operation-specific least-privilege adapter and recoverable saga.
- [x] Direct ascending bids reserve Oteryn Coins transactionally, release an outbid reservation deterministically, reject self-bids and enforce minimum increments.
- [x] Optional buy-now performs the same authorization, reservation and settlement invariants without duplicate settlement.
- [x] Auction close transfers the escrowed character to the winner, settles seller proceeds and commission exactly once, or remains recoverable without inventing success.
- [x] Cancellation returns the escrowed character to the seller and releases every reservation exactly once under documented allowed states.
- [x] Wallet balances use a Platform-owned append-oriented ledger plus transactionally maintained available/reserved balances; the unresolved Canary tournament-coin field is never used.
- [x] Administrator wallet adjustments and marketplace recovery actions require auth, confirmed MFA, exact permission and bounded audit metadata.
- [x] UI is English/Polish, Oteryn-native rather than a visual clone, responsive on desktop/tablet/mobile and keyboard accessible.
- [x] Focused unit, feature, authorization, real-database concurrency/locking, contract and browser acceptance coverage exists for sensitive paths.
- [x] ADR, module catalog, data ownership, Canary transfer contract, deployment prerequisites and reconciliation runbook remain current.
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
  - final exact-head checks are still running
cross_repository_tasks:
  - blakinio/canary is evidence-only; no write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T11:48:00Z
head: fbba2cfb5f4155cc3430ce9fa34c251b868c7514
branch: feat/OTERYN-20260728-character-bazaar
pr: 270
status: validating
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
  - app/CanaryIntegration/CanaryCharacterTransfer.php
  - app/CanaryIntegration/CanaryCharacterTransferDatabasePrivilegeVerifier.php
  - database/migrations/*character_auction*
  - database/migrations/*wallet*
  - database/provisioning/canary-character-transfer.sql.template
  - routes/modules/marketplace.php
  - resources/views/marketplace/**
  - resources/views/admin/marketplace/**
  - public/css/marketplace*.css
  - tests/**/Marketplace/**
  - scripts/acceptance/**marketplace**
  - docs/architecture/adr/0016-character-bazaar-wallet-and-escrow.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/operations/MARKETPLACE_OPERATIONS.md
  - docs/agents/tasks/active/OTERYN-20260728-character-bazaar.md
proven:
  - Oteryn Platform owns the Character Bazaar UI, Platform wallet ledger, identity binding, RBAC, audit and recoverable marketplace saga state.
  - Canary players.account_id is changed only through the dedicated canary_character_transfer connection with operation-specific least privilege and deterministic lock order.
  - Listing transfers eligible offline characters to a configured non-login escrow account before activation; settlement and cancellation reconcile actual ownership and remain recoverable.
  - Oteryn Coins reservations, outbid release, seller proceeds, commission and administrator adjustments are append-oriented and idempotent.
  - Public, authenticated and administrator browser journeys are covered in English/Polish with desktop/tablet/mobile, accessibility and authorization assertions.
  - CI, static analysis, focused MariaDB privilege/concurrency tests, Phase 7, outage, edge, Synology and portal-contract gates passed on the preceding implementation heads.
derived:
  - The branch is functionally complete; only the exact-final-head validation and governance merge/archive lifecycle remain.
unknown:
  - Real production behavior until separately authorized production verification is executed.
conflicts: []
first_failure:
  marker: marketplace-mobile-overflow
  evidence: full browser acceptance exposed document-width overflow in account and administrator tables at 390px; the final implementation contains table containment and stacked-account-table fixes
rejected_hypotheses:
  - Reuse Canary tournament coins; rejected because the field contract remains unresolved and no approved write boundary exists.
  - Leave listed characters on seller accounts; rejected because the asset could be modified during an active auction.
  - Treat cross-database settlement as atomic; rejected in favor of an explicit recoverable saga with actual-owner reconciliation.
changed_paths:
  - app/Marketplace/**
  - app/Wallet/**
  - app/CanaryIntegration/CanaryCharacterTransfer.php
  - app/CanaryIntegration/CanaryCharacterTransferDatabasePrivilegeVerifier.php
  - database/migrations/2026_07_28_070000_create_wallet_tables.php
  - database/migrations/2026_07_28_070100_create_character_bazaar_tables.php
  - database/migrations/2026_07_28_070200_add_marketplace_permission.php
  - database/provisioning/canary-character-transfer.sql.template
  - routes/modules/marketplace.php
  - resources/views/marketplace/**
  - resources/views/admin/marketplace/**
  - public/css/marketplace.css
  - public/css/marketplace-responsive.css
  - scripts/acceptance/tests/marketplace-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/marketplace.json
  - tests/Feature/Marketplace/**
  - tests/Unit/CanaryIntegration/CanaryCharacterTransferDatabasePrivilegeVerifierTest.php
  - docs/architecture/adr/0016-character-bazaar-wallet-and-escrow.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/operations/MARKETPLACE_OPERATIONS.md
validation:
  - command: CI run 30352387523
    result: PASS
    evidence: formatting, PHPStan and full PHPUnit suite succeeded on implementation head 4ab78864b336d76aa6d9b3e4b8135fbf8a4a7365
  - command: Acceptance E2E and Visual UX run 30352387358
    result: PASS
    evidence: critical smoke, portability, responsive, resilience and accessibility profiles succeeded on implementation head 4ab78864b336d76aa6d9b3e4b8135fbf8a4a7365
  - command: Portal Acceptance Contract run 30352387498
    result: PASS
    evidence: strict delivered-surface ledger and account lifecycle succeeded
  - command: exact final head workflows
    result: NOT_RUN
    evidence: the current exact head includes the final responsive containment and checkpoint correction and is awaiting a complete workflow result set
blockers:
  - exact final head checks must all pass before PR readiness and merge
next_action: Wait for every exact-head workflow on the current PR head, remediate any failure, then mark PR #270 ready and merge only after all checks pass.
```

## Notes

Product references supplied by the owner are interaction references only. The Oteryn implementation uses the existing responsive shell, localization and accessibility conventions rather than copying third-party visual assets or markup. Repository and isolated acceptance evidence does not establish production correctness.
