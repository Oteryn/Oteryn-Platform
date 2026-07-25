---
task_id: OTERYN-20260725-public-web-localization
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
search_first:
  - existing localization middleware, routes, language resources and public SEO conventions
  - active tasks and open pull requests touching public routes, CMS, Events, Downloads, Wiki or shared public views
  - merged public module translation schemas and publication rules
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
---

# OTERYN-20260725-public-web-localization

## Goal

Deliver a stable Polish and English localization foundation for completed public modules, including deterministic locale-aware routing, language selection, truthful translation publication states, locale formatting and focused browser/feature acceptance.

## Acceptance criteria

- [x] Supported public locales are exactly `en` and `pl`, with an explicit deterministic default and negotiation policy.
- [x] Locale-aware public URLs are stable and canonical; legacy non-localized bookmarks follow an intentional tested compatibility policy.
- [x] The language switcher preserves equivalent public routes where possible and never fabricates missing translated content.
- [x] Missing, incomplete, draft or stale editorial translations are explicit and are not automatically published or silently replaced with another language.
- [x] Public navigation, footer, dates, numbers, 404 and unavailable states are localized.
- [x] Existing Downloads, Events, Wiki and PublicGameData domain rules remain unchanged.
- [x] Translation-focused feature and representative browser tests pass together with required CI on the exact implementation head.

## Ownership

