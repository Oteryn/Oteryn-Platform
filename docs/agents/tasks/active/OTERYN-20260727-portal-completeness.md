---
task_id: OTERYN-20260727-portal-completeness
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
search_first:
  - Issue 240 portal completeness scope
  - active tasks and open PR ownership
  - current public, Identity, Account, Character, Admin and Wiki routes
  - exact Synology staging release identity evidence
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/testing/E2E_COVERAGE_ROADMAP.md
---

# OTERYN-20260727-portal-completeness

## Goal

Establish a fail-closed route-by-route completeness programme and deliver the first bounded remediation for the directly proven Account Center, public character presentation and obsolete public design-preview defects.

## Acceptance criteria

- [x] A durable completeness matrix inventories every public, Identity, Account, Character, Admin, CMS, Events, Downloads, Wiki, Media and error-state surface.
- [x] Direct operator evidence showing Synology output that differs from repository views is recorded as `DEPLOYMENT_DRIFT` rather than promoted to a staging PASS.
- [x] Account Center reads and presents only active characters owned through the authenticated Identity's ready immutable Canary binding.
- [x] Account Center distinguishes populated, empty, not-ready, dependency-unavailable and active-character-limit states without exposing internal account identifiers.
- [x] Public character, highscores and online views use human-readable vocation names instead of raw vocation IDs.
- [x] Public character profile presents an explicit guild link or explicit no-guild state using the read-only Canary boundary.
- [x] The obsolete public `/design/home-v2` route is removed and protected by a regression test.
- [ ] Focused formatter/static analysis/unit/feature validation passes on the exact implementation head.
- [ ] Required GitHub checks pass on the exact final head.
- [ ] Exact running Synology Platform image SHA is reconciled before deployment-completeness claims resume.
- [ ] The complete Issue #240 matrix is executed through later bounded child slices and exact-SHA live staging acceptance.

## Ownership

```yaml
owned_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
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
modules:
  - AgentGovernance
  - Testing
  - PublicPortal
  - Accounts
  - Characters
  - PublicGameData
dependencies:
  - Issue 240
  - Issue 91 production boundary
blockers:
  - exact running Synology Platform image SHA is not directly proven from the operator-observed environment
cross_repository_tasks:
  - blakinio/canary remains read-only and was inspected only for schema and vocation evidence
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T17:02:00+02:00
head: 096edde1cad0e316c6bc59e9979c7d8d940cd0a9
branch: audit/OTERYN-20260727-portal-completeness
pr: 242
status: validating
context_routes:
  - agent-governance
  - testing
  - web-cms
  - accounts-characters
  - public-game-data
  - security
  - accessibility
  - deployment
owned_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
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
proven:
  - Issue 240 records the fail-closed full-portal audit and remediation programme
  - current main Account Overview was a narrow provisioning/security summary without an owned-character list
  - current main Character Profile selected and rendered only id, name, level and numeric vocation
  - operator-observed Synology Character Profile does not match current main or recorded final public-staging SHA 415aa3febd04c8d9c61082d4a7451352bf084013
  - final public Synology smoke recorded only six public assertions and did not prove every portal surface
  - Canary schema and ADR 0005 prove the supported vocation ID mapping used by the presentation service
  - the current implementation branch adds bound-account active-character reads, explicit Account Center character states, readable vocation labels, a read-only guild relationship and removes the obsolete public design preview route
  - internal Canary account identifiers remain excluded from Account Center and public profile output
  - no Canary, login-server, production, router or DSM write occurred
derived:
  - deployed release identity remains DEPLOYMENT_DRIFT until the running image SHA and deployed file content are reconciled
  - the first Account Center and Character Profile skeleton defects are addressed in source but are not yet validated or deployed
  - the administrator homepage-template selector requires a separate bounded persistence, RBAC, audit, preview and rollback design rather than a public query parameter
unknown:
  - exact running Synology Platform image digest and application SHA for the operator-observed screens
  - exact result of formatter, PHPStan, focused tests, complete tests and browser acceptance on the implementation head
  - remaining Issue 240 row classifications after direct audit of every route and meaningful state
conflicts:
  - repository/staging completion declarations conflict with direct operator evidence of unfinished screens and code/deployment mismatch
first_failure:
  marker: deployed-character-view-mismatch
  evidence: operator screenshot renders Sex, Vocation, Guild, Residence and Last login while repository character.blade.php at main and SHA 415aa3 renders only Level and numeric Vocation ID
rejected_hypotheses:
  - the observed Character Profile is the exact view from current main: repository source differs materially
  - the final six-route public smoke proves complete portal acceptance: it did not exercise Identity, Account, Character, Admin or most public modules
  - adding every available Canary field is automatically safe: last-login, sex and other profile fields require explicit privacy and product decisions
changed_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/CanaryGameDataRepository.php
  - app/PublicGameData/CharacterPresentation.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/testing/PORTAL_COMPLETENESS_MATRIX.md
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
  - command: repository and staging evidence reconciliation through GitHub connector plus direct operator screenshots
    result: PASS
    evidence: deployment drift and original Account/Character skeletons are directly proven and recorded fail closed
  - command: formatter, static analysis and focused/full tests
    result: NOT_RUN
    evidence: local checkout cannot reach GitHub and connector-authored commits emitted no PR workflow runs; PR reopen is being used to request an actual pull_request validation event
blockers:
  - exact running Synology Platform image SHA cannot be read through the current environment
  - exact-head implementation validation remains pending
next_action: Reopen PR #242, inspect every resulting required workflow, and fix the first failing root cause.
```

## Notes

PR #242 is the first bounded remediation under Issue #240. Later route families and the audited homepage-template selector remain separate child slices. Production verification remains Issue #91.
