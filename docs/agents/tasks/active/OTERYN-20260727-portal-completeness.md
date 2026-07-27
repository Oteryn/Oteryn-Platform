---
task_id: OTERYN-20260727-portal-completeness
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
search_first:
  - Issue 240 portal completeness scope
  - current open PR ownership
  - exact Synology release evidence
optional_reads:
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260727-portal-completeness

## Goal

Deliver the first bounded Issue #240 remediation: complete the authenticated account character overview, replace raw vocation IDs with readable presentation, enrich the approved public character profile and remove the obsolete public design-preview route.

## Acceptance criteria

- [x] The exact running Synology staging release is reconciled from sanitized live evidence.
- [x] Account Center lists only active characters owned through the authenticated Identity's ready immutable Canary binding.
- [x] Account Center has populated, empty, not-ready, unavailable and character-limit states without exposing Canary account identifiers.
- [x] Character Profile, Highscores and Online use readable vocation labels.
- [x] Character Profile presents an approved read-only guild relationship or explicit no-guild state.
- [x] `/design/home-v2` is no longer routable.
- [x] Focused formatter, static analysis and tests pass on the exact implementation head.
- [x] Required GitHub checks pass on the exact implementation head.
- [x] Affected Chromium, Firefox and WebKit acceptance passes before merge.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - lang/en/game.php
  - lang/pl/game.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/game/character.blade.php
  - resources/views/game/highscores.blade.php
  - resources/views/game/online.blade.php
  - routes/modules/public-portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/HomePreviewTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
  - tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
  - tests/Unit/PublicGameData/CharacterPresentationTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
modules:
  - Accounts
  - Characters
  - PublicGameData
  - PublicPortal
  - Testing
  - CI
dependencies:
  - Issue 240
  - Issue 244 for the later audited template selector
  - PR 241 owns exhaustive acceptance infrastructure
blockers:
  - none
cross_repository_tasks:
  - blakinio/canary remains read-only
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T17:40:00+02:00
head: 6fbaea6b64691cae36f2b420866b8cbb2d2c17d7
branch: feat/OTERYN-20260727-portal-completeness
pr: 246
status: review
context_routes:
  - agent-governance
  - testing
  - web-cms
  - accounts-characters
  - public-game-data
  - security
  - accessibility
  - ci
owned_paths:
  - .github/workflows/ci.yml
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - lang/en/game.php
  - lang/pl/game.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/game/character.blade.php
  - resources/views/game/highscores.blade.php
  - resources/views/game/online.blade.php
  - routes/modules/public-portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/HomePreviewTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
  - tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
  - tests/Unit/PublicGameData/CharacterPresentationTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
proven:
  - live Synology preflight run 30275482522 reported deployed release SHA 415aa3febd04c8d9c61082d4a7451352bf084013 and immutable matching Platform/Gateway images
  - the operator screenshot matches the sparse character view in deployed SHA 415aa3; the defect is product incompleteness rather than release drift
  - Account Center now reads only active characters for the authenticated ready immutable binding and handles populated, empty, unavailable, not-ready and limit states
  - readable localized vocation presentation is used by Account Center, Character Profile, Highscores and Online
  - Character Profile exposes only approved read-only level, vocation and guild/no-guild information and hides Canary account identifiers
  - the obsolete public design preview returns 404 and the retired public-preview tests now enforce that decision
  - CI failure diagnostics retain a bounded PHPUnit JUnit artifact only when tests fail
  - all required PR 246 workflows succeeded on implementation head 6fbaea6b64691cae36f2b420866b8cbb2d2c17d7
  - no Canary, login-server, production, router or DSM write occurred
derived:
  - exact deployed release identity is reconciled and is no longer classified DEPLOYMENT_DRIFT
  - this source slice is merge-ready but requires a later exact-SHA Synology deployment/live run before its UI can be classified staging-proven
unknown:
  - remaining Issue 240 surface classifications outside this bounded slice
conflicts: []
first_failure:
  marker: retired-public-preview-contract
  evidence: initial CI retained two tests requiring public 200/noindex behavior for design/home-v2; both contracts were updated to the approved 404 behavior and all reruns passed
rejected_hypotheses:
  - Synology was running an unknown or mismatched Platform build
  - every available Canary player field should be made public without a privacy/product contract
  - the new Account Center heading should replace the established accessible Account overview heading contract
changed_paths:
  - .github/workflows/ci.yml
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - lang/en/game.php
  - lang/pl/game.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/game/character.blade.php
  - resources/views/game/highscores.blade.php
  - resources/views/game/online.blade.php
  - routes/modules/public-portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/HomePreviewTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
  - tests/Feature/PublicPortal/HomepageNavigationSeoTest.php
  - tests/Unit/PublicGameData/CharacterPresentationTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
validation:
  - command: inspect sanitized Synology production-target preflight artifact
    result: PASS
    evidence: run 30275482522 artifact reports deployed_release_sha 415aa3febd04c8d9c61082d4a7451352bf084013
  - command: CI run 30280097524
    result: PASS
    evidence: formatting, PHPStan and complete PHPUnit suite succeeded
  - command: Phase 7 Production-Like Validation run 30280097228
    result: PASS
    evidence: exact-SHA critical regressions and production-like boundaries succeeded
  - command: Acceptance E2E and Visual UX run 30280095875
    result: PASS
    evidence: required critical Chromium, Firefox and WebKit profile succeeded
  - command: Agent Governance, Platform DB Outage, Game Auth Ticket Concurrency, Edge Security Emulation, Build Synology Staging Images and Synology Target Preflight
    result: PASS
    evidence: runs 30280095881, 30280097530, 30280097316, 30280098390, 30280097399 and 30280097354 succeeded
blockers:
  - none
next_action: Mark PR 246 ready and squash-merge after the docs-only checkpoint commit receives required checks.
```

## Notes

This task is the first runtime slice under Issue #240. PR #241 owns exhaustive acceptance infrastructure and Issue #244 owns the later administrator template selector. Production remains governed separately by Issue #91.
