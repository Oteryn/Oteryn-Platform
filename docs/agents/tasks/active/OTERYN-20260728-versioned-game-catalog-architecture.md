---
task_id: OTERYN-20260728-versioned-game-catalog-architecture
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
search_first:
  - active Wiki, Integration, route, migration, RBAC and acceptance tasks
  - open PRs touching Wiki, routes, migrations, navigation or integration contracts
optional_reads:
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
---

# OTERYN-20260728-versioned-game-catalog-architecture

## Goal

Persist the reviewed architecture, cross-repository contract and bounded implementation handoff for a version-aware Oteryn Game Catalog integrated with Wiki navigation while keeping structured game data separate from editorial Wiki articles.

## Acceptance criteria

- [x] Define Canary and Platform ownership boundaries.
- [x] Define version, completeness, availability and relation visibility semantics.
- [x] Define immutable snapshot import, validation, activation and rollback.
- [x] Define proposed Platform file and database structure.
- [x] Define the first items, creatures and loot vertical slice.
- [x] Preserve NPCs, quests, map availability and historical profiles as later slices.
- [x] Store an implementation prompt that a new agent can execute without chat history.
- [x] Add the proposed schema v1 and prove byte identity with Canary.
- [x] Open the draft architecture PR.
- [ ] Review and accept both architecture PRs and the shared contract.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/architecture/adr/README.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/prompts/OTERYN_GAME_CATALOG_IMPLEMENTATION_PROMPT.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
modules:
  - GameCatalog
  - Wiki
  - Integration
  - Admin
  - Audit
dependencies:
  - contract: oteryn.game-catalog/v1
  - CAN-20260728-game-catalog-export-architecture
blockers:
  - none for architecture documentation
cross_repository_tasks:
  - CAN-20260728-game-catalog-export-architecture
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T07:23:00Z
head: tracked-by-live-pr
branch: docs/OTERYN-20260728-versioned-game-catalog-architecture
pr: https://github.com/blakinio/Oteryn-Platform/pull/271
status: ready-for-review
context_routes:
  - architecture
  - public-game-data
  - canary-integration
  - web-cms
  - admin-rbac
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/architecture/adr/README.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/prompts/OTERYN_GAME_CATALOG_IMPLEMENTATION_PROMPT.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
proven:
  - Oteryn Wiki already owns localized editorial articles, categories, public reads, search and administration.
  - Canary is the semantic authority for game runtime definitions.
  - Structured catalogue records must not be represented as thousands of Wiki Markdown articles.
  - Current public Wiki routes contain a generic /wiki/{slug} route, so catalogue routes must be registered before it.
  - Platform and Canary schema files have the same Git blob SHA a3c239a6d61385edde0b06f72cdf781f4ce58df3.
  - The shared schema content SHA-256 is 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b.
derived:
  - The Platform requires a dedicated GameCatalog module and immutable imported snapshots.
  - Entity and relation version ranges must be independent.
  - Public visibility should be precomputed per active profile and snapshot.
unknown:
  - complete historical introduced/removed release metadata for every entity
  - complete quest registry and availability evidence
  - exact sprite rendering source approved for public use
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Store every item, creature, NPC and quest as an ordinary Wiki article.
  - Treat external wikis as the production source of truth.
  - Represent Tibia versions as floating-point numbers.
changed_paths:
  - docs/agents/prompts/OTERYN_GAME_CATALOG_IMPLEMENTATION_PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/architecture/adr/README.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
validation:
  - command: repository connector review
    result: PASS
    evidence: required architecture, Wiki and integration boundaries inspected before writing
  - command: compare schema Git blob SHA across repositories
    result: PASS
    evidence: both paths resolve to blob a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - command: parse proposed schema as JSON and calculate SHA-256
    result: PASS
    evidence: JSON valid; SHA-256 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b
blockers:
  - none
next_action: Review Oteryn Platform PR #271 together with Canary PR #989 and either accept schema v1 or request a coordinated versioned correction.
```

## Notes

This task records architecture only. It does not import a snapshot, add migrations, expose new routes, deploy, or modify Canary runtime behavior.
