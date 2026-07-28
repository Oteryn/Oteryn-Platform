# Oteryn Game Catalog Import Contract

Status: Proposed  
Contract ID: `oteryn.game-catalog`  
Initial schema version: `1.0.0`  
Producer: `blakinio/canary`  
Consumer: `blakinio/Oteryn-Platform`

## 1. Purpose

This contract defines the only supported boundary for importing structured game catalogue data from Canary into Oteryn Platform. It is read-only with respect to Canary and Platform-owned after import.

It does not authorize Platform to mutate Canary gameplay data, infer missing content, import external wiki data, or claim historical parity.

## 2. Ownership

- Canary owns source runtime semantics and snapshot production.
- Platform owns imported snapshot persistence, profiles, visibility, localization, UI and administration.
- One snapshot is an immutable representation of one exact producer state.
- Platform never silently repairs producer facts. A corrected producer result is a new snapshot.

## 3. Transport

The first supported transport is a local operator/deployment file:

```text
game-catalog.json
game-catalog.json.sha256
```

No public or administrator browser upload is allowed in schema v1.

The consumer must:

- enforce a configured maximum byte size before full processing;
- compute SHA-256 itself;
- compare the optional sidecar with the computed value;
- reject malformed UTF-8 or JSON;
- reject unsupported contract/schema versions;
- import transactionally into an inactive snapshot.

## 4. Top-level shape

```json
{
  "contract": "oteryn.game-catalog",
  "schema_version": "1.0.0",
  "snapshot": {},
  "releases": [],
  "entities": [],
  "relations": []
}
```

Unknown top-level fields are rejected in v1 unless the shared JSON Schema explicitly allows them.

## 5. Snapshot provenance

Required fields:

```text
generated_at
canary_commit_sha
protocol_profile
runtime_release
content_target_release
verified_content_through_release
appearances_sha256
```

Optional but recommended:

```text
datapack_commit_sha
contains_content_through_release
map_sha256
producer_build_id
```

A commit SHA must be a complete lowercase hexadecimal Git SHA supported by the schema. Hashes must use lowercase hexadecimal SHA-256.

Protocol, runtime and content versions are independent facts. The consumer must not replace one with another.

## 6. Release records

Each release contains:

```text
key
display_label
major
minor
patch
build nullable
release_order
protocol_family nullable
released_at nullable
```

Rules:

- `key` is unique;
- `release_order` is unique;
- ordering uses `release_order`, never floating-point conversion;
- every referenced release must exist in the same document or in an explicitly compatible retained registry;
- conflicting definitions for a retained release key are blocking.

## 7. Entities

Required fields:

```text
type
canonical_key
introduced_in nullable
removed_in nullable
completeness
availability
runtime_present
enabled
identifiers
data
```

Initial supported types:

```text
item
creature
```

Reserved later types:

```text
npc
quest
spell
area
```

Rules:

- `(type, canonical_key)` is unique;
- canonical keys are stable language-independent identifiers;
- names are display data, not identity;
- identifiers are namespaced and snapshot-specific;
- `removed_in`, when present, must be later than `introduced_in`;
- `partial`, `unverified`, `disabled` and `missing_dependencies` cannot be interpreted as `complete`;
- `runtime_present=false` cannot be made public without a separately reviewed future contract change.

## 8. Item payload

The v1 item payload may contain bounded values for:

```text
server_id
client_id
ware_id
name
description
category
weapon_type
attack
defense
extra_defense
armor
range
weight
minimum_level
vocations
slot_position
imbuement_slots
upgrade_classification
element_type
element_value
stackable
pickupable
image_key
attributes
```

The shared JSON Schema defines required fields and numeric/string bounds. Consumer code must ignore no required field silently.

## 9. Creature payload

The v1 creature payload may contain:

```text
name
description
race_id
look_type
health
max_health
experience
speed
armor
defense
mitigation
is_boss
is_reward_boss
bestiary_class
bestiary_race
bestiary_occurrence
bestiary_to_kill
charm_points
elements
immunities
attacks
defenses
attributes
```

Unknown or unsupported combat structures must remain bounded opaque attributes or be rejected according to the schema. They must not be executed.

## 10. Relations

Required fields:

