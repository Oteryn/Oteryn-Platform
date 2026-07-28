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
- [ ] Public item, weapon, creature and visible-loot routes are version/completeness/availability gated.
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
updated_at: 2026-07-28T19:26:00+02:00
head: a6dd3726b9e14a81ae7504d911933b7ece3dd4db
branch: feat/OTERYN-20260728-game-catalog-slice-1
pr: 272
status: implementing
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
proven:
  - PR 270 merged into main and the Game Catalog branch is synchronized through main commit 3993263a002010ac8511a1c6e9fcccfb597adc1c
  - Platform and Canary schema files have identical Git blob SHA a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - the deterministic Canary exporter and both repository contract gates use schema version 1.0.0 and expected SHA-256 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b
  - Platform persistence covers releases, immutable snapshots, profiles, stable entities, identifiers, typed item, creature and loot records, import runs, findings, projected visibility, translations, Wiki links, overrides and audit events
  - fixed-schema validation enforces the registered schema hash, file limits, JSON syntax, duplicate keys, supported Draft 2020-12 keywords and semantic integrity
  - transactional import deduplicates by content hash and publishes only an inactive validated snapshot
  - activation, rollback, visibility projection, snapshot diff and verification are profile-scoped and transactionally tested
  - rejected validation, persistence and activation attempts preserve previously active public state
  - focused Game Catalog formatting and level-10 static analysis are enforced by a dedicated pull-request workflow
  - production deployment and production profile activation remain excluded
  - draft PR 272 tracks the Platform implementation

derived:
  - public reads can use precomputed profile entity and relation projections without evaluating raw snapshot policy per request
  - merged Character Bazaar integration removes the previous shared-path ownership blocker
  - shared routing, RBAC and layout integration can now proceed against current main conventions
unknown:
  - final public item, weapon, creature and visible-loot route behavior and visual acceptance
  - final administrator snapshot, profile, finding and visibility inspection behavior
  - cross-repository E2E using a generated staging-only Canary snapshot
  - complete historical content and availability facts listed by the architecture
conflicts: []
first_failure:
  marker: resolved-command-output-capture
  evidence: CI run 30382093117 exposed a PendingCommand output-capture mismatch; commit 21b0297c50f763ae5b808d95b0dc1c9f1a2646d3 replaced it with direct Artisan exit-code and output assertions
rejected_hypotheses:
  - external wiki data is authoritative
  - imported snapshots activate automatically
  - unknown values may be converted to zero or guessed
  - a passing focused contract check substitutes for full repository CI
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/Application/Activation/CatalogActivationResult.php
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Activation/VisibilityProjectionResult.php
  - app/GameCatalog/Application/Activation/VisibilityProjector.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Diff/CatalogSnapshotDiff.php
  - app/GameCatalog/Application/Diff/CatalogSnapshotDiffService.php
  - app/GameCatalog/Application/Import/CatalogImportResult.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/CatalogSemanticValidator.php
  - app/GameCatalog/Application/Import/CatalogSnapshotValidator.php
  - app/GameCatalog/Application/Import/ValidatedCatalogSnapshot.php
  - app/GameCatalog/Application/PublicRead/PublicCatalogContext.php
  - app/GameCatalog/Application/PublicRead/PublicCatalogContextResolver.php
  - app/GameCatalog/Application/Verification/CatalogVerificationResult.php
  - app/GameCatalog/Application/Verification/CatalogVerificationService.php
  - app/GameCatalog/Console/ActivateCatalogCommand.php
  - app/GameCatalog/Console/DiffCatalogCommand.php
  - app/GameCatalog/Console/ImportCatalogCommand.php
  - app/GameCatalog/Console/RollbackCatalogCommand.php
  - app/GameCatalog/Console/ValidateCatalogCommand.php
  - app/GameCatalog/Console/VerifyCatalogCommand.php
  - app/GameCatalog/Domain/CatalogValidationFinding.php
  - app/GameCatalog/Domain/Exceptions/CatalogValidationException.php
  - app/GameCatalog/GameCatalogServiceProvider.php
  - app/GameCatalog/Infrastructure/Json/BundledJsonSchemaValidator.php
  - app/GameCatalog/Infrastructure/Json/DuplicateJsonKeyDetector.php
  - app/GameCatalog/Infrastructure/Persistence/CatalogDatabaseRow.php
  - bootstrap/providers.php
  - config/game-catalog.php
  - database/migrations/2026_07_28_110000_create_game_catalog_tables.php
  - database/migrations/2026_07_28_110100_add_game_catalog_profile_policies.php
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - tests/Feature/GameCatalog/CatalogActivationTest.php
  - tests/Feature/GameCatalog/CatalogImportTest.php
  - tests/Fixtures/GameCatalog/v1/minimal-snapshot.json
  - tests/Unit/GameCatalog/DuplicateJsonKeyDetectorTest.php
  - tools/game-catalog/validate_contract_fixture.py
validation:
  - command: GitHub schema blob comparison across Platform and Canary
    result: PASS
    evidence: both v1 schema paths resolve to blob a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - command: Canary CI and Game Catalog Contract
    result: PASS
    evidence: workflow runs 30345311203 and 30345310976 succeeded on Canary implementation head 018e0e69ca0890c36fcd062cb48eab283ee76edf
  - command: Platform Game Catalog Contract after main synchronization
    result: PASS
    evidence: workflow run 30382689429 completed validation, Pint and focused PHPStan successfully on head a6dd3726b9e14a81ae7504d911933b7ece3dd4db
  - command: Platform repository CI after main synchronization
    result: PASS
    evidence: workflow run 30382693692 completed Composer validation, audit, Pint, full PHPStan and 413 PHPUnit tests successfully
  - command: branch synchronization with current main
    result: PASS
    evidence: compare reports behind_by 0 with merge base 3993263a002010ac8511a1c6e9fcccfb597adc1c
  - command: local checkout/build/test
    result: NOT_RUN
    evidence: repository checkout is unavailable in the sandbox; executable evidence was collected from repository CI
blockers: []
next_action: Implement localized public item, weapon, creature and visible-loot query routes against projected visibility, then add focused public acceptance tests before administrator surfaces.
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