```yaml
owned_paths:
  - app/Localization/**
  - app/Cms/Editorial/**
  - app/Cms/Models/EditorialTranslation.php
  - app/Cms/Actions/SaveEditorialTranslation.php
  - app/Http/Middleware/*PublicLocale*.php
  - app/Http/Middleware/SetPublicLocale.php
  - app/Http/Controllers/Admin/AdminEditorialTranslationController.php
  - app/Http/Requests/Admin/AdminEditorialTranslationRequest.php
  - config/localization.php
  - database/migrations/*editorial_translations*
  - lang/en/public.php
  - lang/pl/public.php
  - lang/pl.json
  - resources/views/game/**
  - resources/views/news/**
  - resources/views/pages/**
  - resources/views/support/**
  - resources/views/events/**
  - resources/views/downloads/**
  - resources/views/announcements/**
  - resources/views/errors/**
  - resources/views/admin/translations/**
  - resources/navigation/public/**
  - routes/web.php
  - routes/localization.php
  - bootstrap/app.php
  - tests/Feature/Localization/**
  - scripts/acceptance/tests/public-localization.spec.mjs
  - docs/architecture/PUBLIC_LOCALIZATION_POLICY.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-localization.md
modules:
  - PublicPortal
  - CMS
  - Announcements
  - Events
  - Downloads
  - Localization
dependencies:
  - PUBLIC_WEBSITE_EXPANSION_PLAN.md localization slice
  - WIKI_IMPLEMENTATION_PLAN.md internationalization contract
  - merged public module routes and publication rules
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T19:06:00Z
head: 0b380fa125d8057239804f315b414824d54cc577
branch: feat/OTERYN-20260725-public-web-localization
pr: 175
status: ready
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - testing
owned_paths:
  - localization and public presentation paths listed in Ownership
proven:
  - supported public locales are exactly en and pl
  - legacy root remains / while canonical localized home routes are /en and /pl
  - other canonical public routes use an explicit locale prefix and legacy public bookmarks remain deterministic English compatibility endpoints
  - public navigation, SEO metadata and language switching point to canonical locale-aware routes
  - Events retains its existing translation and scheduling model, including equivalent localized slugs
  - Downloads artifact approval rules and PublicGameData read contracts are unchanged
  - News, managed pages, announcements and client release notes use additive editor-controlled translation records
  - translation states are missing, incomplete, draft, published and stale
  - Polish editorial reads require a complete published fresh translation and never substitute English source content
  - translation mutation routes preserve existing exact permissions and confirmed MFA
  - authentication, account and administrator routes remain outside the locale-prefixed namespace
  - authentication redirect behavior remains legacy-compatible at /
  - schema changes are additive and reversible
  - Canary/login-server schema and session compatibility do not change
  - no secrets, production-only configuration, machine translation, automatic content duplication or commerce are involved
  - all temporary diagnostic and one-shot workflow files were removed from the feature diff
  - PR 175 is mergeable against current main and merged media PR 176 has no localization path conflict
  - exact implementation head 0b380fa125d8057239804f315b414824d54cc577 passed all required workflows
  - CI run 30170704911 passed Composer validation, dependency audit, Pint, PHPStan and full tests
  - Agent Governance run 30170704878 passed
  - Platform DB Outage Validation run 30170704882 passed
  - Game Auth Ticket Concurrency run 30170704885 passed
  - Phase 7 Production-Like Validation run 30170704883 passed
  - Acceptance E2E and Visual UX run 30170704890 passed all required browser and visual profiles
  - Build Synology Staging Images run 30170704877 passed after retrying only the transient deployment-package validator; all image builds passed on the original attempt
  - focused localization tests passed 9 scenarios with 96 assertions
  - trust boundary affected: public routing and editor-controlled CMS publication only
  - authentication and authorization invariant affected: no new permission; existing exact permission plus confirmed MFA is preserved
  - rollback required: reversible migration and application revert
  - secret or production-only configuration involved: none
derived:
  - the implementation satisfies the no-cross-locale-editorial-fallback architecture without changing completed module domain ownership
  - canonical and compatibility URL behavior coexist without uncontrolled locale-dependent duplicates because every public response emits explicit canonical and alternate metadata
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: all required workflows passed on exact implementation head 0b380fa125d8057239804f315b414824d54cc577
rejected_hypotheses:
  - serving English editorial content under Polish URLs is acceptable: conflicts with the truthful-publication requirement
  - automatic source copying can bootstrap Polish records: prohibited by task constraints
  - support.content.manage may edit translations for arbitrary managed pages: controller boundary restricts it to reserved editorial slugs
  - authentication redirects should be moved into the localized namespace: explicitly excluded and contradicted existing compatibility acceptance
  - canonical localized home must replace the legacy home route name: separate home and localized.home names preserve both contracts
changed_paths:
  - app/Localization/**
  - app/Cms/Editorial/**
  - app/Cms/Actions/SaveEditorialTranslation.php
  - app/Cms/Models/EditorialTranslation.php
  - app/Cms/Models/NewsPost.php
  - app/Http/Controllers/Admin/AdminEditorialTranslationController.php
  - app/Http/Controllers/Identity/SessionController.php
  - app/Http/Controllers/Identity/Mfa/MfaChallengeController.php
  - app/Http/Middleware/**PublicLocale*.php
  - app/Http/Middleware/SetPublicLocale.php
  - app/Http/Middleware/RequestCorrelation.php
  - app/Http/Requests/Admin/AdminEditorialTranslationRequest.php
  - app/Providers/AppServiceProvider.php
  - app/PublicPortal/Navigation/PublicNavigationRegistry.php
  - bootstrap/app.php
  - config/localization.php
  - database/migrations/2026_07_25_090000_create_editorial_translations_table.php
  - lang/**
  - resources/views/**
  - public/css/public-shell.css
  - routes/localization.php
  - routes/web.php
  - scripts/acceptance/tests/public-localization.spec.mjs
  - tests/Feature/Localization/PublicLocalizationTest.php
  - docs/architecture/PUBLIC_LOCALIZATION_POLICY.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-localization.md
validation:
  - command: composer validate --strict; composer audit --no-interaction; vendor/bin/pint --test; composer analyse; composer test
    result: PASS
    evidence: CI run 30170704911 on 0b380fa125d8057239804f315b414824d54cc577
  - command: focused localization feature tests
    result: PASS
    evidence: 9 scenarios and 96 assertions
  - command: Agent Governance
    result: PASS
    evidence: run 30170704878
  - command: Platform DB Outage Validation
    result: PASS
    evidence: run 30170704882
  - command: Game Auth Ticket Concurrency
    result: PASS
    evidence: run 30170704885
  - command: Phase 7 Production-Like Validation
    result: PASS
    evidence: run 30170704883
  - command: Acceptance E2E and Visual UX
    result: PASS
    evidence: run 30170704890
  - command: Build Synology Staging Images
    result: PASS
    evidence: run 30170704877; validator retry passed and all three image builds passed
  - command: exact-head final checkpoint validation
    result: NOT_RUN
    evidence: this document-only checkpoint commit triggers the final authoritative pull-request workflows
blockers:
  - none
next_action: Validate the final document-only head, mark PR 175 ready and squash-merge.
```

## Notes

No machine translation, automatic content duplication, commerce, Canary/login-server change or cross-repository write is authorized.
