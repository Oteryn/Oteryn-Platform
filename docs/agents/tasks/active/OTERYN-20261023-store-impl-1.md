---
task_id: OTERYN-20261023-store-impl-1
governing_issue: TBD
required_reads:
  - docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
search_first:
  - docs/contracts/fixtures/store-wire-v1/manifest.json
optional_reads: []
---

# OTERYN-20261023 Store foundation (STORE-IMPL-1)

## Goal

Implement the default-off Platform Store foundation and S-5 verbatim price import in this existing isolated worktree. Governing Issue: TBD, explicitly requested by the owner; no Issue or PR is created by this worker.

Owner grant (2026-10-23): the repository owner explicitly authorized Codex to implement Platform Store work on this machine, stacked on the Proposed contracts. This task-specific grant authorizes this foundation despite their implementation disclaimer. It grants no purchase enablement, sale, deployment, production mutation or cross-repository work. No commits or pushes are authorized.

## Delegated decisions

- Q1: v1 holds COINS only. TRANSFERABLE_COINS offers will answer OFFER_UNAVAILABLE and balance will omit that currency in later lanes.
- Q2 / S-5: Game authors a closed JSON price list. Platform imports exact canonical bytes verbatim. catalog_revision is their lowercase SHA-256 hex digest, overriding the Proposed wire's uint64 revision. Only one import is active; activation is a separate explicit operator command. Imported offers are never edited.
- Q3/Q4: support PREMIUM_TIME, COSMETIC and ITEM in stored shapes; Profile-B fences remain outside scope. enabled_unit_kinds defaults to an empty list.
- Q5: every fulfilment unit of every kind gets a DELIVER intent in the purchase lane.
- Q6: a Platform PITR fence is a precondition for purchases on a restorable stack. The testing/preproduction environment limit remains mandatory.
- Q7: one dedicated Store mTLS identity in v1.
- Q8: identities.account_id is the unique canonical AccountId. Resolve Identity by account_id, then use Identity.id as wallet_accounts.identity_id and WalletMutator::lock(int). AccountId is never a numeric Wallet key. No account/Wallet mutation is implemented here.

## Acceptance criteria

- [x] Validated default-off config and dedicated TLS 1.3 guard with 503/401/429/source-authority ordering.
- [x] Seven independently owned route controllers perform transport/canonical-head validation and remain unavailable.
- [x] Additive Store schema/models with PostgreSQL closed sets and idempotency constraints, exact immutable byte storage and reconcile history capacity.
- [x] Closed S-5 schema, valid/invalid fixtures, verbatim import, digest revision and serialized explicit activation.
- [x] Focused Store PostgreSQL tests, Pint, PHPStan and documentation validators recorded.
- [x] Whole diff and constructor/caller sites reviewed; no contract documents or protected CI routing edited.

## Ownership

