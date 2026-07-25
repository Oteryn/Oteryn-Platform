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

- [ ] Supported public locales are exactly `en` and `pl`, with an explicit deterministic default and negotiation policy.
- [ ] Locale-aware public URLs are stable and canonical; legacy non-localized bookmarks follow an intentional tested compatibility policy.
- [ ] The language switcher preserves equivalent public routes where possible and never fabricates missing translated content.
- [ ] Missing, incomplete, draft or stale editorial translations are explicit and are not automatically published or silently replaced with another language.
- [ ] Public navigation, footer, dates, numbers, 404 and unavailable states are localized.
- [ ] Existing Downloads, Events, Wiki and PublicGameData domain rules remain unchanged.
- [ ] Translation-focused feature and representative browser tests pass together with required CI on the exact head.

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
updated_at: 2026-07-25T13:43:06Z
head: 275704190efab3f9500a1951f290fb8af2f1f40e
branch: feat/OTERYN-20260725-public-web-localization
pr: 175
status: validating
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - testing
owned_paths:
  - localization and public presentation paths listed in Ownership
proven:
  - current implementation head is 275704190efab3f9500a1951f290fb8af2f1f40e on PR 175
  - supported target locales are exactly en and pl
  - canonical public routes use an explicit locale prefix while legacy non-localized public URLs remain deterministic English compatibility endpoints
  - Events retains its existing locale-specific translation model and localized slugs
  - Downloads artifact approval rules and PublicGameData read contracts are unchanged
  - News, managed pages, announcements and client release notes use additive editor-controlled translation records
  - translation states are missing, incomplete, draft, published and stale
  - public Polish editorial reads require a complete published fresh translation and never substitute English source content
  - open PR 176 owns isolated editorial media paths and does not integrate CMS, Events, Wiki or public localization
  - repository checkout cannot be cloned in the execution sandbox because DNS resolution is unavailable
  - no secrets or production configuration are involved
  - schema changes are additive and reversible; Canary/login-server schema and session compatibility do not change
  - rollback is the migration down path plus application revert
  - public translation mutation routes retain existing exact content permissions and confirmed MFA
  - authentication, account and administrator routes remain outside the locale-prefixed public namespace
  - the one-shot transport workflow removed itself and all payload fragments from the final PR diff
  - local syntax validation passed for 62 PHP files, the Playwright scenario, lang/pl.json and translation-key parity
  - local git diff whitespace validation passed
  - trust boundary affected: public routing and editor-controlled CMS publication only
  - authentication and authorization invariant affected: no new permission; existing exact permission plus confirmed MFA is preserved
  - rollback required: reversible migration and application revert
  - secret or production-only configuration involved: none
derived:
  - the implementation satisfies the no-cross-locale-editorial-fallback architecture without changing completed module domain ownership
  - canonical and compatibility URL behavior can coexist without uncontrolled locale-dependent duplicates because every response emits an explicit canonical URL
unknown:
  - required GitHub check results on the exact current head
conflicts: []
first_failure:
  marker: none
  evidence: focused local syntax and structural checks pass; GitHub CI is pending
rejected_hypotheses:
  - serving English editorial content under Polish URLs is acceptable: conflicts with the explicit truthful-publication requirement
  - automatic source copying can bootstrap Polish records: prohibited by task constraints
  - support.content.manage may edit translations for arbitrary managed pages: controller boundary restricts it to reserved editorial slugs
changed_paths:
  - app/Localization/**
  - app/Cms/Editorial/**
  - app/Cms/Actions/SaveEditorialTranslation.php
  - app/Cms/Models/EditorialTranslation.php
  - app/Http/Controllers/Admin/AdminEditorialTranslationController.php
  - app/Http/Middleware/**PublicLocale*.php
  - app/Http/Middleware/SetPublicLocale.php
  - app/Http/Requests/Admin/AdminEditorialTranslationRequest.php
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
  - command: repository and open-PR overlap review
    result: PASS
    evidence: PR 176 excludes consumer integration and does not own localization paths
  - command: PHP syntax validation for implementation files
    result: PASS
    evidence: 62 PHP files passed php -l before commit and again in the one-shot apply workflow
  - command: node --check scripts/acceptance/tests/public-localization.spec.mjs
    result: PASS
    evidence: local and one-shot workflow validation
  - command: python -m json.tool lang/pl.json and EN/PL key parity
    result: PASS
    evidence: local structural validation
  - command: git diff --check
    result: PASS
    evidence: local and one-shot workflow validation
  - command: required GitHub checks on exact head
    result: NOT_RUN
    evidence: task checkpoint commit will trigger the authoritative pull-request workflows
blockers:
  - none
next_action: Inspect required GitHub checks on the exact head, fix root causes until green, then mark PR 175 ready and squash-merge.
```

## Notes

No machine translation, automatic content duplication, commerce, Canary/login-server change or cross-repository write is authorized.
