---
task_id: OTERYN-20260729-game-catalog-null-boundary
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
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

- [x] Schema 1.1.0 permits `snapshot.verified_content_through_release` to be a release key or null; schema 1.0.0 remains unchanged.
- [x] The importer selects only explicitly supported schema versions and verifies each bundled schema by its registered SHA-256.
- [x] A schema 1.1.0 snapshot with a null verified-content boundary imports transactionally.
- [x] The nullable boundary is persisted without a sentinel release.
- [x] Activation and public projection fail closed when the verified-content boundary is unknown.
- [x] Stored schema 1.0.0 snapshots remain eligible for rollback/activation under the new build.
- [x] Unsupported schema versions and schema/version mismatches are rejected.
- [x] The shared Canary/Platform schema bytes and SHA-256 are identical for 1.1.0, and each repository's sanitized fixture validates.
- [x] Focused tests, static analysis, migration tests and exact-head CI pass.
- [x] Contract, module catalogue, changelog and task checkpoint are current.
- [x] The Canary producer PR is implementation-ready and remains ordered after this consumer merge.

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
  - Consumer CI must pass before Canary schema 1.1.0 producer output may merge.
cross_repository_tasks:
  - CAN-20260729-game-catalog-schema-1-1
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T16:00:08Z
head: 8055a58bec18c935d49c1692d9394aa652201542
branch: feat/OTERYN-20260729-game-catalog-null-boundary
pr: 299
status: ready
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
  - Schema 1.1.0 is pinned as SHA-256 323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac in both repositories.
  - Schema 1.1.0 snapshots persist a null verified-content boundary without a sentinel and cannot be activated.
  - Schema 1.0.0 snapshots retain activation and rollback compatibility under the new consumer.
  - Canary producer PR 1006 passed contract, compilation, export-only runtime, CI, ownership and E2E checks at 57dd84c10ba582597ba00daa38437a3c88b99c4d.
derived:
  - Schema 1.1.0 must represent the boundary as nullable without a sentinel.
  - New consumer code must retain schema 1.0.0 activation compatibility for stored snapshots.
  - Import may accept unknown evidence for review, but activation and public projection must remain fail closed.
unknown: []
conflicts:
  - none
first_failure:
  marker: none
  evidence: Schema representation, persistence, activation and compatibility failures are resolved and exact-head validation passed.
rejected_hypotheses:
  - Use protocol 15.25 as content-completeness evidence.
  - Invent a sentinel release.
  - Mutate schema 1.0.0 in place.
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/2026_07_29_152000_make_game_catalog_verified_content_boundary_nullable.php
  - resources/schemas/game-catalog/v1.1/game-catalog-snapshot.schema.json
  - resources/views/game-catalog/admin/**
  - tests/Feature/GameCatalog/**
  - tests/Fixtures/GameCatalog/v1.1/minimal-snapshot.json
  - tools/game-catalog/validate_contract_fixture.py
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0018-game-catalog-unknown-verified-boundary.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - CHANGELOG.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-null-boundary.md
validation:
  - command: overlap search
    result: PASS
    evidence: No active Platform task or open Game Catalog PR matched on 2026-07-29.
  - command: Canary pinned snapshot validator against Platform schema 1.1.0 fixture
    result: PASS
    evidence: Schema SHA-256 323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac and fixture SHA-256 6b809e339cdf2c933c032a32e06e649d277eafb84ce9c5e1681d6b29ead4e61a validated locally.
  - command: python3 tools/game-catalog/validate_contract_fixture.py
    result: NOT_RUN
    evidence: Local Python environment lacks jsonschema; the pinned workflow installs jsonschema 4.26.0 before running both schema versions.
  - command: focused PHP/Laravel tests and migration rollback
    result: PASS
    evidence: CI 30467829945 and Phase 7 30467822827 passed formatting, PHPStan, tests, clean migration, upgrade and rollback at 8055a58bec18c935d49c1692d9394aa652201542.
  - command: Game Catalog Contract
    result: PASS
    evidence: Run 30467822606 passed Pint, PHPStan and both pinned schema fixtures at 8055a58bec18c935d49c1692d9394aa652201542.
  - command: broad exact-head workflows
    result: PASS
    evidence: Agent Governance 30467830114, Edge Security 30467823707, Game Auth 30467822748, DB Outage 30467822594, Synology build 30467822602, Acceptance E2E 30467829895 and Portal Acceptance 30467822754 passed.
blockers: []
next_action: Mark PR 299 ready and squash-merge the consumer before finalizing Canary producer PR 1006.
```

## Notes

Coordination ID: `OTS-20260728-game-catalog-v1`. No production import or activation is authorized by this task.
