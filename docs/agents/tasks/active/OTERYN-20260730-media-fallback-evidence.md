---
task_id: OTERYN-20260730-media-fallback-evidence
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue #357, parent #326 and open PRs touching media rendering, previews, thumbnails or fallback behavior
  - rendered views and browser specs that display user-visible images or downloadable media previews
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-evidence-dimensions.json
---

# OTERYN-20260730-media-fallback-evidence

## Goal

Deliver Issue #357 as a bounded fail-closed audit, runtime fallback repair and evidence contract for media-consuming rendered surfaces, without claiming the remaining exhaustive state matrix under parent #326.

## Acceptance criteria

- [x] Every covered rendered surface is explicitly classified as `media_consumer`, `not_applicable` or a justified supporting endpoint.
- [x] Every applicable normal, missing, broken/integrity-failed and no-image state maps to exact executable evidence.
- [x] Referenced evidence files, stable markers, Playwright projects and npm profiles exist.
- [x] Public Wiki, Wiki administration and administrator Editorial Media render a visible, accessible fallback when a referenced object is absent or fails integrity delivery.
- [x] Unknown consumers, missing mappings and orphan evidence fail deterministically.
- [x] Deterministic negative fixtures cover the contract failure modes.
- [x] Strict Portal Acceptance executes the validator and fixtures.
- [x] Parent #326 and all production nonclaims remain open.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - app/Wiki/Infrastructure/Rendering/WikiMediaImageRenderer.php
  - resources/views/wiki/article.blade.php
  - resources/views/admin/media/index.blade.php
  - resources/views/admin/wiki/articles/form.blade.php
  - public/js/media-fallbacks.js
  - public/js/wiki-admin-media.js
  - public/css/wiki.css
  - public/css/editorial-media-admin.css
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
  - Wiki
  - EditorialMedia
dependencies:
  - Issue #357
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T16:10:00Z
head: 45d00342c0ac7e5b8a5591a10e9cae74c4e42288
implementation_head: 45d00342c0ac7e5b8a5591a10e9cae74c4e42288
branch: test/OTERYN-20260730-media-fallback-evidence
pr: 358
status: validating
context_routes:
  - agent-governance
  - testing
  - architecture
  - web-cms
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - app/Wiki/Infrastructure/Rendering/WikiMediaImageRenderer.php
  - resources/views/wiki/article.blade.php
  - resources/views/admin/media/index.blade.php
  - resources/views/admin/wiki/articles/form.blade.php
  - public/js/media-fallbacks.js
  - public/js/wiki-admin-media.js
  - public/css/wiki.css
  - public/css/editorial-media-admin.css
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Parent Issue #326 requires missing/broken media states only on actual media-consuming surfaces.
  - Exactly three rendered consumers exist in the delivered inventory: Public Wiki article media, Wiki administrator media discovery/preview and the administrator Editorial Media library.
  - All other canonical delivered surfaces have bounded not-applicable rationales; protected byte routes remain supporting endpoints rather than rendered UX.
  - A referenced Wiki object whose byte endpoint returned 404 or 503 previously remained a broken img element.
  - Administrator media and Wiki picker thumbnails had the same broken-img behavior when storage was absent or integrity verification failed.
  - The shared capture-phase fallback now replaces failed images idempotently with visible role-img content, preserves authoritative alt text and exposes bounded administrator preview-unavailable copy.
  - Deterministic acceptance fixtures remove stored objects or corrupt bytes without deleting metadata or references.
  - Three consumers times four required states produce twelve exact evidence mappings and zero gaps.
  - Strict closure requires strict_closure true, zero gaps and valid canonical surfaces, files, markers, profiles and projects.
  - Eleven negative fixtures fail closed on missing/orphan classifications, supporting-endpoint promotion, missing states/markers, unknown profiles/projects, weak rationales, disabled strict closure and reintroduced gaps.
  - Existing zero-retry Chromium desktop/tablet/mobile profiles execute Editorial Media and responsive Wiki evidence.
  - Current main at task start is eb5736610f4554b196d870d88f4dea2b541db708.
derived:
  - The smallest truthful repair is one shared client-side image-error fallback plus exact rendered-state evidence; weakening private storage or integrity validation is unnecessary.
unknown:
  - Exact final workflow set on the documentation checkpoint head.
conflicts: []
first_failure:
  marker: broken referenced media remained an img element after byte delivery failure
  evidence: original Wiki renderer, media library and Wiki picker emitted img elements without visible delivery-error replacement
rejected_hypotheses:
  - Treat every rendered route as a media consumer.
  - Treat a successful image URL response as proof of visible fallback UX.
  - Treat protected media byte endpoints as standalone rendered surfaces.
  - Weaken integrity failures into successful image responses.
changed_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - app/Wiki/Infrastructure/Rendering/WikiMediaImageRenderer.php
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - public/css/editorial-media-admin.css
  - public/css/wiki.css
  - public/js/media-fallbacks.js
  - public/js/wiki-admin-media.js
  - resources/views/admin/media/index.blade.php
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/wiki/article.blade.php
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/validate-portal-media-strict-closure.mjs
  - scripts/acceptance/package.json
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
validation:
  - command: Strict Portal Acceptance on 5328a3c6dc535907e55a8dc1809c5bd9080428d8
    result: PASS
    evidence: strict portal coverage closure passed with zero media gaps and all negative fixtures
  - command: Editorial Media Acceptance on 5328a3c6dc535907e55a8dc1809c5bd9080428d8
    result: PASS
    evidence: complete zero-retry Chromium desktop/tablet/mobile lifecycle including missing and corrupt stored objects
  - command: GitHub Actions on final documentation checkpoint
    result: NOT_RUN
    evidence: exact final workflow run set must complete before ready-for-review or merge
blockers:
  - none
next_action: Require the exact final checkpoint workflow set to pass, reconcile reviews, update the PR evidence summary and merge without closing parent #326.
```