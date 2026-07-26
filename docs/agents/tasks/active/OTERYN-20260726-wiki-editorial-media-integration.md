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
- [ ] Trusted Wiki article editors can discover and insert existing approved EditorialMedia objects without gaining upload or deletion authority implicitly.
- [ ] Wiki create, update and revision-restore paths validate referenced media and synchronize bounded `EditorialMediaConsumer::WIKI` references transactionally.
- [ ] Removing a media token releases its reference, while referenced objects remain protected by the existing fail-closed deletion boundary.
- [ ] Stale article writes still return HTTP 409 and never mutate media references from the rejected request.
- [ ] Public Wiki rendering accepts only the canonical local media syntax; remote, malformed, unknown and unsupported image targets remain inert or fail closed without XSS or URL injection.
- [ ] Public image bytes are served from the existing private disk only after integrity verification and only when an effective published Wiki translation currently references the object.
- [ ] Draft, review, archived, future-published, missing-locale and stale Polish translations cannot authorize public image delivery.
- [ ] Public delivery uses verified MIME, `nosniff`, bounded cache semantics and truthful not-found/unavailable behavior without exposing private paths.
- [ ] Signed administrator preview can render authorized referenced media without creating an anonymous draft-media route.
- [ ] Focused authorization, publication leakage, storage-integrity, reference-sync, deletion-safety, rendering, responsive and browser-accessibility regressions pass on the exact final head.
- [ ] No new upload format, executable/public storage path, wildcard permission, arbitrary HTML, Canary/login-server change, production action or external-repository write is introduced.

## Ownership

```yaml
owned_paths:
  - app/EditorialMedia/Application/**Wiki**
  - app/EditorialMedia/Http/Public/**
  - app/Wiki/Application/**Media**
  - app/Wiki/Infrastructure/Rendering/**Media**
  - app/Wiki/Application/WikiAdminArticleWriter.php
  - app/Wiki/Http/Admin/AdminWikiArticleController.php
  - routes/modules/wiki.php
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/admin/wiki/articles/preview.blade.php
  - resources/views/wiki/article.blade.php
  - public/css/wiki-admin.css
  - public/css/wiki.css
  - tests/Feature/EditorialMedia/WikiEditorialMedia*.php
  - tests/Feature/Wiki/WikiEditorialMedia*.php
  - tests/Unit/Wiki/*Media*RendererTest.php
  - scripts/acceptance/seed-admin-wiki-permissions.php
  - scripts/acceptance/seed-public-wiki.php
  - scripts/acceptance/tests/wiki-editorial-media*.spec.mjs
  - scripts/acceptance/playwright.config.mjs
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
  - current session lacks a mounted writable checkout for runtime implementation and validation
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T09:45:00Z
head: 9f048cf29b93b3c24b1960c5a6d349e659b31d90
branch: feat/OTERYN-20260726-wiki-editorial-media-integration
pr: 199
status: blocked
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
  - trusted base main is ef8d0fc2454f59a707e14f39c22d502612677734 after merged PR 202
  - draft PR 199 is the only open implementation owner for the bounded Wiki-to-EditorialMedia consumer scope
  - duplicate PR 200 is closed and contains no runtime implementation
  - programme checkpoint now identifies PR 199 as authoritative
  - PR 199 includes trusted main through merge commit 9f048cf29b93b3c24b1960c5a6d349e659b31d90
  - ADR 0014 accepts canonical Markdown targets of the exact form wiki-media:<positive-decimal-id>
  - ADR 0014 makes bounded contextual Markdown alternative text authoritative and uses EditorialMedia alt_text only as an insertion default
  - ADR 0014 assigns one current-reference row per translation and media using consumer_id translation:<translation-id> and usage body.<media-id>
  - ADR 0014 excludes historical revisions from deletion-blocking references and requires restore-time revalidation
  - ADR 0014 requires effective published-locale authorization on every public media request and public no-cache revalidation with immutable ETag
  - ADR 0014 keeps draft preview authenticated, confirmed-MFA, exact-permission and short-lived signed
  - EditorialMedia accepts only normalized JPEG, PNG and WebP objects stored on the private editorial_media disk
  - EditorialMedia storage and integrity fields are immutable and safe deletion refuses any referenced object
  - EditorialMediaConsumer reserves the exact WIKI consumer value
  - EditorialMediaReferenceManager validates bounded identifiers and transactionally attaches or releases usage slots
  - current EditorialMedia byte delivery is administrator-only and verifies path, byte size and SHA-256 before responding
  - current public Wiki renderer replaces every CommonMark image with an inert placeholder
  - public Wiki reads and search expose only effective published locale content and reject stale Polish translations
  - Wiki administration create, update and revision restore already use existing lifecycle, optimistic locking, exact permissions, MFA and audit boundaries
  - no schema, session or compatibility change with Canary or login-server is required by the accepted design
  - the pre-ADR documentation head 320d45a1dc8edf8f33fd26f5e125217be265c0fb passed CI, Agent Governance, Platform DB Outage Validation, Phase 7 Production-Like Validation and Game Auth Ticket Concurrency
  - the sandbox has no mounted Oteryn checkout and cannot resolve github.com to clone one
  - no external repository, production, router, DSM or Internet-exposure write occurred
derived:
  - public image authorization depends on an effective published Wiki translation reference rather than media existence alone
  - draft-time reference synchronization reuses the existing reference manager while public delivery independently enforces publication state
  - upload and deletion authority remains under media.manage while Wiki editing uses its existing exact article permission
  - runtime implementation can proceed without a migration, permission grant or cross-repository contract change
  - a CODEX-capable writable checkout with repository network/dependency access is required for the multi-file implementation, formatter, tests and browser acceptance
unknown: []
conflicts: []
first_failure:
  marker: execution-capability
  evidence: no checkout exists under /mnt/data and `git ls-remote https://github.com/blakinio/Oteryn-Platform.git HEAD` fails with `Could not resolve host: github.com`
rejected_hypotheses:
  - expose the private storage disk through public/storage: rejected by ADR 0011 and ADR 0014
  - allow arbitrary remote CommonMark images: rejected by ADR 0012 and ADR 0014
  - use media-record alternative text as rendered truth: rejected because it is not contextual or localized
  - keep deletion-blocking references for historical revisions: rejected because non-current content must not create indefinite locks
  - attach media only when publishing: rejected because draft updates and restores require deterministic deletion protection
  - use a positive public cache lifetime: rejected because unpublish and reference removal must take effect on the next request
changed_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media-integration.md
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
validation:
  - command: repository, task, pull-request and focused source reconciliation
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
  - command: locate mounted repository checkout
    result: BLOCKED
    evidence: no Oteryn Platform checkout exists under /mnt/data
  - command: git ls-remote https://github.com/blakinio/Oteryn-Platform.git HEAD
    result: BLOCKED
    evidence: container DNS cannot resolve github.com
  - command: local implementation, formatter, static analysis, feature tests and browser acceptance
    result: BLOCKED
    evidence: writable checkout and repository network access are unavailable in the current sandbox
blockers:
  - writable CODEX-capable checkout unavailable in the current session
next_action: Continue PR 199 in a CODEX-capable writable checkout, implement ADR 0014 with focused failing tests first, and run focused plus required final validation.
```

## Notes

The first consumer is Wiki only. Events and CMS integration, localized media metadata, new upload formats, public original-file retention and production deployment remain separate reviewed work.
