# Versioned Oteryn Game Catalog Architecture

Status: Proposed  
Contract: `oteryn.game-catalog/v1`  
Repositories: `Oteryn/Oteryn-Platform`, `blakinio/canary`

> Compatibility scope: this document describes the delivered/planned Legacy Canary Compatibility catalogue. ADR 0034 and `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md` own the separate native Oteryn-v2 target and supersede any implication that Canary schemas or identifiers define native content authority.

## 1. Purpose

Oteryn needs a public catalogue of items, weapons, creatures, loot, NPCs and quests that displays only content intentionally available for the selected server content profile. The catalogue must support release gating such as 15.20, 8.60 or a future 7.60 profile without pretending that a modern datapack contains historically exact older data.

External sites such as Tibiopedia, RubinOT Wiki and Tibia Fandom are useful UX references only. They are not runtime authorities and must not feed production catalogue records directly.

## 2. Architectural decision

Create a dedicated Platform-owned `GameCatalog` module integrated with Wiki navigation and editorial links, but separate from the Wiki article persistence model.

Use immutable, provenance-pinned snapshots exported from Canary. Import, validate and retain snapshots in Oteryn Platform. An administrator-controlled content profile selects one validated snapshot and a target release. Public queries use a precomputed visibility projection for that exact profile and snapshot.

```text
Canary final runtime definitions
  + reviewed datapack catalogue metadata
  -> deterministic JSON snapshot + SHA-256
  -> Platform schema and semantic validation
  -> immutable imported snapshot
  -> profile activation transaction
  -> entity and relation visibility projections
  -> public Game Catalog and Wiki links
```

## 3. Responsibility boundary

### Canary owns

- final item runtime definitions after appearances and item configuration are loaded;
- final creature definitions and loot;
- later NPC definitions and shop offers;
- later spawn, raid, map and quest availability evidence;
- server IDs and runtime-specific identifiers;
- datapack catalogue manifests;
- export provenance and deterministic serialization.

### Oteryn Platform owns

- snapshot ingestion and validation;
- immutable imported snapshot persistence;
- release registry and content profiles;
- activation and rollback;
- public filtering, pagination and search;
- administrator visibility, validation findings and comparisons;
- EN/PL catalogue translations;
- explicit catalogue-to-Wiki editorial links;
- RBAC, MFA and audit for privileged catalogue operations.

### Wiki continues to own

- editorial articles;
- guides, walkthroughs and lore;
- localized Markdown content;
- publication lifecycle and revisions.

The catalogue must not duplicate structured runtime records into ordinary Wiki articles.

## 4. Version model

Versions must never be stored or compared as floating-point numbers. A release is a registered record:

```text
game_catalog_releases
- key              e.g. tibia-15-20
- display_label    e.g. 15.20
- major
- minor
- patch
- build nullable
- release_order    explicit sortable integer
- protocol_family nullable
- released_at nullable
```

The architecture separates:

- `protocol_profile`;
- `runtime_release`;
- `content_target_release`;
- `verified_content_through_release`;
- `contains_content_through_release`;
- `datapack_revision`;
- `appearances_revision` or hash;
- `map_sha256`;
- `canary_commit_sha`.

A server may accept a 15.25 protocol while exposing only content verified through 15.20. These facts must not be collapsed into one `server_version` field.

### Version range semantics

Every entity and every relation has an independent range:

```text
introduced_release <= profile.target_release
AND
(removed_release IS NULL OR profile.target_release < removed_release)
```

`removed_release` is an exclusive upper bound.

## 5. Completeness and availability

### Completeness

```text
complete
partial
unverified
disabled
missing_dependencies
```

Public profiles are `complete_only` by default. Importing a record does not promote it to `complete`.

### Availability

```text
obtainable
encounterable
quest_only
boss_only
event_only
npc_only
starter
registered_only
admin_only
unreachable
unknown
```

Completeness and availability are independent. A creature can be fully defined but only `registered_only`; it remains hidden from a normal public profile.

## 6. Public visibility

An entity is public only when all conditions are true:

1. it belongs to the active snapshot;
2. `runtime_present` is true;
3. `enabled` is true;
4. its release range contains the profile target release;
5. its completeness is allowed by the profile;
6. its availability is allowed by the profile;
7. all required dependencies are present;
8. no explicit profile exclusion applies.

A relation is public only when:

1. its own release, completeness and enabled rules pass;
2. its source entity is public;
3. its target entity, when present, is public;
4. no explicit profile exclusion applies.

Therefore a creature and item may both exist while a future loot relation between them remains hidden.

## 7. Stable identity

Snapshot-specific server IDs are not stable cross-version identities. Use a language-independent canonical key:

