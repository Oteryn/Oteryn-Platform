# Implementation prompt: Versioned Oteryn Game Catalog

Use this prompt in a new agent session. Do not rely on the chat that produced this document.

```text
Continue the Versioned Oteryn Game Catalog programme from repository state.

AUTHORIZED REPOSITORIES

Writes are authorized only in:
- Oteryn/Oteryn-Platform
- blakinio/canary

Never write to opentibiabr/canary or another external repository. Use a separate branch, task record, worktree and PR in each repository. Never push directly to main.

GOAL

Deliver the first complete vertical slice of a version-aware catalogue integrated with Oteryn Wiki navigation but separate from Wiki article persistence.

The first slice covers:
- items;
- creatures;
- creature loot;
- release registry;
- immutable snapshots;
- content profiles;
- version, completeness and availability gating;
- transactional activation and rollback;
- public item and creature lists/details;
- administrator snapshot, version and visibility information.

NPCs, quests, map availability, public sprite sourcing, full historical metadata and 7.60 runtime compatibility are later child tasks unless required only as explicit extension points.

REQUIRED PLATFORM READS

- AGENTS.md
- docs/agents/REPOSITORY_MAP.md
- docs/agents/CONTEXT_ROUTING.md
- docs/agents/PROJECT_STATE.md
- docs/agents/BUILD_TEST_MATRIX.md
- docs/architecture/GAME_CATALOG_ARCHITECTURE.md
- docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
- docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
- docs/architecture/MODULE_CATALOG.md relevant Wiki, Integration, PublicGameData, Admin and Audit sections
- docs/architecture/DATA_OWNERSHIP.md
- docs/architecture/SECURITY_ARCHITECTURE.md relevant sections
- docs/architecture/TEST_STRATEGY.md relevant sections
- current Wiki routes, controllers, queries, administration, navigation and acceptance tests
- current active tasks and open PRs overlapping routes, migrations, Wiki, integration, RBAC or acceptance

REQUIRED CANARY READS

- AGENTS.md
- docs/agents/REPOSITORY_MAP.md
- docs/agents/CONTEXT_ROUTING.md
- docs/contracts/GAME_CATALOG_EXPORT_CONTRACT.md
- docs/systems/GAME_CATALOG_EXPORTER.md
- matching MODULE_CATALOG, KNOWN_RISKS, BUILD_TEST_MATRIX and CROSS_REPO_CONTRACTS sections
- docs/agents/REAL_TIBIA_EVIDENCE_SOURCES.md relevant provenance rules
- src/main.cpp
- src/canary_server.hpp
- src/canary_server.cpp
- item registry/loading code
- MonsterType and Monsters registry code
- current CMake/test conventions
- current active tasks and open PRs overlapping startup, items, monsters, datapacks, protocol profiles or build files

STARTUP

1. Verify current main and open PR state in both repositories.
2. Search active task records and owned paths for conflicts.
3. Read only targeted context.
4. Create implementation child task records rather than editing the architecture task as an implementation claim.
5. Record the exact cross-repository contract version and rollout order.
6. Preserve UNKNOWN facts instead of guessing.

CONTRACT

Contract ID: oteryn.game-catalog
Initial schema version: 1.0.0

Create byte-identical schema files:

Canary:
schemas/game-catalog/v1/game-catalog-snapshot.schema.json

Platform:
resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json

Prove identical SHA-256 values in automated tests.

The contract must include:
- independent protocol/runtime/content provenance;
- releases ordered by explicit release_order;
- stable canonical entity keys;
- snapshot-specific namespaced identifiers;
- independent entity and relation version ranges;
- completeness;
- availability;
- runtime presence;
- typed item data;
- typed creature data;
- typed creature loot relations;
- deterministic ordering;
- strict size, count, string and integer bounds;
- referential integrity;
- SHA-256 semantics.

Never represent versions as floats.
removed_release is an exclusive upper bound.

CANARY EXPORTER

Implement an offline CLI mode following the existing CLI-only startup precedent, for example:

canary --export-game-catalog-only --game-catalog-output=/path/game-catalog.json

Requirements:
- load selected config and datapack;
- collect final runtime item definitions after authoritative loaders;
- collect final MonsterType definitions;
- export creature loot without changing chance semantics;
- merge reviewed metadata from <DATA_DIRECTORY>/catalog/**;
- validate before writing;
- sort deterministically;
- write a temporary file, then atomically rename;
- write a SHA-256 sidecar;
- return non-zero on failure;
- do not open services or start the world;
- do not mutate the database;
- do not process houses, market expiration, world schedulers or webhooks;
- do not expose secrets or absolute machine paths.

Do not create a second partial parser that claims to reproduce final item or monster runtime state.

Inspect loadModules carefully. If late loaders require DB/runtime state, split a bounded catalogue-loading path without weakening normal startup validation.

Initial metadata layout:

<DATA_DIRECTORY>/catalog/profile.json
<DATA_DIRECTORY>/catalog/releases.json
<DATA_DIRECTORY>/catalog/versioning/items.json
<DATA_DIRECTORY>/catalog/versioning/creatures.json
<DATA_DIRECTORY>/catalog/availability/items.json
<DATA_DIRECTORY>/catalog/availability/creatures.json
<DATA_DIRECTORY>/catalog/overrides/approved-backports.json

Malformed, conflicting or ambiguous metadata fails closed. Do not mass-annotate versions by guessing.

PLATFORM MODULE

Create a dedicated app/GameCatalog module following current repository conventions.

Implement:
- releases;
- snapshots;
- profiles;
- stable entities;
- entity snapshots;
- typed item snapshots;
- typed creature snapshots;
- relation snapshots;
- typed loot snapshots;
- import runs and findings;
- profile entity visibility;
- profile relation visibility;
- exact translations extension point;
- explicit Wiki links extension point.

Import flow:
1. enforce limits;
2. compute and verify SHA-256;
3. deduplicate by hash;
4. parse and validate JSON Schema;
5. validate semantic rules;
6. import transactionally into an inactive immutable snapshot;
7. verify counts and references;
8. mark validated or rejected;
9. do not alter public state.

Activation flow:
1. lock profile;
2. require validated snapshot;
3. verify profile compatibility;
4. rebuild entity visibility;
5. rebuild relation visibility;
6. set active snapshot;
7. audit bounded metadata;
8. commit.

Any failure preserves the previous active snapshot. Rollback reactivates an earlier validated snapshot.

PUBLIC VISIBILITY

An entity is visible only if:
- it belongs to the active snapshot;
- runtime_present is true;
- enabled is true;
- its release range contains the profile target;
- its completeness is allowed;
- its availability is publicly allowed;
- dependencies are satisfied;
- no explicit exclusion applies.

A relation also requires visible source and target entities.

Public profiles default to complete-only and no implicit backports.

ROUTES

Register catalogue routes before the generic Wiki /wiki/{slug} route.

Initial routes:
- /{locale}/wiki/catalog
- /{locale}/wiki/items
- /{locale}/wiki/items/{slug}
- /{locale}/wiki/creatures
- /{locale}/wiki/creatures/{slug}

Reserve later NPC and quest routes but do not expose empty or speculative surfaces.

ADMIN SECURITY

Use exact permissions:
- game_catalog.access
- game_catalog.snapshots.view
- game_catalog.snapshots.import
- game_catalog.snapshots.activate
- game_catalog.profiles.manage
- game_catalog.translations.manage
- game_catalog.overrides.manage

Every privileged browser route requires auth, mfa.confirmed and its exact permission. Preserve CSRF. Audit activation, rollback, profile and override changes. No wildcard permissions.

The first slice imports through CLI/deployment files only. Do not add a browser upload.

COMMANDS

Implement or adapt:
- php artisan game-catalog:validate <path>
- php artisan game-catalog:import <path>
- php artisan game-catalog:activate <snapshot-id> --profile=<profile-key>
- php artisan game-catalog:diff <snapshot-a> <snapshot-b>
- php artisan game-catalog:verify --profile=<profile-key>

Import must not activate by default.

SHARED FIXTURE

Create a minimal sanitized fixture with:
- two releases;
- one visible item;
- one future item;
- one complete creature;
- one partial creature;
- one visible loot relation;
- one future loot relation.

Canary validates or generates it. Platform imports it and proves visibility for at least two target releases.

TESTS — CANARY

Prove at minimum:
- version ordering does not use floats;
- exclusive removed-release semantics;
- invalid ranges fail;
- duplicate canonical keys fail;
- dangling relations fail;
- final item runtime values are exported;
- MonsterType values are exported;
- loot chance/count values are preserved;
- deterministic ordering and hashes;
- exporter failure is non-zero;
- no network services start;
- no database mutation occurs;
- atomic output behavior;
- malformed metadata fails closed.

TESTS — PLATFORM

Prove at minimum:
- migration up/down;
- valid import;
- schema and unsupported-version rejection;
- duplicate-hash idempotency;
- invalid reference/range rejection;
- failed import leaves no partial state;
- activation is transactional;
- activation failure preserves old snapshot;
- rollback works;
- future, partial and unverified entities are hidden;
- independently future loot is hidden;
- hidden endpoints hide relations;
- administrator sees internal version/visibility information;
- public output does not expose internal provenance/findings;
- item/creature pagination, filters and detail navigation;
- exact permissions and MFA;
- desktop, tablet, mobile and keyboard acceptance.

ROLLOUT

1. Finalize the shared contract and fixtures.
2. Merge Platform storage/import without public activation.
3. Merge Canary exporter.
4. generate and review a staging snapshot;
5. validate activation and rollback in a non-production environment;
6. add public item and creature surfaces;
7. create separate children for NPCs, quests, availability and historical profiles.

Do not deploy or activate production. Do not claim PRODUCTION_PROVEN.

UNKNOWN — DO NOT GUESS

- complete historical introduced/removed metadata;
- complete quest registry;
- full spawn/raid/map availability;
- exact 15.20 completeness boundary;
- approved public sprite source;
- historical ID mappings;
- 7.60 runtime compatibility.

At completion report:
- task IDs;
- branches and PR URLs;
- contract/schema version and SHA-256;
- changed paths;
- exact validation commands and outcomes;
- final-head CI runs;
- remaining UNKNOWN facts;
- deferred child tasks;
- exactly one next_action.
```
