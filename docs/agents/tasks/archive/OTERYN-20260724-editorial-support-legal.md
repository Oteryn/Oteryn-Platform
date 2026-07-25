---
task_id: OTERYN-20260724-editorial-support-legal
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
search_first:
  - docs/agents/tasks/active/**
  - open pull requests for CMS managed-page, editorial-route or support-content ownership
  - existing managed-page models, services, controllers, requests, views and tests
optional_reads: []
---

# OTERYN-20260724-editorial-support-legal

## Goal

Extend the existing CMS with typed editorial mappings, published-only public routes and a confirmed-MFA administrator workflow for launch Server Information, Beginner's Guide, support guidance, rules and required legal documents without introducing a second CMS, stored support tickets, arbitrary HTML or media uploads.

## Acceptance criteria

- [x] All eight required public routes use stable typed keys and deterministic published, unpublished and missing behavior.
- [x] Draft and future-scheduled content is never exposed publicly.
- [x] Legal documents require and preserve version plus effective-date history.
- [x] Administrator mutations require `auth`, `mfa.confirmed` and exact `support.content.manage` authorization.
- [x] Privileged mutations create bounded audit records without page bodies, personal data or secrets.
- [x] Approved Discord, contact and support links are configuration-backed and reject unsafe or unapproved external URLs.
- [x] No stored ticket submission path, arbitrary HTML, executable upload or media upload is introduced.
- [x] Focused tests and all required CI pass on the implementation head.
- [x] PR #159 was squash-merged into `main`.

## Ownership

```yaml
owned_paths:
  - .env.example
  - app/Cms/Editorial/**
  - app/Cms/Models/ManagedPage.php
  - app/Cms/Models/ManagedPageLegalVersion.php
  - app/Cms/Actions/SaveManagedPage.php
  - app/Http/Controllers/Support/**
  - app/Http/Controllers/Admin/AdminManagedPageController.php
  - app/Http/Controllers/Admin/AdminSupportContentController.php
  - app/Http/Controllers/Cms/PublicPageController.php
  - app/Http/Requests/Admin/AdminManagedPageRequest.php
  - app/Http/Requests/Admin/AdminSupportContentRequest.php
  - app/Support/**
  - config/support.php
  - database/migrations/2026_07_24_230000_add_editorial_support_legal_to_managed_pages.php
  - database/migrations/2026_07_24_230100_grant_support_content_permission_to_editor_roles.php
  - resources/navigation/public/support.php
  - resources/views/support/**
  - resources/views/admin/support-content/**
  - routes/modules/support.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Feature/Support/**
  - docs/agents/tasks/archive/OTERYN-20260724-editorial-support-legal.md
modules:
  - CMS
  - Support
  - Admin
  - Audit
dependencies:
  - PR #143 merged
  - PR #146 merged
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T09:31:30+02:00
head: 96521c71ce166e8eb4f242706b6b5fde2dd8bce2
branch: main
pr: 159
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
  - .env.example
  - app/Cms/Editorial/**
  - app/Cms/Models/ManagedPage.php
  - app/Cms/Models/ManagedPageLegalVersion.php
  - app/Cms/Actions/SaveManagedPage.php
  - app/Http/Controllers/Support/**
  - app/Http/Controllers/Admin/AdminManagedPageController.php
  - app/Http/Controllers/Admin/AdminSupportContentController.php
  - app/Http/Controllers/Cms/PublicPageController.php
  - app/Http/Requests/Admin/AdminManagedPageRequest.php
  - app/Http/Requests/Admin/AdminSupportContentRequest.php
  - app/Support/**
  - config/support.php
  - database/migrations/2026_07_24_230000_add_editorial_support_legal_to_managed_pages.php
  - database/migrations/2026_07_24_230100_grant_support_content_permission_to_editor_roles.php
  - resources/navigation/public/support.php
  - resources/views/support/**
  - resources/views/admin/support-content/**
  - routes/modules/support.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Feature/Support/**
  - docs/agents/tasks/archive/OTERYN-20260724-editorial-support-legal.md
proven:
  - PR #143 and PR #146 are merged and provide the public expansion plan plus deterministic module-local route, navigation and permission integration.
  - Existing managed pages remain the sole editorial persistence boundary; no second CMS was introduced.
  - Eight typed routes resolve fixed managed-page keys and return safe distinct 404 states for missing and unpublished content.
  - Draft and future-scheduled title/body content is never passed to public views.
  - Generic managed-page administration and /pages/{slug} reject all reserved editorial slugs.
  - Published legal versions are preserved in additive immutable snapshots keyed by managed page and version.
  - Publishing legal content requires both version and effective date; changing published meaning requires a new version.
  - Support administration composes auth, confirmed MFA and exact support.content.manage middleware.
  - support.content.manage is explicitly granted only to content_editor and platform_admin roles.
  - Audit metadata is bounded to identifiers and publication/legal metadata and excludes page bodies, contact data and secrets.
  - Support links are emitted only from validated configuration-backed email or HTTPS allowlisted hosts.
  - Report-a-bug is guidance-only and no support POST route, ticket model, arbitrary HTML field, executable upload or media upload exists.
  - All changed paths avoid routes/web.php, the shared layout, header, footer, homepage, Downloads, Events, Wiki and PublicGameData implementation paths.
  - PR #159 was squash-merged into main as commit 96521c71ce166e8eb4f242706b6b5fde2dd8bce2.
derived:
  - Fixed typed mappings reuse managed-page persistence while providing stable primary route contracts.
  - Immutable legal snapshots preserve historical meaning without requiring a parallel legal CMS.
unknown: []
conflicts: []
first_failure:
  marker: local-checkout-unavailable
  evidence: sandbox DNS could not resolve github.com; repository reads and writes continued through the GitHub connector and exact-head GitHub CI
rejected_hypotheses:
  - A second support CMS is necessary.
  - A stored support-ticket form is required for report-a-bug guidance.
  - Existing generic /pages/{slug} is sufficient as the launch route contract.
  - Raw database date text is portable across SQLite and MariaDB.
changed_paths:
  - .env.example
  - app/Cms/Actions/SaveManagedPage.php
  - app/Cms/Editorial/EditorialPageKey.php
  - app/Cms/Editorial/EditorialPageQuery.php
  - app/Cms/Editorial/EditorialPageResult.php
  - app/Cms/Editorial/EditorialPageState.php
  - app/Cms/Models/ManagedPage.php
  - app/Cms/Models/ManagedPageLegalVersion.php
  - app/Http/Controllers/Admin/AdminManagedPageController.php
  - app/Http/Controllers/Admin/AdminSupportContentController.php
  - app/Http/Controllers/Cms/PublicPageController.php
  - app/Http/Controllers/Support/EditorialPageController.php
  - app/Http/Controllers/Support/SupportPageController.php
  - app/Http/Requests/Admin/AdminManagedPageRequest.php
  - app/Http/Requests/Admin/AdminSupportContentRequest.php
  - app/Support/ApprovedSupportLinks.php
  - app/Support/PublicEditorialPage.php
  - config/support.php
  - database/migrations/2026_07_24_230000_add_editorial_support_legal_to_managed_pages.php
  - database/migrations/2026_07_24_230100_grant_support_content_permission_to_editor_roles.php
  - docs/agents/tasks/archive/OTERYN-20260724-editorial-support-legal.md
  - resources/navigation/public/support.php
  - resources/views/admin/support-content/form.blade.php
  - resources/views/admin/support-content/index.blade.php
  - resources/views/support/editorial/show.blade.php
  - routes/modules/support.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Feature/Support/EditorialSupportLegalTest.php
validation:
  - command: overlap and precondition review against main and open pull requests
    result: PASS
    evidence: PR #143 and #146 merged; parallel Downloads, Events, Wiki and PublicGameData work did not overlap owned implementation paths
  - command: find /tmp/oteryn_impl -type f -name '*.php' -print0 | xargs -0 -n1 php -l
    result: PASS
    evidence: all staged implementation, route, config, view and test PHP files reported no syntax errors
  - command: GitHub CI run 30131176277 on 6b97a8a7069e0cb878921e0854642aabd6bd449e
    result: PASS
    evidence: Composer validation, dependency audit, Pint, PHPStan and full PHPUnit suite passed
  - command: Agent Governance run 30131176267
    result: PASS
    evidence: final implementation head passed repository governance
  - command: Acceptance E2E and Visual UX run 30131176272
    result: PASS
    evidence: exact-SHA smoke, browser portability, responsive, dependency resilience and keyboard accessibility profiles passed; bounded optional profiles were intentionally skipped by workflow policy
  - command: Platform DB Outage Validation run 30131176263
    result: PASS
    evidence: fail-closed mutation and recovery validation passed
  - command: Game Auth Ticket Concurrency run 30131176256
    result: PASS
    evidence: independent-process MariaDB concurrency proof passed
  - command: Phase 7 Production-Like Validation run 30131176251
    result: PASS
    evidence: production-like validation passed
  - command: Build Synology Staging Images run 30131176253
    result: PASS
    evidence: staging image build passed
  - command: squash merge PR #159
    result: PASS
    evidence: GitHub merged the current reviewed head into main as 96521c71ce166e8eb4f242706b6b5fde2dd8bce2
blockers:
  - none
next_action: No further action is required for this task; future editorial copy population is a separate product and legal approval workflow.
```

## Completion

- Completed: 2026-07-25T09:31:30+02:00
- Pull request: #159
- Merge method: squash
- Merge commit: `96521c71ce166e8eb4f242706b6b5fde2dd8bce2`
- Result: editorial public-content baseline delivered and validated.

## Notes

The public missing and unpublished states intentionally return HTTP 404 while rendering distinct safe guidance; neither state receives draft title or body data. Legal snapshots are immutable per page/version, so changing published legal meaning requires a new version. Authoritative rates, rules and legal wording are not fabricated or seeded; the administrator workflow identifies the required launch topics and publishes only reviewed content.