```text
item:dragon-shield
creature:dragon
npc:benjamin
quest:the-postman-missions
```

Snapshot-specific identifiers are separate namespaced values:

```json
{
  "namespace": "canary.server_item_id",
  "value": "2516"
}
```

Do not merge records only because names match. Ambiguous identity remains unresolved until reviewed evidence exists.

## 8. Snapshot model

### `game_catalog_snapshots`

```text
id
contract_version
schema_version
content_sha256 UNIQUE
canary_commit_sha
datapack_commit_sha nullable
protocol_profile
runtime_release_id
content_target_release_id
verified_content_through_release_id
contains_content_through_release_id nullable
appearances_sha256
map_sha256 nullable
generated_at
imported_at
status
entity_count
relation_count
validation_summary JSON
created_at
updated_at
```

Statuses:

```text
pending
validating
validated
active
rejected
superseded
```

Snapshots are immutable after validation. A corrected export is a new snapshot.

### `game_catalog_profiles`

```text
id
key UNIQUE
name
target_release_id
active_snapshot_id nullable
complete_only
public_enabled
allow_backports
lock_version
created_at
updated_at
```

Example profiles:

- `oteryn-current`;
- `oteryn-legacy-860`;
- `oteryn-legacy-760`.

A historical profile requires a historically appropriate snapshot. Filtering a modern snapshot alone does not prove historical stats, IDs, mechanics or availability.

## 9. Entity persistence

### `game_catalog_entities`

```text
id
entity_type
canonical_key
created_at
updated_at
UNIQUE(entity_type, canonical_key)
```

Initial types:

```text
item
creature
npc
quest
spell
area
```

### `game_catalog_entity_snapshots`

```text
id
snapshot_id
entity_id
introduced_release_id nullable
removed_release_id nullable
completeness
availability
runtime_present
enabled
data_sha256
source_path nullable
source_key nullable
created_at
UNIQUE(snapshot_id, entity_id)
```

### Typed item projection

`game_catalog_item_snapshots` stores bounded searchable fields:

```text
entity_snapshot_id
server_id
client_id nullable
ware_id nullable
name
description nullable
category
weapon_type nullable
attack nullable
defense nullable
extra_defense nullable
armor nullable
range nullable
weight nullable
minimum_level nullable
vocations JSON nullable
slot_position nullable
imbuement_slots nullable
upgrade_classification nullable
element_type nullable
element_value nullable
stackable
pickupable
image_key nullable
attributes JSON
```

### Typed creature projection

`game_catalog_creature_snapshots`:

```text
entity_snapshot_id
name
description nullable
race_id nullable
look_type nullable
health
max_health
experience
speed
armor
defense
mitigation nullable
is_boss
is_reward_boss
bestiary_class nullable
bestiary_race nullable
bestiary_occurrence nullable
bestiary_to_kill nullable
charm_points nullable
elements JSON
immunities JSON
attacks JSON
defenses JSON
attributes JSON
```

NPC and quest typed projections are reserved for later reviewed slices.

## 10. Relation persistence

### `game_catalog_relation_snapshots`

```text
id
snapshot_id
relation_type
canonical_key
source_entity_id
target_entity_id nullable
introduced_release_id nullable
removed_release_id nullable
completeness
enabled
data_sha256
attributes JSON
created_at
UNIQUE(snapshot_id, relation_type, canonical_key)
```

Relation types:

```text
creature_loot
npc_buy_offer
npc_sell_offer
quest_reward
quest_requirement
creature_spawn
npc_spawn
item_source
item_transform
item_upgrade
article_reference
```

### Loot projection

`game_catalog_loot_snapshots`:

```text
relation_snapshot_id
chance_numerator
chance_denominator
minimum_count
maximum_count
container_path nullable
condition_data JSON nullable
```

The denominator is explicit. Platform code must not assume one global loot denominator.

## 11. Localized catalogue content

`game_catalog_entity_translations`:

```text
id
entity_id
locale
display_name
slug
summary nullable
description_markdown nullable
source_name_sha256
translation_status
created_at
updated_at
UNIQUE(entity_id, locale)
UNIQUE(locale, slug)
```

Statuses:

```text
approved
stale
missing
```

Snapshot import must not overwrite reviewed translations. A changed source name or description marks the translation stale.

## 12. Wiki integration

`game_catalog_wiki_links`:

```text
entity_id
wiki_article_id
link_type
sort_order
PRIMARY KEY(entity_id, wiki_article_id, link_type)
```

Link types:

```text
guide
strategy
lore
quest_walkthrough
related
```

Catalogue detail pages may show related published Wiki articles. Wiki articles may link to catalogue entities using reviewed route helpers rather than hard-coded external URLs.

## 13. Import and validation

