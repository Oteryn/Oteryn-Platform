---
task_id: OTERYN-20261028-store-impl-3
governing_issue: TBD
required_reads:
  - docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md
  - docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
search_first:
  - docs/contracts/fixtures/store-wire-v1/manifest.json
optional_reads: []
---

# OTERYN-20261028 Store delivery feed (STORE-IMPL-3)

## Goal

Implement the default-off pending, exact-read, receipt and reconcile delivery routes on the Store foundation, without purchases, new operation IDs, activation or restore-notice implementation. Governing Issue is TBD by explicit owner instruction; this worker creates no Issue or PR.

Owner grant (2026-10-23, renewed 2026-10-28 "zacznij teraz"): the repository owner explicitly authorized Codex to implement Platform Store work on this machine, stacked on the PROPOSED contracts. Everything stays default-off. This grants no purchase enablement, sale, deployment, production mutation or cross-repository work. No commits or pushes are authorized.

## Acceptance criteria

- [x] Pending is bounded, ordered by revision and operation ID, cursor based, read only, without TTL.
- [x] Read returns immutable stored intent bytes and rejects unknown issuer/kind/id keys.
- [x] Receipts enforce closed wire schemas, canonical member order, integer bounds, binding digest, echo and quantity rules.
- [x] First receipt wins under restore/operation/unit locks; exact republication writes nothing; conflicts alarm without rewriting.
- [x] Terminal operation and fulfilment delivery states change only with a receipt in the same transaction; dependency failure rolls everything back.
- [x] Reconcile lists all states, current digests and generations in bounded pages; superseded history remains intact.
- [ ] All repository checks pass (full PHPStan has an unrelated existing error; final validation recorded below).
- [ ] Coordinator resolves foundation/contract gaps and live lifecycle readiness.

## Ownership

```yaml
owned_paths:
  - app/ProductsEntitlements/Store/Delivery/**
  - app/ProductsEntitlements/Store/Http/StorePendingController.php
  - app/ProductsEntitlements/Store/Http/StoreReadController.php
  - app/ProductsEntitlements/Store/Http/StoreReceiptController.php
  - app/ProductsEntitlements/Store/Http/StoreReconcileController.php
  - tests/Feature/ProductsEntitlements/StoreDelivery*
  - tests/Feature/ProductsEntitlements/StoreReconcile*Test.php
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-3.md
modules:
  - products-entitlements
dependencies:
  - STORE-IMPL-1 (PR 1669)
  - STORE-WIRE-1 (Proposed)
blockers:
  - Governing Issue is TBD by owner instruction
cross_repository_tasks:
  - none
```

The existing HTTP foundation test is updated only to require empty 400 for incomplete delivery bodies after implementation; purchase stubs retain empty 503. No assertions or scenarios are removed. STORE-IMPL-2's purchase controllers and Purchase directory are untouched.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-28T13:50:08Z
head: b7d6df3e0aaef84a84757b8ceb3a058a88b2bbf2
branch: claude/happy-davinci-ijz3wy-store-delivery
pr: none
status: waiting
context_routes:
  - products-entitlements
