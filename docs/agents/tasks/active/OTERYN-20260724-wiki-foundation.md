---
task_id: OTERYN-20260724-wiki-foundation
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - active Wiki or CMS tasks and open pull requests
  - existing CMS models, services, factories and migrations
  - existing exact-permission, audit and optimistic-locking conventions
  - relevant CMS, RBAC, audit and production-migration ADRs
optional_reads: []
---

# OTERYN-20260724-wiki-foundation

## Goal

Implement only the Oteryn Wiki architecture and persistence foundation: Wiki-owned schema, models, lifecycle and revision invariants, exact authorization boundaries, bounded audit definitions, focused tests and a durable ADR. Public activation remains out of scope.

## Acceptance criteria

- [x] Reversible migrations create `wiki_articles`, `wiki_article_translations`, `wiki_categories`, `wiki_category_translations`, `wiki_article_category` and `wiki_revisions` without assuming an empty database.
- [x] Supported locale, localized slug uniqueness, lifecycle, publication-content, append-only revision, restore-as-new-revision and stale-edit invariants are enforced and tested.
- [x] Exact reserved permissions are used deny-by-default; no wildcard or implicit role grant is introduced.
- [x] Wiki audit metadata is bounded and excludes complete article bodies.
- [x] No public Wiki route, navigation link, media upload, arbitrary HTML, search service, comments or player editing is activated.
- [x] Wiki ADR and module implementation status are current.
- [x] Formatting, PHPStan, focused/full tests and required CI pass on the implementation validation head.

## Ownership

```yaml
owned_paths:
  - app/Wiki/**
  - app/Providers/WikiServiceProvider.php
  - routes/wiki.php
  - database/migrations/*_create_wiki_*.php
  - database/factories/Wiki/**
  - tests/Unit/Wiki/**
  - tests/Feature/Wiki/**
  - docs/architecture/adr/*wiki*.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260724-wiki-foundation.md
modules:
  - Wiki
  - Admin/RBAC
  - Audit
  - Database
dependencies:
  - PR #142 Wiki architecture plan
  - PR #146 public-web parallel foundation and reserved Wiki permission keys
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T00:39:24+02:00
head: b577c582744d83a902aaf99f0af22fd7b1d5b5f2
branch: feat/OTERYN-20260724-wiki-foundation
pr: 158
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - admin-rbac
  - security
  - testing
owned_paths:
  - app/Wiki/**
  - database/migrations/2026_07_24_231000_create_wiki_foundation_tables.php
  - tests/Unit/Wiki/**
  - tests/Feature/Wiki/**
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260724-wiki-foundation.md
proven:
  - PR 142 and PR 146 are merged; the exact wiki.access, wiki.articles.manage, wiki.categories.manage and wiki.publish keys are reserved without automatic role grants.
  - the active editorial CMS task in PR 159 does not own or modify Wiki namespaces, Wiki migrations, Wiki tests or the Wiki ADR.
  - six additive Platform-owned Wiki tables have a reversible migration and tested schema constraints.
  - supported locales are exactly en and pl; article and category slugs are unique by locale.
  - article lifecycle transitions are explicit; stale article and category edits fail before overwrite.
  - article create, update and restore append revisions; restore references the source revision and never mutates it.
  - publication requires complete English and Polish content.
  - Wiki application services require one exact permission and fail closed without it; no role receives Wiki authority implicitly.
  - audit events contain bounded state metadata and exclude complete article bodies and category descriptions.
  - content type, category key and revision-note limits match their schema columns exactly.
  - no public Wiki route, route provider, navigation contribution or homepage change exists in this slice.
  - restricted Markdown source rejects raw HTML and dangerous protocols; rendering remains a later reviewed slice.
derived:
  - the persistence foundation is ready for a later administration/public-read slice without coupling Wiki storage to managed pages.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: formatting, PHPStan and the full PHPUnit suite pass on b577c582744d83a902aaf99f0af22fd7b1d5b5f2
rejected_hypotheses:
  - Wiki permission keys require a shared-registry edit: PR 146 already reserved all four keys.
  - the parallel editorial CMS task overlaps Wiki ownership: PR 159 is confined to managed pages and Support/legal paths.
  - a placeholder public route is required for the foundation: public activation is explicitly deferred and tested absent.
changed_paths:
  - app/Wiki/Application/**
  - app/Wiki/Domain/**
  - app/Wiki/Infrastructure/**
  - database/migrations/2026_07_24_231000_create_wiki_foundation_tables.php
  - tests/Unit/Wiki/WikiDomainRulesTest.php
  - tests/Feature/Wiki/WikiAuthorizationTest.php
  - tests/Feature/Wiki/WikiFoundationTest.php
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260724-wiki-foundation.md
validation:
  - command: precondition and ownership-overlap verification
    result: PASS
    evidence: PR 142 and PR 146 are merged; PR 159 has no Wiki path ownership overlap.
  - command: php -l on all added PHP files and final boundary changes
    result: PASS
    evidence: no syntax errors detected in the local sandbox copies.
  - command: vendor/bin/pint --test
    result: PASS
    evidence: CI run 30131483590, head b577c582744d83a902aaf99f0af22fd7b1d5b5f2.
  - command: composer analyse
    result: PASS
    evidence: CI run 30131483590, head b577c582744d83a902aaf99f0af22fd7b1d5b5f2.
  - command: composer test
    result: PASS
    evidence: CI run 30131483590, head b577c582744d83a902aaf99f0af22fd7b1d5b5f2.
  - command: exact-permission, locale, slug, lifecycle, stale-edit, revision, restore, audit, schema-bound length and no-public-route coverage
    result: PASS
    evidence: focused Wiki tests are included in the passing full suite.
  - command: final PR changed-path review
    result: PASS
    evidence: temporary CI diagnostics were removed; no net workflow, public navigation, homepage or routes/web.php change remains.
blockers: []
next_action: Verify required workflows on the final checkpoint-only head, mark PR 158 ready and hand off for review.
```

## Notes

This slice deliberately excludes public Wiki routes, rendering, media, search, navigation and editor UI. No module route/provider placeholder was added because the repository extension contract prohibits activating incomplete public surfaces.
