---
task_id: OTERYN-20260810-wiki-expected-content-inventory
mode: implementation
issue: 488
branch: repair/issue-488
status: validating
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

- [x] A versioned authoritative expected launch inventory independently pins exactly 4 category keys and 13 EN/PL article slug pairs from the already-reviewed launch corpus; no new editorial/game facts are invented.
- [x] Inventory declares expected locale set, independently pinned catalog version, category/article counts through exact sets, category/article metadata, approved internal-link surface and launch media/fallback policy.
- [x] Validation fails closed for missing/extra/duplicate categories, missing/extra/duplicate article identities/slugs, locale asymmetry, category-reference drift, metadata drift and invalid sort/order expectations.
- [x] Every article must have non-empty normalized repository source references and each referenced repository path must exist on the checked-out exact head.
- [x] Internal Markdown links are bounded to approved first-party routes or expected localized Wiki article slugs; external/unexpected references fail validation.
- [x] Launch content install command executes expected-content validation before publisher lookup/persistence mutation and fails closed on inventory/catalog drift.
- [x] Machine validation can run without a publisher/database mutation through `wiki:launch-content:validate --json` and has focused unit/feature coverage.
- [ ] Existing public Wiki indexing, EN/PL rendering, administration lifecycle, publication feedback, responsive/accessibility behavior and corrupt/missing EditorialMedia fallback remain green on exact-head acceptance.
- [ ] Wiki paths run with zero retry in applicable current acceptance profiles and Firefox/WebKit portability evidence is captured on the exact head.
- [x] No deployment, production, credential, external-repository or new editorial-content mutation occurs.

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
updated_at: 2026-08-11T00:13:00+02:00
head: 4c419b14188326381197dc7aaeffdbb85a079341
branch: repair/issue-488
pr: 972
status: validating
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
  - The repair branch was created from protected main d1e2f27d5dee7e3bb650bad38b4d03eaff6d4249 with no competing #488 branch/PR at claim time; PR #972 is now the single authoritative delivery PR.
  - WikiExpectedContentInventory version 2026-08-10.1 independently pins reviewed catalog version 2026-07-26.1, exactly 4 category identities and exactly 13 EN/PL article identities plus metadata and approved first-party link surface.
  - WikiExpectedContentValidator compares the actual catalog to that independent inventory, validates locale/slug/category/metadata drift, repository source paths, internal links and the declared zero launch EditorialMedia-token/fallback policy.
  - A read-only wiki:launch-content:validate command exposes machine PASS/FAIL without publisher or persistence mutation; InstallWikiLaunchContent now runs the validator before publisher lookup or installer writes.
  - Focused unit tests cover exact-match PASS plus missing article, metadata/category drift, locale asymmetry, missing repository source, unexpected external link and duplicate category identity; feature coverage checks the read-only command performs no DB/audit mutation.
  - Existing current-main Wiki acceptance remains zero-retry in admin Wiki specs; Playwright portability projects cover public/admin Wiki in Chromium, Firefox and WebKit plus responsive desktop/tablet/mobile and accessibility; admin media acceptance covers corrupt/missing image fallback.
  - No acceptance scripts or other forbidden paths were modified.
derived:
  - The independent literal catalog-version pin prevents a catalog version edit from automatically approving itself through the expected inventory.
  - Existing browser/failure acceptance should be re-used as exact-head evidence rather than duplicated or weakened.
unknown:
  - exact-head CI/Playwright results for PR #972
  - future post-launch Wiki expansion beyond the versioned launch inventory
  - future editorial media assignments for launch articles
conflicts: []
first_failure:
  marker: wiki-launch-content-not-independent-expected-inventory
  evidence: The missing independent expected contract is implemented on PR #972; terminal resolution remains gated on exact-head machine/browser validation.
rejected_hypotheses:
  - The presence of WikiLaunchContentCatalog alone proves CONTENT_COMPLETE; Issue #488 explicitly rejects sample/fixture presence as sufficient completion evidence.
  - New speculative articles are required to fix this gap; the audit asks for an authoritative expected inventory, and the reviewed launch corpus already provides the bounded release content to inventory.
  - Existing browser/failure tests need to be rewritten; current zero-retry and Firefox/WebKit/responsive coverage already exists and should be revalidated unchanged unless exact-head failures prove otherwise.
  - The inventory may reference WikiLaunchContentCatalog::VERSION directly; that would let catalog version drift self-approve instead of remaining independently pinned.
changed_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
validation:
  - command: bounded whole-diff source/ownership inspection before PR validation
    result: PASS
    evidence: Current branch diff changes exactly the seven claimed paths and no restricted path.
  - command: runtime/browser acceptance
    result: NOT_RUN
    evidence: PR #972 is open; exact-head CI and browser workflows are now the next gate.
blockers: []
next_action: Validate PR #972 exact head, repair any formatting/static/test/inventory/link findings on the same PR, then run exact-final whole-diff review and applicable zero-retry cross-browser Wiki acceptance before deciding whether Issue #488 is terminal.
```
