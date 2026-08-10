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

Close Issue #488 by making the reviewed Wiki launch corpus independently machine-verifiable, binding that inventory to the canonical Portal Exhaustive Audit, and reconciling every remaining Wiki/EditorialMedia failure, portability, accessibility, locale and overflow finding with exact executable evidence or machine-proven non-applicability.

## Feature scope

```yaml
feature_scope:
  type: content_integrity_and_acceptance_evidence
  complete_user_facing_feature: false
  backend_required: true
  frontend_required: false
  runtime_required: true
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
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tools/audit/portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
restricted_paths:
  - app/Wiki/Models/**
  - app/Wiki/Domain/**
  - app/Wiki/Actions/**
  - database/**
  - routes/**
  - resources/views/**
  - deploy/**
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: wiki:expected-content-inventory
```

## Acceptance inventory

- [x] Independent inventory pins catalog version, exact reviewed catalog Git blob, four categories, thirteen EN/PL article identities, metadata and exact per-article provenance.
- [x] Canonical `docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json` is strictly bound to the PHP inventory before runtime validation or installation.
- [x] CommonMark AST link validation catches inline and reference-style external/unexpected links.
- [x] Install validation occurs before publisher lookup or writes; read-only `wiki:launch-content:validate --json` exposes machine PASS/FAIL.
- [x] Canonical Portal Exhaustive Audit validates the Wiki runtime inventory before generation and waits fail-closed for exact-head Wiki Reconciliation, Editorial Media Acceptance and full Acceptance E2E success; run IDs/conclusions are retained in the audit artifact.
- [x] Strictness gaps are represented explicitly: real 404/419/500→recovery probes for admin Wiki and EditorialMedia, existing public Wiki 404/429/503→recovery evidence, explicit accessibility/overflow markers, and machine-verified N/A only for read-only CSRF, no-throttle rate-limit, and unlocalized trusted operator UI rules.
- [x] Failure injection uses only disposable acceptance MariaDB table renames through existing Laravel fixture commands; no tracked application/view file is renamed or modified.
- [x] Standard portability profile executes the real EditorialMedia lifecycle in bounded Firefox and WebKit as well as Chromium.
- [ ] Final exact-head Portal Exhaustive Audit contains no material Wiki/EditorialMedia findings and records runtime-inventory PASS plus exact-head executed workflow evidence.
- [ ] Final exact-head CI, Agent Governance, Wiki Reconciliation, Editorial Media Acceptance and full Acceptance E2E all pass.
- [ ] Final whole-diff self-review has no open material finding and all PR review threads are terminal.
- [x] No application route/view/schema/deployment/production/credential/external-repository behavior is changed.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2352
  classified_at: 2026-08-10T23:52:00+02:00
  risk: high
  triggers:
    - Issue 488 proven HIGH Wiki completeness gap
    - bilingual published Wiki launch content
    - machine-readable audit completion contract
    - canonical audit runtime-validator and executed-workflow coupling
    - privileged Wiki and EditorialMedia failure/CSRF/recovery acceptance
    - cross-browser EditorialMedia mutation lifecycle
    - historical intermittent Wiki publication/media defects
  unknown_or_conflict: []
  rationale: Completion requires independent content truth plus actually executed exact-head acceptance; static markers or standalone JSON cannot self-approve closure.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T01:31:00+02:00
head: 1ad5c373d6c0fa8d481f3ded90f05c0ba85f8247
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
  - acceptance
  - audit
  - ci-workflow
owned_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tools/audit/portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
proven:
  - Issue #488 is the canonical remediation/evidence owner and PR #972 is the single authoritative delivery PR.
  - User authorization and deterministic claim plus all bounded scope extensions are durable in Issue #488 comments.
  - Exact inventory chain is machine JSON -> PHP inventory -> exact reviewed catalog Git blob -> actual categories/articles/locales/provenance/CommonMark links.
  - Prior Codex findings for unpinned bilingual copy, provenance, reference-style links, missing canonical inventory and disconnected runtime validator have focused fixes/tests.
  - Exact-head artifact 9082825304 on 4a2a7ec6 proved zero Wiki/EditorialMedia findings after inventory/strictness closure, before the final executed-workflow gate.
  - Codex review 4901648665 then identified two remaining evidence-safety gaps: static source markers did not prove tests executed, and tracked Blade renames were unsafe fault injection.
  - Canonical Portal Exhaustive Audit now queries GitHub Actions with `actions: read`, waits for exact-head success of Wiki Reconciliation Acceptance, Editorial Media Acceptance and Acceptance E2E and Visual UX, rejects any failed/cancelled conclusion, and persists exact run evidence before audit generation.
  - Workflow contract unit coverage pins runtime-inventory validation and executed-workflow gate ordering before audit generation.
  - Wiki/EditorialMedia strictness failure injection now renames only disposable acceptance database tables through fixture commands and restores them in before/after/finally paths; checkout files are never mutated.
  - PORTAL_STRICTNESS_EVIDENCE recovery markers are bound to the new database-fixture restore helpers.
  - Existing public Wiki and EditorialMedia lifecycle/browser evidence remains unchanged and must be re-executed on final head.
derived:
  - A canonical audit cannot produce terminal #488 zero-findings evidence unless the same exact head has both runtime inventory PASS and actual required browser workflow PASS results.
  - A killed strictness test can at worst leave a renamed table inside the disposable job database, which is destroyed with the job; it cannot corrupt the source checkout.
unknown:
  - final exact-head gate results after executed-evidence and safe-fault-injection changes
  - future post-launch Wiki expansion beyond this versioned launch inventory
conflicts: []
first_failure:
  marker: strictness-execution-evidence-and-fault-injection-safety
  evidence: Codex review 4901648665 findings 3754111625 and 3754111628 on 4a2a7ec6.
rejected_hypotheses:
  - Static source markers prove browser execution; canonical audit now requires exact-head workflow success.
  - Tracked Blade renames are acceptable fault injection; they are replaced by disposable DB fixture operations.
  - A standalone status=complete JSON proves Wiki completion; runtime validator and canonical audit gate both fail closed.
  - Any existing file is acceptable provenance; exact per-article sets are pinned.
  - Inline Markdown regex is sufficient; CommonMark AST is required.
changed_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tools/audit/portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
validation:
  - command: CI on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: PASS
    evidence: run 31440984623
  - command: Agent Governance on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: PASS
    evidence: run 31440984618
  - command: Wiki Reconciliation on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: PASS
    evidence: run 31440984596
  - command: Editorial Media Acceptance on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: PASS
    evidence: run 31440984578
  - command: Portal Exhaustive Audit on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: PASS
    evidence: run 31440984594 / artifact 9082825304; zero Wiki/EditorialMedia findings before final execution-gate correction
  - command: final exact-head gates
    result: NOT_RUN
    evidence: pending after this task checkpoint
blockers: []
next_action: Freeze the new head; run exact-head Governance, required CI, Wiki Reconciliation, Editorial Media Acceptance and full Acceptance; require canonical Audit to consume those exact successes and produce validator/execution evidence plus zero #488 findings; then complete self-review/review-thread resolution and merge/closeout.
```
