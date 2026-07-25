---
task_id: OTERYN-20260725-safe-editorial-media
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
search_first:
  - existing editorial media implementation
  - active tasks and open pull requests touching EditorialMedia
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
---

# OTERYN-20260725-safe-editorial-media

## Goal

Provide a reusable private and secure editorial raster-image boundary for later CMS, Events and Wiki consumers without integrating those consumers in this task.

## Acceptance criteria

- [x] Platform-owned media and bounded reference records exist.
- [x] Only verified JPEG, PNG and WebP images are accepted.
- [x] Byte, dimension and decoded-pixel limits fail closed.
- [x] Accepted images are decoded, re-encoded and stripped of metadata and appended payloads.
- [x] Private random storage names, SHA-256 integrity and bounded thumbnails are implemented.
- [x] Alt text, exact authorization, MFA, CSRF and audit requirements are enforced.
- [x] Referenced images cannot be deleted.
- [x] Malicious, malformed, mismatched, over-limit and storage-failure regressions are covered.
- [x] PR #176 passed all required checks and was squash-merged.
- [x] PR #180 archived this completed record.

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
  - public/css/editorial-media-admin.css
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/**
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-media.ini
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/archive/OTERYN-20260725-safe-editorial-media.md
modules:
  - EditorialMedia
  - Admin
  - Audit
dependencies:
  - PUBLIC_WEBSITE_EXPANSION_PLAN.md Slice 8
  - WIKI_IMPLEMENTATION_PLAN.md Slice 7
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T20:12:00+02:00
head: b158b108b2370d27922ee3ae14202b4510147649
branch: main
pr: 180
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - admin-rbac
  - database
  - security
  - testing
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260725-safe-editorial-media.md
proven:
  - PR #176 was squash-merged as f073b2e9c802c83e2c13e578023acd32497747d4.
  - Final PR #176 head 6417b416b474e3fabd384911c82eb7fd05eaecd3 passed all seven required workflows.
  - PR #180 was squash-merged as b158b108b2370d27922ee3ae14202b4510147649 and moved the task record from active to archive.
  - Final PR #180 head 6db9a21aabfcab60f1232ad3b24e5638978a7987 passed every workflow triggered for the documentation-only cleanup.
  - Editorial images use a private fail-closed disk and are verified by extension, MIME, image header, exact container boundary and successful decode.
  - Images are re-encoded with metadata removed, immutable random names, SHA-256 integrity and bounded thumbnails.
  - Every administrator route requires auth, confirmed MFA and exact media.manage authorization.
  - media.manage is granted only to content_editor and platform_admin.
  - References block deletion in the application and database layers.
  - Partial storage deletion restores removed objects and rolls back database and audit changes.
derived:
  - Future consumers can attach explicit references without transferring their lifecycle rules into EditorialMedia.
unknown: []
conflicts: []
first_failure:
  marker: acceptance-service-container-initialization
  evidence: one PR #176 E2E attempt failed before checkout while initializing service containers; the same job passed on retry without a code change
rejected_hypotheses:
  - reuse the public disk: it is publicly linked and did not provide the required fail-closed boundary
  - accept SVG or arbitrary files: prohibited by the task and security architecture
  - retain original uploads: re-encoding is required to remove metadata and appended payloads
changed_paths:
  - app/EditorialMedia/**
  - app/Admin/AdminPermission.php
  - app/Http/Controllers/Admin/AdminEditorialMediaController.php
  - app/Http/Requests/Admin/AdminEditorialMediaUploadRequest.php
  - config/editorial_media.php
  - config/filesystems.php
  - database/migrations/*editorial_media*
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-media.ini
  - resources/views/admin/media/**
  - public/css/editorial-media-admin.css
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/**
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/archive/OTERYN-20260725-safe-editorial-media.md
validation:
  - command: required workflows on PR #176 head 6417b416b474e3fabd384911c82eb7fd05eaecd3
    result: PASS
    evidence: all seven workflows passed before squash merge
  - command: squash merge PR #176
    result: PASS
    evidence: main commit f073b2e9c802c83e2c13e578023acd32497747d4
  - command: required workflows on PR #180 head 6db9a21aabfcab60f1232ad3b24e5638978a7987
    result: PASS
    evidence: all five triggered cleanup workflows passed
  - command: squash merge PR #180
    result: PASS
    evidence: main commit b158b108b2370d27922ee3ae14202b4510147649
blockers:
  - none
next_action: Keep this archived record as the completed source of truth.
```

## Notes

The trust boundary is confirmed-MFA administrator input to private Platform-owned storage. Canary and login-server contracts did not change. Migrations are additive and reversible. No secret or production-only value was introduced.
