---
task_id: OTERYN-20260725-safe-editorial-media
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
search_first:
  - existing image upload or media abstractions
  - active tasks and open pull requests for overlapping routes, storage, admin, RBAC and audit paths
  - deployed PHP image capabilities and persistent storage topology
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260725-safe-editorial-media

## Goal

Implement an isolated reusable secure editorial raster-image library for later Wiki, Events and CMS consumption without integrating those consumers in this pull request.

## Acceptance criteria

- [x] Dedicated Platform-owned media and bounded reference records exist.
- [x] Only JPEG, PNG and WebP with matching extension, detected MIME and decodable content are accepted.
- [x] Uploads enforce byte, dimension and decoded-pixel limits before decode and fail closed when processing or storage is unavailable.
- [x] Accepted images are decoded, re-encoded, stripped of metadata, stored under immutable random names and recorded with SHA-256.
- [x] Bounded thumbnails are generated only for images larger than the administrator preview boundary.
- [x] Alt text is required and bounded.
- [x] One exact media-management permission protects the administrator library behind authentication and confirmed MFA.
- [x] Upload and deletion operations append bounded non-secret administrator audit events.
- [x] Referenced media cannot be deleted and database constraints preserve the same invariant.
- [x] Malicious, malformed, mismatched and over-limit fixtures are covered together with permission, MFA and CSRF regressions.
- [ ] Required CI passes on the exact current head.

## Ownership

```yaml
owned_paths:
  - app/EditorialMedia/**
  - app/Http/Controllers/Admin/AdminEditorialMediaController.php
  - app/Http/Requests/Admin/AdminEditorialMediaUploadRequest.php
  - app/Admin/AdminPermission.php
  - config/editorial_media.php
  - config/filesystems.php
  - database/migrations/*editorial_media*
  - resources/views/admin/media/**
  - resources/views/admin/layout.blade.php
  - public/css/editorial-media-admin.css
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/**
  - .github/workflows/ci.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-media.ini
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260725-safe-editorial-media.md
modules:
  - EditorialMedia
  - Admin
  - Audit
dependencies:
  - PUBLIC_WEBSITE_EXPANSION_PLAN.md Slice 8
  - WIKI_IMPLEMENTATION_PLAN.md Slice 7 media security boundary
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T10:01:00Z
head: UNKNOWN
branch: feat/OTERYN-20260725-safe-editorial-media
pr: 176
status: validating
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - admin-rbac
  - database
  - security
  - testing
owned_paths:
  - paths listed in Ownership
proven:
  - PR 176 contains the isolated EditorialMedia schema, processing boundary, administrator library, exact permission, bounded audit and reference-safe deletion
  - accepted source formats are restricted to JPEG, PNG and WebP with extension, fileinfo MIME, image-header and exact container-boundary agreement
  - accepted source bytes are decoded and re-encoded through GD before private storage, and source metadata is not retained
  - the dedicated editorial_media disk is private, outside the public storage symlink and configured to throw and report storage failures
  - generated storage names use 192 bits of randomness and immutable normalized extensions
  - consumer references are restricted to cms, events and wiki and database deletion is restricted while references exist
  - every administrator route requires auth, mfa.confirmed and the exact media.manage permission
  - content_editor and platform_admin receive media.manage through an explicit reviewed migration; no wildcard authority is introduced
  - the Platform deployment image and CI now require GD JPEG, PNG and WebP codec support
  - the branch was synchronized with main commit bd0bd9883e2753c8a385b3297aaed7a1cb2ce429 through GitHub merge commit 30b87ea9f5c7d0d5700309a008e98ce90ff08bfc
  - EditorialMedia persistence is Platform-owned and does not change Canary or login-server contracts
derived:
  - the implementation is isolated and can be consumed later without transferring Wiki, Events or CMS publication rules into the media module
unknown:
  - exact required CI result on the checkpoint commit
conflicts: []
first_failure:
  marker: none
  evidence: exact-head CI has not completed yet
rejected_hypotheses:
  - reuse current public disk: it is publicly linked and configured with throw=false
  - add a Wiki-specific upload surface: the requested boundary must be reusable and consumer integration is out of scope
  - accept SVG or arbitrary files: prohibited by task and security architecture
  - retain original uploads after validation: decode and re-encode is required to remove metadata and appended active payloads
changed_paths:
  - app/EditorialMedia/**
  - app/Http/Controllers/Admin/AdminEditorialMediaController.php
  - app/Http/Requests/Admin/AdminEditorialMediaUploadRequest.php
  - app/Admin/AdminPermission.php
  - config/editorial_media.php
  - config/filesystems.php
  - database/migrations/2026_07_25_090000_create_editorial_media_tables.php
  - database/migrations/2026_07_25_090100_add_editorial_media_permission.php
  - resources/views/admin/media/index.blade.php
  - resources/views/admin/layout.blade.php
  - public/css/editorial-media-admin.css
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/AdminEditorialMediaTest.php
  - .github/workflows/ci.yml
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-media.ini
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260725-safe-editorial-media.md
validation:
  - command: GitHub required checks on synchronized PR 176 head
    result: NOT_RUN
    evidence: checkpoint commit is starting the exact-head validation cycle
blockers:
  - none
next_action: inspect exact-head CI and fix every task-owned failure before readiness
```

## Notes

Trust boundary: authenticated confirmed-MFA administrator upload input to private Platform-owned storage. Authorization invariant: every administrator media route requires the exact `media.manage` permission. Canary/login-server schema and session compatibility do not change. Migrations are additive and reversible. No secret, credential or production-only value is introduced.
