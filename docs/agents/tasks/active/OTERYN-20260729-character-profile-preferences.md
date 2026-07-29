---
task_id: OTERYN-20260729-character-profile-preferences
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - active tasks and open PRs overlapping Issue #277, character profile or account privacy paths
  - current Account Center, public character profile and Identity privacy implementation
  - existing Canary read ownership and mutation prohibitions
optional_reads:
  - docs/agents/tasks/archive/OTERYN-20260729-community-data-completeness.md
  - docs/architecture/adr/0017-account-security-lifecycle.md
---

# OTERYN-20260729-character-profile-preferences

## Goal

Deliver Issue #307, the Platform-owned character-profile slice of parent Issue #277: owner-editable public comments, per-character field visibility and an optional main-character preference, with no Canary mutation.

## Acceptance criteria

- [x] Current account-level privacy, public-profile assembly, Account Center inventory and Canary ownership boundaries are inventoried before implementation.
- [ ] Platform persistence stores one preference row per identity/player with bounded public comment, explicit field visibility and transactional main-character uniqueness.
- [ ] Every owner mutation verifies a ready immutable binding and current active Canary character ownership through the read-only connection before writing Platform state.
- [ ] Account-level association/status privacy remains an upper bound; per-character settings may only narrow disclosure.
- [ ] Public profile comment, main badge, guild, house, skills, deaths, kills, status and related-character sections follow effective visibility without leaking Platform/Canary identifiers.
- [ ] Related-character lists exclude siblings that explicitly hide account association.
- [ ] Account Center exposes EN/PL owner management, validation, success, unavailable and stale-ownership states with desktop/tablet/mobile behavior.
- [ ] Main-character replacement is deterministic, transactional and leaves at most one selected character per identity.
- [ ] Audit events are emitted without storing comment text or private values in audit metadata.
- [ ] Rename, delete, restore, transfer, whole-profile hiding and achievements remain explicitly outside this slice.
- [ ] Unit/feature, real-MariaDB ownership/concurrency and zero-retry browser evidence pass on the exact final head.
- [ ] Product, route, architecture, security and operations ledgers are updated only after evidence exists.
- [ ] PR merges and Issue #307 closes; parent Issue #277 remains open for the unapproved mutation lifecycles.
- [ ] Task is archived in a separate documentation PR.

## Ownership

```yaml
owned_paths:
  - app/CharacterProfiles/**
  - app/Http/Controllers/CharacterProfiles/**
  - app/Http/Requests/CharacterProfiles/**
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - app/Audit/SecurityEventRecorder.php
  - database/migrations/*character_profile_preferences*.php
  - routes/web.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/en/character_profiles.php
  - lang/pl/character_profiles.php
  - public/css/community.css
  - tests/Feature/CharacterProfiles/**
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - scripts/acceptance/**character-profile-preferences**
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - docs/architecture/adr/*character-profile*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md
modules:
  - CharacterProfiles
  - Identity
  - Accounts
  - PublicGameData
  - Audit
  - Testing
dependencies:
  - ready immutable Identity-to-Canary binding
  - read-only Canary active-character ownership lookup
  - current account-level privacy controls
blockers:
  - none for Platform-owned preferences
cross_repository_tasks:
  - none; Canary remains read-only and no external write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T16:53:00Z
head: 3f8b57370596bba277ad436197562ed7cbea8d96
branch: feat/OTERYN-20260729-character-profile-preferences
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - canary-integration
  - database
  - security
  - web-cms
  - testing
owned_paths:
  - app/CharacterProfiles/**
  - app/Http/Controllers/CharacterProfiles/**
  - app/Http/Requests/CharacterProfiles/**
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - app/Audit/SecurityEventRecorder.php
  - database/migrations/*character_profile_preferences*.php
  - routes/web.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/{en,pl}/character_profiles.php
  - public/css/community.css
  - tests/Feature/CharacterProfiles/**
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/PublicGameData/CharacterProfilePresentationTest.php
  - scripts/acceptance/**character-profile-preferences**
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - docs/{architecture,operations,testing}/**character-profile**
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md
proven:
  - Account-level public_account_association and public_status_visible flags are Platform-owned, private by default and applied by PublicCharacterProfileService.
  - Account Center reads active characters from Canary through the ready immutable binding and never writes public profile data to Canary.
  - Current public profile renders Canary comment, guild, house, skills, deaths, kills, related characters and optional status.
  - CANARY_DATA_CONTRACT explicitly forbids generic player UPDATE/DELETE; rename, deletion, restore and transfer need separate operation-specific contracts.
  - No active task or open PR owns Issue #307 or the proposed Platform preference paths.
derived:
  - Platform preferences can safely override presentation only after active ownership is revalidated through the read-only Canary connection.
  - Existing public behavior should remain the default for guild, house, skills, deaths and kills; account association and status remain private unless both account and character settings allow them.
  - A missing preference row means legacy presentation defaults; a saved row owns its comment and explicit visibility choices.
unknown:
  - Exact migration timestamp and final route/view composition after current repository conventions are inspected.
  - Whether a dedicated acceptance workflow is required or an existing bounded profile can own the new routes.
conflicts: []
first_failure:
  marker: none
  evidence: Implementation and validation have not started.
rejected_hypotheses:
  - Store owner comments in Canary players.comment.
  - Reuse a generic Canary UPDATE principal.
  - Close parent Issue #277 after only this Platform-owned slice.
  - Hide entire profiles without reconciling search, highscores, guild, deaths and sitemap leakage.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: Implementation has not started.
blockers:
  - none
next_action: Inspect current repository methods for active-character ownership and public profile assembly, then implement the Platform preference model, owner routes and effective visibility rules.
```

## Boundaries

This task writes only Platform-owned preference state. It does not rename, delete, restore, transfer or otherwise mutate a Canary character and does not claim production deployment.
