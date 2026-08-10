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
- [x] Canonical Portal Exhaustive Audit workflow executes `wiki:launch-content:validate --json` before audit generation and preserves the PASS report inside the audit artifact; a validator failure cannot produce a green canonical audit artifact.
- [x] Remaining strictness gaps are represented explicitly: real 404/419/500→recovery probes for admin Wiki and EditorialMedia, existing public Wiki 404/429/503→recovery evidence, explicit accessibility/overflow markers, and machine-verified N/A only for read-only CSRF, no-throttle rate-limit, and unlocalized trusted operator UI rules.
- [x] Standard portability profile executes the real EditorialMedia lifecycle in bounded Firefox and WebKit as well as Chromium.
- [ ] Final exact-head Portal Exhaustive Audit contains no material Wiki/EditorialMedia findings owned by #488 and records `wiki-expected-content-validation.json` with status PASS.
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
    - canonical audit workflow/runtime-validator coupling
    - privileged Wiki and EditorialMedia failure/CSRF/recovery acceptance
    - cross-browser EditorialMedia mutation lifecycle
    - historical intermittent Wiki publication/media defects
  unknown_or_conflict: []
  rationale: Completion requires independent content truth and exact executable evidence; neither a standalone JSON status nor a generic browser marker may self-approve a missing surface-specific contract.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T01:17:00+02:00
head: 75d620ab4309341d8470d2fd820c6379618c0bd2
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
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
proven:
  - Issue #488 is the canonical remediation/evidence owner and explicitly forbids creating a duplicate inventory Issue.
  - User authorization for autonomous PORTAL-CLOSEOUT and bounded #488 implementation is recorded in Issue comment 5246540485; deterministic claim is comment 5246544759.
  - Scope extensions are durable in Issue comments 5246809100, 5246907704, 5246976857, 5247075545 and 5247090714.
  - PR #972 is the single authoritative delivery PR from repair/issue-488.
  - Exact inventory chain is JSON -> PHP inventory -> exact reviewed catalog Git blob -> actual catalog categories/articles/locales/provenance/CommonMark links.
  - Codex findings for unpinned bilingual copy, unpinned provenance, regex-only links, missing canonical audit inventory and disconnected canonical audit gate have all been addressed with focused regression/contracts.
  - Portal Exhaustive Audit artifact 9082825304 on 4a2a7ec6 already proved zero Wiki/EditorialMedia findings after strictness closure; final workflow-coupled head must reproduce this while retaining runtime validator PASS evidence.
  - Public Wiki has exact 404, feature 429, browser 503/recovery, EN/PL, accessibility and overflow evidence.
  - New strictness specs add zero-retry admin Wiki and EditorialMedia 404, CSRF 419, forced 500 and restoration probes without modifying production behavior.
  - PORTAL_STRICTNESS_EVIDENCE requires exact source markers for covered dimensions and validates non-applicability from route method/middleware/operator-route properties rather than accepting prose alone.
  - EditorialMedia is included in standard portabilityMatches; 4a2a7ec6 full Acceptance already passed bounded browser portability and responsive profiles before the final audit-workflow-only correction.
  - Canonical Portal Exhaustive Audit workflow now runs the Wiki runtime validator after exact dependency installation and before audit generation, asserts PASS metadata, copies the report into the audit artifact, and its unit test pins this ordering.
derived:
  - The final workflow-only correction cannot change application runtime behavior; previous runtime/browser evidence remains useful H1 evidence, while final exact-head repository checks will verify the audit gate itself and any automatically triggered acceptance workflows.
  - A stale/self-declared Wiki JSON can no longer yield a green canonical audit workflow when PHP inventory/catalog/blob validation fails.
unknown:
  - final exact-head CI/Governance/Audit conclusions after workflow gate addition
  - final exact-head automatically triggered browser conclusions after workflow gate addition
  - future post-launch Wiki expansion beyond the versioned launch inventory
conflicts: []
first_failure:
  marker: canonical-wiki-validator-not-wired-into-audit-gate
  evidence: Codex review 4901648665 / discussion_r3754111622 on 4a2a7ec6 proved the audit artifact could previously be generated from self-declared JSON without executing the runtime Wiki validator.
rejected_hypotheses:
  - A standalone status=complete JSON proves Wiki completion; runtime validator binds it to exact PHP inventory and catalog source, and canonical audit workflow now requires validator PASS.
  - Any existing repository file is acceptable provenance; exact per-article sets are pinned.
  - Inline Markdown regex is sufficient; CommonMark AST is required.
  - Existing consumer portability substitutes for `/admin/media`; the real EditorialMedia lifecycle is included in Firefox/WebKit portability.
  - Missing strictness categories can be silenced by prose; N/A must pass machine topology/middleware rules.
  - Re-running the runtime validator once per Wiki route inside the audit is necessary; one fail-closed canonical workflow preflight before artifact generation proves the same gate without redundant repeated execution.
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
    evidence: run 31440984594 / artifact 9082825304; zero Wiki/EditorialMedia findings
  - command: Full Acceptance on 4a2a7ec6b4f9037080023accd608c30a80217578
    result: NOT_RUN
    evidence: run 31440984704 produced useful H1 portability/responsive evidence but was superseded by the workflow-only head move; exact-final acceptance remains pending.
  - command: final workflow-coupled exact-head gates
    result: NOT_RUN
    evidence: pending after this ownership checkpoint
blockers: []
next_action: Validate the workflow-coupled exact head through Governance, required CI and canonical Portal Exhaustive Audit; verify the audit artifact contains validator PASS plus zero Wiki/EditorialMedia findings, then finish exact-final self-review/review-thread resolution and merge/closeout.
```
