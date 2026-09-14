# CrystalServer source-material inventory for Oteryn content completion

## Scope

This is a bounded inventory of the repository-owner-supplied `crystalserver-main.zip`. It is evidence for programme decomposition, not a statement that CrystalServer is an authoritative native Oteryn producer.

```yaml
archive_sha256: 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
archive_entries: 9486
regular_files: 8819
root_license_observed: GNU GPL v2
default_datapack_from_config_lua_dist: data-global
alternate_datapack: data-crystal
```

The committed companion JSON is the machine-readable inventory. Revalidate against the exact archive hash before using a count as task acceptance.

## Key findings

| Family | Observed source | Bounded count | Initial class | High-value Platform use |
|---|---|---:|---|---|
| Items | `data/items/items.xml` | 17,571 XML definitions; 38,059 expanded unique server IDs | `DIRECT_STRUCTURED` | Game Catalog, item reference, equipment tools |
| Creatures | `data-global/monster/**/*.lua` | 1,802 files | `TRANSFORM_REQUIRED` | creature reference, hunt tooling |
| Creature loot | same monster corpus | 1,754 files with `monster.loot` assignment | `TRANSFORM_REQUIRED` | loot relations, profit/reference |
| Bestiary-like metadata | same monster corpus | 799 files with Bestiary markers | `TRANSFORM_REQUIRED` | charms/Bestiary tooling |
| NPCs | `data-global/npc/**/*.lua` | 1,112 files | `TRANSFORM_REQUIRED` | NPC catalogue/reference |
| Structured NPC shops | `npcConfig.shop` tables | 290 files / 10,907 buy-or-sell rows | `TRANSFORM_REQUIRED` | buy/sell reference; schema 1.3 target |
| NPC world entries | `data-global/world/world-npc.xml` | 1,043 entries | `DIRECT_STRUCTURED` with profile caveat | location/reference |
| Player spells | `data/scripts/spells/**/*.lua` | 218 files | `TRANSFORM_REQUIRED` | spell catalogue/planner |
| Achievements | `register_achievements.lua` | 558 table entries | `DIRECT_STRUCTURED` for core fields | achievement reference/tracker |
| Quests | `data-global/scripts/quests/**/*.lua` | 1,061 files / 119 top-level families | `PARTIAL_SEMANTICS` | quest inventory/access research |
| Imbuements | `data/XML/imbuements.xml` | 72 elements | `DIRECT_STRUCTURED` | imbuement reference/planner |
| Mounts | `data/XML/mounts.xml` | 250 | `DIRECT_STRUCTURED` | collection reference/tracker |
| Outfits | `data/XML/outfits.xml` | 260 | `DIRECT_STRUCTURED` | collection reference/tracker |
| Vocations | `data/XML/vocations.xml` | 11 | `DIRECT_STRUCTURED` | spell/vocation/build reference |
| Wheel | `src/creatures/players/wheel/**` | 5 source files | `PARTIAL_SEMANTICS` | later build planner after authority review |

`data-crystal` is an alternative datapack, not an additive extension to mix blindly with `data-global`. It contains 1,665 monster Lua files, 33 NPC Lua files and 48 quest Lua files in the scanned families. Any use needs explicit profile semantics.

## Game Catalog fit

Current Platform schema v1.2 already models item, creature and creature-loot entities/relations, and the Platform already has validation/import/diff/activation/rollback machinery. This makes items/creatures/loot the strongest candidate for a bounded source-adapter or completeness task, but only within accepted source-authority semantics.

Draft PR #338 already owns the inactive schema 1.3 NPC/shop consumer paths. New NPC/shop work must reuse, wait for, or deliberately supersede that branch through live coordination; duplicating its paths is forbidden.

The existing `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM` remains the specialized Game Catalog lifecycle. It historically prefers final runtime registry authority over Lua parsing for production truth. Therefore this archive can provide reference/provenance/completeness evidence but cannot silently replace that authority model.

## Wiki fit

Structured reference pages can be sourced from accepted catalogue facts while manual Wiki prose remains editorial. High-value reference candidates include items, creatures, NPC shops, spells, achievements, imbuements, mounts, outfits and vocations.

Quest scripts can help build an inventory and link graph but are not automatically safe walkthrough prose or a complete mission model.

## Player Companion fit

The source families support independent tool candidates after accepted data contracts exist:

- equipment/item comparator;
- creature/loot/hunt reference;
- Bestiary/charm planner;
- NPC buy/sell reference;
- imbuement planner;
- spell/vocation planner;
- achievement tracker;
- quest/access tracker;
- mount/outfit tracker;
- later Wheel/build planning after ruleset authority is resolved.

Each tool must be a separate complete vertical slice rather than one broad toolbox PR.

## Provenance and publication risks

The archive root contains GPL-2.0 text and README lineage to The Forgotten Server/Open Tibia. That is not enough to classify every embedded Tibia-derived name, description, NPC dialogue, map, image or other content as unrestricted project-owned publication material.

Safe default:

- use source files for analysis and deterministic fact extraction;
- preserve archive SHA, datapack, source path and transformation identity;
- prefer normalized facts and Oteryn-owned UI wording;
- treat long descriptions/dialogue/assets/map/media as a separate publication-rights decision;
- keep unclear semantics or rights `UNKNOWN`, `AUTHORITY_REQUIRED` or `DECISION_REQUIRED`.

## Required re-audit

This inventory is intentionally not the final product audit. The first programme auditor must attempt to falsify it and additionally measure actual Platform data population, expected inventories, public/admin reachability, staging/runtime state when authorized, and the real gap between existing engines and player-useful content.