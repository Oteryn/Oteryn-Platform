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
- [ ] Review and accept the architecture PR.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/architecture/adr/README.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/prompts/OTERYN_GAME_CATALOG_IMPLEMENTATION_PROMPT.md
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
updated_at: 2026-07-28T08:00:00Z
head: pending-first-document-commit
branch: docs/OTERYN-20260728-versioned-game-catalog-architecture
pr: none
status: documenting
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
proven:
  - Oteryn Wiki already owns localized editorial articles, categories, public reads, search and administration.
  - Canary is the semantic authority for game runtime definitions.
  - Structured catalogue records must not be represented as thousands of Wiki Markdown articles.
  - Current public Wiki routes contain a generic /wiki/{slug} route, so catalogue routes must be registered before it.
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
  - docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md
validation:
  - command: repository connector review
    result: PASS
    evidence: required architecture, Wiki and integration boundaries inspected before writing
blockers:
  - none
next_action: Open the draft architecture PR and review the cross-repository contract against the matching Canary architecture PR.
```

## Notes

This task records architecture only. It does not import a snapshot, add migrations, expose new routes, deploy, or modify Canary runtime behavior.