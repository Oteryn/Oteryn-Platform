# OTERYN-20260812 Worldmap reconstruction and comparison plan

Task: `OTERYN-20260811-tibia-client-analysis`
Branch: `ops/oteryn-tibia-client-analysis-20260811`
PR: `#1006` (draft)

## Objective

Prepare a deterministic path from decoded official-client worldmap data to a reconstructed OTBM-compatible map and compare the reconstructed result against the existing reference maps used by the project.

## Reference maps for comparison

The reconstructed map must be compared against all of the following reference sources when the corresponding data is available:

1. CrystalServer world map (`zimbadev/crystalserver`, `data-global/world`) — server-side OTBM/content baseline.
2. Renemap — independent map/diff source previously used to identify areas absent from CrystalServer.
3. TibiaMaps — geographic/coverage reference for identifying whether a location exists in current Tibia and for validating broad positional coverage.

These references are comparison inputs, not automatic sources of truth for every field. Conflicts must be preserved explicitly instead of silently selecting one source.

## Reconstruction pipeline

The target pipeline is:

`decoded Worldmap message`
`-> (x,y,z) field coordinate`
`-> ordered field contents`
`-> AppearanceInstance numeric ID`
`-> appearance classification/properties`
`-> client/object ID to server/OTB ID translation`
`-> normalized tile model`
`-> OTBM export`
`-> tile-by-tile comparison against CrystalServer / Renemap / TibiaMaps`

The existing runtime evidence already proves `(x,y,z) -> ordered field contents -> appearance-factory numeric IDs`; see `OTERYN-20260812-worldmap-runtime-capture.md`.

## Normalized tile model

Before writing OTBM, every observed tile should be stored in a neutral representation containing at minimum:

```text
coordinate:
  x
  y
  z
contents:
  - stack_index
    client_appearance_id
    semantic_class
    server_otb_id
    mapping_status
source:
  capture/session identifier
  client version
  evidence timestamp
```

`semantic_class` must distinguish at least:

- `ground`
- `ground_border`
- `static_item`
- `dynamic_item`
- `creature`
- `npc`
- `effect_or_ephemeral`
- `unknown`

No `unknown` content may be silently converted to a guessed OTB item.

## Ground and static environment reconstruction

For every captured `AppearanceInstance` ID:

1. Resolve the exact appearance definition from the official client appearance catalogue.
2. Read the appearance properties used by OTClient-compatible loaders.
3. Classify ground/floor separately from borders and normal items.
4. Translate the client/object ID to the corresponding server/OTB ID.
5. Preserve ordered stack position for all non-ground contents.
6. Emit the normalized tile only when the mapping status is recorded.

The reconstruction must cover not only floor tiles but also walls, borders, doors, stairs, rocks, vegetation, furniture, decorations and other static map elements visible in decoded field contents.

## OTBM comparison contract

Comparison must be coordinate-based and structural, not visual/OCR-based.

For each `(x,y,z)` compare:

- tile presence/absence;
- ground ID;
- ordered static item IDs;
- count of static items;
- stack/order differences when semantically relevant;
- missing elements;
- additional elements;
- unknown/unmapped client IDs;
- source conflicts.

Use these statuses:

- `MATCH`
- `MISSING_IN_REFERENCE`
- `MISSING_IN_RECONSTRUCTION`
- `GROUND_MISMATCH`
- `ITEM_MISMATCH`
- `STACK_ORDER_MISMATCH`
- `UNMAPPED_ID`
- `REFERENCE_CONFLICT`
- `NOT_OBSERVED`

A tile that was never received by the official client must be `NOT_OBSERVED`, never treated as empty.

## Creature, NPC and spawn evidence

Creatures and NPCs observed in live field contents must be stored separately from the static OTBM tile model.

An individual creature observation may contain:

- coordinate;
- observed creature/NPC identity or type when decoded;
- appearance;
- first/last observation time;
- movement events;
- appearance/disappearance events.

A single observation does **not** prove a spawn definition.

Spawn reconstruction requires repeated observations and must separately infer/verify:

- creature type;
- spawn center/area;
- amount;
- candidate spawn positions;
- respawn timing;
- whether movement from another spawn could explain the observation.

Use evidence states `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT`. Do not emit a server spawn XML/OTBM spawn claim as `PROVEN` from a single sighting.

NPC reconstruction may record stable position/name/appearance when decoded, but scripts, trade offers, dialogue and quest logic remain separate data and must not be inferred from map presence alone.

## Acceptance milestones

### M1 — appearance ID semantics

Prove the exact field name/meaning of `AppearanceInstance+0x30` and classify captured live IDs from official appearance data.

### M2 — client ID to OTB translation

Produce a deterministic mapping for observed client/object IDs to server/OTB IDs, with explicit `UNMAPPED_ID` handling.

### M3 — normalized map collector

Produce a collector/export format that accumulates observed `(x,y,z)` tiles and deduplicates later updates while preserving the latest authoritative tile state and evidence provenance.

### M4 — OTBM writer

Export a bounded reconstructed region to valid OTBM without inserting guessed tiles or IDs.

### M5 — reference comparison

Compare the same bounded coordinates against CrystalServer, Renemap and TibiaMaps where available and generate a machine-readable diff plus a human summary.

### M6 — scale-out

Only after M1-M5 are validated, expand collection coverage and separately accumulate creature/NPC observations for spawn analysis.

## Safety and provenance

- Do not commit proprietary Tibia binaries/assets.
- Do not commit credentials, session material, account data or character-private data.
- Store only normalized IDs/properties/evidence necessary to reproduce the map analysis.
- Preserve client version and source identity with every generated mapping dataset so future client updates cannot silently corrupt translation results.
- Never overwrite an existing reference map during comparison; comparison output and reconstructed OTBM must be separate artifacts until explicitly approved.

## Current next action

Resolve the exact `AppearanceInstance+0x30` field semantics and decode the already captured live IDs against the official appearance catalogue, then build the first normalized tile record and compare that bounded coordinate set against the available reference maps.
