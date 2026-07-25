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

- [ ] Dedicated Platform-owned media and bounded reference records exist.
- [ ] Only JPEG, PNG and WebP with matching extension, detected MIME and decodable content are accepted.
- [ ] Uploads enforce byte, dimension and decoded-pixel limits before decode and fail closed when processing or storage is unavailable.
- [ ] Accepted images are decoded, re-encoded, stripped of metadata, stored under immutable random names and recorded with SHA-256.
- [ ] Bounded thumbnails are generated only for images larger than the administrator preview boundary.
- [ ] Alt text is required and bounded.
- [ ] One exact media-management permission protects the administrator library behind authentication and confirmed MFA.
- [ ] Upload and deletion operations append bounded non-secret administrator audit events.
- [ ] Referenced media cannot be deleted and database constraints preserve the same invariant.
- [ ] Malicious, malformed, mismatched and over-limit fixtures are rejected; permission, MFA and CSRF regressions pass.
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
  - routes/modules/editorial-media.php
  - tests/Feature/EditorialMedia/**
  - .github/workflows/ci.yml
  - deploy/synology/docker/platform.Dockerfile
  - docs/architecture/adr/0010-safe-editorial-media-boundary.md
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
updated_at: 2026-07-25T08:44:17Z
head: 7164edd1308d9f43cfbc20fb37901e66448fe165
branch: feat/OTERYN-20260725-safe-editorial-media
pr: none
status: implementing
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
  - main head at task start is 7164edd1308d9f43cfbc20fb37901e66448fe165
  - no open pull request currently changes the proposed media implementation paths
  - open localization PR 175 owns routes/web.php and public module paths, so this task will use routes/modules/editorial-media.php
  - existing privileged routes require auth, mfa.confirmed and an exact admin.permission middleware value
  - current deployed Platform image lacks the GD extension and current filesystem disks use throw=false
  - Synology staging persists the complete Platform storage directory through platform_storage
  - no Composer image-processing dependency is installed
  - EditorialMedia persistence is Platform-owned and does not change Canary or login-server contracts
derived:
  - a dedicated private throw-on-error disk plus GD decode and re-encode is the smallest reusable fail-closed boundary
  - a generic media.manage permission is required because Wiki, Events and CMS are future consumers rather than owners of the library
unknown:
  - exact CI result on the implementation head
conflicts: []
first_failure:
  marker: local-checkout-unavailable
  evidence: execution sandbox cannot resolve github.com, so validation must be performed by GitHub CI and no local pass will be claimed
rejected_hypotheses:
  - reuse current public disk: it is publicly linked and configured with throw=false
  - add a Wiki-specific upload surface: the requested boundary must be reusable and consumer integration is out of scope
  - accept SVG or arbitrary files: prohibited by task and security architecture
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-safe-editorial-media.md
validation:
  - command: local application validation
    result: BLOCKED
    evidence: local checkout unavailable because github.com DNS resolution fails in the execution sandbox
blockers:
  - none
next_action: implement the isolated EditorialMedia schema, processing actions, administrator routes and security regression tests
```

## Notes

Trust boundary: authenticated confirmed-MFA administrator upload input to private Platform-owned storage. Authorization invariant: every administrator media route requires the exact `media.manage` permission. Canary/login-server schema and session compatibility do not change. Migrations are additive and reversible. No secret, credential or production-only value is introduced.