# Oteryn Dynamic Semantic Atlas — target architecture

## Status and authority

**Status: PROPOSED DESIGN BASELINE / documentation-only.**

Date: 2026-08-17  
Programme alias: `DYN-ATLAS`  
First bounded proof: `DYN-ATLAS-001 — Semantic Thais Z7 Proof`

This document records the agreed target direction for the Dynamic Semantic Atlas. It is subordinate to accepted ADRs and contracts and MUST NOT silently override them. In particular:

- `docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md` remains the Platform-side accepted authority for Game/Atlas ownership and artifact-first integration;
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` remains the accepted Platform owner for hunt guidance, progression tracking, recommendations and private player planning;
- current `blakinio/Oteryn-v2@5577f6fc7c1f7ddef482f0f7b08039047704e36b` is read-only cross-repository evidence for this task;
- `Oteryn-v2/docs/contracts/OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md` owns Game -> Atlas public-safe export semantics;
- `Oteryn-v2/docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md` owns the native World/Content model and Studio boundary;
- `Oteryn-v2/docs/architecture/ANL-02_GAMEPLAY_BALANCE_WORLD_ANALYTICS_CONTRACT_CANDIDATE.md` is useful **nonbinding candidate evidence** for hunt/session/world analytics and MUST NOT be promoted here to accepted Game authority;
- `Oteryn-v2/docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md` is owner-accepted pre-contract input for privacy-first presence, not a browser-map authorization shortcut;
- `Oteryn-v2/docs/agents/evidence/OTV2-20260816-game-atlas-physical-profile-readiness.md` records `EVIDENCE_GAP` for the first physical Game -> Atlas profile and therefore prevents this document from freezing a serializer, permanent chunk size or coordinate profile.

If a later accepted ADR or Game-owned contract conflicts with this document, the higher-authority source wins and this document must be reconciled explicitly.

## Product direction

The existing Atlas assumptions remain. The target is an **evolution of the main Atlas**, not a replacement product and not a second independent map/wiki stack.

The long-term product is:

> **World Map + World Knowledge + Player Companion + Authorized Live State**

The same map surface should be able to grow from today’s navigation/visualisation into a semantic player companion supporting, when the required upstream contracts exist:

- semantic tile/object inspection;
- NPC locations, services, shops, travel and public dialogue/read-model information;
- monster locations, spawn/encounter areas and public loot knowledge;
- hunt discovery and personalized hunt recommendations;
- bounty/task planning and player-specific progress;
- routing and access-aware navigation;
- owner/party/consent-authorized live positions;
- privacy-safe occupancy/activity overlays;
- live event/raid/world-state overlays that are explicitly public-safe;
- later reuse of rendering/inspection components in Oteryn Studio.

These capabilities do not move canonical gameplay authority into Atlas.

## Non-negotiable ownership model

```text
Oteryn-Game / current Oteryn-v2 lineage
  owns canonical World/Content, identities, rules, authored interactions,
  NPC/monster/content definitions, authoritative mutable gameplay state,
  compiler/validation and Game-owned Atlas export semantics

Oteryn-Atlas
  owns derived immutable ingestion, indexes, map/search/runtime presentation,
  semantic picking, routing presentation, browser caches and release lifecycle

Oteryn-Platform
  owns web Identity/auth, PlayerCompanion workflows, owner-private plans,
  recommendation orchestration, portal discovery and approved web integration
```

Atlas MUST NOT become a second canonical `Semantic World Model`. The browser/runtime model is an **Atlas Semantic Projection / Atlas Scene Read Model**, deterministically derived from Game-owned canonical semantics.

Atlas MUST NOT use OTBM, Legacy IR, Canary/Crystal Lua/XML, undocumented Game database tables, live GameNode memory or Platform tables as a fallback world authority when the Game export omits a fact.

## Four data planes

The target Atlas separates four concerns that may be composed visually but have different owners and security properties.

### 1. Immutable static world projection

Contains public-safe static presentation/navigation facts derived from one immutable Game export revision:

- terrain and static presentation records;
- ordered tile/object placement;
- public areas/subareas/zones;
- transitions/teleports where public;
- static collision/navigation facts where public policy permits;
- stable content/entity references;
- asset/appearance references permitted by the export policy.

Published static bytes are immutable. Correction means a new export identity/digest.

### 2. Semantic knowledge projection

Contains public-safe facts used for inspection/search/details without becoming gameplay authority:

- NPC placement, services, shop offers, travel destinations and public quest relations;
- monster definitions, spawn/encounter presentation and public loot information;
- POIs, towns, houses, waypoints and public encounter/event definitions;
- typed public interaction/conversation read models where Game explicitly exports them;
- search-source records and semantic cross-links.

### 3. Player context projection

Authenticated owner-private or explicitly shared data used by Platform `PlayerCompanion`:

- selected character context;
- quest/access/bestiary/bounty progress when an authoritative projection exists;
- saved hunt preferences and goals;
- private session history/derived metrics;
- personalized recommendations and explanations.

This plane is private by default and MUST NOT be embedded into public/shared caches as if it were public map data.

### 4. Authorized live-state projection

Optional future live overlays from authoritative runtime state through an explicit allowlisted bridge:

- the owner’s own current position/state when authorized;
- party/accepted-contact location only under the applicable consent/privacy contract;
- approved dynamic entities/state such as public events or explicitly disclosed world signals;
- dynamic door/tile/field state needed for an authorized view;
- privacy-safe aggregate occupancy/activity rather than public exact player tracking.

Live state is never persisted back into immutable static chunks as the current authored map.

## Target data flow

```text
legacy/reference inputs
        |
        v
