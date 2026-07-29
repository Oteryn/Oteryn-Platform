---
task_id: OTERYN-20260729-game-catalog-runtime-threshold
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/**
  - docs/architecture/adr/**
optional_reads: []
---

# OTERYN-20260729-game-catalog-runtime-threshold

## Goal

Add consumer-first Game Catalog schema 1.2 support for exact Canary runtime loot thresholds without misrepresenting modifier-dependent thresholds as bounded static probabilities, while preserving schema 1.0/1.1 import and rollback compatibility.

## Acceptance criteria

- [x] Define a durable schema 1.2 loot model with an explicit versioned model identifier, configured threshold, and declared roll maximum.
- [x] Keep schema 1.0.0 and 1.1.0 bytes and hashes unchanged.
- [x] Reject mixed, incomplete, unsupported, or out-of-range loot model payloads fail closed.
- [x] Import schema 1.2 snapshots transactionally and inactive by default.
- [x] Persist threshold-model rows without clamping or synthesizing a probability fraction.
- [x] Preserve existing rational-probability rows and stored schema 1.0/1.1 activation/rollback behavior.
- [x] Present the model truthfully in public/admin reads; never render a threshold greater than the roll maximum as a percentage above 100%.
- [x] Add a backward-compatible migration with a tested rollback guard when threshold rows cannot be represented by the old table.
- [x] Pin a byte-identical shared schema and sanitized fixture across Platform and Canary in consumer-first order.
- [x] Pass focused schema, import, migration, public-read, static-analysis, and exact-head CI.
- [x] Update the import contract, module catalogue, ADR, changelog, and cross-repository checkpoint.
- [x] Do not import, activate, deploy, or mutate a production snapshot.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Queries/Public/**
  - config/game-catalog.php
  - database/migrations/*game_catalog_loot_chance_model*.php
  - resources/schemas/game-catalog/v1.2/**
  - resources/views/game-catalog/**
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - tests/Fixtures/GameCatalog/v1.2/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - .github/workflows/game-catalog-contract.yml
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0019-game-catalog-runtime-loot-thresholds.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-runtime-threshold.md
  - CHANGELOG.md
modules:
  - Game Catalog
  - Integration
dependencies:
  - OTS-20260728-game-catalog-v1
  - CAN-20260729-game-catalog-loot-integrity
blockers:
  - Canary producer schema work must wait for Platform PR 310 to merge.
cross_repository_tasks:
  - CAN-20260729-game-catalog-loot-threshold-schema
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T18:03:30Z
head: 8f83e4c2655fc0febad00e7433c10f06165ce306
branch: feat/OTERYN-20260729-game-catalog-runtime-threshold
pr: 310
status: ready
context_routes:
  - agent-governance
  - architecture
  - canary-integration
  - database
  - testing
owned_paths:
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Queries/Public/**
  - config/game-catalog.php
  - database/migrations/*game_catalog_loot_chance_model*.php
  - resources/schemas/game-catalog/v1.2/**
  - tests/Fixtures/GameCatalog/v1.2/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - .github/workflows/game-catalog-contract.yml
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0019-game-catalog-runtime-loot-thresholds.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-runtime-threshold.md
proven:
  - Platform schema 1.0 and 1.1 accept only chance_numerator and chance_denominator and semantic validation rejects numerator above denominator.
  - Existing persistence and public DTOs require and render a rational numerator/denominator pair.
  - Canary runtime compares a configured loot threshold after runtime modifiers against its loot roll.
  - Canary baseline evidence contains 92 configured thresholds above the declared roll maximum.
  - Platform PR 309 archived the stale architecture owner and merged as c6d1ce417f226f529637109296c81cc52d2012f7.
  - Schema 1.2 is pinned locally as SHA-256 a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de.
  - The schema 1.2 fixture is pinned locally as SHA-256 42b832954f9aa68cf7e2465351f92266771b8132d9634757391d010eaec84855 and contains a valid threshold 12 over roll maximum 10.
  - Canary Game Catalog run 30476329935 proves zero dangling endpoints and preserves exactly 92 configured thresholds above the schema 1.1 denominator without publishing invalid output.
  - Platform schema 1.2 validates the explicit canary_dynamic_threshold_v1 model and persists threshold 12 with roll maximum 10 without a rational probability.
  - Legacy schema 1.0 and 1.1 hashes remain pinned and unchanged.
derived:
  - Schema 1.2 must distinguish a contextual runtime threshold from legacy rational probability instead of reinterpreting old fields.
  - Consumer-first rollout requires Platform support before Canary emits schema 1.2.
unknown: []
conflicts:
  - Schema 1.1 probability semantics cannot represent every configured Canary runtime threshold.
first_failure:
  marker: none
  evidence: Initial PHPStan findings in runs 30477260530 and 30477651768 were corrected; exact implementation-head contract and CI now pass.
rejected_hypotheses:
  - Clamp threshold to roll maximum because that loses configured runtime evidence.
  - Increase one denominator because ordinary threshold meaning would change.
  - Mutate schema 1.1 in place because pinned bytes and stored compatibility must remain stable.
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Queries/Public/**
  - config/game-catalog.php
  - database/migrations/2026_07_29_174500_add_game_catalog_loot_chance_model.php
  - resources/schemas/game-catalog/v1.2/**
  - resources/views/game-catalog/**
  - lang/*/game_catalog.php
  - tests/Fixtures/GameCatalog/v1.2/**
  - tests/Feature/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0019-game-catalog-runtime-loot-thresholds.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-runtime-threshold.md
  - CHANGELOG.md
validation:
  - command: repository and open-PR overlap search
    result: PASS
    evidence: No active Game Catalog task or open implementation PR overlaps this scope after PR 309.
  - command: python3 -m py_compile tools/game-catalog/validate_contract_fixture.py
    result: PASS
    evidence: Contract validator compiles.
  - command: parse schema and fixture JSON and verify pinned SHA-256
    result: PASS
    evidence: Both JSON documents parse and match their registered hashes.
  - command: focused PHP, schema and migration validation
    result: PASS
    evidence: Game Catalog Contract run 30477778313 passed schema validation, fixture validation, formatting, and PHPStan on 8f83e4c2655fc0febad00e7433c10f06165ce306.
  - command: CI 30477777513
    result: PASS
    evidence: Full formatting, static analysis, and repository tests passed on 8f83e4c2655fc0febad00e7433c10f06165ce306.
  - command: Platform supporting workflows
    result: PASS
    evidence: Agent Governance 30477777508, Platform DB Outage 30477778758, Edge Security 30477777682, Game Auth Concurrency 30477777627, Synology build 30477777579, Phase 7 30477777532, Acceptance E2E 30477777494, and Portal Acceptance 30477777641 passed on the same head.
blockers: []
next_action: Publish this final checkpoint, require every exact-final-head workflow, audit reviews and base drift, then squash-merge PR 310 before creating the Canary producer task.
```

## Notes

The versioned model identifier will bind the runtime algorithm in the contract. The snapshot carries configured threshold evidence, not a claim of one context-free effective drop probability. Production import and activation remain excluded.
