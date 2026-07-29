---
task_id: OTERYN-20260729-game-catalog-null-boundary
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/GAME_CATALOG_EXPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/**
optional_reads: []
---

# OTERYN-20260729-game-catalog-null-boundary

## Goal

Add a versioned Game Catalog consumer contract that can import an explicitly unknown verified-content boundary without inventing release evidence, while preserving fail-closed activation and compatibility with stored schema 1.0.0 snapshots.

## Acceptance criteria

- [ ] Schema 1.1.0 permits `snapshot.verified_content_through_release` to be a release key or null; schema 1.0.0 remains unchanged.
- [ ] The importer selects only explicitly supported schema versions and verifies each bundled schema by its registered SHA-256.
- [ ] A schema 1.1.0 snapshot with a null verified-content boundary imports transactionally.
- [ ] The nullable boundary is persisted without a sentinel release.
- [ ] Activation and public projection fail closed when the verified-content boundary is unknown.
- [ ] Stored schema 1.0.0 snapshots remain eligible for rollback/activation under the new build.
- [ ] Unsupported schema versions and schema/version mismatches are rejected.
- [ ] The shared Canary/Platform schema and fixture hashes are byte-identical for 1.1.0.
- [ ] Focused tests, static analysis, migration tests and exact-head CI pass.
- [ ] Contract, module catalogue, changelog and task checkpoint are current.
- [ ] The atomic cross-repository merge gate remains blocked until the Canary producer PR is ready.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/CatalogSnapshotValidator.php
  - app/GameCatalog/Application/Import/ValidatedCatalogSnapshot.php
  - app/GameCatalog/Application/PublicRead/PublicCatalogContextResolver.php
  - app/GameCatalog/Http/Admin/AdminGameCatalogController.php
  - config/game-catalog.php
  - database/migrations/*game_catalog_verified_content_boundary*.php
  - resources/schemas/game-catalog/v1.1/**
  - tests/Fixtures/GameCatalog/v1.1/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - .github/workflows/game-catalog-contract.yml
  - resources/views/game-catalog/admin/snapshot.blade.php
  - resources/views/game-catalog/admin/snapshots.blade.php
  - docs/contracts/GAME_CATALOG_EXPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/**
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-null-boundary.md
  - CHANGELOG.md
modules:
  - Game Catalog
dependencies:
  - OTS-20260728-game-catalog-v1
blockers:
  - Canary schema 1.1.0 producer task must be ready before either side merges.
cross_repository_tasks:
  - CAN-20260729-game-catalog-schema-1-1
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T15:18:48Z
head: 2d7c02fb8c73bb38f9a7031f8cb49aa6d9f1912d
branch: feat/OTERYN-20260729-game-catalog-null-boundary
pr: pending
status: investigating
context_routes:
  - agent-governance
  - architecture
  - canary-integration
  - database
  - testing
owned_paths:
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/*game_catalog_verified_content_boundary*.php
  - resources/schemas/game-catalog/v1.1/**
  - tests/Fixtures/GameCatalog/v1.1/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - .github/workflows/game-catalog-contract.yml
  - docs/contracts/GAME_CATALOG_EXPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/**
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-null-boundary.md
  - CHANGELOG.md
proven:
  - Platform main stores verified_content_through_release_id as a non-null foreign key.
  - Platform main accepts only schema 1.0.0 and verifies one bundled schema hash.
  - Existing public context uses an inner join to the verified release.
  - Existing activation does not explicitly reject an unknown verified-content boundary because schema 1.0.0 could not represent one.
  - No active Platform task or open Game Catalog PR overlaps this scope.
derived:
  - Schema 1.1.0 must represent the boundary as nullable without a sentinel.
  - New consumer code must retain schema 1.0.0 activation compatibility for stored snapshots.
  - Import may accept unknown evidence for review, but activation and public projection must remain fail closed.
unknown:
  - Exact final schema and fixture SHA-256 values until both repositories generate the same bytes.
conflicts:
  - none
first_failure:
  marker: v1-verified-boundary-unrepresentable
  evidence: Schema 1.0.0 and the Platform persistence column require a concrete release while the Canary datapack-wide boundary is unproven.
rejected_hypotheses:
  - Use protocol 15.25 as content-completeness evidence.
  - Invent a sentinel release.
  - Mutate schema 1.0.0 in place.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-null-boundary.md
validation:
  - command: overlap search
    result: PASS
    evidence: No active Platform task or open Game Catalog PR matched on 2026-07-29.
blockers:
  - Atomic merge is held until the Canary producer task is ready.
next_action: Publish the draft PR and implement versioned dual-schema validation plus nullable persistence and activation guards.
```

## Notes

Coordination ID: `OTS-20260728-game-catalog-v1`. No production import or activation is authorized by this task.
