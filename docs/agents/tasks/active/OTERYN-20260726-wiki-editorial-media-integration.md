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

- [ ] A durable ADR defines the canonical Wiki media-reference syntax, localized alternative-text rule, public delivery authorization and cache behavior.
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
updated_at: 2026-07-26T09:20:00Z
head: dc65cf0c33df96ab56e3e185147b5303d7c09d36
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
  - trusted base main is 57716094cde335a0e8a661953bd3a5809ec12cb6
  - draft PR 199 is the only open implementation owner for the bounded Wiki-to-EditorialMedia consumer scope
  - EditorialMedia accepts only normalized JPEG, PNG and WebP objects stored on the private editorial_media disk
  - EditorialMedia storage and integrity fields are immutable and safe deletion refuses any referenced object
  - EditorialMediaConsumer reserves the exact WIKI consumer value
  - EditorialMediaReferenceManager validates bounded identifiers and transactionally attaches or releases usage slots
  - current EditorialMedia byte delivery is administrator-only and verifies path, byte size and SHA-256 before responding
  - ADR 0011 explicitly requires a separate public consumer authorization, route and cache decision
  - current public Wiki renderer replaces every CommonMark image with an inert placeholder
  - public Wiki reads and search expose only effective published locale content and reject stale Polish translations
  - Wiki administration create, update and revision restore already use existing lifecycle, optimistic locking, exact permissions, MFA and audit boundaries
  - no schema, session or compatibility change with Canary or login-server is required by the proven foundation
  - no external repository, production, router, DSM or Internet-exposure write occurred
derived:
  - public image authorization must depend on an effective published Wiki reference rather than media existence alone
  - draft-time reference synchronization can reuse the existing reference manager while public delivery independently enforces publication state
  - upload and deletion authority can remain under media.manage while Wiki editing uses its existing exact article permission
  - a CODEX-capable writable checkout is required for the multi-file implementation, formatter, tests and browser acceptance
unknown:
  - canonical stored Markdown media-target syntax
  - whether Markdown alt text or the media record alt_text is authoritative for rendered Wiki output
  - exact public cache lifetime and validator policy compatible with unpublish/reference removal
conflicts: []
first_failure:
  marker: execution-capability
  evidence: no Oteryn Platform checkout is mounted under /mnt/data, so runtime source edits and required validation cannot be executed in this session
rejected_hypotheses:
  - expose the private storage disk through public/storage: rejected by ADR 0011 and the established private-disk trust boundary
  - allow arbitrary remote CommonMark images: rejected by ADR 0012 and the current fail-closed renderer
  - attach media only when publishing: insufficient because draft updates, restores and deletion protection require deterministic reference tracking
changed_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media-integration.md
validation:
  - command: repository, task, pull-request and focused source reconciliation
    result: PASS
    evidence: trusted main 57716094cde335a0e8a661953bd3a5809ec12cb6, draft PR 199, PRs 176/194/196, ADRs 0011/0012, EditorialMedia models/reference manager/routes and current blocked Wiki image renderer inspected through GitHub
  - command: local implementation, formatter, static analysis, feature tests and browser acceptance
    result: BLOCKED
    evidence: no mounted writable repository checkout in the current sandbox
blockers:
  - writable CODEX-capable checkout unavailable in the current session
next_action: Continue PR 199 in a CODEX-capable writable checkout, resolve the three recorded design unknowns in ADR 0014, implement the bounded Wiki-to-EditorialMedia integration, and run focused plus required final validation.
```

## Notes

The first consumer is Wiki only. Events and CMS integration, localized media metadata, new upload formats, public original-file retention and production deployment remain separate reviewed work.
