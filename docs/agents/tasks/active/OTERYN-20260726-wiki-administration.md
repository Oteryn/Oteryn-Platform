---
task_id: OTERYN-20260726-wiki-administration
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
  - docs/architecture/adr/0012-public-wiki-read-search.md
search_first:
  - active tasks and open pull requests touching Wiki, administrator routes, permissions, EditorialMedia or acceptance tests
  - existing Wiki lifecycle services, optimistic locking, authorization, revisions and public rendering
  - current administrator controller, request, route, layout and browser-test conventions
  - current exact Wiki permission keys and role-bundle behavior
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-wiki-administration

## Goal

Deliver a production-capable trusted-editor Wiki administration surface that reuses the existing Wiki lifecycle, revision, RBAC, MFA, audit and restricted Markdown boundaries without adding media-consumer integration or gameplay content.

## Acceptance criteria

- [x] `/admin/wiki` provides a useful Wiki dashboard and article/category discoverability.
- [x] Trusted editors can list, create and update Wiki drafts with English and optional Polish translations.
- [x] Article editing supports featured ordering and category assignment without bypassing the Wiki application service.
- [x] Trusted category managers can list, create and update category/subcategory records with optimistic-lock conflict handling.
- [x] Review submission, return-to-draft, publish, unpublish and archive actions use the existing lifecycle service and exact permissions.
- [x] Revision history is visible and a publisher can restore a historical revision as a new revision.
- [x] A short-lived signed, authenticated and MFA-confirmed preview renders unpublished Markdown through the existing restricted renderer.
- [x] Every privileged route uses `auth`, `mfa.confirmed` and the narrowest exact Wiki permission.
- [x] Stale edits return an explicit HTTP 409 rather than silently overwriting newer work.
- [x] Administrator navigation, validation, authorization, audit, responsive, keyboard and security regressions pass on the exact implementation head.
- [x] No EditorialMedia storage/upload implementation, gameplay content, commerce or external-repository write is introduced.

## Ownership

```yaml
owned_paths:
  - app/Wiki/Application/WikiAdminArticleWriter.php
  - app/Wiki/Application/WikiAdminCategoryWriter.php
  - app/Wiki/Http/Admin/**
  - app/Wiki/Infrastructure/Audit/WikiAuditAction.php
  - routes/modules/wiki.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/wiki/**
  - public/css/wiki-admin.css
  - tests/Feature/Wiki/AdminWiki*.php
  - scripts/acceptance/seed-admin-wiki-permissions.php
  - scripts/acceptance/tests/admin-wiki*.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - docs/architecture/adr/0013-wiki-administration.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-administration.md
modules:
  - Wiki
  - AdminRBAC
  - Audit
  - Testing
dependencies:
  - PR #158 Wiki foundation
  - PR #194 public Wiki reads and restricted Markdown rendering
  - Issue #145
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T08:45:00Z
head: fa69e757d20f70a3690899c5444f94459de69c31
branch: feat/OTERYN-20260726-wiki-administration
pr: 196
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
  - Wiki administration paths listed in Ownership
proven:
  - trusted base main is c53e0f2a1a93de9275439aff573e5a713f5621b1
  - PR 196 is the only active Wiki administration implementation owner and has no review threads or submitted reviews
  - trusted editors can manage bilingual articles, categories, lifecycle, signed previews and append-only revision restores through existing Wiki services
  - administrator routes require auth, mfa.confirmed, wiki.access and the narrowest exact mutation permission
  - article presentation and category assignment are transactional and audit metadata excludes article bodies
  - stale article and category writes fail explicitly with HTTP 409
  - exact implementation head fa69e757d20f70a3690899c5444f94459de69c31 removed the temporary diagnostic workflow
  - CI run 30195019633 passed on the exact implementation head
  - Agent Governance run 30195019643 passed on the exact implementation head
  - Platform DB Outage Validation run 30195019625 passed on the exact implementation head
  - Game Auth Ticket Concurrency run 30195019628 passed on the exact implementation head
  - Phase 7 Production-Like Validation run 30195019624 passed on the exact implementation head
  - Build Synology Staging Images run 30195019627 passed on the exact implementation head
  - Acceptance E2E and Visual UX run 30195019626 passed responsive, portability, resilience and keyboard profiles on the exact implementation head
  - trust boundary affected is administrator-only Wiki editorial control; public reads remain published-only
  - authentication and authorization invariant is auth plus confirmed MFA plus exact deny-by-default Wiki permission
  - no schema, session or compatibility change with Canary or login-server is introduced
  - no migration rollback, secret or production-only configuration is involved
  - no external repository, production, router, DSM or Internet-exposure write occurred
derived:
  - the Wiki administration child acceptance criteria are complete and PR 196 is ready for merge-lifecycle verification
  - EditorialMedia-to-Wiki integration remains the next independent Issue 145 child after this task is merged and archived
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: previous Pint, PHPStan and responsive-suite failures were remediated; every required workflow passed on fa69e757d20f70a3690899c5444f94459de69c31
rejected_hypotheses:
  - combine safe media integration with administration: exceeds a bounded independently reviewable slice
  - create a second Wiki lifecycle implementation in controllers: duplicates the existing audited service boundary
  - weaken formatting, static-analysis or browser gates: exact-head required workflows pass without weakening controls
changed_paths:
  - app/Wiki/Application/WikiAdminArticleWriter.php
  - app/Wiki/Application/WikiAdminCategoryWriter.php
  - app/Wiki/Http/Admin/**
  - app/Wiki/Infrastructure/Audit/WikiAuditAction.php
  - routes/modules/wiki.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/wiki/**
  - public/css/wiki-admin.css
  - tests/Feature/Wiki/AdminWikiAdministrationTest.php
  - scripts/acceptance/seed-admin-wiki-permissions.php
  - scripts/acceptance/tests/admin-wiki-administration.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - docs/architecture/adr/0013-wiki-administration.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-wiki-administration.md
validation:
  - command: vendor/bin/pint --test, composer analyse and composer test
    result: PASS
    evidence: CI run 30195019633 on fa69e757d20f70a3690899c5444f94459de69c31
  - command: Acceptance E2E and Visual UX
    result: PASS
    evidence: run 30195019626 on fa69e757d20f70a3690899c5444f94459de69c31
  - command: Phase 7 Production-Like Validation
    result: PASS
    evidence: run 30195019624 on fa69e757d20f70a3690899c5444f94459de69c31
  - command: Agent Governance, Platform DB Outage Validation, Game Auth Ticket Concurrency and Build Synology Staging Images
    result: PASS
    evidence: runs 30195019643, 30195019625, 30195019628 and 30195019627
blockers:
  - none
next_action: Complete the PR 196 merge lifecycle by verifying the current checkpoint-only head, marking the PR ready, squash-merging it, and archiving this task.
```

## Notes

EditorialMedia references, initial gameplay content, homepage/SEO closure and final Synology deployment remain separate Issue #145 child tasks.
