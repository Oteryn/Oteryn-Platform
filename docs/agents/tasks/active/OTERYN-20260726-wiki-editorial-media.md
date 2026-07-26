---
task_id: OTERYN-20260726-wiki-editorial-media
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/adr/0012-public-wiki-read-search.md
search_first:
  - active tasks and open pull requests with overlapping Wiki, EditorialMedia, route, view or test ownership
  - EditorialMedia consumer enum, reference manager, private serving and deletion invariants
  - Wiki article create, update, restore, preview and publication transaction boundaries
  - restricted CommonMark image rendering and public article query context
  - existing Wiki and EditorialMedia feature, unit and browser acceptance coverage
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-wiki-editorial-media

## Goal

Integrate the existing private EditorialMedia library with Wiki article authoring, preview and published public rendering without exposing arbitrary storage paths, remote images, drafts or unreferenced media.

## Acceptance criteria

- [ ] Trusted Wiki editors can select approved existing EditorialMedia images for Wiki article content without entering storage paths or remote image URLs.
- [ ] Wiki media references are attached, replaced and released transactionally across article create, update and revision-restore workflows while preserving optimistic locking and audit boundaries.
- [ ] Public and signed-preview rendering resolve only valid Wiki-owned references and continue to neutralize remote, malformed, missing or unauthorized image targets.
- [ ] Public image delivery verifies the immutable EditorialMedia integrity metadata before returning bytes and uses safe MIME, disposition, cache and `nosniff` headers.
- [ ] Draft-only media is not exposed through an unsafely guessable public route; published-content access and signed administrator preview remain fail closed.
- [ ] Referenced media remains deletion-protected, while removal or replacement of the final Wiki reference permits the existing safe deletion workflow.
- [ ] Rendered images preserve bounded alternative text, responsive layout, keyboard flow and existing Wiki readability.
- [ ] Focused unit, feature, authorization, storage-failure and browser acceptance regressions cover the integration, followed by the full exact-head validation required by changed paths.
- [ ] No migration, dependency, external-repository, production, router, DSM or Internet-exposure change is introduced unless separately proven necessary and authorized.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
  - app/EditorialMedia/Application/EditorialMediaReferenceManager.php
  - app/EditorialMedia/Application/EditorialMediaFileResponse.php
  - app/Wiki/Application/Media/**
  - app/Wiki/Application/Rendering/**
  - app/Wiki/Http/Admin/AdminWikiArticleController.php
  - app/Wiki/Http/Admin/Requests/AdminWikiArticleRequest.php
  - app/Wiki/Http/Public/PublicWikiMediaController.php
  - app/Wiki/Infrastructure/Rendering/**
  - routes/modules/wiki.php
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/wiki/**
  - public/css/wiki.css
  - public/css/wiki-admin.css
  - tests/Feature/Wiki/**
  - tests/Unit/Wiki/**
  - scripts/acceptance/tests/*wiki*
modules:
  - Wiki
  - EditorialMedia
  - AdminRBAC
  - Localization
  - Testing
dependencies:
  - Issue #145
  - PR #176 / ADR 0011 safe EditorialMedia boundary
  - PR #194 public Wiki reads and restricted rendering
  - PR #196 trusted Wiki administration
blockers:
  - current session has no writable repository checkout with PHP, Composer and browser validation capability
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T09:20:00Z
head: 57716094cde335a0e8a661953bd3a5809ec12cb6
branch: feat/OTERYN-20260726-wiki-editorial-media
pr: none
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
  - localization
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/0014-wiki-editorial-media-integration.md
  - app/EditorialMedia/Application/EditorialMediaReferenceManager.php
  - app/EditorialMedia/Application/EditorialMediaFileResponse.php
  - app/Wiki/Application/Media/**
  - app/Wiki/Application/Rendering/**
  - app/Wiki/Http/Admin/AdminWikiArticleController.php
  - app/Wiki/Http/Admin/Requests/AdminWikiArticleRequest.php
  - app/Wiki/Http/Public/PublicWikiMediaController.php
  - app/Wiki/Infrastructure/Rendering/**
  - routes/modules/wiki.php
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/wiki/**
  - public/css/wiki.css
  - public/css/wiki-admin.css
  - tests/Feature/Wiki/**
  - tests/Unit/Wiki/**
  - scripts/acceptance/tests/*wiki*
proven:
  - trusted main is 57716094cde335a0e8a661953bd3a5809ec12cb6 after merged PR 198
  - Issue 145 remains open and names Wiki-to-EditorialMedia integration as the next bounded implementation child
  - no open implementation pull request owns the Wiki-to-EditorialMedia integration paths
  - EditorialMedia stores normalized JPEG, PNG and WebP objects on a dedicated private disk with immutable integrity metadata
  - EditorialMediaConsumer already reserves the wiki consumer and EditorialMediaReferenceManager provides locked attach and bounded release operations
  - existing EditorialMedia serving is administrator-only and ADR 0011 requires a separate public consumer serving decision
  - the current CommonMark Wiki renderer strips raw HTML, disallows unsafe links and neutralizes every image node
  - Wiki article create and update already run through transactional writer and service boundaries with optimistic locking and append-only revisions
  - public Wiki reads are published-only and administrator preview uses short-lived signed routes
  - no write occurred outside blakinio/Oteryn-Platform and no production or infrastructure action occurred
derived:
  - this integration requires an explicit publication-aware media-resolution and serving boundary rather than exposing the private storage disk
  - media-reference synchronization must compose with existing Wiki transactions rather than run as an unrelated controller-side mutation
  - security-sensitive multi-file implementation and validation require a CODEX-capable writable checkout
unknown:
  - final internal Wiki media reference syntax and stable reference-slot convention
  - whether historical revisions retain deletion-blocking media references or restore missing media as a neutralized placeholder
  - whether rendered alternative text is sourced from the media record, article Markdown or a validated combination
  - exact public cache policy and route identity for immutable media bytes
conflicts: []
first_failure:
  marker: execution-capability
  evidence: git clone from the current sandbox failed because github.com could not be resolved, so application edits, formatter, PHPStan, tests and browser acceptance cannot be executed locally
rejected_hypotheses:
  - public Wiki media is already available: ADR 0011 and the current renderer prove serving and consumer integration were intentionally deferred
  - direct public-disk URLs are acceptable: ADR 0011 explicitly rejects the public storage symlink and arbitrary-path exposure
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260726-wiki-editorial-media.md
validation:
  - command: repository, issue, active-task and pull-request overlap reconciliation
    result: PASS
    evidence: main 57716094cde335a0e8a661953bd3a5809ec12cb6, Issue 145 programme checkpoint, PRs 176, 194, 196 and open PR search
  - command: local checkout and runtime validation preflight
    result: BLOCKED
    evidence: sandbox DNS prevented cloning github.com; no writable checkout or installed project dependencies are available
blockers:
  - writable checkout with PHP, Composer and browser validation capability is unavailable in the current execution environment
next_action: Resume this task in a CODEX-capable writable checkout and implement the smallest complete Wiki-to-EditorialMedia integration beginning with focused failing tests for reference synchronization and publication-aware image rendering.
```

## Notes

Preserve the existing exact Wiki RBAC, confirmed-MFA, optimistic-lock, revision, signed-preview, restricted Markdown and EditorialMedia integrity/deletion boundaries. Do not record image bytes, storage paths, secrets or production values in task evidence.
