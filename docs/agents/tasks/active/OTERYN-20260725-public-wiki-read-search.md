---
task_id: OTERYN-20260725-public-wiki-read-search
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
  - docs/architecture/PUBLIC_LOCALIZATION_POLICY.md
  - docs/architecture/adr/0010-wiki-module-and-persistence-foundation.md
search_first:
  - active tasks and open pull requests touching Wiki, localization, public routes, navigation or acceptance tests
  - current Wiki models, services, lifecycle, authorization and foundation tests
  - current localized public route registration and canonical/hreflang behavior
  - maintained Markdown renderer options compatible with PHP 8.5 and Laravel 13
  - existing rate limiter, pagination, error-state and browser-acceptance conventions
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
---

# OTERYN-20260725-public-wiki-read-search

## Goal

Deliver the first production-capable public Wiki slice: published-only Polish and English reads, safe restricted Markdown rendering, category navigation and bounded locale-isolated search, integrated into the existing public shell without adding administration or media-consumer writes.

## Acceptance criteria

- [x] Public Wiki routes are locale-aware, canonical and deterministic for `en` and `pl`.
- [x] Public reads expose only published articles and visible categories; drafts, review records, archived content and missing translations never leak.
- [x] Wiki homepage, category pages, article pages, breadcrumbs, generated table of contents and related-article presentation are implemented.
- [x] Restricted Markdown is rendered by a maintained dependency with raw HTML disabled, dangerous protocols rejected and links/images constrained to approved policies.
- [x] Search is database-backed behind an explicit interface, published-only, locale-isolated, bounded, paginated, rate-limited and deterministically ordered.
- [x] Missing, unpublished, stale-translation, empty and unavailable states are explicit and truthful.
- [x] Canonical, `hreflang`, metadata and public navigation integration are complete for the delivered Wiki routes.
- [x] Desktop, tablet, mobile, keyboard and focus behavior meet existing acceptance conventions without horizontal overflow.
- [x] Focused unit/feature/security tests, migration compatibility checks and representative browser acceptance pass on the exact final head.
- [x] Existing Wiki lifecycle, authorization, audit, EditorialMedia, Events, Downloads, PublicGameData and authentication boundaries remain unchanged.

## Explicit exclusions

- administrator Wiki routes and editing UI;
- media upload or Wiki media-reference integration;
- revision restore UI or signed draft preview;
- initial gameplay-content publication;
- automatic import from Canary or client data;
- public/player editing, comments or raw HTML;
- commerce, payments, shop or premium systems;
- external-repository, production, router, DSM or Internet-exposure writes.

## Ownership

```yaml
owned_paths:
  - composer.json
  - composer.lock
  - app/Wiki/Application/Rendering/**
  - app/Wiki/Application/Search/**
  - app/Wiki/Infrastructure/Rendering/**
  - app/Wiki/Infrastructure/Search/**
  - app/Wiki/Http/Public/**
  - app/Wiki/Queries/Public/**
  - app/Wiki/ViewModels/Public/**
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Localization/LocalizedPublicUrls.php
  - app/Localization/LocalizedUrlGenerator.php
  - app/Providers/AppServiceProvider.php
  - routes/modules/wiki.php
  - resources/navigation/public/wiki.php
  - resources/views/wiki/**
  - public/css/wiki.css
  - lang/en/public.php
  - lang/pl/public.php
  - tests/Feature/Wiki/PublicWiki*.php
  - tests/Feature/Wiki/WikiFoundationTest.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Unit/Wiki/*Renderer*.php
  - tests/Unit/Wiki/*Search*.php
  - scripts/acceptance/tests/public-wiki*.spec.mjs
  - scripts/acceptance/seed-public-wiki.php
  - scripts/acceptance/playwright.config.mjs
  - docs/architecture/adr/0012-public-wiki-read-search.md
  - docs/agents/tasks/active/OTERYN-20260725-public-wiki-read-search.md
modules:
  - Wiki
  - Localization
  - PublicPortal
  - Testing
dependencies:
  - PR #158 Wiki foundation
  - PR #175 localization foundation
  - PR #190 public website programme reconciliation
  - Issue #145
blockers:
  - none
cross_repository_tasks:
  - none
```

## Required design invariants