owned_paths:
  - app/ProductsEntitlements/Store/Delivery/**
  - app/ProductsEntitlements/Store/Http/StorePendingController.php
  - app/ProductsEntitlements/Store/Http/StoreReadController.php
  - app/ProductsEntitlements/Store/Http/StoreReceiptController.php
  - app/ProductsEntitlements/Store/Http/StoreReconcileController.php
  - tests/Feature/ProductsEntitlements/StoreDelivery*
  - tests/Feature/ProductsEntitlements/StoreReconcile*Test.php
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-3.md
proven:
  - Origin is Oteryn/Oteryn-Platform; clean dedicated delivery branch at admission.
  - Bound META organization policy read at exact commit 1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
  - Live foundation PR 1669 is open and stacked on the Store wire branch, with head matching this workspace at admission.
  - Live Store Issues and active packets inspected; no mapped delivery Issue exists in the inspected open Issue page and the owner specifies TBD.
  - Baseline Store tests passed on PostgreSQL with 130 tests and 2957 assertions, three skips and one PHPUnit notice.
  - New feature tests dispatch through the real registered routes and dedicated TLS guard against PostgreSQL.
  - Pending and reconcile use one bounded joined query, without time-based expiry or state writes.
  - Receipt writes take the shared restore lock then the intent and fulfilment unit locks in a single transaction.
  - Existing database states are used without changing foundation models or schema.
  - Whole tracked/untracked delta reread against the owner CODEX-CHECKLIST; all new production public methods have route callers and dispatch tests.
  - New constructor fields and all callers searched across app, bootstrap, config, database, routes, tests and tools; no enum variants or trait methods were added.
  - No contract, immutable fixture, CI, purchase-lane or credential files changed.
derived:
  - Enabling delivery is not qualified while the cross-contract and restore-notice gaps below remain.
unknown:
  - Governing delivery Issue and mapped Jira Story remain pending; worker does not load Jira.
conflicts:
  - Requested APPLIED and REJECTED unit names are absent from the foundation state constraint; existing DELIVERED and MANUAL_REVIEW are used.
  - Wire section 6.2 requires a new CLAIM intent after ITEM delivery; this lane explicitly forbids minting any operation ID.
first_failure:
  marker: full-phpstan-existing-error
  evidence: app/Console/Commands/InstallWikiLaunchContent.php line 49 has function.alreadyNarrowedType outside owned scope.
rejected_hypotheses:
  - Editing foundation models, migrations, settings or wire helpers to hide scope conflicts.
  - Creating purchases or CLAIM/reversal operations in the delivery lane.
changed_paths:
  - app/ProductsEntitlements/Store/Delivery/**
  - app/ProductsEntitlements/Store/Http/StorePendingController.php
  - app/ProductsEntitlements/Store/Http/StoreReadController.php
  - app/ProductsEntitlements/Store/Http/StoreReceiptController.php
  - app/ProductsEntitlements/Store/Http/StoreReconcileController.php
  - tests/Feature/ProductsEntitlements/StoreDelivery*
  - tests/Feature/ProductsEntitlements/StoreReconcile*Test.php
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-3.md
validation:
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter Store"
    result: PASS
    evidence: "Baseline before edits; 130 tests, 2957 assertions, three unrelated skips, one unrelated PHPUnit notice, exit 0."
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter Store --display-skipped --display-phpunit-notices"
    result: PASS
    evidence: "Final PostgreSQL run; 144 tests, 3707 assertions, baseline three skips and one notice, exit 0."
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter 'StoreDelivery|StoreReconcile' --display-skipped --display-phpunit-notices"
    result: PASS
    evidence: "Final focused PostgreSQL run; 14 tests, 750 assertions, zero skips or notices, exit 0."
  - command: "php vendor/bin/phpstan analyse --memory-limit=1G --no-progress"
    result: FAIL
    evidence: "One existing unrelated error in untouched InstallWikiLaunchContent.php line 49; function.alreadyNarrowedType. No error remains in changed files."
  - command: "php vendor/bin/phpstan analyse --memory-limit=1G --no-progress app/ProductsEntitlements/Store/Delivery app/ProductsEntitlements/Store/Http/StorePendingController.php app/ProductsEntitlements/Store/Http/StoreReadController.php app/ProductsEntitlements/Store/Http/StoreReceiptController.php app/ProductsEntitlements/Store/Http/StoreReconcileController.php tests/Feature/ProductsEntitlements/StoreDeliveryFixtures.php tests/Feature/ProductsEntitlements/StoreDeliveryFeedTest.php tests/Feature/ProductsEntitlements/StoreDeliveryReceiptTest.php tests/Feature/ProductsEntitlements/StoreReconcileFeedTest.php tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php"
    result: PASS
    evidence: "All changed PHP files, level 10; no errors."
  - command: "php vendor/bin/pint --test"
    result: PASS
    evidence: "Full repository formatting check, final PHP files."
  - command: "python3 tools/agents/documentation_ia.py"
    result: PASS
    evidence: "24 prompts and 3 handovers."
  - command: "python3 tools/validation/legacy_token_ratchet.py"
    result: PASS
    evidence: "Enforced categories pass; pre-existing history report-only notes remain."
  - command: "python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint"
    result: PASS
    evidence: "19 task checkpoints structurally valid. TBD is an owner-requested lifecycle exception, not readiness proof."
  - command: "python3 tools/agents/test_checkpoint.py"
    result: PASS
    evidence: "12 tests."
  - command: "python3 tools/agents/source_branch_closeout.py --active docs/agents/tasks/active"
    result: PASS
    evidence: "No completed active tasks or changed archive tasks."
  - command: "python3 scripts/validate_native_protocol_contract.py"
    result: PASS
    evidence: "Native schema valid."
  - command: "python3 scripts/validate_native_protocol_change_boundary.py --base-ref origin/main"
    result: NOT_APPLICABLE
    evidence: "No native-protocol producer signal in changed paths; Store is not the native client protocol producer."
  - command: "python3 scripts/test_native_protocol_change_boundary.py"
    result: PASS
    evidence: "9 tests."
  - command: "python3 tools/validation/prompt_eval.py"
    result: PASS
    evidence: "24 deterministic cases; no model trials claimed."
  - command: "python3 tools/validation/verify_integration_test_registration.py"
    result: PASS
    evidence: "Zero explicitly registered Integration tests; new Store tests use existing Feature discovery."
  - command: "python3 inline jsonschema/hashlib fixture check"
    result: PASS
    evidence: "16 Store wire schemas, 30 valid fixtures matching schemas/sizes/digests, 23 schema-invalid cases rejected."
  - command: "git diff --check"
    result: PASS
    evidence: "No whitespace errors; every new PHP file is below 400 lines; full tracked and untracked delta reviewed against owner checklist."
blockers:
  - Governing Issue is TBD; lifecycle/exact-head CI and independent review remain coordinator work.
next_action: Coordinator reviews this uncommitted candidate, resolves the recorded foundation conflicts and assigns the governing delivery Issue before integration.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: local implementation only; owner prohibits commits and publication
source_branch_evidence: current task instruction
```

## Validation details

New dispatch coverage is in StoreDeliveryFeedTest, StoreDeliveryReceiptTest and StoreReconcileFeedTest. These use the existing Feature discovery, with no new integration registration. The fixture helper seeds models directly and asserts PostgreSQL at setup. StoreWire fixture validation covers 16 schemas, 30 valid examples with exact sizes and hashes, and 23 schema-invalid cases using the installed Python jsonschema validator; the HTTP tests also validate fixtures and dispatch the delivery invalid cases.

The broad Store filter also selects unrelated tests. The unchanged baseline notice is PublicSeoConditionalRequestTest::test_sitemap_dependency_failure_remains_no_store_without_etag (PublicWikiQuery mock without expectations). The unchanged skipped cases are CharacterCommandIssuanceOrderPgsqlTest::test_an_intent_drawn_below_a_stored_key_is_refused and GameRestoreReceiptConcurrencyPgsqlTest::{test_a_receipt_committing_during_a_restore_notice_is_reopened,test_a_carried_reopen_answered_while_a_chained_notice_waits_is_reopened_not_dropped}; their separate PostgreSQL integration environment is not configured. They are CI-only and are not claimed as qualified. Every new delivery test ran on PostgreSQL, without skips.

## Unfinished and scope boundaries

- The foundation fulfilment-unit CHECK permits DELIVERED and MANUAL_REVIEW, not the requested APPLIED/REJECTED names. The handler maps committed DELIVER to DELIVERED and rejected DELIVER to MANUAL_REVIEW. Literal APPLIED/REJECTED requires a separate approved additive migration and model/consumer updates; none are made here.
- Wire section 6.2 requires creating a CLAIM intent after committed ITEM DELIVER. This task explicitly says never mint a new operation ID; no CLAIM intent is created here. The owning workflow must resolve this before enablement. CLAIM/reversal receipt bytes and operation terminal states are recorded, but downstream claim/reversal business effects are outside this delivery-only lane.
- The foundation StoreReconcileState stores per-restore membership and superseded/resolved receipt references. There is no configured history-capacity bound in its model, schema or StoreSettings. This lane bounds request/response pages to 32 and never prunes history. A retention/capacity policy would require a separately owned settings/schema decision; inventing one here would violate scope.
- Existing GameRestoreReconciler still selects and acknowledges only Character commands. Store reopen/carry/acknowledgement integration and the wire section 7 restore-contract amendment belong to the restore lane. Store receipts already take its shared lock, enforce global and per-operation restore generations, and resolve seeded StoreReconcileState membership. A stack with Store intents must not enable restore before that integration is qualified.
- Full PHPStan reports one error in untouched app/Console/Commands/InstallWikiLaunchContent.php line 49 (redundant is_string check on the publisher argument). Fixing that file is outside the permitted foundation edits.
- Governing Issue is deliberately TBD; Jira synchronization is pending/unknown. No publication, exact-head CI, merge readiness, deployment, live data mutation or enablement is claimed.
- The broad Store filter has the baseline's three skips and one unrelated PHPUnit notice. New delivery tests assert PostgreSQL explicitly; no skipped PostgreSQL qualification is claimed.

## Changed files

- app/ProductsEntitlements/Store/Delivery/StoreDeliveryFeed.php
- app/ProductsEntitlements/Store/Delivery/StoreDeliveryRefused.php
- app/ProductsEntitlements/Store/Delivery/StoreDeliveryRequest.php
- app/ProductsEntitlements/Store/Delivery/StoreDeliveryResponse.php
- app/ProductsEntitlements/Store/Delivery/StoreReceiptPublisher.php
- app/ProductsEntitlements/Store/Http/StorePendingController.php
- app/ProductsEntitlements/Store/Http/StoreReadController.php
- app/ProductsEntitlements/Store/Http/StoreReceiptController.php
- app/ProductsEntitlements/Store/Http/StoreReconcileController.php
- tests/Feature/ProductsEntitlements/StoreDeliveryFeedTest.php
- tests/Feature/ProductsEntitlements/StoreDeliveryFixtures.php
- tests/Feature/ProductsEntitlements/StoreDeliveryReceiptTest.php
- tests/Feature/ProductsEntitlements/StoreReconcileFeedTest.php
- tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
- docs/agents/tasks/active/OTERYN-20261028-store-impl-3.md
