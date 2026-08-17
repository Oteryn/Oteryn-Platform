# Oteryn Dynamic Semantic Atlas Programme

## Status

**Programme design / execution roadmap.**  
Alias: `DYN-ATLAS`  
Architecture: `docs/architecture/oteryn-dynamic-semantic-atlas.md`

This programme evolves the existing Atlas from raster/static presentation to a semantic, interactive and later player-aware Atlas while preserving the accepted Game/Atlas/Platform authority split.

It does not authorize production deployment, repository migration, Game writes, protected-environment changes or web gameplay mutations.

## Programme principles

1. Preserve the useful behaviour and visual baseline of the existing main Atlas; replace foundations incrementally rather than rebuilding an unrelated map product.
2. Game remains canonical World/Content and mutable gameplay authority.
3. Atlas consumes explicit public-safe projections and owns derived indexes/presentation only.
4. Platform `PlayerCompanion` owns hunt guidance, progress/planning and personalized recommendations.
5. Static immutable world data, semantic knowledge, owner-private context and live dynamic state are separate planes.
6. Public disclosure is default deny; exact player placement is never made public by convenience.
7. No permanent serializer, chunk size or coordinate convention is selected without the owning Game evidence gate.
8. Every phase keeps a rollback/fallback path and names what has actually been proven.
9. Unsupported semantics remain visible diagnostics/unknowns rather than silent approximations.
10. DYN-ATLAS-001 stays small enough to prove the seam before full-map migration.

## Upstream dependencies

Current high-authority inputs include:

- Platform ADR 0041 — Game/Atlas ownership, artifact-first public-safe projection, independent Atlas release/failure domain;
- Platform `PLAYER_COMPANION_ARCHITECTURE.md` — `HuntAdvisor`, `ProgressTracker`, `Recommendations`, private-by-default player workflows;
- Oteryn-v2 ADR-0005 — native World/Content and Studio authority;
- Oteryn-v2 `OTERYN_GAME_ATLAS_EXPORT_CONTRACT_V1.md` — immutable Game-owned Atlas export semantics;
- Oteryn-v2 physical-profile readiness evidence — first physical profile still blocked by missing canonical coordinate/floor/order/anchor authority;
- Oteryn-v2 ANL-02 candidate — nonbinding evidence for future decision-grade hunt/world analytics;
- Oteryn-v2 privacy-first social presence baseline — exact placement is not public by default.

## Target capability families

The programme ultimately enables five product families over one Atlas foundation:

```text
A. World Map
   static semantic scene, floors, search, POI, routing

B. World Knowledge
   NPC, shops, monsters, public loot, spawns, interactions, travel

C. Hunt Intelligence
   deterministic eligibility + analytics + occupancy + player preferences

D. Player Progress / Bounty Companion
   private quest/access/bounty progress, goals, route and hunt planning

E. Authorized Live Atlas
   owner/party/consent-authorized positions and approved dynamic world overlays
```

No phase may implement a later family by violating the authority boundaries of an earlier phase.

## Phase 0 — Architecture contract gate

### Goal

Make the cross-system seams explicit before implementation.

### Required outcomes

- Atlas model named as a derived semantic projection/read model, not canonical world authority;
- immutable-static versus dynamic-live boundary explicit;
- PlayerCompanion versus Atlas responsibility explicit;
- exact Game-owned export contract/reference pinned;
- privacy and public-field allowlist acknowledged;
- physical serializer/chunk-size decision explicitly deferred;
- missing Game canonical coordinate/floor/stack profile recorded as a blocker for final executable physical compatibility.

### Exit gate

No unresolved contradiction with ADR 0041, PlayerCompanion or the current Game-owned export contract.

## Phase 1 — `DYN-ATLAS-001`: Semantic Thais Z7 Proof

### Goal

Prove a bounded static vertical slice using semantic scene data rather than a raster-only or browser-OTBM model.

### Scope

- explicit reproducible Thais Z7 bounds;
- semantic tile/object/appearance identity and provenance;
- ordered stack/layer representation;
- logical chunk -> immutable content identity;
- deduplicated sprite pixel store while preserving semantic appearance identity;
- browser rendering proof using the recommended Svelte/TypeScript + PixiJS/WebGL2 candidate stack unless benchmark preflight justifies another candidate;
- semantic picking/tile inspector;
- deterministic static navigation fixture;
- reference/parity screenshots or equivalent named visual evidence;
- physical/render measurement set.

