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

- [ ] `/admin/wiki` provides a useful Wiki dashboard and article/category discoverability.
- [ ] Trusted editors can list, create and update Wiki drafts with English and optional Polish translations.
- [ ] Article editing supports featured ordering and category assignment without bypassing the Wiki application service.
- [ ] Trusted category managers can list, create and update category/subcategory records with optimistic-lock conflict handling.
- [ ] Review submission, return-to-draft, publish, unpublish and archive actions use the existing lifecycle service and exact permissions.
- [ ] Revision history is visible and a publisher can restore a historical revision as a new revision.
- [ ] A short-lived signed, authenticated and MFA-confirmed preview renders unpublished Markdown through the existing restricted renderer.
- [ ] Every privileged route uses `auth`, `mfa.confirmed` and the narrowest exact Wiki permission.
- [ ] Stale edits return an explicit HTTP 409 rather than silently overwriting newer work.
- [ ] Administrator navigation, validation, authorization, audit, responsive, keyboard and security regressions pass.
- [ ] No EditorialMedia storage/upload implementation, gameplay content, commerce or external-repository write is introduced.

## Ownership

```yaml
owned_paths:
  - app/Wiki/Application/WikiArticleService.php
  - app/Wiki/Http/Admin/**
  - app/Wiki/Queries/Admin/**
  - routes/modules/wiki.php
  - resources/views/admin/layout.blade.php
  - resources/views/admin/wiki/**
  - public/css/wiki-admin.css
  - tests/Feature/Wiki/AdminWiki*.php
  - scripts/acceptance/tests/admin-wiki*.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - docs/architecture/adr/0013-wiki-administration.md
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
updated_at: 2026-07-26T07:30:00Z
head: c53e0f2a1a93de9275439aff573e5a713f5621b1
branch: feat/OTERYN-20260726-wiki-administration
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
  - accessibility
owned_paths:
  - Wiki administration paths listed in Ownership
proven:
  - trusted base main is c53e0f2a1a93de9275439aff573e5a713f5621b1
  - PR 158 delivered Wiki lifecycle services, revisions, optimistic locking, exact permissions and audit
  - PR 194 delivered published-only public reads, restricted CommonMark rendering and locale-isolated search
  - no active task or open pull request owns Wiki administration paths
  - existing administrator mutations use auth, mfa.confirmed, exact permission middleware and bounded audit
  - no external repository write is required
  - no production, router, DSM or Internet-exposure action is authorized
derived:
  - the administration slice can reuse WikiArticleService, WikiCategoryService and WikiMarkdownRenderer
  - media-consumer integration remains an independent later child task
unknown:
  - exact CI fixes, if any, until the implementation head is exercised by GitHub Actions
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - combine safe media integration with administration: exceeds a bounded independently reviewable slice
  - create a second Wiki lifecycle implementation in controllers: duplicates the existing audited service boundary
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260726-wiki-administration.md
validation:
  - command: repository, task and pull-request overlap reconciliation
    result: PASS
    evidence: trusted main, Issue #145, archived public Wiki task and current open PR state inspected through GitHub
blockers:
  - none
next_action: Open the draft PR, then implement the Wiki administrator request, query, controller and route boundary.
```

## Notes

Media references, initial gameplay content, homepage/SEO closure and final Synology deployment remain separate Issue #145 child tasks.