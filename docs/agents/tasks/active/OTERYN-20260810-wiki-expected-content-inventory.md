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

Repair the remaining high-severity Wiki completeness gap in Issue #488 by turning the reviewed `WikiLaunchContentCatalog` launch scope into an explicit versioned, machine-verifiable expected-content contract consumed by the canonical Portal Exhaustive Audit, then revalidate the Issue's current failure/recovery, accessibility/responsive and browser-portability acceptance on the exact repair head.

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
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
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
- [x] Inventory declares expected locale set, independently pinned catalog version and exact reviewed catalog Git-blob identity, category/article metadata, exact per-article provenance, approved internal-link surface and launch media/fallback policy.
- [x] Canonical `docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json` exists with `status: complete`, expected records and the same versioned contract consumed by `Portal Exhaustive Audit`.
- [x] Runtime validation fails closed if the machine-readable JSON drifts from the PHP inventory or if the exact reviewed catalog source blob changes.
- [x] Validation fails closed for missing/extra/duplicate categories, missing/extra/duplicate article identities/slugs, locale asymmetry, category-reference drift, metadata drift and invalid sort/order expectations.
- [x] Every article has independently pinned non-empty normalized repository source references; unrelated existing files cannot replace reviewed provenance and every referenced path must exist on the checked-out head.
- [x] Internal Markdown links are parsed through CommonMark AST and bounded to approved first-party routes or expected localized Wiki article slugs; inline and reference-style external/unexpected links fail validation.
- [x] Launch content install command executes expected-content validation before publisher lookup/persistence mutation and fails closed on inventory/catalog drift.
- [x] Machine validation can run without a publisher/database mutation through `wiki:launch-content:validate --json` and has focused unit/feature coverage.
- [ ] Existing public Wiki indexing, EN/PL rendering, administration lifecycle, publication feedback, responsive/accessibility behavior and corrupt/missing EditorialMedia fallback remain green on exact-head acceptance.
- [ ] Wiki paths run with zero retry in applicable current acceptance profiles and Firefox/WebKit portability evidence is captured on the exact head.
- [ ] Required exact-head CI, Agent Governance and Portal Exhaustive Audit pass with the HIGH Wiki expected-inventory finding absent.
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
    - machine-readable Portal Exhaustive Audit content-completeness contract
    - internal route/reference and provenance integrity
    - cross-browser public/admin editorial lifecycle acceptance
    - historical intermittent publication-feedback and media-isolation defects
  unknown_or_conflict: []
  rationale: The change promotes the reviewed launch corpus into a release-completeness contract consumed by the repository audit and prevents either runtime or audit evidence from self-approving independent drift.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T00:19:00+02:00
head: 99903a49af320562b4d9d265fe33e700f7c2fa1a
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
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
proven:
  - Issue #488 is the canonical owner for the Wiki completeness audit and explicitly says no duplicate Issue should be created for the expected-inventory gap.
  - The user explicitly authorized autonomous PORTAL-CLOSEOUT execution; Issue comment 5246540485 records bounded current-main implementation authorization for #488.
  - The repair branch was created from protected main d1e2f27d5dee7e3bb650bad38b4d03eaff6d4249 with no competing #488 branch/PR at claim time; PR #972 is the single authoritative delivery PR.
  - Claim extension comment 5246809100 adds the canonical machine-readable inventory path after exact review proved Portal Exhaustive Audit requires it.
  - WikiExpectedContentInventory version 2026-08-10.2 independently pins reviewed catalog version 2026-07-26.1, exact reviewed catalog Git blob 07ff3324a4530958f9f4e164c5f7a2a399a1bb8b, four category identities, thirteen EN/PL article identities and exact per-article provenance.
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json carries the same explicit expected-record contract and is the canonical file consumed by Portal Exhaustive Audit for Wiki CONTENT_COMPLETE.
  - WikiExpectedContentValidator requires machine-readable JSON equality with the runtime inventory, exact reviewed catalog source identity, exact category/article metadata and locales, exact provenance, first-party CommonMark AST links and the declared media policy.
  - A read-only wiki:launch-content:validate command exposes machine PASS/FAIL without publisher or persistence mutation; InstallWikiLaunchContent runs the validator before publisher lookup or installer writes.
  - Focused tests cover exact-match PASS, missing article, metadata/category drift, locale asymmetry, unrelated-existing-file provenance substitution, inline external link, CommonMark reference-style external link and duplicate category identity.
  - Historical exact-head PR validation already proved Wiki reconciliation and cross-browser flows on earlier heads; the expanded audit-linked final package still requires fresh exact-head gates.
derived:
  - The explicit JSON-to-PHP-to-catalog chain prevents a standalone status=complete JSON file from serving as sufficient evidence while actual launch content or runtime expectations drift.
  - Existing browser/failure acceptance should be reused unchanged and revalidated on the final code head rather than rewritten.
unknown:
  - exact-head CI/Playwright/audit results for the expanded audit-linked PR #972 package
  - future post-launch Wiki expansion beyond this versioned launch inventory
  - future editorial media assignments for launch articles
conflicts: []
first_failure:
  marker: wiki-content-completeness-audit-contract-missing
  evidence: Codex exact-head review on 0b9658a5 proved Portal Exhaustive Audit only recognizes docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json; the missing contract is now implemented and runtime-bound, pending exact-head validation.
rejected_hypotheses:
  - The presence of WikiLaunchContentCatalog alone proves CONTENT_COMPLETE; Issue #488 explicitly rejects sample/fixture presence as sufficient completion evidence.
  - New speculative articles are required to fix this gap; the audit asks for an authoritative expected inventory, and the reviewed launch corpus already provides the bounded release content to inventory.
  - A PHP-only inventory closes #488; canonical Portal Exhaustive Audit requires the machine-readable docs/testing inventory contract.
  - Any existing repository file is sufficient provenance; exact per-article provenance sets are independently pinned.
  - Inline Markdown-link regex is sufficient; CommonMark reference links require parser/AST validation.
changed_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
validation:
  - command: prior H1 Wiki Reconciliation Acceptance
    result: PASS
    evidence: Run 31438283245 passed the full production-like zero-retry Wiki reconciliation matrix before the audit-contract extension; final expanded head must revalidate.
  - command: prior H1 browser portability/responsive evidence
    result: PASS
    evidence: Run 31438283218 reached green Chromium, Firefox, WebKit and responsive Wiki coverage before the final audit-contract extension; exact expanded head must revalidate.
  - command: final expanded exact-head validation
    result: NOT_RUN
    evidence: Required CI, Agent Governance, Portal Exhaustive Audit, Wiki reconciliation and full Acceptance E2E will run after this scope-coordination checkpoint.
blockers: []
next_action: Run exact-head CI/Governance/Portal Exhaustive Audit/Wiki reconciliation/full Acceptance on the expanded audit-linked package, repair any material finding on PR #972, then freeze the final runtime head for whole-diff self-review and closeout.
```