Oteryn-Game bounded importer / native authoring
        |
        v
Canonical Oteryn World/Content Model
        |
        +--> Game runtime / World Bundle / Studio
        |
        +--> Game-owned public-safe Atlas Projection
                    |
                    v
          immutable versioned artifacts
                    |
                    v
              Oteryn-Atlas
       validate -> index -> cache -> render
                    |
                    +--> static map + semantic knowledge
                    |
                    +<-- authorized live projection (future)
                    |
                    +<--> Platform PlayerCompanion composition
                          through explicit read/command contracts
```

The primary Game -> Atlas world-data path remains artifact-first. A future live bridge is a separate bounded dynamic-state contract and must not redefine the immutable export as a synchronous Game Server API.

## Immutable chunk and manifest model

The semantic design distinguishes **logical location** from **content identity**.

Conceptually:

```text
LogicalChunkKey(world/export/floor/chunkX/chunkY)
        -> ContentHash
        -> immutable chunk bytes
```

A manifest maps logical chunk keys to immutable content identities and binds all derived datasets to one coherent export revision.

The future physical profile must carry semantic equivalents of:

- export/contract/schema revision;
- world/content/map/ruleset compatibility identities where applicable;
- producer revision and source provenance;
- coordinate profile;
- required/optional capabilities;
- chunk/object identities or index roots;
- asset-catalog identity;
- navigation/knowledge index identities where separately packaged;
- root/artifact digest.

Canonical hashing requires canonical physical encoding rules. No hash algorithm or serializer is frozen here.

### Chunk size remains evidence-gated

`32x32` and `64x64` remain candidates. The current Oteryn-v2 physical-profile spike shows useful locality/size trade-offs but explicitly does not select a permanent size. DYN-ATLAS MUST preserve this gate.

## Stable identity and sprites

Semantic identity and pixel identity are different domains.

- canonical Game entities use Game-owned stable identities/keys;
- placement/export record identity is deterministic and producer-owned when independent addressability is needed;
- legacy numeric IDs and build-local compact IDs are mappings, not universal semantic identity;
- identical decoded sprite pixels may be content-deduplicated without collapsing distinct semantic item/appearance identities;
- GPU texture pages/atlases/arrays are runtime caches and MUST NOT become semantic identity.

A practical sprite pipeline should allow many semantic appearances/frames to reference the same content-addressed decoded pixel blob while preserving dimensions, layers, animation phases, displacement, patterns and other presentation metadata.

## Tile stack and rendering semantics

Same-position ordering is a contract concern, not a browser sorting convenience. Atlas must consume explicit Game-owned coordinate/floor/stack/layer semantics once the canonical coordinate profile exists.

The renderer MUST NOT infer final ordering from legacy numeric IDs, arbitrary JSON order or a local heuristic that can diverge from Game/Studio presentation semantics.

## Browser rendering technology direction

### Recommended implementation candidate

For the browser Atlas, the current preferred proof stack is:

```text
Svelte 5 + TypeScript
        |
        v
PixiJS 8 scene/runtime
        |
        v
