---
task_id: OTERYN-20261028-store-impl-2
governing_issue: TBD
required_reads:
  - docs/agents/tasks/active/OTERYN-20261023-store-impl-1.md
  - docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
search_first:
  - docs/contracts/fixtures/store-wire-v1/manifest.json
optional_reads: []
---

# OTERYN-20261028 Store purchase, balance and history (STORE-IMPL-2)

## Goal

Implement the default-off purchase lane on STORE-IMPL-1. Governing Issue: TBD by explicit owner instruction. No Issue, PR, commit or push is authorized for this worker.

Owner grant (2026-10-23, renewed 2026-10-28 “zacznij teraz”): the repository owner explicitly authorized Codex to implement Platform Store work on this machine, stacked on the PROPOSED contracts. Everything stays default-off. This grants no purchase enablement, sale, deployment, production mutation or cross-repository work.

Foundation decisions Q1–Q8 remain binding: COINS only; canonical SHA-256 catalog revisions; configured unit-kind gates; every unit receives a DELIVER intent; PITR and Profile-B qualification remain preconditions to enablement; canonical identities.account_id maps to Identity.id for Wallet locking.

## Acceptance criteria

- [x] Atomic PostgreSQL purchase with all terminal results, byte-exact retry and conflict alarm.
- [x] Wallet debit, order, fulfilment units, immutable intents and answer commit together.
- [x] Read-only COINS balance and bounded account-isolated ledger history with stable cursors.
- [x] PostgreSQL route, rollback, concurrent debit/retry and fixture-schema evidence.
- [x] Changed-file PHPStan, full Pint, documentation validators and whole-diff owner checklist reviewed.
- [ ] Full repository PHPStan blocked by an unchanged wiki command error outside this lane.

## Ownership

