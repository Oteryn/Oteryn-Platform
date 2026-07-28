---
task_id: OTERYN-20260728-game-catalog-slice-1
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
search_first:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - current Wiki routes, navigation, controllers and acceptance tests
  - current RBAC, MFA, audit and administration conventions
optional_reads: []
---

# OTERYN-20260728-game-catalog-slice-1

## Goal

Deliver the Platform half of the first production-quality version-aware Oteryn Game Catalog slice: immutable transactional import, profile activation and rollback, projected public visibility, public item/weapon/creature/loot surfaces and secured administrative inspection. Production deployment and production profile activation are excluded.

## Acceptance criteria

- [x] Shared fixture and schema hash validation are byte-identical with `blakinio/canary`.
- [x] Storage, import, activation, rollback and verification commands satisfy the merged architecture and import contract.
- [x] Public item, weapon, creature and visible-loot routes are version/completeness/availability gated.
- [ ] Administrator surfaces use exact RBAC permissions, confirmed MFA, CSRF and bounded audit events.
- [ ] Focused, repository-required and cross-repository E2E validation pass on the final head.
- [x] No production deployment or production profile activation occurs.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - .github/workflows/game-catalog-contract.yml
  - bootstrap/providers.php
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - resources/schemas/game-catalog/v1/**
  - resources/fixtures/game-catalog/v1/**
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - scripts/acceptance/**/*game-catalog*
shared_integration_paths:
  - app/Admin/AdminPermission.php
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Providers/AppServiceProvider.php
  - resources/views/admin/layout.blade.php
  - resources/views/game/layout.blade.php
  - routes/console.php
modules:
  - GameCatalog
  - Wiki public navigation integration
  - Admin RBAC/MFA/audit integration
dependencies:
  - CAN-20260728-game-catalog-exporter-slice-1
  - oteryn.game-catalog schema version 1.0.0
blockers: []
cross_repository_tasks:
  - CAN-20260728-game-catalog-exporter-slice-1
```

## Cross-repository contract and rollout

```yaml
contract_id: oteryn.game-catalog
schema_version: 1.0.0
expected_schema_sha256: 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b
platform_schema: resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
canary_schema: schemas/game-catalog/v1/game-catalog-snapshot.schema.json
compatibility: atomic-required
rollout_order:
  - validate byte-identical schema and shared fixture in both repositories
  - deliver Canary deterministic offline exporter
  - deliver Platform inactive transactional importer
  - deliver Platform profile activation and rollback
  - deliver public and administrative surfaces
  - run cross-repository E2E and generate a staging-only snapshot
production_activation: forbidden
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T19:52:00+02:00
head: 4bfa80838ca14227bc9a20dc34a764d68d77d47d
branch: feat/OTERYN-20260728-game-catalog-slice-1
pr: 272
status: validating
context_routes:
  - agent-governance
  - architecture
  - database
  - web-cms
  - admin-rbac
  - security
  - testing
  - canary-integration
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - .github/workflows/game-catalog-contract.yml
  - bootstrap/providers.php
  - app/GameCatalog/**
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Providers/AppServiceProvider.php
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - scripts/acceptance/**/*game-catalog*
proven:
  - PR 272 is open and mergeable on branch feat/OTERYN-20260728-game-catalog-slice-1 at implementation head 4bfa80838ca14227bc9a20dc34a764d68d77d47d
  - Platform and Canary use byte-identical Game Catalog schema v1 with expected SHA-256 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b
  - transactional import, activation, rollback, visibility projection, diff and verification preserve inactive or previously active public state on failure
  - localized public catalog routes, bounded item and creature queries, item and creature details, weapon filters, reverse item sources and visible loot use the active public profile projections
  - PublicGameCatalogTest passed 4 tests and 47 assertions in CI run 30384504995
  - Game Catalog Contract run 30384505183 passed schema validation, Pint and focused PHPStan on head 4bfa80838ca14227bc9a20dc34a764d68d77d47d
  - full repository CI run 30384504995 passed formatting and full PHPStan before one PHPUnit expectation failure
  - production deployment and production profile activation did not occur
derived:
  - the current task-code failure is a stale expected public-navigation label list rather than a public catalog visibility or query defect
  - administrator inspection surfaces can proceed after the repository CI baseline is restored
unknown:
  - exact final repository CI result after updating the public navigation expectation
  - final administrator snapshot, profile, finding and visibility inspection behavior
  - public desktop, tablet, mobile and keyboard visual acceptance on the final head
  - cross-repository E2E using a generated staging-only Canary snapshot
  - complete historical content and availability facts listed by the architecture
conflicts: []
first_failure:
  marker: public-navigation-expectation
  evidence: CI run 30384504995 failed PublicPortalExtensionTest::test_public_navigation_exposes_only_registered_named_routes because the expected labels omitted Game Catalog between Wiki and Support
rejected_hypotheses:
  - public catalog projection gating failed: PublicGameCatalogTest passed all four focused tests
  - public catalog formatting or static analysis failed: Game Catalog Contract run 30384505183 passed Pint and focused PHPStan
  - external wiki data is authoritative
  - imported snapshots activate automatically
  - unknown values may be converted to zero or guessed
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/**
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Providers/AppServiceProvider.php
  - bootstrap/providers.php
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - resources/navigation/public/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - tests/Feature/GameCatalog/**
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tools/game-catalog/**
validation:
  - command: Canary CI and Game Catalog Contract
    result: PASS
    evidence: workflow runs 30345311203 and 30345310976 passed on Canary implementation head 018e0e69ca0890c36fcd062cb48eab283ee76edf
  - command: Platform Game Catalog Contract
    result: PASS
    evidence: workflow run 30384505183 passed schema validation, Pint and focused PHPStan on head 4bfa80838ca14227bc9a20dc34a764d68d77d47d
  - command: Public Game Catalog feature tests
    result: PASS
    evidence: CI run 30384504995 recorded 4 tests and 47 assertions passing in PublicGameCatalogTest
  - command: Platform repository CI
    result: FAIL
    evidence: run 30384504995 passed Pint and full PHPStan; 417 tests completed with one failure and two skipped, failing only the stale PublicPortalExtensionTest navigation labels
  - command: Agent Governance
    result: PASS
    evidence: workflow run 30384504898 passed on head 4bfa80838ca14227bc9a20dc34a764d68d77d47d
  - command: local checkout/build/test
    result: NOT_RUN
    evidence: repository checkout is unavailable in the sandbox; executable evidence was collected from repository CI
blockers: []
next_action: Update tests/Feature/PublicPortal/PublicPortalExtensionTest.php to include Game Catalog in the expected public navigation priority order, then rerun current-head CI and inspect the first remaining failure.
```

## Deferred child tasks

- NPC catalogue.
- Quests.
- Spawn and raid availability.
- Map reachability.
- Public sprite sourcing.
- Historical release metadata.
- Backport administration.
- 7.60 compatibility.