### Import command flow

```text
1. enforce file-size and count limits;
2. compute SHA-256;
3. return the existing import result for an identical hash;
4. parse JSON;
5. validate JSON Schema;
6. verify contract and schema versions;
7. validate releases and ranges;
8. validate canonical-key uniqueness;
9. validate all relation endpoints;
10. import into a new inactive snapshot transaction;
11. verify declared and imported counts;
12. mark validated or rejected;
13. leave the current public snapshot unchanged.
```

The first slice imports only from an operator CLI/deployment path. It must not provide a browser upload.

### Activation

Activation is a transaction:

```text
1. lock the profile row;
2. verify the snapshot is validated;
3. verify contract/profile compatibility;
4. rebuild profile entity visibility;
5. rebuild profile relation visibility;
6. set active_snapshot_id;
7. append bounded administrator audit metadata;
8. commit.
```

Any failure preserves the previously active snapshot and projections.

### Rollback

Rollback reactivates an earlier validated snapshot. Imported snapshots are retained; destructive rollback migrations are not required.

## 14. Visibility projections

### `game_catalog_profile_entities`

```text
profile_id
entity_snapshot_id
visible
reason_code
computed_at
PRIMARY KEY(profile_id, entity_snapshot_id)
```

### `game_catalog_profile_relations`

```text
profile_id
relation_snapshot_id
visible
reason_code
computed_at
PRIMARY KEY(profile_id, relation_snapshot_id)
```

Public queries use these projections, not ad hoc rule evaluation on every request.

Reason codes include:

```text
visible
future_release
removed_before_target
partial
unverified
disabled
runtime_missing
availability_not_public
missing_dependency
source_hidden
target_hidden
profile_excluded
```

## 15. Overrides and backports

`game_catalog_profile_overrides` supports reviewed exceptions:

```text
id
profile_id
entity_id nullable
relation_snapshot_id nullable
action
reason
approved_by_identity_id
approved_at
created_at
```

Actions:

```text
include_backport
exclude
force_hidden
```

Every override requires authenticated administrator context, confirmed MFA, an exact permission, a bounded reason and audit. The default is no backports.

## 16. Public routes

Register Game Catalog routes before the generic Wiki article route:

```text
/{locale}/wiki/catalog
/{locale}/wiki/items
/{locale}/wiki/items/{slug}
/{locale}/wiki/creatures
/{locale}/wiki/creatures/{slug}
/{locale}/wiki/npcs
/{locale}/wiki/npcs/{slug}
/{locale}/wiki/quests
/{locale}/wiki/quests/{slug}
```

The first vertical slice activates only items and creatures.

Public users do not see internal source paths, validation diagnostics, completeness administration or raw release metadata. Public pages may show a simple server-content label if product policy later approves it.

## 17. Administrator surfaces

```text
/admin/game-catalog
/admin/game-catalog/snapshots
/admin/game-catalog/snapshots/{snapshot}
/admin/game-catalog/profiles
/admin/game-catalog/profiles/{profile}/edit
/admin/game-catalog/findings
/admin/game-catalog/diff
```

Administrator tables expose:

- introduced release;
- removed release;
- completeness;
- availability;
- snapshot provenance;
- active-profile visibility;
- visibility reason;
- changed fields between snapshots.

Exact permissions:

```text
game_catalog.access
game_catalog.snapshots.view
game_catalog.snapshots.import
game_catalog.snapshots.activate
game_catalog.profiles.manage
game_catalog.translations.manage
game_catalog.overrides.manage
```

Privileged browser routes require `auth`, `mfa.confirmed` and the exact permission. Mutations remain CSRF-protected and audited.

## 18. Proposed Platform file structure

```text
app/GameCatalog/
  GameCatalogServiceProvider.php
  Domain/
    CatalogEntityType.php
    CatalogRelationType.php
    CatalogCompleteness.php
    CatalogAvailability.php
    CatalogSnapshotStatus.php
    CatalogVisibilityReason.php
    GameVersion.php
    Exceptions/
  Application/
    Import/
    Profiles/
    Diff/
    Translation/
  Infrastructure/
    Json/
    Models/
    Persistence/
    Audit/
  Queries/
    Public/
    Admin/
  Http/
    Public/
    Admin/
  ViewModels/
    Public/
    Admin/
  Console/
config/game-catalog.php
database/migrations/*game_catalog*.php
resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
resources/navigation/public/game-catalog.php
resources/navigation/admin/game-catalog.php
resources/views/game-catalog/**
routes/modules/game-catalog.php
lang/en/game_catalog.php
lang/pl/game_catalog.php
public/css/game-catalog.css
tests/Unit/GameCatalog/**
tests/Feature/GameCatalog/**
tests/Integration/GameCatalog/**
scripts/acceptance/tests/*game-catalog*.spec.mjs
```

