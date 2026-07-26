---
task_id: OTERYN-20260726-wiki-editorial-media-integration
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/adr/0012-public-wiki-read-search.md
  - docs/architecture/adr/0013-wiki-administration.md
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
  - docs/agents/tasks/archive/OTERYN-20260725-safe-editorial-media.md
  - docs/agents/tasks/archive/OTERYN-20260726-wiki-administration.md
search_first:
  - active tasks and open pull requests touching Wiki, EditorialMedia, public image delivery, Markdown rendering, administrator article editing or acceptance tests
  - existing EditorialMedia reference, storage-integrity, deletion, authorization and audit boundaries
  - existing Wiki create, update, restore, preview, publication-freshness and rendering services
  - current public route, response cache, dependency-failure and image accessibility conventions
  - exact role bundles for wiki.articles.manage, wiki.publish and media.manage
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-wiki-editorial-media-integration

## Goal

Integrate the existing private, normalized EditorialMedia image library into trusted Wiki editing and published Wiki rendering without duplicating upload processing, weakening publication boundaries, exposing draft media, enabling remote images or changing Canary/login-server contracts.

## Acceptance criteria

- [x] A durable ADR defines the canonical Wiki media-reference syntax, localized alternative-text rule, public delivery authorization and cache behavior.
- [x] Trusted Wiki article editors can discover and insert existing approved EditorialMedia objects without gaining upload or deletion authority implicitly.
- [x] Wiki create, update and revision-restore paths validate referenced media and synchronize bounded `EditorialMediaConsumer::WIKI` references transactionally.
- [x] Removing a media token releases its reference, while referenced objects remain protected by the existing fail-closed deletion boundary.
- [x] Stale article writes still return HTTP 409 and never mutate media references from the rejected request.
- [x] Public Wiki rendering accepts only the canonical local media syntax; remote, malformed, unknown and unsupported image targets remain inert or fail closed without XSS or URL injection.
- [x] Public image bytes are served from the existing private disk only after integrity verification and only when an effective published Wiki translation currently references the object.
- [x] Draft, review, archived, future-published, missing-locale and stale Polish translations cannot authorize public image delivery.
- [x] Public delivery uses verified MIME, `nosniff`, bounded cache semantics and truthful not-found/unavailable behavior without exposing private paths.
- [x] Signed administrator preview can render authorized referenced media without creating an anonymous draft-media route.
- [x] Focused authorization, publication leakage, storage-integrity, reference-sync, deletion-safety, rendering, responsive and browser-accessibility regressions pass on the exact final head.
- [x] No new upload format, executable/public storage path, wildcard permission, arbitrary HTML, Canary/login-server change, production action or external-repository write is introduced.

## Ownership

