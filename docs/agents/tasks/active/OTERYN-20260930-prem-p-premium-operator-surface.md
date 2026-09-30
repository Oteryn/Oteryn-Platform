---
task_id: OTERYN-20260930-prem-p-premium-operator-surface
governing_issue: 1431
required_reads:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
search_first:
  - app/Marketplace/Http/AdminMarketplaceController.php audited operator mutation surface
  - scripts/acceptance/coverage/surfaces/marketplace.json coverage and dimension registration
optional_reads: []
---

# OTERYN-20260930-prem-p-premium-operator-surface

## Goal

Governing GitHub Issue: #1431 (https://github.com/Oteryn/Oteryn-Platform/issues/1431). It is the canonical lifecycle authority. Coordination id: `OTV2-PREMIUM-DELIVERY`.

This task adds the operator web surface for `oteryn.premium_time`, contract section 2.5. It is stacked on the producer PR #1433:

- a new exact permission `products.premium.manage`, granted to `platform_admin` only;
- `/admin/premium`, guarded by `auth`, `mfa.confirmed` and `admin.permission:products.premium.manage`, which looks up an account and grants days or revokes through `PremiumTimeLedger`;
- a navigation entry, a feature test, a Playwright acceptance test, and coverage and dimension registration.

There is no payment and no production configuration.

## Acceptance criteria

- [x] Guest, no-MFA and wrong-permission actors are refused, and the refusal leaves state unchanged.
- [x] Grant, extend and revoke work through the page. An exact form retry is idempotent, a changed reuse conflicts, and each change is audited once.
- [x] Validation and domain refusals are shown and leave state unchanged.
- [x] The navigation link is shown only with the exact permission.
- [ ] Exact-head CI passes, including the Playwright evidence.

## Ownership

```yaml
owned_paths:
  - app/Admin/AdminPermission.php
  - app/ProductsEntitlements/Http/AdminPremiumTimeController.php
  - database/migrations/2026_09_30_230000_add_premium_time_permission.php
  - routes/modules/products-entitlements.php
  - resources/views/admin/premium/index.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - tests/Feature/ProductsEntitlements/AdminPremiumTimeTest.php
  - scripts/acceptance/seed-premium.php
  - scripts/acceptance/tests/premium-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-operator-surface.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-operator-surface.md
modules:
  - ProductsEntitlements
  - Admin (permission catalogue and navigation entry only)
dependencies:
  - PR 1433 producer (stacked base)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T19:40:00Z
head: pending
branch: claude/prem-p-premium-operator-surface
pr: none
status: implementing
context_routes:
  - security
  - frontend
owned_paths:
  - app/Admin/AdminPermission.php
  - app/ProductsEntitlements/Http/AdminPremiumTimeController.php
  - database/migrations/2026_09_30_230000_add_premium_time_permission.php
  - routes/modules/products-entitlements.php
  - resources/views/admin/premium/index.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - tests/Feature/ProductsEntitlements/AdminPremiumTimeTest.php
  - scripts/acceptance/seed-premium.php
  - scripts/acceptance/tests/premium-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-operator-surface.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-operator-surface.md
proven:
  - AdminAuthorization denies any permission key missing from AdminPermission::all(), so the key is registered there.
  - Route module files under routes/modules are loaded by glob from routes/web.php.
derived:
  - The shared acceptance browser administrator keeps its grants; a separate acceptance-only role adds products.premium.manage so the denied state is observable first.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Adding the permission to seed-browser-admin.php was rejected because it would change every visual-review admin session.
changed_paths:
  - app/Admin/AdminPermission.php
  - app/ProductsEntitlements/Http/AdminPremiumTimeController.php
  - database/migrations/2026_09_30_230000_add_premium_time_permission.php
  - routes/modules/products-entitlements.php
  - resources/views/admin/premium/index.blade.php
  - resources/views/admin/partials/navigation.blade.php
  - lang/en/portal_art.php
  - lang/pl/portal_art.php
  - tests/Feature/ProductsEntitlements/AdminPremiumTimeTest.php
  - scripts/acceptance/seed-premium.php
  - scripts/acceptance/tests/premium-admin-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/products-entitlements.json
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
validation:
  - command: php -l and node --check on changed files
    result: PASS
    evidence: local
  - command: node validate-portal-coverage.mjs --strict --manifest-only; validate-portal-evidence-dimensions.mjs; validate-critical-viewport-evidence.mjs
    result: PASS
    evidence: local, errors []
  - command: phpunit / pint / phpstan / Playwright locally
    result: BLOCKED
    evidence: session egress policy denies third-party Composer package downloads; exact-head CI is the validation of record
blockers:
  - none
next_action: open the stacked PR and drive exact-head CI to green
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