```yaml
owned_paths:
  - app/ProductsEntitlements/Store/**
  - config/products-entitlements.php
  - routes/internal.php
  - bootstrap/providers.php
  - bootstrap/app.php
  - database/migrations/2026_10_23_*_create_store_*.php
  - tests/Feature/ProductsEntitlements/Store*Test.php
  - tests/Unit/ProductsEntitlements/StoreWireTest.php
  - docs/contracts/fixtures/store-price-list-v1/**
  - docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md
modules:
  - products-entitlements
dependencies:
  - STORE-WIRE-1 (Proposed)
blockers:
  - Governing Issue is TBD by owner instruction
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-23T20:46:58Z
head: 12821f1d656a5829c7f729ee9d52f7ae9e91e27e
branch: claude/happy-davinci-ijz3wy-store-foundation
pr: none
status: validating
context_routes:
  - products-entitlements
owned_paths:
  - app/ProductsEntitlements/Store/**
  - config/products-entitlements.php
  - routes/internal.php
  - bootstrap/providers.php
  - bootstrap/app.php
  - database/migrations/2026_10_23_*_create_store_*.php
  - tests/Feature/ProductsEntitlements/Store*Test.php
  - tests/Unit/ProductsEntitlements/StoreWireTest.php
  - docs/contracts/fixtures/store-price-list-v1/**
  - docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md
proven:
  - Local origin is Oteryn/Oteryn-Platform and the existing dedicated task branch is clean at admission.
  - Bound META organization policy was read at exact commit 1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
  - Public GitHub preflight finds Store Issue 1658 and PR 1659 for wire documentation; no matching foundation Issue is present in the inspected open Issue page.
  - Existing command decoder and bundled JSON Schema validator are reused without dependencies.
  - All seven routes are dispatched through their guard and transport validator and then answer empty 503 without writes.
  - Catalog imports and offers reject PostgreSQL UPDATE/DELETE; intent bindings and purchase answers are immutable; receipts preserve superseded history.
  - Activation takes the seeded singleton row FOR UPDATE and changes only the one active revision pointer.
  - The size-check deferral applies only to the seven registered POST routes; other routes retain Laravel ValidatePostSize.
  - The owner checklist was read and the complete tracked and untracked delta reviewed; constructors, fields and caller sites were searched across app/bootstrap/config/database/routes/tests/tools.
  - No existing test, contract document, CI workflow or repository policy was changed.
  - identities.account_id maps to identities.id which is the Wallet identity key.
derived:
  - The owner Q2 override must remain documented outside the unchanged Proposed wire fixtures.
unknown:
  - Governing foundation Issue and programme Jira mapping are pending; this worker does not load Jira.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Implementing purchase or delivery behavior in this foundation lane.
changed_paths:
  - app/ProductsEntitlements/Store/**
  - bootstrap/app.php
  - bootstrap/providers.php
  - config/products-entitlements.php
  - routes/internal.php
  - database/migrations/2026_10_23_*_create_store_*.php
  - docs/contracts/fixtures/store-price-list-v1/**
  - tests/Feature/ProductsEntitlements/Store*Test.php
  - tests/Unit/ProductsEntitlements/StoreWireTest.php
  - docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md
validation:
  - command: /home/mole/oteryn-cache/codex-bin/plat-phpunit --filter Store
    result: PASS
    evidence: 110 tests, 2782 assertions, exit 0; one unrelated existing sitemap PHPUnit notice reproduced alone.
  - command: /home/mole/oteryn-cache/codex-bin/plat-phpunit --filter 'StoreCatalogImportTest|StoreHttpFoundationTest|StorePersistenceTest|StoreWireTest' --display-phpunit-notices
    result: PASS
    evidence: 69 tests, 1727 assertions, no notices; actual PostgreSQL, no early-return qualification.
  - command: php vendor/bin/phpstan analyse --memory-limit=1G --no-progress --error-format=table
    result: PASS
    evidence: full repository analysis, level 10, no errors.
  - command: php vendor/bin/pint --test
    result: PASS
    evidence: full repository formatting check.
  - command: python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: 24 prompts and 3 handovers.
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: enforced categories pass; existing history report-only notes remain.
  - command: python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: 15 active checkpoints structurally valid; Issue TBD is an owner-requested lifecycle exception, not readiness proof.
  - command: python3 tools/agents/test_checkpoint.py
    result: PASS
    evidence: 12 tests.
  - command: python3 tools/agents/source_branch_closeout.py --active docs/agents/tasks/active
    result: PASS
    evidence: no completed active tasks or changed archive tasks.
  - command: python3 scripts/validate_native_protocol_contract.py
    result: PASS
    evidence: native contract schema valid.
  - command: python3 scripts/validate_native_protocol_change_boundary.py --base-ref origin/main
    result: NOT_APPLICABLE
    evidence: no native-protocol producer signal; Store foundation is not the native client protocol producer.
  - command: python3 scripts/test_native_protocol_change_boundary.py
    result: PASS
    evidence: 9 tests.
  - command: python3 tools/validation/prompt_eval.py
    result: PASS
    evidence: 24 deterministic cases; no model trials claimed.
  - command: python3 tools/validation/verify_integration_test_registration.py
    result: PASS
    evidence: zero explicitly registered Integration tests; Store tests use existing Unit/Feature discovery.
  - command: python3 inline jsonschema/hashlib fixture check
    result: PASS
    evidence: 16 existing wire schemas plus new price-list schema valid; 30 existing valid examples match schemas/sizes/digests and 23 schema-invalid cases rejected.
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors; all new files below 400 lines.
blockers:
  - Issue TBD prevents lifecycle readiness; local implementation is owner-authorized.
next_action: Coordinator attaches the governing foundation Issue and reviews this uncommitted local candidate before assigning the purchase and delivery lanes.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: local implementation only; owner prohibits commits and publication
source_branch_evidence: current task instruction
```

## Notes

Purchase transactions, balance/history, delivery handlers, restore extension and reversals remain for later lanes. The guard recognizes the configured restore flag for delivery routing; every handler still answers 503, so this lane cannot serve the exchange or qualify enablement. Jira synchronization is pending/unknown. Exact-head CI and live lifecycle closeout remain coordinator work.

## Unfinished and scope boundaries

