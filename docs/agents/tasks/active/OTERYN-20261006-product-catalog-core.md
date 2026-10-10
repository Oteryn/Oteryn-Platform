---
task_id: OTERYN-20261006-product-catalog-core
governing_issue: 322
required_reads:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
search_first:
  - app/ProductsEntitlements
  - database/migrations product catalog
  - open PRs overlapping ProductsEntitlements/Catalog
optional_reads:
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
---

# OTERYN-20261006 Product catalogue immutable version core

## Goal

Start the now-unblocked non-production implementation of Issue #322 with one bounded catalogue authority slice.

Deliver an additive immutable product/version registry carrying the accepted ProductsEntitlements delivery-profile axis, target scope, payment-boundary price/currency facts and EN/PL presentation. The slice stores no order, entitlement, voucher, Wallet delivery or service request and exposes no public purchase/catalogue route.

## Scope and invariants

- Product versions are immutable semantic snapshots identified by `(product_id, version)`.
- An exact retry is idempotent; changed reuse of one version is a conflict.
- Delivery profile is exactly A-E from the accepted entitlement/game-delivery contract.
- Price is a positive integer minor-unit amount and currency must already be accepted by the provider-neutral Payments boundary.
- EN and PL plain-text presentations are both required and are part of the immutable digest.
- Availability window metadata is immutable and does not itself publish/activate a product.
- No actual product is registered by migration or repository seed.
- No public/admin route, checkout, provider call, Wallet mutation, entitlement issuance, voucher redemption or Character/game mutation is added.
- Production commerce remains blocked by provider sandbox/production and LegalCommerce gates.

## Acceptance criteria

- [x] Additive reversible schema separates product identity, immutable versions and localized presentation.
- [x] Exact semantic retry is idempotent and changed version reuse fails closed.
- [x] Price/currency reuse the Payments configuration boundary rather than inventing provider truth.
- [x] No catalogue publication or product activation is possible from this slice.
- [ ] Focused feature tests pass on exact candidate.
- [ ] Exact-head required CI/Phase7/governance is green.
- [ ] Whole-diff review has no material finding.

## Ownership

```yaml
owned_paths:
  - app/ProductsEntitlements/Catalog/**
  - database/migrations/2026_10_06_110000_create_product_catalog_core.php
  - tests/Feature/ProductsEntitlements/ProductCatalogRegistryTest.php
  - docs/agents/tasks/active/OTERYN-20261006-product-catalog-core.md
modules:
  - ProductsEntitlements/Catalog
dependencies:
  - Issue #322
  - terminal provider-neutral payment foundation #321
  - accepted entitlement/game-delivery architecture #925
blockers:
  - production activation remains separately blocked by #1236 provider sandbox/production evidence and LegalCommerce decisions
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-06T08:24:00Z
status: validating
phase: exact_head_ci
branch: feat/322-product-catalog-core
head: 762d4cad4e4d72c8fb7920b356c6d7e6a572aec6
pr: 1466
context_routes:
  - payments
  - testing
owned_paths:
  - app/ProductsEntitlements/Catalog/**
  - database/migrations/2026_10_06_110000_create_product_catalog_core.php
  - tests/Feature/ProductsEntitlements/ProductCatalogRegistryTest.php
  - docs/agents/tasks/active/OTERYN-20261006-product-catalog-core.md
proven:
  - Issue #321 is closed and its provider-neutral payment/security foundation is terminal.
  - PR #925 established the accepted A-E delivery-profile authority split.
  - Premium time is already a bounded ProductsEntitlements subdomain; no general product catalogue/order/voucher/service tables exist on current main.
  - No open PR or branch owns Issue #322 product-catalogue paths at claim time.
derived:
  - Non-production catalogue/version implementation can proceed independently of real-provider activation and without touching Payments, Wallet, Premium or game authority.
unknown:
  - Real provider, merchant/tax/legal activation policy and actual sellable catalogue contents; none are invented here.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treat a payment order as product or entitlement truth.
  - Seed synthetic repository products and call them commercially approved.
  - Add public checkout/catalogue routes before a publication/activation policy exists.
changed_paths:
  - app/ProductsEntitlements/Catalog/ProductCatalogContract.php
  - app/ProductsEntitlements/Catalog/ProductCatalogException.php
  - app/ProductsEntitlements/Catalog/ProductCatalogRegistry.php
  - app/ProductsEntitlements/Catalog/ProductCatalogVersion.php
  - database/migrations/2026_10_06_110000_create_product_catalog_core.php
  - tests/Feature/ProductsEntitlements/ProductCatalogRegistryTest.php
  - docs/agents/tasks/active/OTERYN-20261006-product-catalog-core.md
validation:
  - command: pre-PR whole-diff self-review
    result: PASS
    evidence: bounded seven-path candidate is disjoint from active Premium/LCFA lanes; integrity readback, presentation key-order independence, nullable-window narrowing and test typing were hardened before PR creation
  - command: exact-head required repository workflows
    result: NOT_RUN
    evidence: PR #1466 is open; exact-head workflow results are pending
blockers:
  - production activation only; implementation slice is unblocked
next_action: Freeze this checkpoint successor, inspect PR #1466 exact-head CI/Phase7/governance, and repair only evidence-backed validation findings.
```
