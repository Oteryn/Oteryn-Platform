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
- [ ] Locale-aware public URLs are stable and canonical; legacy non-localized bookmarks follow an intentional tested redirect policy.
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
updated_at: 2026-07-25T12:45:00Z
head: bd0bd9883e2753c8a385b3297aaed7a1cb2ce429
branch: feat/OTERYN-20260725-public-web-localization
pr: 175
status: implementing
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - database
  - testing
owned_paths:
  - localization and public presentation paths listed in Ownership
proven:
  - current main and task branch baseline is bd0bd9883e2753c8a385b3297aaed7a1cb2ce429
  - supported target locales are exactly en and pl
  - Events already owns locale-specific translations; Downloads artifact rules and PublicGameData read contracts must remain unchanged
  - News, managed pages, announcements and client release notes currently use English source fields without a shared translation workflow
  - open PR 176 owns isolated editorial media paths and does not integrate CMS, Events, Wiki or public localization
  - repository checkout cannot be cloned in the execution sandbox because DNS resolution is unavailable; exact validation will run in GitHub Actions
  - no secrets or production configuration are involved
  - schema changes are additive and reversible; Canary/login-server schema and session compatibility do not change
  - rollback is the migration down path plus application revert
  - public translation publication remains editor-controlled behind existing exact permissions and confirmed MFA
  - the public boundary denies cross-locale editorial fallback
  - authentication and account routes remain outside the locale-prefixed public namespace
derived:
  - canonical public routes can be introduced by wrapping existing route actions without changing module domain rules
  - a shared additive translation table can cover existing English-source editorial modules while preserving Events-owned translations
unknown:
  - exact CI failures, if any, until the implementation is committed and workflows complete
conflicts: []
first_failure:
  marker: none
  evidence: implementation validation has not started
rejected_hypotheses:
  - serving English editorial content under Polish URLs is acceptable: conflicts with the explicit truthful-publication requirement
  - automatic source copying can bootstrap Polish records: prohibited by task constraints
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-public-web-localization.md
validation:
  - command: repository and open-PR overlap review
    result: PASS
    evidence: PR 176 excludes consumer integration and does not own localization paths
  - command: local full repository validation
    result: BLOCKED
    evidence: no checkout or DNS access; GitHub Actions will be authoritative
blockers:
  - none
next_action: Commit the complete localization implementation and run required GitHub checks on the exact PR head.
```

## Notes

No machine translation, automatic content duplication, commerce, Canary/login-server change or cross-repository write is authorized.
