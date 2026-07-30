---
task_id: OTERYN-20260730-game-catalog-schema-1-3-consumer
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
related_issue: 330
status: active
agent: "GPT-5.6 Thinking"
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
base_branch: main
created: 2026-07-30T06:17:00Z
updated: 2026-07-30T06:17:00Z
risk: high
---

# Goal

Implement the complete consumer-first Game Catalog schema `1.3.0` support in Oteryn Platform for inactive NPC and NPC shop-offer snapshots. Preserve schemas `1.0.0`-`1.2.0`, reject unknown types fail closed, persist all new records transactionally, expose bounded administrator candidate diagnostics, and keep public projection and every activation/deployment operation outside this task.

# Acceptance criteria

- [ ] Add the complete canonical Draft 2020-12 schema `1.3.0` by retaining all `1.2.0` constraints and adding only `npc`, `npc_buy_offer` and `npc_sell_offer`.
- [ ] Pin exact schema and fixture SHA-256 values without modifying old schema bytes.
- [ ] Register `1.3.0` as an inactive import consumer while unsupported versions still fail closed.
- [ ] Add explicit typed entity and relation dispatch; never map an unknown type to creature or loot persistence.
- [ ] Persist NPC records, currency endpoints, buy/sell relations, runtime paths, exact prices/subtypes and nullable exact storage conditions transactionally.
- [ ] Reject duplicate keys, dangling/wrong-type source, target or currency endpoints, invalid canonical relation identity and count mismatch.
- [ ] Preserve null/unknown facts and keep verified-boundary activation guards unchanged.
- [ ] Extend administrator candidate inspection with typed counts and unknown/unverified counts without adding public NPC/shop routes.
- [ ] Prove failed import leaves no partial snapshot and preserves the prior active snapshot.
- [ ] Add focused schema, validation, persistence, rollback/candidate isolation and authorization tests.
- [ ] Do not perform staging or production import/activation.

# Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - config/game-catalog.php
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Application/AdminRead/**
  - app/GameCatalog/Domain/**
  - database/migrations/*game_catalog*.php
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - .github/workflows/game-catalog-contract.yml
modules:
  - GameCatalog import
  - GameCatalog persistence
  - GameCatalog administrator review
dependencies:
  - Platform issue 330
  - merged Platform PR 331
  - merged Platform PR 332
blockers:
  - local edit/build/test capability is not available through the GitHub connector; exact-head CI will be the executable validation boundary
cross_repository_tasks:
  - CAN-20260730-game-catalog-npc-runtime-authority
  - CAN-20260730-game-catalog-schema-1-3-producer
```

# Constraints

- Repository writes remain limited to `blakinio/Oteryn-Platform` and `blakinio/canary`.
- Schema `1.3.0` support is consumer-first and inactive by default.
- Public NPC/shop projection is a later independent task.
- No staging, production, secrets, deployment routing or environment mutation is authorized.
- Unknown historical, availability and dynamic player-specific shop state remains unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T06:17:00Z
head: d2a03b2cda05f5b42b135d847c95416a18b3d822
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
pr: none
status: investigating
context_routes:
  - architecture
  - public-game-data
  - canary-integration
  - database
  - admin-rbac
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - config/game-catalog.php
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Application/AdminRead/**
  - app/GameCatalog/Domain/**
  - database/migrations/*game_catalog*.php
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - .github/workflows/game-catalog-contract.yml
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
proven:
  - Platform main at task creation is d2a03b2cda05f5b42b135d847c95416a18b3d822.
  - Platform PR 331 and PR 332 are merged.
  - Existing import dispatch maps every non-item entity to creature persistence and every relation to loot persistence.
  - Existing activation requires a concrete verified-content boundary and import never activates automatically.
derived:
  - Schema 1.3.0 requires explicit typed dispatch, new typed persistence and fail-closed endpoint validation before Canary producer support.
unknown:
  - Exact final canonical schema and fixture hashes.
  - Exact migration shape compatible with retained rollback snapshots.
  - Exact administrator preview query changes required by current code.
  - Live staging and production state.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reuse creature persistence for NPCs.
  - Reuse loot persistence for shop offers.
  - Add public projection or activation to the consumer task.
validation:
  - command: GitHub preflight and merged architecture inspection
    result: PASS
    evidence: exact main, merged programme/architecture PRs, current import dispatch and contract proposal inspected.
blockers:
  - Local executable validation unavailable in CHAT; use exact-head CI and record any first failure.
next_action: Inspect current validator, schema registry, database schema, administrator preview and focused tests before editing consumer code.
```