```yaml
owned_paths:
  - app/EditorialMedia/Application/**Wiki**
  - app/EditorialMedia/Http/Public/**
  - app/Wiki/Application/**Media**
  - app/Wiki/Application/Rendering/WikiMarkdownRenderer.php
  - app/Wiki/Infrastructure/Rendering/**Media**
  - app/Wiki/Infrastructure/Rendering/BlockedWikiImageRenderer.php
  - app/Wiki/Infrastructure/Rendering/CommonMarkWikiRenderer.php
  - app/Wiki/Application/WikiAdminArticleWriter.php
  - app/Wiki/Http/Admin/AdminWikiArticleController.php
  - app/Wiki/Http/Admin/AdminWikiMediaController.php
  - app/Wiki/Http/Public/PublicWikiController.php
  - app/Wiki/Queries/Public/DatabasePublicWikiQuery.php
  - app/Wiki/ViewModels/Public/WikiArticlePageViewModel.php
  - routes/modules/wiki.php
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/admin/wiki/articles/preview.blade.php
  - resources/views/wiki/article.blade.php
  - public/css/wiki-admin.css
  - public/css/wiki.css
  - public/js/wiki-admin-media.js
  - tests/Feature/EditorialMedia/WikiEditorialMedia*.php
  - tests/Feature/Wiki/WikiEditorialMedia*.php
  - tests/Unit/Wiki/*Media*RendererTest.php
  - scripts/acceptance/seed-admin-wiki-permissions.php
  - scripts/acceptance/seed-wiki-editorial-media.php
  - scripts/acceptance/seed-public-wiki.php
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - .github/workflows/acceptance-validation.yml
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media-integration.md
modules:
  - Wiki
  - EditorialMedia
  - AdminRBAC
  - Testing
dependencies:
  - Issue #145
  - PR #176 safe EditorialMedia foundation
  - PR #194 public Wiki read/render/search
  - PR #196 Wiki administration
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T12:08:27Z
head: e84cb22fb5666b05929581d61cc3e7c9d14de916
branch: feat/OTERYN-20260726-wiki-editorial-media-integration
pr: 199
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - admin-rbac
  - database
  - security
  - testing
  - accessibility
owned_paths:
  - Wiki-to-EditorialMedia integration paths listed in Ownership
proven:
  - trusted base main is 97e8b8cbc62d9c359f71b24228d689612d79c276 after merged PRs 203 and 204
  - draft PR 199 is the only open implementation owner for the bounded Wiki-to-EditorialMedia consumer scope
  - duplicate PR 200 is closed and contains no runtime implementation
  - programme checkpoint now identifies PR 199 as authoritative
  - local PR 199 implementation is cleanly rebased on trusted main 97e8b8cbc62d9c359f71b24228d689612d79c276
  - ADR 0014 accepts canonical Markdown targets of the exact form wiki-media:<positive-decimal-id>
  - ADR 0014 makes bounded contextual Markdown alternative text authoritative and uses EditorialMedia alt_text only as an insertion default
  - ADR 0014 assigns one current-reference row per translation and media using consumer_id translation:<translation-id> and usage body.<media-id>
  - ADR 0014 excludes historical revisions from deletion-blocking references and requires restore-time revalidation
  - ADR 0014 requires effective published-locale authorization on every public media request and public no-cache revalidation with immutable ETag
  - ADR 0014 keeps draft preview authenticated, confirmed-MFA, exact-permission and short-lived signed
  - PR 199 has no review comments or review threads
  - EditorialMedia accepts only normalized JPEG, PNG and WebP objects stored on the private editorial_media disk
  - EditorialMedia storage and integrity fields are immutable and safe deletion refuses any referenced object
  - EditorialMediaConsumer reserves the exact WIKI consumer value
  - EditorialMediaReferenceManager validates bounded identifiers and transactionally attaches or releases usage slots
  - current EditorialMedia byte delivery is administrator-only and verifies path, byte size and SHA-256 before responding
  - the context-free renderer remains inert while validated translation contexts resolve only exact canonical referenced media
  - public Wiki reads and search expose only effective published locale content and reject stale Polish translations
  - Wiki administration create, update and revision restore already use existing lifecycle, optimistic locking, exact permissions, MFA and audit boundaries
  - no schema, session or compatibility change with Canary or login-server is required by the accepted design
  - the pre-ADR documentation head 320d45a1dc8edf8f33fd26f5e125217be265c0fb passed CI, Agent Governance, Platform DB Outage Validation, Phase 7 Production-Like Validation and Game Auth Ticket Concurrency
  - media.manage belongs only to content_editor and platform_admin while Wiki permissions remain separately exact and are not wildcard grants
  - the acceptance Wiki administrator receives exact Wiki permissions without media.manage
  - no external repository, production, router, DSM or Internet-exposure write occurred
  - PR 203 repaired the pre-existing order-dependent TrustedProxySchemeTest and squash-merged as 835db2c789699040babad4859051511673123785
  - PR 204 archived the completed prerequisite task and squash-merged as 97e8b8cbc62d9c359f71b24228d689612d79c276
  - exact Wiki editor permissions expose a read-only approved-media picker and verified thumbnails without media.manage, upload or deletion routes
  - current translation references use consumer_id translation:<translation-id> and usage body.<media-id> exactly as ADR 0014 requires
  - create, update and restore parse every image node, validate all referenced private-disk media before reference mutation and reconcile current translation references inside the outer article transaction
  - public delivery joins the referenced translation to the effective publication query and reparses its current source before integrity-checked private-disk delivery
  - preview media URLs are signed over article, locale, translation and media identifiers and the controller rechecks their current relationship and exact reference
  - successful public media uses no-cache max-age=0 revalidation while unauthorized and unavailable responses are no-store
  - exact-head Acceptance run 30200877625 passed smoke and 18 existing portability tests but failed the new Wiki media scenario in all three browser engines at keyboard insertion
  - non-append picker requests now remove stale result controls before awaiting replacement data
  - exact-head Acceptance run 30201158173 passes smoke, Chromium Firefox and WebKit portability, desktop tablet and mobile responsive coverage, resilience and keyboard accessibility
derived:
  - public image authorization depends on an effective published Wiki translation reference rather than media existence alone
  - draft-time reference synchronization reuses the existing reference manager while public delivery independently enforces publication state
  - upload and deletion authority remains under media.manage while Wiki editing uses its existing exact article permission
  - runtime implementation can proceed without a migration, permission grant or cross-repository contract change
  - responsive, portability and accessibility projects now select the bounded admin Wiki media Playwright scenario by filename
  - non-append picker requests must remove stale cards before awaiting replacement data so disposable controls cannot retain transient keyboard focus
unknown: []
conflicts:
  - none
first_failure:
  marker: none
  evidence: the stale-card failure from run 30200877625 is fixed and exact-head run 30201158173 passes every required browser profile
rejected_hypotheses:
  - expose the private storage disk through public/storage: rejected by ADR 0011 and ADR 0014
  - allow arbitrary remote CommonMark images: rejected by ADR 0012 and ADR 0014
  - use media-record alternative text as rendered truth: rejected because it is not contextual or localized
  - keep deletion-blocking references for historical revisions: rejected because non-current content must not create indefinite locks
  - attach media only when publishing: rejected because draft updates and restores require deterministic deletion protection
  - use a positive public cache lifetime: rejected because unpublish and reference removal must take effect on the next request
  - continue duplicate PR 200: rejected because the user explicitly selected PR 199 and overlapping ownership is prohibited
  - rely on implicit route binding without a typed model argument: rejected after focused tests proved Laravel leaves the route value as a scalar in that controller shape
  - keep the trusted-proxy isolation repair inside PR 199: rejected because it was a pre-existing unrelated main defect and repository policy required a narrow prerequisite PR
  - browser-engine-specific keyboard activation defect: rejected because Chromium, Firefox and WebKit failed identically and the picker exposes stale controls during its asynchronous replacement request
changed_paths:
  - app/EditorialMedia Wiki response and public route boundary
  - app/Wiki media parser, reference, rendering, article-write, preview and public-read boundaries
  - Wiki admin/public CSS, picker JavaScript and article form
  - routes/modules/wiki.php
  - focused PHPUnit and Playwright acceptance coverage
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media-integration.md
validation:
  - command: required reads, search_first reconciliation, repository, task and pull-request preflight
    result: PASS
    evidence: trusted main ef8d0fc2454f59a707e14f39c22d502612677734, draft PR 199, closed duplicate PR 200, PRs 176/194/196, ADRs 0011-0014 and current source inspected through GitHub
  - command: architecture decision review
    result: PASS
    evidence: ADR 0014 resolves token syntax, alt-text authority, reference identity, revision behavior, public authorization/cache and signed-preview boundaries without schema or permission expansion
  - command: authoritative programme correction
    result: PASS
    evidence: PR 202 merged as ef8d0fc2454f59a707e14f39c22d502612677734 and records PR 199 as sole owner
  - command: synchronize authoritative child with trusted main
    result: PASS
    evidence: merge commit 9f048cf29b93b3c24b1960c5a6d349e659b31d90 includes main ef8d0fc2454f59a707e14f39c22d502612677734
  - command: exact-head workflows before ADR addition
    result: PASS
    evidence: commit 320d45a1dc8edf8f33fd26f5e125217be265c0fb; runs 30196315208, 30196315212, 30196315214, 30196315211 and 30196315215
  - command: composer validate --strict and composer audit --no-interaction
    result: PASS
    evidence: composer metadata is valid and no security vulnerability advisories were reported
  - command: vendor/bin/pint --test
    result: PASS
    evidence: all 434 PHP files pass formatting on rebased head 0fdca21780bcffdfdecc24ff40dd0d6552a49edd
  - command: vendor/bin/phpstan analyse --memory-limit=1G
    result: PASS
    evidence: level-10 analysis passes with no errors on rebased head 0fdca21780bcffdfdecc24ff40dd0d6552a49edd
  - command: focused Wiki EditorialMedia unit and feature tests
    result: PASS
    evidence: 31 tests and 341 assertions pass on rebased head 0fdca21780bcffdfdecc24ff40dd0d6552a49edd for strict parsing/rendering, translation-owned reference synchronization, rollback/stale/restore/deletion safety, public/preview authorization, storage integrity and existing Wiki administration/public-read regressions
  - command: implementation commit
    result: PASS
    evidence: commit 9d20769c2972d2279e55811b1c14bc3ae882f897 is rebased on authoritative PR head 5e395cbc03824f6f7a944ceac692a86859ada08f and contains the ADR 0014 implementation
  - command: php artisan route:list --path=wiki --except-vendor
    result: PASS
    evidence: all 32 legacy, localized public and exact administrator Wiki routes register without collision
  - command: node --check on picker, Playwright spec and config; git diff --check; checkpoint validator and validator tests
    result: PASS
    evidence: syntax, whitespace and governance checks pass
  - command: composer test
    result: BLOCKED
    evidence: the wrapper exceeded its 300-second timeout on the superseded head; direct full-suite validation follows after merged PR 203 fixed the independently proven order-dependent proxy regression
  - command: prerequisite trusted-proxy test-isolation lifecycle
    result: PASS
    evidence: PR 203 full suite passed 331 tests and 2473 assertions; all exact-head checks passed and PR 204 archived the task
  - command: php artisan test --display-warnings
    result: PASS
    evidence: 349 tests and 2679 assertions pass with 10 documented skips on rebased head 0fdca21780bcffdfdecc24ff40dd0d6552a49edd
  - command: node syntax, route registration, whitespace and agent-governance validation
    result: PASS
    evidence: picker, Playwright and config syntax pass; 32 Wiki routes register; 11 checkpoints and 9 validator tests pass; diff check is clean
  - command: exact-head GitHub CI and browser acceptance
    result: PASS
    evidence: final runtime head e84cb22fb5666b05929581d61cc3e7c9d14de916 passes CI 30201158151, governance 30201158153, concurrency 30201158175, outage 30201158148, Phase 7 30201158167, build 30201158150 and Acceptance 30201158173
blockers:
  - none
next_action: Publish this ready checkpoint, verify the docs-only final head checks, mark PR 199 ready and squash merge it.
```

## Notes

The first consumer is Wiki only. Events and CMS integration, localized media metadata, new upload formats, public original-file retention and production deployment remain separate reviewed work.
