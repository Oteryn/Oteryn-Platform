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

- [ ] Shared fixture and schema hash validation are byte-identical with `blakinio/canary`.
- [ ] Storage, import, activation, rollback and verification commands satisfy the merged architecture and import contract.
- [ ] Public item, weapon, creature and visible-loot routes are version/completeness/availability gated.
- [ ] Administrator surfaces use exact RBAC permissions, confirmed MFA, CSRF and bounded audit events.
- [ ] Focused, repository-required and cross-repository E2E validation pass on the final head.
- [ ] No production deployment or production profile activation occurs.

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
blockers:
  - PR #270 currently owns shared permission, localized routing, layout and routes/console paths; do not edit them until reconciled.
  - Local PHP/Node execution is unavailable in the current sandbox; CI evidence is required for executable validation.
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
updated_at: 2026-07-28T11:28:00+02:00
head: b98edaaf77b9f9d1541cb5b684efa8c045a1091a
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
  - main contains architecture merge commit 8aa1fc29dd13895efb2a7006204a6b88105e6972
  - Platform and Canary schema files have identical Git blob SHA a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - Game Catalog Contract run 30342833946 passed the expected schema SHA-256 and shared fixture
  - PR #270 does not own bootstrap/providers.php or dedicated GameCatalog paths
  - persistence covers releases, immutable snapshots, profiles, stable entities, identifiers, typed item/creature/loot records, import runs/findings, projected visibility, translations, Wiki links, overrides and audit events
  - fixed-schema validation checks the registered schema hash, file limits, syntax, duplicate JSON keys, Draft 2020-12 keywords used by schema v1 and semantic integrity
  - importer deduplicates by content hash and writes an inactive validated snapshot in one transaction
  - validation and persistence failures do not change active profile state
  - validation/import commands are registered through GameCatalogServiceProvider
  - draft PR #272 tracks this task
  - production deployment and production activation are excluded
derived:
  - isolated persistence/import/provider work proceeds without editing PR #270-owned paths
  - public queries can use precomputed profile entity/relation projections rather than raw snapshot evaluation
unknown:
  - PHP migration, format, static-analysis and focused-test results for current head
  - final shared routing/RBAC/layout integration shape after PR #270 lands or explicit ownership reconciliation
  - complete historical content and availability facts listed by the architecture
conflicts:
  - PR #270 overlap remains limited to shared permission, localized routing, console and layout aggregation paths
first_failure:
  marker: checkpoint-validation
  evidence: Agent Governance run 30342833669 failed before the initial workflow path was added to changed_paths
rejected_hypotheses:
  - external wiki data is authoritative
  - imported snapshots activate automatically
  - unknown values may be converted to zero or guessed
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/Application/Import/CatalogImportResult.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/CatalogSemanticValidator.php
  - app/GameCatalog/Application/Import/CatalogSnapshotValidator.php
  - app/GameCatalog/Application/Import/ValidatedCatalogSnapshot.php
  - app/GameCatalog/Console/ImportCatalogCommand.php
  - app/GameCatalog/Console/ValidateCatalogCommand.php
  - app/GameCatalog/Domain/CatalogValidationFinding.php
  - app/GameCatalog/Domain/Exceptions/CatalogValidationException.php
  - app/GameCatalog/GameCatalogServiceProvider.php
  - app/GameCatalog/Infrastructure/Json/BundledJsonSchemaValidator.php
  - app/GameCatalog/Infrastructure/Json/DuplicateJsonKeyDetector.php
  - bootstrap/providers.php
  - config/game-catalog.php
  - database/migrations/2026_07_28_110000_create_game_catalog_tables.php
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - tests/Feature/GameCatalog/CatalogImportTest.php
  - tests/Fixtures/GameCatalog/v1/minimal-snapshot.json
  - tests/Unit/GameCatalog/DuplicateJsonKeyDetectorTest.php
  - tools/game-catalog/validate_contract_fixture.py
validation:
  - command: GitHub repository and main-head inspection
    result: PASS
    evidence: main contains 8aa1fc29dd13895efb2a7006204a6b88105e6972
  - command: GitHub schema blob comparison
    result: PASS
    evidence: both schema paths resolve to blob a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - command: Game Catalog Contract
    result: PASS
    evidence: workflow run 30342833946
  - command: open PR ownership inspection
    result: PASS_WITH_CONFLICT_RECORDED
    evidence: PR #270 changed-file inventory reviewed at head 8a1cd49d490d45b2c0ed4253d53975739cd60c4a
  - command: Platform final-head CI
    result: QUEUED
    evidence: current implementation head b98edaaf77b9f9d1541cb5b684efa8c045a1091a awaits workflow dispatch
  - command: local checkout/build/test
    result: NOT_RUN
    evidence: sandbox DNS cannot resolve github.com
blockers:
  - shared integration paths remain held pending reconciliation with PR #270
next_action: Inspect Platform final-head CI, fix the first migration, syntax, schema or importer failure, then implement profile activation visibility projections and rollback.
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