WebGL2 production baseline
WebGPU optional later backend after measured compatibility/performance evidence
```

Rationale:

- the workload is a large interactive 2D sprite scene rather than a geographic Mercator map;
- semantic picking, batching, sprite/texture lifecycle and dynamic overlays are first-class needs;
- the UI needs normal web composition for panels, search, NPC/shop details, hunt recommendations and authenticated PlayerCompanion surfaces;
- keeping the Atlas browser renderer independent from canonical Game Rust crates preserves repository/release boundaries.

MapLibre/Leaflet are not recommended as the primary scene engine because the world is not a geospatial/Web-Mercator product. Phaser adds game-engine lifecycle/physics abstractions that Atlas does not require. Three.js is not preferred for the main 2D sprite workload. A custom Rust/WASM renderer remains possible only if later benchmarks prove a material advantage that justifies browser/UI integration cost.

**This technology direction is a programme recommendation, not an accepted permanent framework/serializer contract.** DYN-ATLAS-001 must measure it before a later durable freeze.

### Worker model

Chunk decode, indexing preparation and expensive route/search computations should be worker-capable. The first proof should keep the design replaceable and avoid making a specific worker topology part of the public Game -> Atlas contract.

## Physical encoding decision

**DECISION_DEFERRED.**

Do not freeze FlatBuffers, Protobuf, JSON, JSONL, compression, archive/container format or a custom binary profile from this document.

Current candidates include:

- canonical JSON / record-oriented JSONL as debuggable baselines;
- a schema-defined binary format such as FlatBuffers as a strong browser-oriented candidate;
- other bounded schema formats if they outperform on evolution, safety, tooling and browser decode cost.

The decision requires the missing Game-owned canonical spatial/coordinate profile plus a representative real-world proof, not synthetic size alone.

## NPC semantic model

When Game classifies the fields public-safe, Atlas should be able to consume a typed NPC read model equivalent to:

```text
NPC
  stable identity
  public name/presentation
  position/placement
  services
  shop offers
  travel destinations
  public quest/access relations
  public conversation graph/read model
  provenance + content revision
```

Shop offers must reference stable item/currency semantics and distinguish buy/sell direction. This enables both `NPC -> what is sold/bought` and `item -> which NPC buys/sells it` queries.

Atlas may simulate an informational conversation over a public exported conversation graph. A **live authoritative NPC conversation** is a later authenticated game interaction and requires a separate command/session contract.

## Monster and loot knowledge

Atlas should support public-safe monster knowledge linked to world positions/encounters and PlayerCompanion hunt workflows.

Potential public fields include only those explicitly permitted by the Game export policy:

- name/presentation;
- public combat profile/resistances where allowed;
- spawn/encounter presentation;
- public loot entries/value hints;
- links to areas/routes/NPC resale information.

Exact server-side loot tables or hidden mechanics MUST NOT be exposed merely because Game can calculate them. A future Game policy may classify loot knowledge by disclosure level; this document does not grant public visibility.

## Hunt Intelligence integration

`PlayerCompanion.HuntAdvisor` remains the Platform owner of personalized hunt ranking/orchestration. Atlas supplies spatial/knowledge presentation and route context; it does not become the canonical owner of recommendation logic.

A future recommendation may combine, when each source exists and is authorized:

- deterministic eligibility from authoritative GameCatalog/content/ruleset facts;
- map/area/spawn/navigation facts from the Atlas projection;
- character level/vocation/build/access context from authorized player projections;
- measured GameAnalytics aggregates such as XP/h, profit/h, death/risk, supply use and spawn utilization;
- privacy-safe occupancy/activity signals;
- user preferences and explicit goals;
- editorial or non-authoritative reference evidence with visible limitations.

Every recommendation must preserve evidence class, revisions, freshness, sample/exposure quality where relevant and a human-readable explanation of why the hunt was ranked.

A useful target UI is not merely `level -> hunt`, but an evidence-backed recommendation such as:

```text
Hunt score
expected XP range
expected profit range
risk/difficulty
recent occupancy class
travel/access burden
party suitability
sample/evidence quality
why recommended / limitations
```

Analytics remains observational. It cannot autonomously change gameplay, spawn rates, rewards or balance.

## Bounty/task integration

No separate authoritative Bounty contract is frozen by this Platform document.

The target ownership model is:

```text
BountyDefinition / TaskDefinition
  -> Game-owned versioned content/rules

CharacterBountyState / CharacterTaskState
  -> authoritative Game/player durable state

Atlas + PlayerCompanion
  -> display, route, planning, recommendation and progress projection