```yaml
owned_paths:
  - app/ProductsEntitlements/Store/Purchase/**
  - app/ProductsEntitlements/Store/Http/StorePurchaseController.php
  - app/ProductsEntitlements/Store/Http/StoreBalanceController.php
  - app/ProductsEntitlements/Store/Http/StoreHistoryController.php
  - tests/Feature/ProductsEntitlements/StorePurchase*
  - tests/Feature/ProductsEntitlements/StoreBalance*
  - tests/Feature/ProductsEntitlements/StoreHistory*
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-2.md
modules:
  - products-entitlements
dependencies:
  - STORE-IMPL-1
blockers:
  - Governing Issue is TBD by owner instruction
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-28T13:54:39Z
head: b7d6df3e0aaef84a84757b8ceb3a058a88b2bbf2
branch: claude/happy-davinci-ijz3wy-store-purchase
pr: none
status: waiting
context_routes:
  - products-entitlements
owned_paths:
  - app/ProductsEntitlements/Store/Purchase/**
  - app/ProductsEntitlements/Store/Http/StorePurchaseController.php
  - app/ProductsEntitlements/Store/Http/StoreBalanceController.php
  - app/ProductsEntitlements/Store/Http/StoreHistoryController.php
  - tests/Feature/ProductsEntitlements/StorePurchase*
  - tests/Feature/ProductsEntitlements/StoreBalance*
  - tests/Feature/ProductsEntitlements/StoreHistory*
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-2.md
proven:
  - Clean isolated Platform worktree at admission; no Game repository access.
  - Bound META organization policy read at exact commit 1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
  - Live PR 1669 is open; its description differs from local foundation stubs and is not implementation evidence.
  - Targeted live Store Issue and PR search found no mapped purchase-lane Issue; owner requires TBD.
  - Baseline Store filter passed with 130 tests and 2957 assertions, three skips and one PHPUnit notice.
  - Every new production method has a route/container caller; all three routes are exercised through real middleware and PostgreSQL.
  - Purchase request advisory locking serializes first attempts and exact retries; Identity row and active catalog shared locks fence account and import changes.
  - Wallet savepoint rolls back a missing-row insert on insufficient balance; refusals persist only their answer.
  - Lock timeout is three seconds and statement timeout five seconds inside the transaction.
  - All fulfilment kinds get immutable DELIVER bytes with their actual allocated source revision and digest; quantity multiplication is bounded.
  - Pending DELIVER capacity is serialized among purchases; exact retries bypass new-purchase capacity checks.
  - Concurrent route tests prove one debit, byte-identical same-id answers and no negative balance; independent Wallet lock timeout and injected post-debit SQL failure roll back all purchase effects.
  - Balance reads create no Wallet row; history reads at most max_entries plus one and resolves Store provenance only through this account's stored order.
  - Complete tracked and untracked delta reviewed against the owner checklist; all production files remain below 400 lines; no new enum or shared model field.
  - Production caller and constructor searches cover app, tests, routes and tools; Rust workspace paths do not exist in this repository.
derived:
  - Proposed fixture numeric revisions need an explicit Q2 projection for schema checks; fixture files remain unchanged.
unknown:
  - Governing Issue and Jira mapping are pending; worker does not load Jira.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Editing delivery lane or shared foundation schema, settings, routes or wire helper.
changed_paths:
  - app/ProductsEntitlements/Store/Purchase/**
  - app/ProductsEntitlements/Store/Http/StorePurchaseController.php
  - app/ProductsEntitlements/Store/Http/StoreBalanceController.php
  - app/ProductsEntitlements/Store/Http/StoreHistoryController.php
  - tests/Feature/ProductsEntitlements/StorePurchase*
  - tests/Feature/ProductsEntitlements/StoreBalance*
  - tests/Feature/ProductsEntitlements/StoreHistory*
  - tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php
  - docs/agents/tasks/active/OTERYN-20261028-store-impl-2.md
validation:
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter Store"
    result: PASS
    evidence: "Pre-edit baseline, 130 tests, 2957 assertions, three skips and one notice, exit 0."
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter Store"
    result: PASS
    evidence: "Final PostgreSQL run, 150 tests, 3235 assertions, three pre-existing skips and one pre-existing notice, exit 0."
  - command: "/home/mole/oteryn-cache/codex-bin/plat-phpunit --filter 'StorePurchaseTest|StorePurchaseConcurrencyTest|StoreBalanceTest|StoreHistoryTest' --display-phpunit-notices"
    result: PASS
    evidence: "20 lane tests, 278 assertions, no skips or notices, exit 0; includes separate processes and actual PostgreSQL lock timeout."
  - command: "php vendor/bin/phpstan analyse --memory-limit=1G --no-progress"
    result: FAIL
    evidence: "One unchanged error at app/Console/Commands/InstallWikiLaunchContent.php:49, function.alreadyNarrowedType. No Store findings; outside authorized production paths."
  - command: "php vendor/bin/phpstan analyse --memory-limit=1G --no-progress app/ProductsEntitlements/Store/Purchase app/ProductsEntitlements/Store/Http/StorePurchaseController.php app/ProductsEntitlements/Store/Http/StoreBalanceController.php app/ProductsEntitlements/Store/Http/StoreHistoryController.php tests/Feature/ProductsEntitlements/StorePurchaseTestCase.php tests/Feature/ProductsEntitlements/StorePurchaseTest.php tests/Feature/ProductsEntitlements/StorePurchaseConcurrencyTest.php tests/Feature/ProductsEntitlements/StoreBalanceTest.php tests/Feature/ProductsEntitlements/StoreHistoryTest.php tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php"
    result: PASS
    evidence: "All changed PHP files, no errors."
  - command: "php vendor/bin/pint --test"
    result: PASS
    evidence: "Full repository formatting, exit 0."
  - command: "python3 tools/agents/documentation_ia.py"
    result: PASS
    evidence: "24 prompts and 3 handovers."
  - command: "python3 tools/validation/legacy_token_ratchet.py"
    result: PASS
    evidence: "Enforced categories pass; historical report-only notes remain."
  - command: "python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint"
    result: PASS
    evidence: "19 structurally valid checkpoints; TBD remains an owner-requested lifecycle exception."
  - command: "python3 tools/agents/test_checkpoint.py"
    result: PASS
    evidence: "12 tests."
  - command: "python3 tools/agents/source_branch_closeout.py --active docs/agents/tasks/active"
    result: PASS
    evidence: "No completed active tasks or changed archived tasks."
  - command: "python3 scripts/validate_native_protocol_contract.py"
    result: PASS
    evidence: "Native schema digest valid."
  - command: "python3 scripts/validate_native_protocol_change_boundary.py --base-ref origin/main"
    result: NOT_APPLICABLE
    evidence: "No native-protocol producer signal in changed paths."
  - command: "python3 scripts/test_native_protocol_change_boundary.py"
    result: PASS
    evidence: "9 tests."
  - command: "python3 tools/validation/prompt_eval.py"
    result: PASS
    evidence: "24 deterministic cases; no model trials claimed."
  - command: "python3 tools/validation/verify_integration_test_registration.py"
    result: PASS
    evidence: "Existing Feature discovery includes Store tests; no new Integration registration."
  - command: "python3 inline jsonschema/hashlib wire fixture validation"
    result: PASS
    evidence: "16 schemas, 30 valid fixtures with canonical sizes and digests, 23 invalid schema cases. Files unchanged. Runtime response schemas are tested through routes; only delegated Q2 catalog_revision is projected to numeric fixture shape."
  - command: "git diff --check"
    result: PASS
    evidence: "No whitespace errors. New PHP files also checked via git diff --no-index --check against /dev/null."
blockers:
  - Lifecycle readiness requires the coordinator to attach a governing Issue.
  - Full PHPStan has one unchanged out-of-scope wiki command error.
next_action: Coordinator attaches the governing Issue, resolves the unrelated PHPStan blocker and qualifies this local candidate through live review and exact-head CI.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: local implementation only; owner prohibits commits and publication
source_branch_evidence: current task instruction
```

