---
task_id: OTERYN-20260726-homepage-navigation-seo
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260724-public-web-parallel-foundation.md
  - docs/agents/tasks/active/OTERYN-20260724-announcements-events.md
  - docs/agents/tasks/active/OTERYN-20260724-download-center.md
  - docs/agents/tasks/archive/OTERYN-20260724-editorial-support-legal.md
  - docs/agents/tasks/archive/OTERYN-20260724-public-game-statistics.md
  - docs/agents/tasks/archive/OTERYN-20260725-public-web-localization.md
search_first:
  - active tasks and open pull requests touching homepage, public navigation, localization, SEO, sitemap, robots, Open Graph or acceptance tests
  - existing homepage view models and truthful dependency-state composition
  - reusable AnnouncementTickerProvider, UpcomingEventProvider and their view components
  - current route-driven public navigation registry and module contributions
  - current localized canonical, hreflang, language-switcher and metadata conventions
  - all public, administrator, account, preview, search and signed routes relevant to crawl policy
  - current sitemap, robots, Open Graph and structured-data implementations
  - responsive, portability, resilience and keyboard-accessibility acceptance coverage for the shared public shell
optional_reads:
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
---

# OTERYN-20260726-homepage-navigation-seo

## Goal

Close the remaining bounded homepage, shared public navigation, localization and search-engine discoverability scope for Issue #145 by composing existing truthful module providers, exposing only live public routes and adding fail-closed localized metadata, sitemap and robots behavior without new persistence or external writes.

## Acceptance criteria

- [ ] The homepage composes the existing active-announcement ticker and upcoming-event provider/component boundaries without duplicating their queries or hiding AVAILABLE, EMPTY or UNAVAILABLE states.
- [ ] Homepage quick links and shared EN/PL navigation expose the existing Download Center, guild index, Wiki, Events and approved support routes without dead links or invented runtime/content state.
- [ ] Every crawlable public route has a unique escaped title, description, canonical URL, localized alternates where an equivalent exists and bounded Open Graph metadata.
- [ ] The XML sitemap contains only effective crawlable public URLs and published localized dynamic content, with deterministic deduplication and no drafts, previews, searches, account/security, administrator or signed routes.
- [ ] `robots.txt` truthfully points to the sitemap and excludes administrator, account/security, preview, search and other non-indexable surfaces without being treated as an authorization boundary.
- [ ] Missing translations, unpublished content and unavailable dependencies neither authorize sitemap entries nor produce false alternates or fabricated homepage data.
- [ ] Existing publication freshness, private media, CSP, escaping, external-link validation, public-game-data freshness and cache boundaries remain unchanged.
- [ ] Focused unit/feature/security tests and responsive, portability and keyboard-accessibility Playwright coverage pass on the exact final head.
- [ ] No migration, new persistence, arbitrary HTML/remote image, upload format, wildcard permission, Canary/login-server change, production action or external-repository write is introduced.

## Ownership

