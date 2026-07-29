---
task_id: OTERYN-20260730-game-catalog-schema-1-3-architecture
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
related_issue: 330
status: active
agent: chatgpt
branch: docs/OTERYN-20260730-game-catalog-schema-1-3-architecture
base_branch: main
created: 2026-07-29T22:25:00Z
updated: 2026-07-29T22:25:00Z
risk: medium
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - config/game-catalog.php
  - resources/schemas/game-catalog/v1.2/game-catalog-snapshot.schema.json
search_first:
  - open Game Catalog PRs and branches
  - issue 301
  - issue 330
  - Canary Npcs NpcType ShopBlock runtime authority
---

# OTERYN-20260730-game-catalog-schema-1-3-architecture

## Goal

Design the consumer-first schema-next contract for NPC entities and NPC buy/sell offers as an independent architecture slice. Produce a strict extension schema proposal, full fixture proposal, identity and semantic invariants, compatibility/rollout plan and child-task decomposition without registering schema support, changing persistence, importing a snapshot, exposing public data or activating any environment.

## Acceptance criteria

- [ ] Select and justify schema version `1.3.0` without modifying `1.0.0`–`1.2.0`.
- [ ] Define NPC canonical identity, runtime/display names, aliases, provenance, registration and availability states.
- [ ] Model buy and sell offers as typed NPC-to-item relations with exact currency, amount, price, subtype, storage requirement and nested path semantics.
- [ ] Define deterministic duplicate/collision behavior and fail-closed dangling endpoint validation.
- [ ] Preserve null/unknown historical and availability facts.
- [ ] Explicitly defer NPC location/reachability to the spawn/map slice.
- [ ] Prevent shop existence alone from proving NPC encounterability or item obtainability.
- [ ] Provide a machine-readable strict extension schema proposal and a full snapshot fixture proposal.
- [ ] Document consumer-first compatibility, rollback and public-activation gates.
- [ ] Decompose implementation into independent Platform consumer, Canary authority, Canary producer, staging and public tasks.
- [ ] Do not add `1.3.0` to `config/game-catalog.php` or modify application code, migrations, routes, views, workflows or deployed state.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-architecture.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/game-catalog/v1.3/game-catalog-npc-shop-extension.schema.json
  - docs/contracts/game-catalog/v1.3/minimal-snapshot.proposal.json
  - docs/contracts/game-catalog/v1.3/COMPATIBILITY.md
modules:
  - GameCatalog contract architecture
dependencies:
  - Oteryn Platform issue 330
  - OTERYN-20260730-game-catalog-program-audit
  - Canary final Npcs/NpcType/ShopBlock source inspection
blockers:
  - none for architecture proposal
cross_repository_tasks:
  - CAN-20260730-game-catalog-npc-runtime-authority
  - CAN-20260730-game-catalog-schema-1-3-producer
```

## Constraints

- Existing schema bytes and semantics remain immutable.
- The extension schema in this task is a strict proposal for the new structures, not a supported complete consumer schema and not an activation authorization.
- The later consumer implementation must construct and pin the complete canonical `1.3.0` schema while retaining all `1.2.0` item, creature and loot constraints.
- No quest, mission, reward, spawn, raid or location entity is introduced here.
- No external wiki is authority for runtime or history.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T22:25:00Z
head: UNKNOWN
branch: docs/OTERYN-20260730-game-catalog-schema-1-3-architecture
pr: none
status: implementing
context_routes:
  - architecture
  - public-game-data
  - canary-integration
  - database
  - admin-rbac
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-architecture.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/game-catalog/v1.3/game-catalog-npc-shop-extension.schema.json
  - docs/contracts/game-catalog/v1.3/minimal-snapshot.proposal.json
  - docs/contracts/game-catalog/v1.3/COMPATIBILITY.md
proven:
  - Platform main was f90bb8075b300569b7d493c84f0080e6b3295c35 at branch creation.
  - Platform currently supports only item, creature and creature_loot typed persistence.
  - Current import dispatch maps every non-item entity to creature persistence and every relation to loot persistence.
  - Canary final Npcs registry owns NpcType values and each NpcType owns a final shopItemVector.
  - ShopBlock preserves item ID/name, subtype, buy/sell price, storage key/value and nested childShop values.
  - NpcType preserves exact currency item ID independently from offer price.
derived:
  - Schema 1.3.0 must be implemented consumer first and cannot be producer-only.
  - NPC location and encounterability must remain unknown until a separate creation/location evidence slice.
unknown:
  - Safe deterministic Canary registry iteration API.
  - Complete NPC aliases and reviewed public display policy.
  - Live staging and production snapshot/profile state.
conflicts:
  - Issue 301 records older producer-first and Canary-read-only assumptions superseded by issue 330 and current explicit authorization.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Store shop prices as global item facts: final runtime prices are per NPC offer and item globals retain maxima only.
  - Infer NPC availability from registry presence: registration does not prove placement or reachability.
  - Combine quests or spawns into schema 1.3.0: they require separate authorities and rollout slices.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-architecture.md
validation:
  - command: GitHub source and contract preflight
    result: PASS
    evidence: current schema, importer, lifecycle, NPC registry, Lua registration, ShopBlock and currency boundaries were inspected.
blockers:
  - none
next_action: Write and validate the bounded NPC/shop extension schema, fixture and compatibility contract proposal.
```
