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
- [x] PR merges and Issue #307 closes; parent Issue #277 remains open for the unapproved mutation lifecycles.
- [x] Task is archived in a separate documentation PR.

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
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
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
updated_at: 2026-07-29T21:45:00Z
head: 3797a094cfa522f5147d624786f49fee5027c77b
branch: feat/OTERYN-20260729-character-profile-preferences
pr: 308
status: ready
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
  - scripts/acceptance/**character-profile**
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
proven:
  - Platform-owned character profile preferences store a bounded escaped owner comment, per-character field visibility and an optional main-character selection without mutating Canary.
  - Every edit and update revalidates the ready immutable Identity-to-Canary binding and current active character ownership through the read-only Canary connection; foreign, stale and unavailable states fail closed.
  - Account-level association and status privacy remain upper bounds, hidden siblings are excluded from related-character output and no Platform or Canary internal identifiers are exposed.
  - Identity-row locking and the real-MariaDB race test leave at most one main character per Identity.
  - Exact final head 3797a094cfa522f5147d624786f49fee5027c77b passed all 11 required workflows: CI 30490007511, Agent Governance 30490007484, Portal Acceptance Contract 30490007458, Community Data Acceptance 30490007443, Phase 7 Production-Like Validation 30490007483, Platform DB Outage Validation 30490007507, Edge Security Emulation 30490007432, Game Auth Ticket Concurrency 30490007493, Acceptance E2E and Visual UX 30490007509, Synology Production Target Preflight 30490007537 and Build Synology Staging Images 30490007474.
  - Community Data Acceptance proved owner and non-owner behavior, privacy upper bounds, sanitized unavailable states, two concurrent real-MariaDB main-character writers and the complete zero-retry Chromium desktop/tablet/mobile lifecycle in English and Polish.
  - Product and route ledgers passed with 43 capabilities classified as 23 implemented, 3 partial, 14 missing and 3 not applicable.
  - PR #308 was squash-merged as 86847d0068e470274b6c3ee5523fe41cbb9663af and Issue #307 closed as completed; parent Issue #277 remains open for excluded mutation and achievement lifecycles.
derived:
  - Issue #307 is complete for its approved Platform-owned boundary and requires no Canary or production follow-up.
  - Rename, deletion/restore, world or channel transfer and authoritative achievement selection remain separate Issue #277 work requiring explicit contracts and authorization.
unknown:
  - Actual production deployment identity, database state, latency and recovery behavior remain unverified.
conflicts: []
first_failure:
  marker: none
  evidence: Every required exact-final-head workflow passed before PR #308 merged.
rejected_hypotheses:
  - Store owner comments or visibility preferences in Canary players data.
  - Reuse a generic Canary write principal for profile preferences.
  - Treat a stored preference row or browser-supplied identifier as ownership proof.
  - Allow character-level opt-in to override account-level privacy.
  - Close parent Issue #277 after only the Platform-owned profile slice.
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
  - lang/{en,pl}/character_profiles.php
  - tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
  - scripts/acceptance/**character-profile**
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/character-profile-preferences.json
  - .github/workflows/community-data-acceptance.yml
  - docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md
  - docs/operations/CHARACTER_PROFILE_PREFERENCES.md
  - docs/architecture/{MODULE_CATALOG,DATA_OWNERSHIP,SECURITY_ARCHITECTURE}.md
  - docs/testing/{PRODUCT_COMPLETENESS_BENCHMARK.md,product-completeness-benchmark.json}
  - docs/agents/{PROJECT_STATE,ACTIVE_WORK}.md
  - docs/agents/tasks/archive/OTERYN-20260729-character-profile-preferences.md
validation:
  - command: Required exact-final-head workflow suite
    result: PASS
    evidence: All 11 workflow runs listed under proven completed successfully at 3797a094cfa522f5147d624786f49fee5027c77b before merge.
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: Portal Acceptance Contract 30490007458 validated all 43 capabilities.
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs
    result: PASS
    evidence: Portal Acceptance Contract 30490007458 validated route ownership and stable evidence markers.
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance 30490007484 passed on the exact final head.
blockers: []
next_action: Continue parent Issue #277 only through a new bounded active task and separate pull request with explicit authorization for any Canary mutation; no further action remains for Issue #307.
```

## Boundaries

This task writes only Platform-owned preference state. It does not rename, delete, restore, transfer or otherwise mutate a Canary character and does not claim production deployment.
