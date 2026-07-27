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
- [ ] Focused formatter, static analysis and tests pass on the exact head.
- [ ] Required GitHub checks pass on the exact head.
- [ ] Affected browser acceptance passes before merge.

## Ownership

```yaml
owned_paths:
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
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
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
updated_at: 2026-07-27T17:20:00+02:00
head: c45cc8b336029cb011c9e3c2af6798834c2763b4
branch: feat/OTERYN-20260727-portal-completeness
pr: pending
status: validating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - accounts-characters
  - public-game-data
  - security
  - accessibility
owned_paths:
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
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
  - tests/Unit/PublicGameData/CharacterPresentationTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
proven:
  - live Synology preflight run 30275482522 reported deployed release SHA 415aa3febd04c8d9c61082d4a7451352bf084013 and immutable matching Platform/Gateway images
  - the operator screenshot matches the sparse character view in deployed SHA 415aa3; the defect is product incompleteness rather than release drift
  - source changes add owned-character account reads, explicit account states, readable vocation mappings, approved guild presentation and removal of the obsolete preview route
  - Canary account IDs remain excluded from rendered account and public character output
  - no Canary, login-server, production, router or DSM write occurred
derived:
  - exact deployed release identity is reconciled and is no longer classified DEPLOYMENT_DRIFT
  - source remediation is not staging-proven until exact-head checks and a later exact-SHA deployment/browser run pass
unknown:
  - exact-head formatter, static-analysis, feature and browser results
  - remaining Issue 240 surface classifications outside this bounded slice
conflicts: []
first_failure:
  marker: none
  evidence: none observed on the fresh branch yet
rejected_hypotheses:
  - Synology was running an unknown or mismatched Platform build
  - every available Canary player field should be made public without a privacy/product contract
changed_paths:
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
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - tests/Feature/PublicPortal/DesignPreviewRouteTest.php
  - tests/Unit/PublicGameData/CharacterPresentationTest.php
validation:
  - command: inspect sanitized Synology production-target preflight artifact
    result: PASS
    evidence: run 30275482522 artifact reports deployed_release_sha 415aa3febd04c8d9c61082d4a7451352bf084013
  - command: formatter, static analysis, tests and browser acceptance
    result: NOT_RUN
    evidence: pending fresh pull-request workflow execution
blockers:
  - none
next_action: Open the fresh pull request and inspect the first exact-head workflow result.
```

## Notes

This task is the first runtime slice under Issue #240. PR #241 owns exhaustive acceptance infrastructure and Issue #244 owns the later administrator template selector. Production remains governed separately by Issue #91.
