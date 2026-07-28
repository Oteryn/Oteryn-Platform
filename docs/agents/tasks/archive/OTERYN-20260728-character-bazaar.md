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
- [x] Exact final head passes required repository CI and relevant marketplace acceptance checks before merge.

## Final ownership

```yaml
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
  - docs/agents/tasks/archive/OTERYN-20260728-character-bazaar.md
  - docs/agents/ACTIVE_WORK.md
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
  - blakinio/canary remained evidence-only; no write was performed
```

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T12:05:00Z
head: eabdd0ecbd617678c0a338e1c68229c638e4af06
branch: feat/OTERYN-20260728-character-bazaar
pr: 270
merge_sha: 0f19656e0875d0a10b22002ac0e096deb20e94d8
status: completed
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
proven:
  - PR #270 merged the complete Character Bazaar implementation as squash merge 0f19656e0875d0a10b22002ac0e096deb20e94d8 and automatically closed Issue #269.
  - Exact feature head eabdd0ecbd617678c0a338e1c68229c638e4af06 passed CI run 30356211862 and Agent Governance run 30356211977.
  - Acceptance E2E and Visual UX run 30356211988 passed smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, resilience and accessibility profiles.
  - Marketplace Acceptance run 30356211843 passed the exact-head full portal acceptance profile, including public, account and administrator marketplace journeys.
  - Portal Acceptance Contract run 30356211854 passed the strict delivered-surface ledger and complete account lifecycle.
  - Phase 7 run 30356212021, Platform DB Outage run 30356211796, Edge Security run 30356211993, Game Auth concurrency run 30356211869, Synology preflight run 30356211886 and image build run 30356211985 all passed on the same exact head.
  - Character ownership transfer is isolated behind a dedicated least-privilege Canary connection with explicit escrow, offline/session checks, deterministic locking, idempotency and actual-owner recovery.
  - Platform-owned wallet balances use an append-oriented ledger with transactional available/reserved balances; Canary tournament coins and payment-provider processing are not used.
derived:
  - The repository and isolated staging-like evidence is sufficient to close the implementation task and remove its temporary validation workflow.
unknown:
  - Real production behavior until the separately owned production verification in Issue #91 is explicitly authorized and executed.
conflicts: []
first_failure:
  marker: marketplace-mobile-overflow
  evidence: full browser acceptance initially found 390px document overflow in account and administrator tables; responsive containment was added and both critical and full exact-head acceptance then passed
rejected_hypotheses:
  - Reuse Canary tournament coins; rejected because the field contract remains unresolved and no approved write boundary exists.
  - Leave listed characters on seller accounts; rejected because the auctioned asset could be modified after listing.
  - Treat cross-database settlement as atomic; rejected in favor of a recoverable saga and actual-owner reconciliation.
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
  - docs/agents/tasks/archive/OTERYN-20260728-character-bazaar.md
  - docs/agents/ACTIVE_WORK.md
validation:
  - command: exact-head required workflow set on eabdd0ecbd617678c0a338e1c68229c638e4af06
    result: PASS
    evidence: all eleven required PR workflows completed successfully before merge
  - command: PR #270 squash merge
    result: PASS
    evidence: GitHub merged PR #270 as 0f19656e0875d0a10b22002ac0e096deb20e94d8 and closed Issue #269 as completed
blockers:
  - none
next_action: Keep production verification and go-live claims isolated to the separately authorized Issue #91 workflow.
```

## Boundary

This archive closes repository implementation and isolated staging-like acceptance only. It does not claim that the merge is deployed to production and does not establish `PRODUCTION_PROVEN` status.
