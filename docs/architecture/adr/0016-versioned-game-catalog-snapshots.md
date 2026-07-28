# ADR 0016: Use versioned immutable snapshots for the Game Catalog

Status: Proposed  
Date: 2026-07-28

## Context

Oteryn Wiki currently owns localized editorial articles and categories. The requested catalogue must additionally present structured items, weapons, creatures, loot, NPCs and quests that match the selected server content version and completeness boundary.

The current Canary repository is the runtime authority for item and creature definitions. External wikis may describe Real Tibia or other servers, but they cannot prove what the deployed Oteryn server actually loads or exposes.

One global version field is insufficient. Protocol support, runtime release, datapack content, map content and verified completeness can differ. Entity definitions and relationships such as loot or NPC offers can also begin or end in different releases.

## Decision

1. Create a dedicated Platform-owned `GameCatalog` module instead of storing structured catalogue records as ordinary Wiki articles.
2. Integrate catalogue pages with Wiki navigation and explicit editorial links.
3. Consume deterministic immutable snapshots exported by Canary through `oteryn.game-catalog/v1`.
4. Preserve separate provenance fields for protocol, runtime, target content release, verified content release, datapack, appearances, map and Canary commit.
5. Register releases explicitly and compare by `release_order`, never floating-point values.
6. Give every entity and every relation independent `introduced` and exclusive `removed` bounds.
7. Separate completeness from availability.
8. Default public profiles to complete-only, runtime-present and publicly available content.
9. Precompute entity and relation visibility when a validated snapshot is activated for a profile.
10. Keep imported snapshots immutable and inactive by default.
11. Activate and roll back snapshots transactionally.
12. Require exact RBAC, confirmed MFA and audit for privileged activation, profile and override operations.
13. Require historically appropriate snapshots for historical profiles; do not reconstruct exact 8.60 or 7.60 data by filtering a modern snapshot alone.

## Consequences

### Positive

- Public pages fail closed against incomplete or future content.
- Operators can select an explicit content target such as 15.20 without deleting forward content from storage.
- Loot, NPC offers and quest rewards can have their own version ranges.
- Snapshot provenance makes comparisons and rollback reviewable.
- Wiki remains suitable for editorial content instead of becoming an unbounded runtime-data store.
- Future current and legacy server profiles share one architecture without claiming unsupported protocol compatibility.

### Negative

- Two repositories must coordinate one versioned schema.
- Historical profiles require real historical inputs and cannot be generated automatically from modern definitions.
- Version and availability metadata will remain incomplete until reviewed manifests and map/runtime evidence exist.
- Imports, visibility projections and translations add persistence and operational complexity.

## Rejected alternatives

### Copy data from external wikis

Rejected because those sites do not prove Oteryn runtime state, local customizations or completeness.

### Store every object as a Wiki article

Rejected because structured filtering, exact statistics, relationships, version gating and deterministic updates are not editorial article responsibilities.

### Query Canary source files directly from web requests

Rejected because it couples public request handling to repository layout, duplicates loader semantics and cannot safely guarantee final runtime values.

### Use one mutable catalogue table

Rejected because failed imports could expose partial state, rollback would be destructive, and provenance would be lost.

### Use one global `server_version` float

Rejected because values such as 8.60 are not safe numeric versions and because protocol, datapack and verified-content boundaries differ.

## Implementation boundary

The first implementation slice covers:

- releases;
- immutable snapshots;
- content profiles;
- items;
- creatures;
- creature loot;
- import validation;
- visibility projections;
- activation and rollback;
- public item/creature lists and details;
- administrator snapshot and version visibility.

NPCs, quests, map availability, sprite sourcing and historical profiles are separate reviewed slices.

## Related records

- `docs/architecture/GAME_CATALOG_ARCHITECTURE.md`
- `docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md`
- `docs/agents/tasks/active/OTERYN-20260728-versioned-game-catalog-architecture.md`
- Canary contract `docs/contracts/GAME_CATALOG_EXPORT_CONTRACT.md`
- Canary task `CAN-20260728-game-catalog-export-architecture`