- No foundation implementation gap remains after local checks. Governing Issue is deliberately TBD, Jira synchronization is pending/unknown, and live PR/exact-head CI/independent review remain coordinator work. Nothing is committed or published.
- The broad `Store` filter also matches unrelated existing tests. `PublicSeoConditionalRequestTest::test_sitemap_dependency_failure_remains_no_store_without_etag` reports a PHPUnit notice for an existing `PublicWikiQuery` mock with no expectations. Reproduced alone with `/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter test_sitemap_dependency_failure_remains_no_store_without_etag --display-phpunit-notices`: 1 test, 3 assertions, exit 0, one notice. This test is untouched.
- `ActiveStoreCatalog::offer` and the ledger models/configured per-account limits are deliberate purchase/delivery-lane boundaries explicitly requested in this foundation task. Their future handlers are not implemented here. The query is exercised against real imported/activated PostgreSQL state; HTTP handlers remain empty 503 stubs.
- Schema down/up is tested only in the disposable test database. No live data, account/session state, Wallet credit/debit, production service, credential, game repository or protected environment was touched. Rust cargo/clippy/fmt are not applicable to this PHP-only change.
- The wire still says numeric catalog revisions. The delegated SHA-256 decision is implemented in import/storage and recorded here; changing the Proposed wire documents and fixtures remains the contract owner's work.

## Changed files

- `bootstrap/app.php`
- `bootstrap/providers.php`
- `config/products-entitlements.php`
- `routes/internal.php`
- `app/ProductsEntitlements/Store/ActiveStoreCatalog.php`
- `app/ProductsEntitlements/Store/Console/ActivateStoreCatalog.php`
- `app/ProductsEntitlements/Store/Console/ImportStoreCatalog.php`
- `app/ProductsEntitlements/Store/Http/DeferStorePostSize.php`
- `app/ProductsEntitlements/Store/Http/GuardStorePeer.php`
- `app/ProductsEntitlements/Store/Http/StoreBalanceController.php`
- `app/ProductsEntitlements/Store/Http/StoreHistoryController.php`
- `app/ProductsEntitlements/Store/Http/StorePendingController.php`
- `app/ProductsEntitlements/Store/Http/StorePurchaseController.php`
- `app/ProductsEntitlements/Store/Http/StoreReadController.php`
- `app/ProductsEntitlements/Store/Http/StoreReceiptController.php`
- `app/ProductsEntitlements/Store/Http/StoreReconcileController.php`
- `app/ProductsEntitlements/Store/Http/ValidateStoreRequest.php`
- `app/ProductsEntitlements/Store/Models/StoreActiveCatalog.php`
- `app/ProductsEntitlements/Store/Models/StoreFulfilmentUnit.php`
- `app/ProductsEntitlements/Store/Models/StoreOffer.php`
- `app/ProductsEntitlements/Store/Models/StoreOperation.php`
- `app/ProductsEntitlements/Store/Models/StoreOrder.php`
- `app/ProductsEntitlements/Store/Models/StoreOrderLine.php`
- `app/ProductsEntitlements/Store/Models/StorePriceListImport.php`
- `app/ProductsEntitlements/Store/Models/StorePurchaseRequest.php`
- `app/ProductsEntitlements/Store/Models/StoreReceipt.php`
- `app/ProductsEntitlements/Store/Models/StoreReconcileState.php`
- `app/ProductsEntitlements/Store/StoreCatalogActivator.php`
- `app/ProductsEntitlements/Store/StoreCatalogImporter.php`
- `app/ProductsEntitlements/Store/StorePriceList.php`
- `app/ProductsEntitlements/Store/StoreServiceProvider.php`
- `app/ProductsEntitlements/Store/StoreSettings.php`
- `app/ProductsEntitlements/Store/StoreWire.php`
- `database/migrations/2026_10_23_000100_create_store_catalog.php`
- `database/migrations/2026_10_23_000200_create_store_ledger.php`
- `docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md`
- `docs/contracts/fixtures/store-price-list-v1/README.md`
- `docs/contracts/fixtures/store-price-list-v1/price-list.invalid-amount-overflow.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.invalid-duplicate-offer.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.invalid-extra-member.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.invalid-unknown-kind.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.invalid-version-overflow.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.schema.json`
- `docs/contracts/fixtures/store-price-list-v1/price-list.valid.json`
- `tests/Feature/ProductsEntitlements/StoreCatalogImportTest.php`
- `tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php`
- `tests/Feature/ProductsEntitlements/StorePersistenceTest.php`
- `tests/Unit/ProductsEntitlements/StoreWireTest.php`
