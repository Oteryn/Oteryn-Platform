---
task_id: OTERYN-20260810-wiki-expected-content-inventory
mode: implementation
issue: 488
branch: repair/issue-488
status: implementing
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260810-wiki-expected-content-inventory

## Goal

Repair the remaining high-severity Wiki completeness gap in Issue #488 by turning the reviewed `WikiLaunchContentCatalog` launch scope into an explicit versioned, machine-verifiable expected-content contract, then revalidate the Issue's current failure/recovery, accessibility/responsive and browser-portability acceptance on the exact repair head.

## Feature scope

```yaml
feature_scope:
  type: internal_content_integrity
  complete_user_facing_feature: false
  backend_required: true
  frontend_required: false
  integration_required: true
  e2e_required: true
```

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
restricted_paths:
  - app/Wiki/Models/**
  - app/Wiki/Domain/**
  - app/Wiki/Actions/**
  - database/**
  - routes/**
  - resources/views/**
  - scripts/acceptance/**
  - .github/workflows/**
  - deploy/**
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: wiki:expected-content-inventory
```

## Acceptance inventory

- [ ] A versioned authoritative expected launch inventory independently pins exactly 4 category keys and 13 EN/PL article slug pairs from the already-reviewed launch corpus; no new editorial/game facts are invented.
- [ ] Inventory declares expected locale set, catalog version, counts, category/article metadata and the supported internal-link surface required by launch content.
- [ ] Validation fails closed for missing/extra/duplicate categories, missing/extra/duplicate article slug pairs, locale asymmetry, category-reference drift, metadata drift and invalid sort/order expectations.
- [ ] Every article has non-empty normalized repository source references and each referenced repository path exists on the checked-out exact head.
- [ ] Internal Markdown links are bounded to approved first-party routes or expected localized Wiki article slugs; malformed/external/unexpected references fail validation.
- [ ] Launch content install command executes expected-content validation before any persistence mutation and fails without partial installation when the inventory/catalog contract is invalid.
- [ ] Machine validation can run without a publisher/database mutation through a dedicated console command and has focused unit/feature coverage.
- [ ] Existing public Wiki indexing, EN/PL rendering, administration lifecycle, publication feedback, responsive/accessibility behavior and corrupt/missing EditorialMedia fallback remain green on exact-head acceptance.
- [ ] Wiki paths run with zero retry in applicable current acceptance profiles and Firefox/WebKit portability evidence is captured on the exact head.
- [ ] No deployment, production, credential, external-repository or new editorial-content mutation occurs.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2352
  classified_at: 2026-08-10T23:52:00+02:00
  risk: high
  triggers:
    - Issue 488 has a proven HIGH completeness finding
    - published bilingual Wiki launch content
    - internal route/reference integrity
    - cross-browser public/admin editorial lifecycle acceptance
    - historical intermittent publication-feedback and media-isolation defects
  unknown_or_conflict: []
  rationale: The change promotes the reviewed launch corpus into a release-completeness contract and its exact-head validation is also used to reconcile the remaining Wiki audit ledger.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:52:00+02:00
head: d1e2f27d5dee7e3bb650bad38b4d03eaff6d4249
branch: repair/issue-488
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - security
  - public-web
  - testing
  - content
owned_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
proven:
  - Issue #488 is the canonical owner for the Wiki completeness audit and explicitly says no duplicate Issue should be created for the expected-inventory gap.
  - The user has explicitly authorized autonomous PORTAL-CLOSEOUT execution; Issue comment 5246540485 records bounded current-main implementation authorization for #488.
  - No repair/issue-488 branch or open #488 PR existed immediately before claim.
  - Current WikiLaunchContentCatalog version 2026-07-26.1 contains exactly 4 categories, 13 articles and bilingual EN/PL translations; WikiLaunchContentCommandTest proves installation creates 4 categories, 13 articles, 8 category translations and 26 article translations.
  - Existing Wiki acceptance is zero-retry per admin-wiki specs and current Playwright portability projects match public-wiki/admin-wiki in Chromium, Firefox and WebKit plus responsive desktop/tablet/mobile; admin media acceptance covers corrupt/missing image fallbacks and accessibility.
  - Current admin Wiki lifecycle explicitly asserts publication flash feedback, durable Published state and zero unexplained serverErrors after media fixture isolation.
  - The remaining HIGH gap is that reviewed launch content is not represented as an independent authoritative expected inventory with fail-closed structural/source/link validation.
derived:
  - A release-scoped expected inventory can close the HIGH gap without inventing new Wiki articles by pinning the already-reviewed launch corpus and independently validating the catalog that produces it.
  - Existing cross-browser/failure acceptance should be re-used as exact-head evidence rather than duplicated or weakened.
unknown:
  - future post-launch Wiki expansion beyond the versioned launch inventory
  - future editorial media assignments for launch articles
conflicts: []
first_failure:
  marker: wiki-launch-content-not-independent-expected-inventory
  evidence: Current tests assert aggregate counts and sample pages, but no separate expected contract pins the complete category/article/slug metadata or validates source references/internal links before installation.
rejected_hypotheses:
  - The presence of WikiLaunchContentCatalog alone proves CONTENT_COMPLETE; Issue #488 explicitly rejects sample/fixture presence as sufficient completion evidence.
  - New speculative articles are required to fix this gap; the audit asks for an authoritative expected inventory, and the reviewed launch corpus already provides the bounded release content to inventory.
  - Existing browser/failure tests need to be rewritten; current zero-retry and Firefox/WebKit/responsive coverage already exists and should be revalidated unchanged unless exact-head failures prove otherwise.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
validation: []
blockers: []
next_action: Implement the independent expected inventory, validator and fail-closed CLI/install preflight with focused tests, then open the single authoritative #488 PR and run exact-head CI plus applicable Wiki browser acceptance before deciding whether Issue #488 is terminal.
```
