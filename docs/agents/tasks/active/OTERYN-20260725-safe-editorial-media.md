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
checkpoint_version: 2
updated_at: 2026-07-25T15:45:00+02:00
head: dc9c2cd1a41907fe8a158c755998ae4e5ba5aa78
branch: feat/OTERYN-20260725-safe-editorial-media
pr: 176
status: validating_final_readiness
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
  - Agent½Ù•É¹…¹”ÉÕ¸€ÌÀÄØÀÈàÀÄÀĞÁ…ÍÍ•(€€´•ÁÑ…¹”É…¹Y¥ÍÕ…°U`ÉÕ¸€ÌÀÄØÀÈàÀÀäÀÁ…ÍÍ•(€€´A¡…Í”€ÜAÉ½‘ÕÑ¥½¸µ1¥­”Y…±¥‘…Ñ¥½¸ÉÕ¸€ÌÀÄØÀÈàÀÄÄÌÁ…ÍÍ•(€€´A±…Ñ™½É´=ÕÑ…”Y…±¥‘…Ñ¥½¸ÉÕ¸€ÌÀÄØÀÈàÀÀäÔÁ…ÍÍ•(€€´…µ”ÕÑ Q¥­•Ğ½¹ÕÉÉ•¹äÉÕ¸€ÌÀÄØÀÈàÀÄÌÀÁ…ÍÍ•)‘•É¥Ù•è(€€´Ñ¡”É•ÕÍ…‰±”‰½Õ¹‘…Éä…¸‰”½¹ÍÕµ•±…Ñ•ÈÑ¡É½Õ •áÁ±¥¥ĞÉ•™•É•¹•Ìİ¥Ñ¡½ÕĞÑÉ…¹Í™•ÉÉ¥¹œ½¹ÍÕµ•È±¥™•å±”ÉÕ±•Ì¥¹Ñ¼‘¥Ñ½É¥…±5•‘¥„)Õ¹­¹½İ¸è(€€´™¥¹…°•á…Ğµ¡•…É•ÍÕ±ÑÌ…™Ñ•ÈÉ•µ½Ù¥¹œÑ•µÁ½É…Éä$‘¥…¹½ÍÑ¥Ì…¹ÕÁ‘…Ñ¥¹œÑ¡¥Ì¡•­Á½¥¹Ğ)½¹™±¥ÑÌèmt)™¥ÉÍÑ}™…¥±ÕÉ”è(€µ…É­•Èè¹½¹”(€•Ù¥‘•¹”è¥µÁ±•µ•¹Ñ…Ñ¥½¸¡•­Ì…É”É••¸ì™¥¹…°É•…‘¥¹•ÍÌµ½¹±ä½µµ¥ĞÉ•µ…¥¹ÌÑ¼‰”Ù…±¥‘…Ñ•)É•©•Ñ•‘}¡åÁ½Ñ¡•Í•Ìè(€€´É•ÕÍ”ÕÉÉ•¹ĞÁÕ‰±¥Œ‘¥Í¬è¥Ğ¥ÌÁÕ‰±¥±ä±¥¹­•…¹½¹™¥ÕÉ•İ¥Ñ Ñ¡É½Üõ™…±Í”(€€´…‘„]¥­¤µÍÁ•¥™¥ŒÕÁ±½…ÍÕÉ™…”èÑ¡”É•ÅÕ•ÍÑ•‰½Õ¹‘…ÉäµÕÍĞÉ•µ…¥¸É•ÕÍ…‰±”(€€´…•ÁĞMY½È…É‰¥ÑÉ…Éä™¥±•ÌèÁÉ½¡¥‰¥Ñ•‰äÑ…Í¬…¹Í•ÕÉ¥Ñä…É¡¥Ñ•ÑÕÉ”(€€´É•Ñ…¥¸½É¥¥¹…°ÕÁ±½…‘Ì…™Ñ•ÈÙ…±¥‘…Ñ¥½¸è‘•½‘”…¹É”µ•¹½‘”¥ÌÉ•ÅÕ¥É•Ñ¼É•µ½Ù”µ•Ñ…‘…Ñ„…¹…ÁÁ•¹‘•Á…å±½…‘Ì)¡…¹•‘}Á…Ñ¡Ìè(€€´€¹¥Ñ¡Õˆ½İ½É­™±½İÌ½¤¹åµ°(€€´…ÁÀ½‘µ¥¸½‘µ¥¹A•Éµ¥ÍÍ¥½¸¹Á¡À(€€´…ÁÀ½‘¥Ñ½É¥…±5•‘¥„¼¨¨(€€´…ÁÀ½!ÑÑÀ½½¹ÑÉ½±±•ÉÌ½‘µ¥¸½‘µ¥¹‘¥Ñ½É¥…±5•‘¥…½¹ÑÉ½±±•È¹Á¡À(€€´…ÁÀ½!ÑÑÀ½I•ÅÕ•ÍÑÌ½‘µ¥¸½‘µ¥¹‘¥Ñ½É¥…±5•‘¥…UÁ±½…‘I•ÅÕ•ÍĞ¹Á¡À(€€´½¹™¥œ½•‘¥Ñ½É¥…±}µ•‘¥„¹Á¡À(€€´½¹™¥œ½™¥±•ÍåÍÑ•µÌ¹Á¡À(€€´‘…Ñ…‰…Í”½µ¥É…Ñ¥½¹Ì¼ÈÀÈÙ|Àİ|ÈÕ|ÀäÀÀÀÁ}É•…Ñ•}•‘¥Ñ½É¥…±}µ•‘¥…}Ñ…‰±•Ì¹Á¡À(€€´‘…Ñ…‰…Í”½µ¥É…Ñ¥½¹Ì¼ÈÀÈÙ|Àİ|ÈÕ|ÀäÀÄÀÁ}…‘‘}•‘¥Ñ½É¥…±}µ•‘¥…}Á•Éµ¥ÍÍ¥½¸¹Á¡À(€€´‘•Á±½ä½Íå¹½±½ä½‘½­•È½Á±…Ñ™½É´µµ•‘¥„¹¥¹¤(€€´‘•Á±½ä½Íå¹½±½ä½‘½­•È½Á±…Ñ™½É´¹½­•É™¥±”(€€´‘½Ì½…•¹ÑÌ½Ñ…Í­Ì½…Ñ¥Ù”½=QIe8´ÈÀÈØÀÜÈÔµÍ…™”µ•‘¥Ñ½É¥…°µµ•‘¥„¹µ(€€´‘½Ì½…É¡¥Ñ•ÑÕÉ”½5=U1}Q1=¹µ(€€´‘½Ì½…É¡¥Ñ•ÑÕÉ”½…‘È¼ÀÀÄÄµÍ…™”µ•‘¥Ñ½É¥…°µµ•‘¥„µ‰½Õ¹‘…Éä¹µ(€€´ÁÕ‰±¥Œ½ÍÌ½•‘¥Ñ½É¥…°µµ•‘¥„µ…‘µ¥¸¹ÍÌ(€€´É•Í½ÕÉ•Ì½Ù¥•İÌ½…‘µ¥¸½±…å½ÕĞ¹‰±…‘”¹Á¡À(€€´É•Í½ÕÉ•Ì½Ù¥•İÌ½…‘µ¥¸½µ•‘¥„½¥¹‘•à¹‰±…‘”¹Á¡À(€€´É½ÕÑ•Ì½µ½‘Õ±•Ì½•‘¥Ñ½É¥…°µµ•‘¥„¹Á¡À(€€´Ñ•ÍÑÌ½•…ÑÕÉ”½‘¥Ñ½É¥…±5•‘¥„¼¨¨)Ù…±¥‘…Ñ¥½¸è(€€´½µµ…¹è¥Ñ!ÕˆÉ•ÅÕ¥É•¡•­Ì½¸¥µÁ±•µ•¹Ñ…Ñ¥½¸¡•…‘ŒåŒÉÅ„ĞÄäÀİ™”á„ÄÔáŒÜÔÔääá…”Ñ”Õ‰„Õ…„Üà(€€€É•ÍÕ±ĞèAML(€€€•Ù¥‘•¹”è…±°½‘”°ÍÑ…Ñ¥Œµ…¹…±åÍ¥Ì°‘…Ñ…‰…Í”µ½ÕÑ…”°É°ÁÉ½‘ÕÑ¥½¸µ±¥­”°½Ù•É¹…¹”…¹½¹ÕÉÉ•¹ä¡•­ÌÁ…ÍÍ•ìMå¹½±½ä¥µ…”‰Õ¥±İ…ÌÍÑ¥±°ÅÕ•Õ•İ¡•¸É•…‘¥¹•ÍÌ±•…¹ÕÀ‰•…¸)‰±½­•ÉÌè(€€´¹½¹”)¹•áÑ}…Ñ¥½¸èÙ…±¥‘…Ñ”Ñ¡”™¥¹…°•á…ĞAH¡•…°µ…É¬AH€ÄÜØÉ•…‘ä…¹µ•É”)€((ŒŒ9½Ñ•Ì()QÉÕÍĞ‰½Õ¹‘…Éäè…ÕÑ¡•¹Ñ¥…Ñ•½¹™¥Éµ•µ5…‘µ¥¹¥ÍÑÉ…Ñ½ÈÕÁ±½…¥¹ÁÕĞÑ¼ÁÉ¥Ù…Ñ”A±…Ñ™½É´µ½İ¹•ÍÑ½É…”¸ÕÑ¡½É¥é…Ñ¥½¸¥¹Ù…É¥…¹Ğè•Ù•Éä…‘µ¥¹¥ÍÑÉ…Ñ½Èµ•‘¥„É½ÕÑ”É•ÅÕ¥É•ÌÑ¡”•á…Ğµ•‘¥„¹µ…¹…•€Á•Éµ¥ÍÍ¥½¸¸…¹…Éä½±½¥¸µÍ•ÉÙ•ÈÍ¡•µ„…¹Í•ÍÍ¥½¸½µÁ…Ñ¥‰¥±¥Ñä‘¼¹½Ğ¡…¹”¸5¥É…Ñ¥½¹Ì…É”…‘‘¥Ñ¥Ù”…¹É•Ù•ÉÍ¥‰±”¸9¼Í•É•Ğ°É•‘•¹Ñ¥…°½ÈÁÉ½‘ÕÑ¥½¸µ½¹±äÙ…±Õ”¥Ì¥¹ÑÉ½‘Õ•¸(