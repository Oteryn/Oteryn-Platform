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
- [ ] Required checks pass on the final exact PR head after readiness cleanup.

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
updated_at: 2026-07-25T16:03:00+02:00
head: dc9c2cd1a41907fe8a158c755998ae4e5ba5aa78
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
  - the implementation is isolated from Wiki, Events and CMS publication logic and changes no Canary or login-server contract
  - the private editorial_media disk is outside the public storage link and fails closed with throw and report enabled
  - JPEG, PNG and WebP require matching extension, fileinfo MIME, image header, exact container boundary and successful decode
  - accepted images are GD-decoded and re-encoded, removing source metadata and appended payloads
  - byte, dimension and decoded-pixel limits are enforced before persistence
  - immutable storage names use 192 bits of randomness and normalized extensions
  - SHA-256 and dimensions are recorded for originals and generated thumbnails
  - references are bounded to cms, events and wiki and block deletion in both application and database layers
  - every administrator route requires auth, mfa.confirmed and exact media.manage authorization
  - media.manage is granted explicitly only to content_editor and platform_admin
  - upload and deletion write bounded non-secret administrator audit events
  - partial storage deletion is restored and database/audit changes roll back
  - CI run 30160280109 passed codec verification, Composer validation/audit, Pint, PHPStan and the complete test suite on this implementation head
  - Agent Governance run 30160280104 passed
  - Acceptance E2E and Visual UX run 30160280090 passed
  - Phase 7 Production-Like Validation run 30160280113 passed
  - Platform DB Outage Validation run 30160280095 passed
  - Game Auth Ticket Concurrency run 30160280130 passed
derived:
  - the reusable boundary can be consumed later through explicit references without transferring consumer lifecycle rules into EditorialMedia
unknown:
  - final exact-head results after removing temporary CI diagnostics and updating this checkpoint
conflicts: []
first_failure:
  marker: none
  evidence: implementation checks are green; final readiness-only commit remains to be validated
rejected_hypotheses:
  - reuse current public disk: it is publicly linked and configured with throw=false
  - add a Wiki-specific upload surface: the requested boundary must remain reusable
  - accept SVG or arbitrary files: prohibited by task and security architecture
  - retain original uploads after validation: decode and re-encode is required to remove metadata and appended payloads
changed_paths:
  - .github/workflows/ci.yml
  - app/Admin/AdminPermission.php
  - app/EditorialMedia/**
  - app/Http/Controllers/Admin/AdminEditorialMediaController.php
  - app/Http/Requests/Admin/AdminEditorialMediaUploadRequest.php
  - config/editorial_media.php
  - config/filesystems.php
  - database/migrations/2026_07_25_090000_create_editorial_media_tables.php
  - database/migrations/2026_07_25_090100_add_editorial_media_permission.php
  - deploy/synology/docker/platform-media.ini
  - deploy/synology/docker/platform.Dockerfile
  - docs/agents/tasks/active/OTERYN-20260725-safe-editorial-media.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0011-safe-editorial-media-boundary.md
  - public/css/editorial-media-admin.css
  - resources/views/admin/layout.blade.php
  - resources/views/admin/media/index.blade.php
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/**
validation:
  - command: GitHub required checks on implementation head dc9c2cd1a41907fe8a158c755998ae4e5ba5aa78
    result: PASS
    evidence: all code, static-analysis, database-outage, E2E, production-like, governance and concurrency checks passed; Synology image build was still queued when readiness cleanup began
blockers:
  - none
next_action: validate the final exact PR head, mark PR 176 ready and merge
```

## Notes

Trust boundary: authenticated confirmed-MFA administrator upload input to private Platform-owned storage. Authorization invariant: every administrator media route requires the exact `media.manage` permission. Canary/login-server schema and session compatibility do not change. Migrations are additive and reversible. No secret, credential or production-only value is introduced.
