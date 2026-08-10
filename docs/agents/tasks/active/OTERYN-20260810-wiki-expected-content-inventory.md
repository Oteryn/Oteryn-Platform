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
- [x] Exact-head Portal Exhaustive Audit run `31439187717` proves Wiki `content_complete: PASS`; the former HIGH expected-inventory finding is absent.
- [x] Remaining strictness gaps are represented explicitly: real 404/419/500→recovery probes for admin Wiki and EditorialMedia, existing public Wiki 404/429/503→recovery evidence, explicit accessibility/overflow markers, and machine-verified N/A only for read-only CSRF, no-throttle rate-limit, and unlocalized trusted operator UI rules.
- [x] Standard portability profile now executes the real EditorialMedia lifecycle in bounded Firefox and WebKit as well as Chromium.
- [ ] Final exact-head Portal Exhaustive Audit contains no material Wiki/EditorialMedia findings owned by #488.
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
    - privileged Wiki and EditorialMedia failure/CSRF/recovery acceptance
    - cross-browser EditorialMedia mutation lifecycle
    - historical intermittent Wiki publication/media defects
  unknown_or_conflict: []
  rationale: Completion requires both independent content truth and exact executable evidence; neither a standalone JSON status nor a generic browser marker may self-approve a missing surface-specific contract.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T00:19:00+02:00
head: 3decc2338f4848e765092bd9adbee0ad6aebfee2
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
  - Scope extensions are durable in Issue comments 5246809100, 5246907704 and 5246976857.
  - PR #972 is the single authoritative delivery PR from repair/issue-488.
  - Exact inventory chain is JSON -> PHP inventory -> exact reviewed catalog Git blob -> actual catalog categories/articles/locales/provenance/CommonMark links.
  - Codex findings for unpinned bilingual copy, unpinned provenance, regex-only links and missing canonical audit inventory have all been addressed in code with focused regression coverage.
  - Run 31439187717 proved the HIGH Wiki content-completeness finding is gone but retained five Wiki and five EditorialMedia MEDIUM strictness findings; those exact findings drive the current bounded acceptance/evidence work.
  - Public Wiki already has exact 404, feature 429, browser 503/recovery, EN/PL, accessibility and overflow evidence.
  - New strictness specs add zero-retry admin Wiki and EditorialMedia 404, CSRF 419, forced 500 and restoration probes without modifying production behavior.
  - PORTAL_STRICTNESS_EVIDENCE requires exact source markers for covered dimensions and validates non-applicability from route method/middleware/operator-route properties rather than accepting prose alone.
  - EditorialMedia is added to standard portabilityMatches; dimension contract now classifies portability covered by standard-portability Firefox/WebKit projects.
derived:
  - A fresh exact-head audit should remove the ten Issue #488 MEDIUM findings only if the new evidence contract and route-topology rules validate; any unsupported N/A fails the audit itself.
  - Existing long lifecycle tests remain unchanged, reducing regression risk while dedicated strictness probes remain independently maintainable.
unknown:
  - final exact-head outcomes after strictness contract and dedicated acceptance probes
  - future post-launch Wiki expansion beyond the versioned launch inventory
conflicts: []
first_failure:
  marker: issue-488-strictness-evidence-not-explicit
  evidence: Audit artifact 9082179805 from run 31439187717 had zero HIGH Wiki inventory findings but retained exactly 10 MEDIUM Wiki/EditorialMedia strictness findings.
rejected_hypotheses:
  - A standalone status=complete JSON proves Wiki completion; runtime validator now binds it to exact PHP inventory and catalog source.
  - Any existing repository file is acceptable provenance; exact per-article sets are pinned.
  - Inline Markdown regex is sufficient; CommonMark AST is required.
  - Existing consumer portability substitutes for `/admin/media`; the real EditorialMedia lifecycle is now included in Firefox/WebKit portability.
  - Missing strictness categories can be silenced by prose; N/A must pass machine topology/middleware rules.
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
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
validation:
  - command: CI on 48bbd6a495acec1989074c78c58f2efdafaae427
    result: PASS
    evidence: run 31439187744
  - command: Agent Governance on 48bbd6a495acec1989074c78c58f2efdafaae427
    result: PASS
    evidence: run 31439187710
  - command: Wiki Reconciliation on 48bbd6a495acec1989074c78c58f2efdafaae427
    result: PASS
    evidence: run 31439187663
  - command: Portal Exhaustive Audit on 48bbd6a495acec1989074c78c58f2efdafaae427
    result: PASS
    evidence: run 31439187717 / artifact 9082179805; HIGH inventory finding removed, ten MEDIUM strictness findings remain the current target
  - command: final strictness-expanded exact-head gates
    result: NOT_RUN
    evidence: pending after this ownership checkpoint
blockers: []
next_action: Run exact-head governance, CI and Portal Exhaustive Audit first; fix any contract/schema/source-marker failure, then require Wiki Reconciliation, Editorial Media Acceptance and full Acceptance E2E including Firefox/WebKit before final self-review and merge.
```
