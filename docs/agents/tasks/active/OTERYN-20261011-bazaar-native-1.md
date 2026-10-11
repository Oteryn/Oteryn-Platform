---
task_id: OTERYN-20261011-bazaar-native-1
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/CharacterAuthority/CharacterCommandClient.php
  - app/Marketplace/Actions/ReconcileCharacterAuctions.php
optional_reads:
  - app/Marketplace/Exceptions/MarketplaceException.php
---

# OTERYN-20261011 Bazaar native ownership-transfer step

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet BAZAAR-NATIVE-1, the first slice of making the Bazaar work on native Character Authority instead of staying frozen.

The durable, value-sensitive core of the native Bazaar saga: a ledger that binds each saga step attempt (auction, purpose, attempt) to exactly one `TransferCharacterOwnership` operation from the C1 command client, and a step service that requests, observes and retries those operations under the contract's ordering rules. Nothing calls the service yet; the saga switch, the native escrow account and enablement are later packets, so no existing flow changes and the Bazaar stays frozen.

## Acceptance criteria

- [x] Migration `character_auction_transfers`: one row per step attempt, unique per (auction, purpose, attempt) and per operation id; rollback is refused while rows exist.
- [x] A step is requested with the deterministic idempotency key `bazaar:{auction}:{purpose}:{attempt}` in the same transaction as its ledger row; repeating the step returns the same operation and never issues a second one; a concurrent duplicate re-reads the recorded row.
- [x] The outcome is only Game's stored receipt: PENDING until it exists, including past `expires_at_source`; a COMMITTED receipt must match the recorded Character and accounts (`transfer_receipt_conflict` otherwise).
- [x] Ordering: listing escrow is first; settlement or escrow return needs a COMMITTED escrow; settlement and escrow return exclude each other unless the other's latest attempt is REJECTED (checked under the escrow row lock).
- [x] Rejections map to saga failure codes per Game contract §7; only IN_SESSION, INTENT_EXPIRED, HOUSING_NOT_PREPARED and ACCOUNT_CHARACTER_LIMIT allow a new attempt (bounded to 8); unknown or defect codes are `transfer_contract_defect` and never retried.
- [x] Disabled or saturated commands fail closed (`dependency_unavailable`) without a ledger row; a step's binding never changes (`transfer_binding_conflict`).
- [ ] Exact-head CI passes (PHP tests, PHPStan and Pint were not runnable in the authoring sandbox: the vendor tree is unavailable).

## Ownership

```yaml
owned_paths:
  - app/Marketplace/Native/**
  - database/migrations/2026_10_11_180000_create_character_auction_transfers.php
  - tests/Feature/Marketplace/BazaarOwnershipTransfersTest.php
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-native-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-bazaar-native-1.md
modules:
  - marketplace
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T18:00:00Z
head: UNKNOWN
branch: claude/bazaar-1-20261011
pr: 1541
status: validating
context_routes:
  - marketplace
owned_paths:
  - app/Marketplace/Native/**
  - database/migrations/2026_10_11_180000_create_character_auction_transfers.php
  - tests/Feature/Marketplace/BazaarOwnershipTransfersTest.php
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-native-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-bazaar-native-1.md
proven:
  - The C1 client issues one operation per idempotency key and keeps an expired intent PENDING until Game's receipt, so a deterministic per-attempt key makes saga replays safe.
  - Platform contract §7 requires ownership transfer before wallet settlement and no settlement on an ambiguous or rejected transfer; the step exposes COMMITTED only on a matching receipt.
derived:
  - The ledger has no foreign key to `character_auctions` yet; that table still requires legacy engine columns, so the saga-switch packet adds the constraint with native auction writes.
  - Settlement and escrow return serialize on the escrow ledger row lock, so a Character can never be sent to both buyer and seller.
unknown:
  - PHP suite, PHPStan and Pint results; composer cannot install here, so CI is the first run.
  - Concurrency is proven only by design on MariaDB; the suite runs on SQLite, where row locks are no-ops.
  - The Game contract is still Candidate; wire or result-code changes on acceptance may need a follow-up.
  - Follow-ups: BAZAAR-NATIVE-2 (saga switch, native escrow AccountId, foreign key, wallet settlement on COMMITTED settlement only), BAZAAR-NATIVE-3 (eligibility, snapshot, UI, admin recovery), command route enablement, Synology native enable path, unfreeze.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reusing the synchronous transfer gateway shape; the command flow is asynchronous and a timeout proves nothing.
  - Storing the outcome on the ledger row; the stored Game receipt is the single source of truth.
changed_paths:
  - app/Marketplace/Native/BazaarOwnershipTransfers.php
  - app/Marketplace/Native/BazaarTransferStep.php
  - database/migrations/2026_10_11_180000_create_character_auction_transfers.php
  - tests/Feature/Marketplace/BazaarOwnershipTransfersTest.php
  - docs/agents/tasks/active/OTERYN-20261011-bazaar-native-1.md
validation:
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline; new files add no legacy token
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-bazaar-native-1.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php artisan test tests/Feature/Marketplace/BazaarOwnershipTransfersTest.php
    result: NOT_RUN
    evidence: vendor unavailable in the authoring sandbox; runs on the exact head in CI
  - command: composer analyse and pint --test
    result: NOT_RUN
    evidence: vendor unavailable in the authoring sandbox; runs on the exact head in CI
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
