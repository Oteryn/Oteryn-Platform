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

- [ ] Define a durable schema 1.2 loot model with an explicit versioned model identifier, configured threshold, and declared roll maximum.
- [ ] Keep schema 1.0.0 and 1.1.0 bytes and hashes unchanged.
- [ ] Reject mixed, incomplete, unsupported, or out-of-range loot model payloads fail closed.
- [ ] Import schema 1.2 snapshots transactionally and inactive by default.
- [ ] Persist threshold-model rows without clamping or synthesizing a probability fraction.
- [ ] Preserve existing rational-probability rows and stored schema 1.0/1.1 activation/rollback behavior.
- [ ] Present the model truthfully in public/admin reads; never render a threshold greater than the roll maximum as a percentage above 100%.
- [ ] Add a backward-compatible migration with a tested rollback guard when threshold rows cannot be represented by the old table.
- [ ] Pin a byte-identical shared schema and sanitized fixture across Platform and Canary in consumer-first order.
- [ ] Pass focused schema, import, migration, public-read, static-analysis, and exact-head CI.
- [ ] Update the import contract, module catalogue, ADR, changelog, and cross-repository checkpoint.
- [ ] Do not import, activate, deploy, or mutate a production snapshot.

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
  - Canary PR 1010 must prove endpoint integrity and preserve all 92 runtime thresholds before producer schema work begins.
cross_repository_tasks:
  - CAN-20260729-game-catalog-loot-threshold-schema
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T17:50:17Z
head: e911c3633da8d0789941f83fa728a2b89c97f6e1
branch: feat/OTERYN-20260729-game-catalog-runtime-threshold
pr: 310
status: implementing
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
derived:
  - Schema 1.2 must distinguish a contextual runtime threshold from legacy rational probability instead of reinterpreting old fields.
  - Consumer-first rollout requires Platform support before Canary emits schema 1.2.
unknown:
  - Exact Canary PR 1010 default-datapack runtime result.
conflicts:
  - Schema 1.1 probability semantics cannot represent every configured Canary runtime threshold.
first_failure:
  marker: schema-1.1-loot-probability
  evidence: Numerator values above denominator fail both producer and consumer semantic validation even though Canary runtime retains them.
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
    result: NOT_RUN
    evidence: Local environment has no PHP runtime or jsonschema package; exact-head CI will install pinned dependencies.
blockers:
  - Canary PR 1010 exact-head Game Catalog runtime proof is still running.
  - Platform exact-head schema, PHP, migration and static-analysis workflows have not run on the implementation.
next_action: Publish the schema 1.2 implementation to draft PR 310 and inspect exact-head workflow results before any Canary producer change.
```

## Notes

The versioned model identifier will bind the runtime algorithm in the contract. The snapshot carries configured threshold evidence, not a claim of one context-free effective drop probability. Production import and activation remain excluded.