- The public layer queries Wiki-owned models through explicit query/search interfaces; Blade templates do not issue raw database queries.
- Publication, locale and visibility filtering is applied inside the query boundary, not only in controllers or templates.
- Search never indexes or returns non-published content and never falls back across locales.
- Rendering starts from stored source Markdown and produces sanitized output without trusting persisted HTML.
- Raw HTML, scripts, event attributes, iframes, embedded forms, dangerous URL schemes and uncontrolled remote images remain prohibited.
- Generated heading identifiers are deterministic and collision-safe within one article.
- A missing Polish translation is not replaced by English content under a Polish URL.
- Public failure semantics distinguish not found, empty and dependency unavailable states.
- Existing `auth`, `mfa.confirmed` and exact Wiki permission rules are not weakened or reused for anonymous public reads.
- Schema changes, if required, are additive, reversible and reviewed against the existing Wiki migration.

## Validation plan

Focused during implementation:

- Composer manifest/lock consistency immediately after adding a renderer dependency;
- renderer unit tests covering raw HTML, dangerous protocols, malformed Markdown, heading collisions and safe links;
- public query/search feature tests for publication, locale isolation, category visibility, pagination, rate limiting and deterministic ordering;
- route/canonical/hreflang tests for legacy and localized URLs;
- focused responsive and keyboard browser acceptance.

Exact final head:

- `composer validate --strict`;
- `composer audit --no-interaction`;
- `vendor/bin/pint --test`;
- `composer analyse`;
- focused Wiki unit/feature tests;
- full `composer test`;
- required repository workflows;
- representative Chromium, Firefox and WebKit acceptance for the delivered Wiki surfaces.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T07:02:06Z
head: a286814d17a5697f2462f55dedccf1badd85d269
branch: feat/OTERYN-20260725-public-wiki-read-search
pr: 194
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - localization
  - database
  - security
  - testing
  - accessibility
owned_paths:
  - public Wiki read/search paths listed in Ownership
  - app/Localization/LocalizedUrlGenerator.php
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Feature/Wiki/WikiFoundationTest.php
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-public-wiki.php
proven:
  - trusted base main is 4cab5c0842d678968d893dacf744b47ca31ef67c
  - draft PR 194 owns the declared public Wiki read/search paths
  - PR 190 merged the programme reconciliation and identifies public Wiki read/search as missing
  - PR 158 delivered Wiki persistence, lifecycle services, revisions, optimistic locking, exact permissions and audit but no public routes, rendering or search
  - PR 175 delivered deterministic en/pl public localization, canonical and hreflang foundations
  - no competing open pull request owns Wiki, localization, public Wiki navigation or Wiki acceptance paths
  - PR 158 is merged; its stale active task record no longer represents concurrent implementation even though it retains historical app/Wiki ownership
  - current Composer dependencies contain no Markdown renderer
  - current Wiki input rules reject obvious raw HTML and dangerous protocols but do not render public output
  - no public or administrator Wiki route module exists on trusted main
  - no write outside blakinio/Oteryn-Platform is required
  - no production, router, DSM, Internet-exposure or external-repository action is authorized
  - league/commonmark 2.8.3 is maintained, compatible with PHP 8.5 and provides raw-HTML, unsafe-link and parser-complexity controls
  - trust boundary affected: anonymous published-only Wiki reads and editor-controlled source Markdown rendering
  - authentication and authorization invariant affected: no privileged mutation is added; existing exact permissions remain unchanged
  - Canary/login-server schema or session compatibility changes: none
  - rollback required: dependency/application revert and any additive migration rollback
  - secrets or production-only configuration involved: none
  - league/commonmark is a direct requirement with a Composer-consistent lock hash and the locked 2.8.3 package
  - public read and search boundaries exclude drafts, review, archive, future publication, hidden categories, missing translations and Polish translations older than English source
  - localized route cloning preserves source middleware, including the Wiki search rate limiter
  - deterministic browser acceptance covers Wiki reads, search, TOC, bilingual equivalence, accessibility smoke and horizontal overflow in Chromium, Firefox, WebKit, desktop, tablet and mobile projects
  - no migration or Canary-owned schema change is required
  - GitHub acceptance passed on PR head 7db66513b90328fd0eaa5b69efaca080e4062e78, including portability, responsive, dependency-resilience and keyboard-accessibility profiles
  - all required GitHub checks passed after the transient Docker Hub buildkit pull timeout was rerun successfully
