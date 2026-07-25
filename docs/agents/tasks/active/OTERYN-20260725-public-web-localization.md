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
updated_at: 2026-07-25T18:13:00Z
head: 938cf574ce690fe62d453af5e775dbf2e140aabe
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
  - formatted implementation head before this checkpoint is 938cf574ce690fe62d453af5e775dbf2e140aabe on PR 175
  - supported target locales are exactly en and pl
  - canonical public routes use an explicit locale prefix while legacy non-localized public URLs remain deterministic English compatibility endpoints
  - Events retains its existing locale-specific translation model and localized slugs
  - Downloads artifact approval rules and PublicGameData read contracts are unchanged
  - News, managed pages, announcements and client release notes use additive editor-controlled translation records
  - translation states are missing, incomplete, draft, published and stale
  - public Polish editorial reads require a complete published fresh translation and never substitute English source content
  - PR 176 merged the isolated editorial media library into main without a localization path conflict; PR 175 remains mergeable
  - no secrets or production configuration are involved
  - schema changes are additive and reversible; Canary/login-server schema and session compatibility do not change
  - rollback is the migration down path plus application revert
  - public translation mutation routes retain existing exact content permissions and confirmed MFA
  - authentication, account and administrator routes remain outside the locale-prefixed public namespace
  - the one-shot transport and formatter workflows removed themselves from the final PR diff
  - local syntax validation passed for 62 PHP files, the Playwright scenario, lang/pl.json and translation-key parity
  - local git diff whitespace validation passed
  - package discovery initially failed because Laravel route name lookups had not been refreshed before localized route cloning
  - LocalizedPublicRouteRegistrar now refreshes route name lookups before reading source routes
  - Acceptance run 30168851327 installed application dependencies successfully on implementation head 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
  - Agent Governance run 30168851329 and Platform DB Outage run 30168851309 passed on implementation head 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
  - CI run 30168851371 passed Composer validation, dependency installation and audit, then failed only the Pint formatting step
  - one-shot Pint completed and produced canonical formatting commit 938cf574ce690fe62d453af5e775dbf2e140aabe
  - trust boundary affected: public routing and editor-controlled CMS publication only
  - authentication and authorization invariant affected: no new permission; existing exact permission plus confirmed MFA is preserved
  - rollback required: reversible migration and application revert
  - secret or production-only configuration involved: none
derived:
  - the implementation satisfies the no-cross-locale-editorial-fallback architecture without changing completed module domain ownership
  - canonical and compatibility URL behavior can coexist without uncontrolled locale-dependent duplicates because every response emits an explicit canonical URL
unknown:
  - required GitHub check results on the exact checkpoint-triggered head
conflicts: []
first_failure:
  marker: CI / Check formatting on 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
  evidence: run 30168851371 job 89706362520; Composer validation, install and audit passed before Pint failed; fixed by 938cf574ce690fe62d453af5e775dbf2e140aabe
rejected_hypotheses:
  - serving English editorial content under Polish URLs is acceptable: conflicts with the explicit truthful-publication requirement
  - automatic source copying can bootstrap Polish records: prohibited by task constraints
  - support.content.manage may edit translations for arbitrary managed pages: controller boundary restricts it to reserved editorial slugs
  - the public home route is absent: routes/modules/public-portal.php registers the named home route
  - the withRouting then callback necessarily runs before web route registration: dependency installation reached route registration after moving localization into then; the remaining failure was stale name lookup state
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
    evidence: merged PR 176 does not own localization consumer paths and PR 175 remains mergeable
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
  - command: Laravel package discovery on 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
    result: PASS
    evidence: Acceptance run 30168851327 Install application dependencies step
  - command: Agent Governance on 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
    result: PASS
    evidence: run 30168851329
  - command: Platform DB Outage Validation on 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
    result: PASS
    evidence: run 30168851309
  - command: CI on 31c40b2dd37fd7be3e011b38bb2bed98a1bae67d
    result: FAIL
    evidence: run 30168851371 failed Pint after Composer validation, install and audit passed
  - command: vendor/bin/pint
    result: PASS
    evidence: one-shot formatter produced 938cf574ce690fe62d453af5e775dbf2e140aabe and removed its workflow
  - command: required GitHub checks on exact checkpoint-triggered head
    result: NOT_RUN
    evidence: this checkpoint connector commit triggers the authoritative pull-request workflows
blockers:
  - none
next_action: Inspect required GitHub checks on the exact head, fix root causes until green, then mark PR 175 ready and squash-merge.
```

## Notes

No machine translation, automatic content duplication, commerce, Canary/login-server change or cross-repository write is authorized.