## 19. Commands

Proposed operator commands:

```text
php artisan game-catalog:validate <path>
php artisan game-catalog:import <path>
php artisan game-catalog:activate <snapshot-id> --profile=<profile-key>
php artisan game-catalog:diff <snapshot-a> <snapshot-b>
php artisan game-catalog:verify --profile=<profile-key>
```

Import does not activate by default.

## 20. Public presentation

### Item list

Initial fields:

- image when an approved asset source exists;
- name;
- category and weapon type;
- attack;
- defense and armor;
- level and vocations;
- obtainability summary.

### Creature list

Initial fields:

- image when approved;
- name;
- HP;
- experience;
- Bestiary class;
- boss flag.

### Details

Item details show exact active-snapshot attributes and reverse sources. Creature details show active-snapshot attributes and visible loot relations. Wiki guides are shown through explicit links.

## 21. Caching and performance

Cache keys include:

```text
profile key
active snapshot SHA-256
locale
entity type
normalized filters
page
```

Changing the active snapshot naturally changes the cache namespace. Public queries use bounded pagination, selected columns, visibility projections and indexed typed fields.

## 22. Security requirements

Treat snapshots as untrusted input:

- bound file size, entities, relations, strings and integers;
- reject unsupported schema versions;
- reject duplicate keys and dangling references;
- allow only relative sanitized source paths;
- persist no executable source payload;
- allow no arbitrary HTML, PHP, Lua or JavaScript;
- perform transactional inactive imports;
- expose no browser upload in the first slice;
- keep all import failures out of public state;
- log no secrets or full sensitive payloads.

## 23. Validation strategy

### Platform unit and integration evidence

Prove:

- non-float version ordering;
- exclusive removed-release semantics;
- valid and invalid schema handling;
- unsupported schema rejection;
- duplicate-hash idempotency;
- invalid range and dangling relation rejection;
- no partial persistence after failure;
- transactional activation;
- old snapshot preservation on activation failure;
- rollback;
- future and partial entity hiding;
- independently versioned relation hiding;
- hidden endpoints hiding relations;
- admin-only internal metadata;
- public item/creature pagination and filters;
- exact permission and MFA gates.

### Browser acceptance

Cover desktop, tablet, mobile and keyboard interaction for:

- item list and detail;
- creature list and detail;
- loot navigation;
- empty and unavailable states;
- admin snapshot and profile views;
- authorization denied and MFA-required states.

## 24. Delivery slices

### Slice 0 — architecture and contract

- task records;
- ADR;
- export/import contracts;
- JSON Schema v1;
- shared fixture;
- ownership and rollout order.

### Slice 1 — items, creatures and loot

- Canary offline deterministic export;
- Platform validation and immutable import;
- profiles and visibility projections;
- activation and rollback;
- public item and creature pages;
- administrator snapshot/version visibility.

### Slice 2 — NPCs

- safe read-only Canary NPC registry iteration;
- NPC details;
- versioned buy/sell offers;
- availability evidence.

### Slice 3 — quests

- explicit quest manifest;
- requirements and rewards;
- related NPC, creature and item dependencies;
- Wiki walkthrough links.

### Slice 4 — map and event availability

- creature and NPC spawns;
- raids and scheduled events;
- map placement and entry evidence;
- fail-closed reachability.

### Slice 5 — historical profiles

- dedicated 8.60 snapshot and validation;
- later historically appropriate snapshots;
- 7.60 only after protocol, assets, datapack and runtime compatibility are independently proven.

## 25. First-slice completion gate

The first implementation slice is complete only when:

1. Canary produces a deterministic items, creatures and loot snapshot.
2. Export does not start world services or mutate the database.
3. Both repositories use the same schema version and schema hash.
4. Platform rejects malformed and semantically invalid snapshots.
5. Import is idempotent and inactive by default.
6. Activation and rollback are transactional.
7. A 15.20 profile hides records and relations introduced later.
8. `partial` and `unverified` records are hidden publicly.
9. Public item and creature lists/details work.
10. Administrator views show versions and visibility reasons.
11. Public output excludes internal provenance and validation details.
12. Focused unit, integration, contract and E2E validation passes on final heads.

## 26. Explicit unknowns

The following remain `UNKNOWN` until separately proven:

- complete historical introduction/removal versions for all content;
- complete quest registry;
- full spawn, raid and map availability evidence;
- exact current 15.20 completeness boundary;
- approved public sprite source;
- historical cross-version server/client ID mapping;
- actual 7.60 runtime compatibility.

Unknown data must be blocked, marked unverified or deferred. It must never be guessed.