### Required measurements

Record, without inventing production pass/fail thresholds:

- semantic source and output byte sizes;
- number of logical chunks/artifacts;
- unique semantic appearances versus unique pixel blobs;
- decode time and prepared scene size;
- GPU upload/texture memory estimate;
- draw calls/batches and representative FPS/frame-time;
- point/viewport fetch bytes for candidate packaging;
- local-edit invalidation/churn;
- malformed/unsupported diagnostic behavior.

### Exit gate

All DYN-ATLAS-001 acceptance criteria in the architecture document are satisfied or the task stops with the exact upstream evidence blocker. No full-map rollout follows from a visually successful demo alone.

## Phase 2 — Static Semantic Atlas expansion

### Goal

Generalize the proven static semantic model from Thais Z7 to representative regions/floors and then the full supported world.

### Outcomes

- generalized immutable ingestion/validation;
- bounded indexes and cache policy;
- stable deep links and floor/navigation behavior;
- search by public semantic entities;
- raster/reference and semantic rendering may run side by side;
- rollback remains available.

### Exit gate

Representative regions prove deterministic rebuild, resource bounds, corruption rejection and no silent semantic loss.

## Phase 3 — Production browser streaming/render path

### Goal

Harden browser scene streaming and rendering for practical full-world use.

### Candidate direction

- Svelte/TypeScript application UI;
- PixiJS scene renderer;
- WebGL2 production baseline;
- worker-capable decode/index preparation;
- GPU texture/page cache separate from semantic identity;
- lazy visible-region loading and bounded cache eviction;
- optional WebGPU experiments only behind measured compatibility evidence.

### Exit gate

Measured full/representative-world performance, memory and failure behavior satisfy later accepted production thresholds.

## Phase 4 — World Knowledge: NPC / monster / loot / interaction read models

### Goal

Turn the map into a searchable semantic knowledge surface using only Game-approved public-safe facts.

### Candidate features

- click/search NPC;
- show position, services, shop buy/sell offers and travel destinations;
- reverse item -> NPC seller/buyer lookup;
- monster detail, public spawn/encounter presentation and public loot knowledge;
- typed public interaction inspection;
- informational NPC conversation graph when explicitly exported;
- links among map position, NPC, item, monster, encounter and route.

### Constraints

- no hidden loot tables/scripts merely because they exist upstream;
- no arbitrary Lua interpretation as authoritative fact;
- unsupported/opaque semantics stay explicit;
- Game owns the disclosure allowlist and canonical identities.

## Phase 5 — Static navigation and hunt-area model

### Goal

Provide deterministic advisory routes and reusable hunt-area geometry.

### Outcomes

- navigation index/chunks;
- floor-transition graph;
- local and cross-region route planning;
- route overlays/deep links;
- hunt/encounter areas linked to monsters, access requirements and NPC services.

### Exit gate

Deterministic route fixtures, explicit inaccessible/unknown handling and no claim that Atlas route legality overrides Game Server movement authority.

## Phase 6 — PlayerCompanion Hunt Intelligence

### Goal

Use Atlas as the spatial UI for evidence-backed personalized hunt recommendations while keeping recommendation ownership in Platform `PlayerCompanion`.

### Inputs, when available

- authoritative catalog/ruleset facts;
- Atlas spawn/area/navigation knowledge;
- authorized character level/vocation/build/access context;
- GameAnalytics aggregates for XP/h, profit/h, supplies, risk/deaths, spawn utilization and comparable dimensions;
- privacy-safe occupancy/activity classes;
- user goals/preferences;
- explicitly labelled editorial/non-authoritative reference evidence where allowed.

### Required result semantics

Every displayed recommendation preserves:

- result/evidence classification;
- game/ruleset/content revisions;
- source freshness;
- sample/exposure/quality where analytics are used;
- ranking explanation and limitations.

### Target workflows

- “Where should this character hunt for XP/profit/balance?”
- compare hunts side-by-side;
- show expected ranges rather than fabricated precision;
- show travel/access burden;
- show occupancy/risk where privacy-safe evidence exists;
- open the selected hunt directly on the map with route and relevant NPC/loot links.

