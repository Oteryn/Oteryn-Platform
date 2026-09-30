---
task_id: OTERYN-20260930-prem-p-premium-snapshot-producer
governing_issue: 1431
required_reads:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
search_first:
  - app/Http/Middleware/GameAuth mTLS peer middleware and NativeRuntimeStatusSettings purpose separation
  - app/Wallet/Actions/AdjustWalletBalance.php idempotent audited operator mutation
optional_reads: []
---

# OTERYN-20260930-prem-p-premium-snapshot-producer

## Goal

Governing GitHub Issue: #1431 (https://github.com/Oteryn/Oteryn-Platform/issues/1431). It is the canonical lifecycle authority for this task. Coordination id: `OTV2-PREMIUM-DELIVERY`.

This packet covers the #1431 Premium producer implementation, stacked on the contract candidate PR #1432:

- the additive `oteryn.premium_time` v1 schema;
- the `PremiumTimeLedger` grant/revoke service, with interval merging, per-account locking, request-id idempotency and admin audit;
- the private mTLS read `POST /internal/v1/products-entitlements/premium-snapshots/read`, which is disabled by default;
- producer contract tests that validate real responses against the shared fixtures `docs/contracts/fixtures/premium-snapshot-v1/`.

The operator web surface (the exact permission, confirmed-MFA admin page and browser evidence) is the next #1431 item, in its own packet and PR.

## Acceptance criteria

- [x] Grants merge into one interval, revocation ends it, and a re-grant raises the lifecycle revision. The horizon and duration bounds are enforced.
- [x] An exact retry is idempotent, and changed reuse of a request id conflicts. Each mutation writes one audit event without the reason in plaintext.
- [x] The snapshot wire is exact: at most 1,024 bytes, nonce echoed, lease/refresh arithmetic per contract, and `authority_revision` strictly increasing per account.
- [x] mTLS 401, disabled or misconfigured 503, malformed 400, unknown-account 404 without allocating a revision, and rate-bound 429 are all covered.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/ProductsEntitlements/**
  - config/products-entitlements.php
  - database/migrations/2026_09_30_220000_create_premium_time_entitlement_state.php
  - tests/Feature/ProductsEntitlements/**
  - routes/internal.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusSettings.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - .env.example
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-producer.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-snapshot-producer.md
modules:
  - ProductsEntitlements
  - GameAuth (purpose-separation and no-store lists only)
dependencies:
  - PR 1432 contract candidate (stacked base)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game PREMIUM-DELIVERY-0 consumer; no Game repository access
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T18:30:00Z
head: fa9b41f0454a806ecae4e4e1c2f0f67790d9e714
branch: claude/prem-p-premium-snapshot-producer
pr: 1433
status: validating
context_routes:
  - api
  - security
  - database
owned_paths:
  - app/ProductsEntitlements/**
  - config/products-entitlements.php
  - database/migrations/2026_09_30_220000_create_premium_time_entitlement_state.php
  - tests/Feature/ProductsEntitlements/**
  - routes/internal.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusSettings.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - .env.example
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-producer.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-snapshot-producer.md
proven:
  - The Laravel internal route group applies PreventSensitiveGameAuthResponseCaching globally by path.
  - The mTLS provenance fields SSL_CLIENT_VERIFY, SSL_PROTOCOL and SSL_CLIENT_S_DN match the existing Platform peer pattern.
derived:
  - Locking one authority row per account serializes grants and snapshot issuance, so authority_revision is ordered with lifecycle changes.
unknown:
  - Real MariaDB concurrency evidence for simultaneous grant and snapshot; only sqlite feature tests exist in this PR.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Storing interval times as TIMESTAMP was rejected because of the 2038 limit versus the 3660-day horizon; Unix seconds are used instead.
changed_paths:
  - app/ProductsEntitlements/**
  - config/products-entitlements.php
  - database/migrations/2026_09_30_220000_create_premium_time_entitlement_state.php
  - tests/Feature/ProductsEntitlements/**
  - routes/internal.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusSettings.php
  - tests/Unit/Http/Middleware/PreventSensitiveGameAuthResponseCachingTest.php
  - .env.example
validation:
  - command: php -l on every changed PHP file
    result: PASS
    evidence: local PHP 8.4 lint
  - command: composer install / phpunit / pint / phpstan locally
    result: BLOCKED
    evidence: session egress policy denies GitHub archive downloads for third-party Composer packages; exact-head CI is the validation of record
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: private service endpoint with no browser actor; the Game consumer E2E is a Game-side task using the shared fixtures
blockers:
  - none
next_action: drive exact-head CI on draft PR 1433 to green
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The snapshot endpoint is disabled unless `PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_ENABLED=true`, a dedicated mTLS client identity is configured and `PLATFORM_BUILD_REVISION` is set. No production configuration, certificate or secret is part of this task.