```yaml
owned_paths:
  - app/PublicPortal/HomePageQuery.php
  - app/PublicPortal/ViewModels/HomePageViewModel.php
  - app/PublicPortal/Seo/**
  - app/Cms/PublicNewsQuery.php
  - app/Cms/PublicPageQuery.php
  - app/Wiki/Queries/Public/PublicWikiQuery.php
  - app/Wiki/Queries/Public/DatabasePublicWikiQuery.php
  - app/Http/Controllers/PublicPortal/**
  - routes/modules/public-portal.php
  - resources/navigation/public/downloads.php
  - resources/navigation/public/guilds.php
  - resources/views/home.blade.php
  - resources/views/game/layout.blade.php
  - resources/views/game/partials/**
  - resources/views/game/partials/seo.blade.php
  - resources/views/seo/**
  - resources/views/news/show.blade.php
  - resources/views/pages/show.blade.php
  - resources/views/events/show.blade.php
  - resources/views/wiki/index.blade.php
  - resources/views/wiki/category.blade.php
  - resources/views/wiki/article.blade.php
  - resources/views/wiki/search.blade.php
  - resources/views/wiki/unavailable.blade.php
  - resources/views/home-preview.blade.php
  - resources/views/identity/layout.blade.php
  - resources/views/admin/layout.blade.php
  - lang/en/public.php
  - lang/pl/public.php
  - public/css/public-shell.css
  - public/css/home-production.css
  - tests/Feature/HomeTest.php
  - tests/Feature/PublicSiteShellTest.php
  - tests/Feature/PublicPortal/**
  - tests/Feature/Localization/PublicLocalizationTest.php
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - scripts/acceptance/playwright.config.mjs
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-homepage-navigation-seo.md
modules:
  - PublicPortal
  - Announcements
  - Events
  - Downloads
  - PublicGameData
  - Localization
  - Testing
dependencies:
  - Issue #145
  - PR #146 public homepage and shell
  - PR #157 Announcements and Events
  - PR #160 public game statistics and guild index
  - PR #161 Download Center
  - PR #175 public localization
  - PR #194 public Wiki read and search
  - PR #199 Wiki EditorialMedia integration
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T12:51:11Z
head: a7ee7befa8e94fa1427f9909be5e23a2ca62f575
branch: feat/OTERYN-20260726-homepage-navigation-seo
pr: 206
status: implementing
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - public-game-data
  - security
  - testing
  - accessibility
  - localization
owned_paths:
  - homepage, shared navigation, SEO and acceptance paths listed in Ownership
proven:
  - trusted main is 525167db87b3f9309c0100d2c8ed78b83901d970 after the PR 199 archival lifecycle merged through PR 205
  - Issue 145 remains open and its programme checkpoint names homepage-navigation-seo as the next non-overlapping bounded child
  - open PRs 189, 182 and 116 do not overlap the declared runtime or documentation paths
  - AnnouncementTickerProvider and UpcomingEventProvider already exist for homepage composition
  - both providers preserve AVAILABLE, EMPTY and UNAVAILABLE independently and their existing Blade components render bounded localized states
  - Download Center, guild index, Events, Wiki and approved support routes already exist
  - the PublicNavigationRegistry loads route-name-only contributions, omits missing routes and localizes labels through the current request locale
  - localization already provides stable en and pl routes, canonical URLs, freshness-aware alternates and route-preserving language switching
  - current public SEO has canonical and hreflang links plus a few page-local descriptions, but no shared Open Graph metadata, sitemap or robots route
  - existing identity and administrator layouts do not emit noindex metadata while preview, Wiki search, Wiki unavailable and error views already do
  - existing critical acceptance covers shared-shell responsiveness, browser portability and keyboard navigation but does not assert the new homepage blocks, navigation additions or crawl metadata
  - the completed PR 157 and PR 161 task records retain stale active-file locations, but their checkpoints and the Issue 145 programme delegate homepage integration and shared navigation to this later child and no live PR owns those paths
  - no external repository, production, router, DSM or Internet-exposure write is authorized
  - draft PR 206 is the live review surface for this dedicated task branch
  - homepage composition now delegates to AnnouncementTickerProvider and UpcomingEventProvider and renders their existing AVAILABLE, EMPTY and UNAVAILABLE boundaries
  - the navigation registry now receives route-name-only Download Center and guild index contributions and still omits unregistered routes
  - shared public metadata now sanitizes dynamic titles and descriptions, escapes output and derives canonical and translation-aware Open Graph metadata from LocalizedUrlGenerator
  - identity, administrator, preview, Wiki search and Wiki unavailable surfaces emit explicit noindex directives while their authentication and signed-route boundaries remain unchanged
  - the localized sitemap reuses publication and freshness query boundaries for news, managed pages, typed editorial pages, Events and Wiki and returns 503 without partial URLs when a dependency fails
  - robots.txt is public crawl guidance pointing to the sitemap and does not alter authorization
  - focused feature coverage proves homepage composition, live links, metadata escaping, noindex, robots, published-only localized sitemap entries and fail-closed dependency behavior
derived:
  - the task can extend the existing PublicPortal orchestration and navigation registry without new persistence or raw Canary reads
  - one bounded child can close homepage composition, navigation discoverability and their shared SEO/crawl-policy presentation because they converge on the existing public shell
  - character and guild detail routes remain discoverable through live public data and metadata but are intentionally absent from the deterministic sitemap because no authoritative bounded enumeration contract exists
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - create new announcement or event queries: existing module providers are the authoritative reusable boundaries
  - add gameplay-specific Wiki content: approved source text remains UNKNOWN and belongs to a later child
  - make robots directives an access-control mechanism: authentication, MFA, exact permissions and signed preview boundaries remain authoritative
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260726-homepage-navigation-seo.md
  - docs/agents/ACTIVE_WORK.md
  - homepage provider/view-model, homepage Blade and production CSS
  - route-driven Download and guild navigation contributions
  - shared public, identity and administrator metadata layouts
  - publication-aware sitemap and robots query/controller/routes/views
  - narrow News, ManagedPage and Wiki published-slug query extensions
  - focused feature and Playwright acceptance coverage
validation:
  - command: branch, worktree, active-task and open-PR preflight
    result: PASS
    evidence: dedicated clean branch at trusted main 525167db87b3f9309c0100d2c8ed78b83901d970; no overlapping open pull request
  - command: every required_read and every search_first reconciliation
    result: PASS
    evidence: repository governance, programme architecture, completed child checkpoints, PublicPortal/Localization/module source, route inventory and required acceptance profiles inspected on trusted main
  - command: vendor/bin/pint
    result: PASS
    evidence: 441 PHP files formatted
  - command: vendor/bin/phpstan analyse --memory-limit=1G --no-progress
    result: PASS
    evidence: level 10 analysis completed without errors
  - command: php artisan test --compact tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
    result: PASS
    evidence: 6 tests and 56 assertions passed
blockers:
  - none
next_action: Run the complete focused and repository validation plus all required Playwright profiles on the exact implementation head, then publish the reviewed implementation commit to PR 206.
```

## Notes

Initial Wiki publication content and final Synology staging deployment remain separate Issue #145 children. Robots directives are discoverability guidance, never access control.
