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
- [x] Platform persistence stores one preference row per identity/player with bounded public comment, explicit field visibility and transactional main-character uniqueness.
- [x] Every owner mutation verifies a ready immutable binding and current active Canary character ownership through the read-only connection before writing Platform state.
- [x] Account-level association/status privacy remains an upper bound; per-character settings may only narrow disclosure.
- [x] Public profile comment, main badge, guild, house, skills, deaths, kills, status and related-character sections follow effective visibility without leaking Platform/Canary identifiers.
- [x] Related-character lists exclude siblings that explicitly hide account association.
- [x] Account Center exposes EN/PL owner management, validation, success, unavailable and stale-ownership states with desktop/tablet/mobile behavior.
- [x] Main-character replacement is deterministic, transactional and leaves at most one selected character per identity.
- [x] Audit events are emitted without storing comment text or private values in audit metadata.
- [x] Rename, delete, restore, transfer, whole-profile hiding and achievements remain explicitly outside this slice.
- [x] Unit/feature, real-MariaDB ownership/concurrency and zero-retry browser evidence pass on the exact final head.
- [x] Product, route, architecture, security and operations ledgers are updated only after evidence exists.
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
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
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
  - none
cross_repository_tasks:
  - none; Canary remains read-only and no external write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T20:50:00Z
head: d2e611f1c110e57d6cfec5dffedd930eee99aab6
branch: feat/OTERYN-20260729-character-profile-preferences
pr: 308
status: validating
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
  - database/migrations/*character_profile_preferences*.php
  - routes/modules/character-profile-preferences.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/{en,pl}/character_profiles.php
  - tests/Feature/CharacterProfiles/**
  - scripts/acceptance/seed-community-data.php
  - scripts/acceptance/update-character-profile-preference.php
  - scripts/acceptance/assert-character-profile-concurrency.php
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md
proven:
  - Character profile preferences are Platform-owned and store one row per Identity and Canary player ID; the additive migration cascades with the Platform Identity and never mutates Canary.
  - Every edit/update repeats current ownership proof from the ready immutable binding and a read-only active Canary character lookup; missing, foreign and dependency-failure states fail closed without exposing identifiers or SQL details.
  - A missing preference row preserves the delivered public profile; a stored row owns the bounded escaped public comment and can narrow account association, status, guild, house, skills, deaths and kill statistics.
  - Account-level public_account_association and public_status_visible remain upper bounds; feature tests prove per-character opt-in cannot expose related characters or status when those account flags are disabled.
  - Related-character projection excludes sibling preferences that hide account association and never emits Platform Identity, Canary account or internal binding identifiers.
  - Main-character replacement locks the Identity row, demotes any prior main and is bounded by the unique identity/player row key.
  - Community Data run 30489605994 passed focused owner/non-owner, validation, privacy-upper-bound and sanitized-unavailable tests; its two concurrent real-MariaDB processes left exactly one main character and its zero-retry Chromium desktop/tablet/mobile lifecycle passed in EN/PL.
  - CI run 30489605987 passed Pint, Composer audit, PHPStan and the full test suite at d2e611f1c110e57d6cfec5dffedd930eee99aab6.
  - Portal Acceptance run 30489605983 passed strict route/product ledgers with 43 capabilities classified as 23 implemented, 3 partial, 14 missing and 3 not applicable, then passed the complete zero-retry account lifecycle.
  - Phase 7 run 30489606055 passed clean migration, least privilege, failure semantics, exact-SHA regressions, backup/restore, upgrade, rollback and redeploy without a production claim.
  - Acceptance E2E and Visual UX run 30489605985 passed browser portability, responsive, dependency resilience and keyboard accessibility profiles.
  - Edge 30489606033, Game Auth 30489606000, DB Outage 30489606038, Synology preflight 30489606077, image build 30489606022 and Governance 30489606030 all passed at the same exact head.
derived:
  - Issue #307 can close independently because its complete Platform-owned lifecycle is proven while parent #277 retains explicit ownership of rename, deletion/restore, controlled world/channel transfer and authoritative achievement selection.
  - The new public profile fields do not require sitemap/search/cache invalidation because this slice never hides the entire character and only changes rendered owner-controlled fields at request time.
unknown:
  - Whether the evidence-only checkpoint commit exposes any additional governance or exact-head regression.
conflicts: []
first_failure:
  marker: none
  evidence: All required workflows passed at exact head d2e611f1c110e57d6cfec5dffedd930eee99aab6. The prior broad Online text assertion was narrowed to the actual visible-status markup and then passed CI and Community Data.
rejected_hypotheses:
  - Store owner comments in Canary players.comment.
  - Reuse a generic Canary UPDATE principal.
  - Treat a stored preference row as ownership proof.
  - Allow character-level opt-in to override account-level privacy.
  - Close parent Issue #277 after only this Platform-owned slice.
  - Hide entire profiles without reconciling search, highscores, guild, deaths and sitemap leakage.
changed_paths:
  - app/CharacterProfiles/**
  - app/Http/Controllers/CharacterProfiles/**
  - app/Http/Requests/CharacterProfiles/**
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - app/PublicGameData/PublicCharacterProfileService.php
  - database/migrations/2026_07_29_165500_create_character_profile_preferences.php
  - routes/modules/character-profile-preferences.php
  - resources/views/identity/account/**
  - resources/views/game/character.blade.php
  - lang/en/character_profiles.php
  - lang/pl/character_profiles.php
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - scripts/acceptance/seed-community-data.php
  - scripts/acceptance/update-character-profile-preference.php
  - scripts/acceptance/assert-character-profile-concurrency.php
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-character-profile-preferences.md
validation:
  - command: composer audit --locked --no-interaction
    result: PASS
    evidence: CI 30489605987.
  - command: vendor/bin/pint --test
    result: PASS
    evidence: CI 30489605987.
  - command: vendor/bin/phpstan analyse --memory-limit=1G
    result: PASS
    evidence: CI 30489605987.
  - command: php artisan test
    result: PASS
    evidence: CI 30489605987 passed the complete suite, including account privacy upper-bound and sanitized ownership dependency failure.
  - command: focused CharacterProfiles and CommunityData feature tests
    result: PASS
    evidence: Community Data 30489605994.
  - command: concurrent main-character selection using two PHP processes on real MariaDB
    result: PASS
    evidence: Community Data 30489605994 left exactly one main preference row.
  - command: zero-retry owner/public profile browser matrix on desktop, tablet and mobile
    result: PASS
    evidence: Community Data 30489605994 passed all three Chromium projects with isolated fixtures and EN/PL assertions.
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: Portal Acceptance 30489605983 validated all 43 capabilities at 23 implemented, 3 partial, 14 missing and 3 not applicable.
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs
    result: PASS
    evidence: Portal Acceptance 30489605983 classified both new owner routes and all required evidence markers.
  - command: Required exact-head workflow suite
    result: PASS
    evidence: All 11 workflow runs listed under proven completed successfully at d2e611f1c110e57d6cfec5dffedd930eee99aab6.
  - command: Required exact-final-head workflow suite
    result: NOT_RUN
    evidence: Pending on this evidence-only checkpoint commit.
blockers:
  - none
next_action: Run all required workflows on this evidence-only checkpoint head; if every exact-final-head gate passes, update PR #308 with evidence, mark it ready, merge with expected-head protection, verify Issue #307 closes and parent #277 remains open, then archive the task separately.
```

## Boundaries

This task writes only Platform-owned preference state. It does not rename, delete, restore, transfer or otherwise mutate a Canary character and does not claim production deployment.