```

Atlas may show available/active objectives, associated monsters/areas, route suggestions and predicted hunt economics when an authorized projection exists.

A browser action such as **start/accept/claim/cancel bounty** is a gameplay mutation. It MUST NOT mutate Atlas or Platform read models as the source of truth and MUST NOT be tunneled through Game Gateway merely because Gateway already exists. A future explicit, authenticated, idempotent Player Companion / Game command contract is required before web gameplay mutations are enabled.

## Authentication and player context

Atlas must reuse Platform Identity/authentication. It must not create another password store, account authority or game credential flow.

Authenticated Atlas/Companion composition may select an authorized character context and expose owner-private workflows without changing Game session authority.

## Player positions and privacy

Exact live placement is sensitive.

Target rules:

- the authenticated owner may receive their own exact position when an explicit live projection permits it;
- party members or mutually accepted contacts may receive exact/coarse placement only under the applicable consent/privacy policy;
- ordinary public viewers MUST NOT receive exact live player positions merely because they are online on the same world;
- guild/world/chat membership alone does not grant exact placement;
- public map activity should prefer privacy-safe aggregate classes/heatmaps with minimum-sample/suppression rules rather than exact player dots;
- stale authorization/presence must fail toward less disclosure.

The server/producer must enforce privacy before transmission; hiding already-delivered coordinates in browser UI is insufficient.

## Live-state bridge

The future live bridge is a dedicated public-safe semantic projection, not raw `protocol-oteryn` or Canary/Tibia protocol exposure to the browser.

Conceptually:

```text
Game authoritative runtime
   -> visibility/policy filter
   -> Atlas Live Projection
   -> authenticated/public gateway
   -> browser dynamic overlay
```

A later contract should bind at least:

- protocol/schema revision;
- world/channel/instance scope as appropriate;
- world/content/export revision compatibility;
- server epoch/incarnation;
- ordered snapshot/delta sequence;
- visibility/privacy class;
- resync semantics after gaps.

WebSocket is the preferred first browser transport candidate. WebTransport may be evaluated later for high-rate transient data, but neither transport changes live-state authority.

## Static and dynamic composition

The runtime view is conceptually:

```text
EffectiveAtlasScene
  = ImmutableStaticProjection(export_digest)
  + SemanticKnowledgeProjection
  + AuthorizedDynamicSnapshot(epoch)
  + OrderedAuthorizedDeltas(sequence)
  + OwnerPrivateCompanionOverlay
```

Dynamic state never rewrites immutable chunk bytes in place.

## Pathfinding and routing

Atlas routing is advisory and must be built from versioned static navigation plus optional authorized dynamic overlay.

Static data may include:

- walkability/pathability/movement cost;
- stairs/ladders/teleports/floor transitions;
- public door/zone definitions;
- chunk/region boundary connectivity.

Dynamic overlays may include authorized open/closed doors, temporary blocking fields or other state that materially changes a route.

For large worlds the preferred architecture is hierarchical: local/intra-chunk navigation plus cross-chunk/floor transition graph, with local refinement. Exact algorithm/heuristics remain benchmark-driven.

The Game Server remains authoritative for legal movement.

## Interaction IR

A future public Interaction Read Model should be typed enough for search/inspection without pretending arbitrary scripts were fully understood.

Candidate semantic families include triggers, guards, actions and targets. Unsupported or script-defined semantics must remain explicit as opaque/unsupported/unknown evidence, never silently guessed.

Examples include use/use-with/look/step/talk triggers; storage/quest/item/level/zone guards; teleport/transform/message/open-door/conversation/trade actions.

The canonical interaction definition stays Game-owned. Atlas consumes a bounded public read model.

## Oteryn Studio and editor direction

Do not build a second canonical “Atlas editor”.

The long-term goal is to reuse proven Atlas rendering/semantic-inspection components as a viewport/preview capability in Oteryn Studio while the write path remains:

```text
Studio authoring
 -> canonical World Project / semantic commands
 -> Game validation/compiler
 -> new immutable World/Atlas revisions