## Phase 7 — Player progress and bounty/task companion

### Goal

Overlay player-specific objectives on the same semantic world.

### Candidate features

- active/available bounty/task presentation when authoritative projection exists;
- monster/area objective linking;
- progress and remaining objectives;
- route to relevant hunt/turn-in NPC;
- hunt economics for completing the objective;
- quest/access prerequisites;
- saved goals and reminders through existing Platform boundaries.

### Mutation gate

Viewing/planning may be read-only. Starting, accepting, cancelling or claiming a bounty/task is a gameplay mutation and remains **BLOCKED** until an explicit web/PlayerCompanion -> Game command contract is accepted and implemented with authentication, authorization, idempotency, revision binding and authoritative Game result semantics.

Do not route such mutations through Game Gateway by convenience.

## Phase 8 — Atlas Live Projection contract and bridge

### Goal

Introduce dynamic state without contaminating immutable world artifacts.

### Required contract topics

- visibility/public/privacy classes;
- world/channel/instance scope;
- static export/content compatibility binding;
- server epoch/incarnation;
- full snapshot and ordered deltas;
- gap detection and resync;
- bounded entity/update rates;
- authorization and cache isolation;
- stale authorization fails toward less disclosure.

### Transport candidate

WebSocket is the preferred first browser transport candidate. WebTransport may be evaluated later for transient high-rate data after browser/operations evidence justifies it.

## Phase 9 — Authorized Dynamic Atlas

### Candidate features

- owner’s own live position;
- party/contact positions only when the applicable privacy/consent contract authorizes them;
- approved dynamic doors/fields/events/world-state;
- live route refinement using authorized dynamic obstacles/state;
- privacy-safe aggregate occupancy/heatmap overlays.

### Explicit rejection

A public “all online players as exact dots” feed is not the target default and must not be created from ordinary online-player data.

## Phase 10 — Raster retirement and Studio reuse

### Raster retirement gate

Retire the legacy raster path only after:

- semantic renderer parity/burn-in evidence is sufficient;
- rollback/fallback is proven;
- deep links/floors/search remain compatible;
- full-world resource/performance limits are accepted;
- operational publication/rollback is proven.

### Studio integration direction

Reuse proven Atlas renderer/semantic inspector concepts in Oteryn Studio as a viewport/preview component. Do not create a second Atlas-owned authoring authority.

## Cross-repository delivery rules

- Oteryn-Platform changes require their own Platform task/branch/PR.
- Oteryn-Game/Oteryn-v2 exporter, canonical world or analytics producer changes require separately authorized Game task/branch/PR.
- Future Oteryn-Atlas implementation requires its own repository task/branch/PR once that repository coordinate is available and authorized.
- Cross-repository compatibility is bound by versioned contracts and exact immutable evidence, not simultaneous undocumented edits.
- A Platform documentation merge does not authorize Game or Atlas runtime writes.

## Risk register

### P0

- Atlas accidentally becomes a second canonical World/Content owner.
- Browser/live projection leaks hidden player or server-only state.
- Static and dynamic state are mixed so one invalidates or mutates the other.
- A web PlayerCompanion action becomes gameplay authority without a Game-owned command contract.

### P1

- semantic identity is collapsed into sprite/pixel identity;
- tile/floor/stack ordering is inferred incorrectly;
- arbitrary scripts are silently converted to fake Interaction IR;
- hunt recommendations present biased/incomplete analytics as certainty;
- owner-private PlayerCompanion state leaks through shared cache or public map composition;
- final serializer/chunk size is frozen from synthetic evidence before canonical coordinate semantics exist;
- a separate Atlas editor creates a second authoring source.

### P2

- GPU texture bleed/multi-tile/animation artefacts;
- full-map browser memory pressure;
- excessive chunk/file counts or poor viewport locality;
- premature WebGPU dependency;
- raster path removed before rollback confidence exists.

## Programme completion condition

The programme is complete only when the semantic Atlas is a stable independently releasable derived product, supports the approved knowledge/companion/live capability set through explicit contracts, has production-grade bounded validation and rollback, and no longer depends on legacy world formats as its browser/runtime source of truth.