derived:
  - league/commonmark 2.8 is the selected renderer; Wiki adds stricter link and no-image renderers on top of its fail-closed configuration
  - public read/search can be delivered independently of administrator UI and media integration
  - the implementation satisfies the bounded public Wiki slice without expanding privileged mutation or media ownership
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: renderer alt-text fixture, localized throttle cloning and stale Polish publication gaps were fixed and focused tests now pass
rejected_hypotheses:
  - expose source Markdown directly: violates safe-rendering and presentation requirements
  - persist and trust arbitrary rendered HTML: violates the restricted rendering boundary
  - serve English content on Polish URLs when translation is missing: violates truthful localization
  - implement administrator UI in the same first slice: exceeds the bounded independently reviewable public-read scope
  - duplicate EditorialMedia storage or upload handling: PR 176 already owns the reusable media boundary
changed_paths:
  - composer.json
  - composer.lock
  - app/Wiki/**
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Localization/LocalizedUrlGenerator.php
  - app/Providers/AppServiceProvider.php
  - routes/modules/wiki.php
  - resources/navigation/public/wiki.php
  - resources/views/wiki/**
  - public/css/wiki.css
  - lang/en/public.php
  - lang/pl/public.php
  - tests/Feature/Wiki/**
  - tests/Feature/PublicPortal/PublicPortalExtensionTest.php
  - tests/Unit/Wiki/CommonMarkWikiRendererTest.php
  - scripts/acceptance/seed-public-wiki.php
  - scripts/acceptance/tests/public-wiki-read-search.spec.mjs
  - scripts/acceptance/playwright.config.mjs
  - docs/architecture/adr/0012-public-wiki-read-search.md
  - docs/agents/tasks/active/OTERYN-20260725-public-wiki-read-search.md
validation:
  - command: programme, overlap and foundation reconciliation
    result: PASS
    evidence: trusted main, PR 190, Wiki foundation, localization and open PR state inspected through GitHub
  - command: git fetch origin main:refs/remotes/origin/main; git rev-list --left-right --count origin/main...HEAD
    result: PASS
    evidence: origin/main 4cab5c0842d678968d893dacf744b47ca31ef67c; task head is exactly two task-record commits ahead
  - command: gh pr list --repo blakinio/Oteryn-Platform --state open --limit 100
    result: PASS
    evidence: PRs 116, 182 and 189 touch only E2E/Liquid20 documentation; PR 194 is the only open Wiki/localization implementation owner
  - command: composer validate --strict; composer audit --no-interaction
    result: PASS
    evidence: Composer 2 on PHP 8.5.8 accepted the direct CommonMark requirement and lockfile; no advisories
  - command: vendor/bin/pint --test
    result: PASS
    evidence: all 404 PHP files passed on the current working tree
  - command: composer analyse
    result: PASS
    evidence: clean locked vendor environment on PHP 8.5.8 with pdo_mysql; 393 files, no errors
  - command: composer test
    result: PASS
    evidence: full suite passed on the current working tree under PHP 8.5.8 with required GD/PDO extensions
  - command: focused public Wiki and compatibility PHPUnit
    result: PASS
    evidence: 22 tests, 243 assertions across renderer, read/search, Wiki foundation and PublicPortal navigation
  - command: node --check; playwright test --list --project=portability-chromium --grep @wiki
    result: PASS
    evidence: seed/spec/config syntax valid and the Wiki browser test is discovered by the portability project
  - command: GitHub Acceptance E2E and Visual UX
    result: PASS
    evidence: exact PR head 7db66513b90328fd0eaa5b69efaca080e4062e78 passed Chromium smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, dependency-resilience and keyboard-accessibility profiles
  - command: required GitHub checks
    result: PASS
    evidence: CI, Agent Governance, Game Auth Ticket Concurrency, production-like validation, DB-outage validation and Synology image/package checks passed; one Docker Hub buildkit pull timeout was classified as external infrastructure and passed on failed-job rerun
blockers:
  - none
next_action: Commit and push the ready checkpoint, then mark PR 194 ready and merge only after required checks pass on the checkpoint-only head.
```

## Notes

This task is intentionally narrower than the complete Wiki programme. Administrator workflows, media integration and initial content remain separate child tasks after this slice is merged and archived.
