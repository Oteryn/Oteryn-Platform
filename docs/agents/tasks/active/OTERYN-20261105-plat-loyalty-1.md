---
task_id: OTERYN-20261105-plat-loyalty-1
governing_issue: 325
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261105-plat-loyalty-1

## Goal

Governing GitHub Issue: Oteryn/Oteryn-Platform#325 — canonical lifecycle authority for this task (owner task PLAT-LOYALTY-1).

Add the default-off, mTLS-only Platform producer read for an account's consumed-Premium-day loyalty projection. This task performs no enablement or deployment, and the Game consumer remains separate.

## Acceptance criteria

- [x] Return zero for a known account without Premium while advancing the shared revision.
- [x] Count only whole consumed Premium days across expiry, extension, new intervals, and revocation.
- [x] Share the Premium authority revision and preserve per-account ordering.
- [x] Fail closed for malformed requests, wrong peers, disabled or conflicting settings, unknown accounts, and rate limits.
- [x] Document the default-off producer contract and cover its route with focused feature tests.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md
  - app/ProductsEntitlements/Loyalty/**
  - app/ProductsEntitlements/Http/GuardLoyaltySnapshotPeer.php
  - app/ProductsEntitlements/Http/LoyaltySnapshotController.php
  - config/products-entitlements.php
  - routes/internal.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - tests/Feature/ProductsEntitlements/LoyaltySnapshotEndpointTest.php
  - docs/agents/tasks/active/OTERYN-20261105-plat-loyalty-1.md
modules:
  - products-entitlements
dependencies:
  - existing Premium time ledger and authority
blockers:
  - none
cross_repository_tasks:
  - separate Game consumer task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-04T00:00:00Z
head: a0cc62d
branch: claude/happy-davinci-ijz3wy-plat-loyalty-1-v2
pr: 1675
status: ready
context_routes:
  - owner-provided PLAT-LOYALTY-1 contract
owned_paths:
  - app/ProductsEntitlements/Loyalty/**
  - loyalty route, guard, config, cache policy, contract, and feature test
proven:
  - PremiumTimeAuthority is the shared per-account revision source
  - producer remains default-off with no enablement or deployment
derived:
  - lifecycle events reconstruct distinct consumed intervals without a migration
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - owned loyalty producer paths only
validation:
  - command: php -l on every changed PHP file
    result: PASS
    evidence: every file reported no syntax errors
  - command: git diff --cached --check
    result: PASS
    evidence: no whitespace errors
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261105-plat-loyalty-1.md --require-checkpoint
    result: PASS
    evidence: checkpoint validated against contract v1
  - command: focused Laravel feature test
    result: NOT_RUN
    evidence: owner prohibited php artisan and database execution; left for CI
blockers:
  - none
next_action: review and integrate the prepared pull request through CI
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository pull request branch
source_branch_evidence: pending merge
```

## Notes

Default-off. No environment enablement, deployment, migration, or Game consumer change is part of this task.