```text
type
canonical_key
source
target nullable
introduced_in nullable
removed_in nullable
completeness
enabled
data
```

Initial supported relation:

```text
creature_loot
```

Reserved later relations:

```text
npc_buy_offer
npc_sell_offer
quest_reward
quest_requirement
creature_spawn
npc_spawn
item_source
item_transform
item_upgrade
```

Rules:

- relation canonical keys are unique within a snapshot;
- every source and target must resolve to an entity in the snapshot;
- relation ranges are independent from endpoint ranges;
- a public projection must hide a relation when either endpoint is hidden;
- an invalid endpoint is blocking, not a warning.

## 11. Loot payload

```text
chance_numerator
chance_denominator
minimum_count
maximum_count
container_path nullable
condition_data nullable
```

Rules:

- denominator is positive;
- numerator is between zero and denominator;
- counts are bounded non-negative integers;
- `maximum_count >= minimum_count`;
- the consumer must not assume one universal denominator.

## 12. Determinism

Producer requirements:

- stable UTF-8 serialization;
- deterministic object key and array ordering;
- canonical sorting by type and canonical key;
- no timestamps other than the declared generation timestamp;
- no machine-specific absolute paths;
- no nondeterministic map iteration leakage.

The same producer inputs and fixed generation timestamp must create byte-identical output. The producer test suite must also prove semantic equality without fixing the timestamp when appropriate.

## 13. Security and bounds

The shared schema and consumer enforce at least:

- maximum document size;
- maximum releases, entities, relations and identifiers;
- maximum string and array lengths;
- integer bounds safe for PHP and database storage;
- no raw HTML requirement for imported descriptions;
- no executable PHP, Lua or JavaScript semantics;
- no remote resource fetching during import;
- relative sanitized source paths only;
- no credentials, tokens, connection strings or personal data.

## 14. Import states

```text
pending
validating
validated
rejected
active
superseded
```

Import never activates automatically. Validation findings are Platform-owned records associated with the import run.

## 15. Activation compatibility

Before activation the consumer verifies:

- snapshot status is `validated`;
- profile target release exists;
- profile and snapshot contract versions are compatible;
- target release is not beyond the profile's approved verified boundary unless an explicit reviewed policy permits it;
- all visibility projections can be rebuilt successfully.

Activation is transactional. Failure preserves the prior active snapshot.

## 16. Public visibility contract

An entity is visible only if the active profile rules admit:

- its release range;
- completeness;
- availability;
- runtime presence;
- enabled state;
- dependency state;
- explicit profile overrides.

A relation also requires visible endpoints.

Schema v1 defaults to complete-only public content and no implicit backports.

## 17. Schema synchronization

Canonical schema path in Platform:

```text
resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
```

Matching Canary path:

```text
schemas/game-catalog/v1/game-catalog-snapshot.schema.json
```

The implementation programme must prove the two files have identical SHA-256 values. A schema change requires:

1. a new semantic schema version;
2. compatibility analysis;
3. updates in both repositories;
4. shared fixture updates;
5. contract tests in both repositories;
6. rollout ordering documentation.

## 18. Initial shared fixture

The first implementation fixture must contain:

- at least two releases;
- one visible item;
- one future item;
- one complete creature;
- one partial creature;
- one visible loot relation;
- one future loot relation.

Canary validates or generates the fixture. Platform imports it and proves the expected visibility for at least two profiles or target releases.

## 19. Rollout order

Additive rollout:

1. merge compatible architecture and contract records;
2. merge Platform storage/import support without public activation;
3. merge Canary exporter;
4. generate and review a staging snapshot;
5. activate a non-production profile and validate rollback;
6. add public items and creatures surfaces;
7. add NPCs, quests, map availability and historical profiles as separate slices.

No production deployment or profile activation is authorized by this contract.

## 20. Fail-closed rules

Reject the snapshot for:

- unsupported schema;
- invalid hashes;
- conflicting release definitions;
- invalid version ranges;
- duplicate canonical keys;
- dangling relation endpoints;
- declared/imported count mismatch;
- over-limit content;
- unsafe paths or unsupported data forms;
- failed transaction or projection build.

Warnings may describe incomplete but structurally valid records. Warnings never promote public visibility.