## Notes

Only the three purchase-side foundation controllers are modified in production. The head-only foundation probe is strengthened to expect malformed-body rejection on implemented routes; delivery probes remain unchanged. No contract, fixture, migration, settings, route or delivery code changes.

## Unfinished

- No purchase-lane implementation gap or shared foundation production change is needed. Full-repository PHPStan remains blocked by `app/Console/Commands/InstallWikiLaunchContent.php:49` (`function.alreadyNarrowedType`); the required exact change is to remove its redundant `is_string` check, by that path's owner. This worker did not change it. Changed-file analysis passes.
- Governing Issue TBD, Jira mapping/synchronization pending/unknown, independent review and exact-head CI remain coordinator work. No commits, pushes, enablement or production mutations occurred.
- Proposed wire fixtures still use numeric catalog_revision. Owner Q2 digest revision is used in production; contract/fixture reconciliation belongs to the contract lane. Tests project only that member for unchanged schema conformance and explicitly validate digest input at dispatch.
- Premium/cosmetic consumer fences, PITR qualification, reversals and delivery processing remain their existing external lane boundaries. Historical Wallet operation types without an implemented Store association are OTHER, as required for unmapped entries.
- The broad Store filter retains the baseline three skips and one unrelated sitemap PHPUnit notice. All 20 new lane tests run with no skip or notice. Rust cargo/clippy/fmt do not apply to this PHP-only lane.

## Changed files


- `app/ProductsEntitlements/Store/Purchase/PurchaseStoreOffer.php`
- `app/ProductsEntitlements/Store/Purchase/ReadStoreAccount.php`
- `app/ProductsEntitlements/Store/Purchase/StoreAccountLimiter.php`
- `app/ProductsEntitlements/Store/Purchase/StoreInsufficientBalance.php`
- `app/ProductsEntitlements/Store/Purchase/StorePurchaseInput.php`
- `app/ProductsEntitlements/Store/Purchase/StorePurchaseWriter.php`
- `app/ProductsEntitlements/Store/Purchase/StoreReadInput.php`
- `docs/agents/tasks/active/OTERYN-20261028-store-impl-2.md`
- `tests/Feature/ProductsEntitlements/StoreBalanceTest.php`
- `tests/Feature/ProductsEntitlements/StoreHistoryTest.php`
- `tests/Feature/ProductsEntitlements/StorePurchaseConcurrencyTest.php`
- `tests/Feature/ProductsEntitlements/StorePurchaseTest.php`
- `tests/Feature/ProductsEntitlements/StorePurchaseTestCase.php`
- `app/ProductsEntitlements/Store/Http/StoreBalanceController.php`
- `app/ProductsEntitlements/Store/Http/StoreHistoryController.php`
- `app/ProductsEntitlements/Store/Http/StorePurchaseController.php`
- `tests/Feature/ProductsEntitlements/StoreHttpFoundationTest.php`
