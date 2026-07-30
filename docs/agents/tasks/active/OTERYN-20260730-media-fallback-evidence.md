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

- [ ] Every covered rendered surface is explicitly classified as `media_consumer`, `not_applicable` or a justified supporting endpoint.
- [ ] Every applicable normal, missing, broken/integrity-failed and no-image state maps to exact executable evidence.
- [ ] Referenced evidence files, stable markers, Playwright projects and npm profiles exist.
- [ ] Public Wiki and administrator Editorial Media render a visible, accessible fallback when a referenced object is absent or fails integrity delivery.
- [ ] Unknown consumers, missing mappings and orphan evidence fail deterministically.
- [ ] Deterministic negative fixtures cover the contract failure modes.
- [ ] Strict Portal Acceptance executes the validator and fixtures.
- [ ] Parent #326 and all production nonclaims remain open.

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
  - public/js/media-fallbacks.js
  - public/css/wiki.css
  - public/css/editorial-media-admin.css
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
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
updated_at: 2026-07-30T15:25:00Z
head: c77425a6c3bff3e808a82421328102c3f2246cbc
branch: test/OTERYN-20260730-media-fallback-evidence
pr: 358
status: implementing
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
  - public/js/media-fallbacks.js
  - public/css/wiki.css
  - public/css/editorial-media-admin.css
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Parent Issue #326 explicitly requires missing/broken media states only on media-consuming surfaces.
  - The delivered rendered media consumers are Public Wiki article media, Wiki administrator media discovery/preview and the administrator Editorial Media library; protected byte routes are supporting endpoints, not rendered UX.
  - Missing or stale Wiki references already render a text placeholder through WikiMediaImageRenderer.
  - A referenced Wiki object whose byte endpoint returns 404 or 503 remains a broken img element because no runtime error fallback is installed.
  - Administrator media thumbnails have the same broken-img behavior when storage is absent or integrity verification fails.
  - Existing zero-retry Chromium desktop/tablet/mobile profiles already execute Editorial Media and responsive Wiki evidence.
  - Current main at task start is eb5736610f4554b196d870d88f4dea2b541db708.
derived:
  - One small shared client-side image-error fallback can preserve localized alt authority for Wiki and expose a bounded administrator preview-unavailable state without weakening private storage or integrity checks.
unknown:
  - Exact final CI evidence on the repaired branch head.
conflicts: []
first_failure:
  marker: broken referenced media remains an img element after byte delivery failure
  evidence: app/Wiki/Infrastructure/Rendering/WikiMediaImageRenderer.php and resources/views/admin/media/index.blade.php emit img elements without an error fallback
rejected_hypotheses:
  - Treat every rendered route as a media consumer.
  - Treat a successful image URL response as proof of visible fallback UX.
  - Treat protected media byte endpoints as standalone rendered surfaces.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
validation:
  - command: GitHub Actions on c77425a6c3bff3e808a82421328102c3f2246cbc
    result: PASS
    evidence: governance, CI, DB outage, edge, phase 7 and game-auth workflows passed for the initial audit-only head
blockers:
  - none
next_action: Implement the shared visible fallback, add deterministic missing/corrupt/no-image browser assertions, then bind them through a strict fail-closed validator and negative fixtures.
```