```

Atlas publication artifacts are never the canonical authoring source.

## DYN-ATLAS-001 boundary

The first proof is intentionally static and bounded: **Semantic Thais Z7**.

It MUST prove the architecture seam before broad product features. In scope:

- reproducible bounded Thais Z7 fixture/bounds;
- stable semantic tile/object/appearance references;
- explicit same-position ordering;
- immutable logical-chunk -> content-hash mapping;
- sprite pixel deduplication without semantic-ID collapse;
- browser scene rendering from an Atlas projection rather than OTBM parsing;
- semantic picking/tile inspector with provenance;
- deterministic static navigation fixture;
- visual/reference parity evidence;
- baseline size/decode/upload/draw/FPS/VRAM measurements sufficient to compare physical/render candidates.

Out of scope for DYN-ATLAS-001:

- live player positions;
- live bridge/runtime mutation;
- bounty start/claim commands;
- full NPC conversation extraction;
- full Interaction IR coverage;
- production hunt recommendation engine;
- full-map conversion;
- raster retirement;
- editor write path;
- permanent serializer/chunk-size/framework freeze beyond the measured proof.

## DYN-ATLAS-001 minimum acceptance criteria

1. Identical pinned semantic input + compiler/profile produces identical declared manifest/chunk identities.
2. Thais Z7 bounds are explicit and reproducible.
3. Tile/object/reference counts reconcile with zero silent drops.
4. Same-position stack/order semantics are preserved or explicitly blocked by the missing canonical coordinate-profile gate.
5. One local source change invalidates only the expected affected immutable artifact dependencies plus root/manifest identity.
6. Pixel deduplication does not collapse distinct semantic identities.
7. Browser runtime consumes Atlas projection bytes, not OTBM/legacy Game files.
8. Inspector exposes coordinates/floor, ordered stack, semantic identities, flags/refs and provenance available to the projection.
9. Unsupported semantics produce diagnostics, not silent fallback.
10. One deterministic static navigation fixture is reproducible.
11. Representative visual/reference frames have an explicit parity method and named evidence.
12. Baseline metrics are recorded without inventing production thresholds.
13. Physical serializer and permanent chunk size remain deferred unless the proof plus missing upstream Game authority satisfies the owning decision gate.

## Technology decision ledger

| Concern | Current programme disposition |
|---|---|
| Browser UI | `RECOMMENDED_CANDIDATE`: Svelte 5 + TypeScript |
| Browser scene renderer | `RECOMMENDED_CANDIDATE`: PixiJS 8 |
| Production GPU baseline | `RECOMMENDED_CANDIDATE`: WebGL2 |
| Future GPU backend | `EVALUATE_LATER`: WebGPU |
| Game world/compiler/export | `UPSTREAM_AUTHORITY`: Oteryn-Game / Rust |
| Platform personalized workflows | `CURRENT_AUTHORITY`: PlayerCompanion / Laravel modular monolith |
| Game -> Atlas transport | `CURRENT_AUTHORITY`: immutable artifact-first snapshot |
| Live browser transport | `RECOMMENDED_CANDIDATE`: WebSocket first; later evaluate WebTransport |
| Physical serializer | `DECISION_DEFERRED`: benchmark candidates; FlatBuffers not frozen |
| Compression/container | `DECISION_DEFERRED` |
| Chunk size/floor packing | `DECISION_DEFERRED`: 32x32/64x64 and packing require evidence |
| Canonical coordinate/floor/stack profile | `UPSTREAM_BLOCKER`: Game-owned profile required |
| Exact public loot/NPC/interactions fields | `GAME_ALLOWLIST_REQUIRED` |
| Exact public player placement | `PRIVACY_DENY_BY_DEFAULT` |

## Security and failure invariants

- public-safe export and live state are default deny;
- missing/ambiguous facts remain missing/ambiguous rather than reconstructed from legacy sources;
- owner-private PlayerCompanion state never becomes public through composition/cache reuse;
- unsupported required schema/capability fails closed;
- malformed/oversized Atlas artifacts fail validation before partial publication;
- Atlas outage/failure must not make Platform Identity/Accounts/GameAuth unavailable;
- previous validated immutable Atlas publication remains rollbackable;
- no browser map action becomes gameplay authority without an explicit command contract.

## Open decisions / evidence gaps

- canonical Game spatial/coordinate/floor/stack/anchor profile v1;
- first executable Game -> Atlas physical serialization/profile;
- production resource limits and decompression bounds;
- permanent chunk/floor packing profile;
- exact public loot/NPC/interaction disclosure policy;
- authoritative character progress projections needed for bounty/quest/access UI;
- concrete GameAnalytics producer event coverage needed for decision-grade HuntAdvisor metrics;
- live Atlas projection schema and privacy contract;
- web-to-Game gameplay-command boundary, if bounty or other mutations are later enabled;
- exact future Oteryn-Atlas repository/execution coordinates if physical extraction is not yet complete.

## Acceptance invariant

Future work conforms to this design only when:

> The main Atlas evolves into a semantic, interactive player companion without becoming a second Game authority: static world/knowledge arrives through immutable Game-owned public-safe exports; personalized hunt/bounty workflows remain Platform PlayerCompanion concerns; live state is separately authorized and privacy-filtered; and implementation technology remains evidence-gated where current Game contracts deliberately defer it.
