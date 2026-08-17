# DYN-ATLAS-001 — Semantic Thais Z7 Proof execution prompt

Use this as a **thin execution overlay**. Do not duplicate or reinterpret the canonical architecture/programme inside the task.

## Canonical reads

Before work, read the live repository governance and then:

1. `docs/architecture/oteryn-dynamic-semantic-atlas.md`;
2. `docs/maps/oteryn-dynamic-semantic-atlas-program.md`;
3. the active task/checkpoint for the concrete DYN-ATLAS-001 implementation;
4. Platform ADR 0041 and any later accepted supersession that governs Game/Atlas authority;
5. current Game-owned Atlas export/coordinate/physical-profile contracts required by the proof.

If the accepted target implementation repository is not available/authorized, **do not implement Atlas runtime inside Oteryn-Platform as a substitute**. Record the exact repository/contract blocker and stop that implementation task truthfully.

## Mission

Implement only the bounded **Semantic Thais Z7 Proof** needed to validate the Dynamic Semantic Atlas seam.

The proof must show that a browser renderer can consume a deterministic semantic Atlas projection with stable identities, explicit ordering, immutable chunk/content identity, sprite deduplication, semantic picking and deterministic static navigation without parsing OTBM or adopting a second canonical World model.

## Hard constraints

- Preserve the existing main Atlas product assumptions and visual/reference baseline.
- Game remains canonical World/Content authority.
- Atlas runtime model is a derived semantic projection/read model only.
- Do not read OTBM/Legacy IR/Canary/Crystal files in the browser runtime as a fallback source of truth.
- Do not invent canonical coordinate/floor/stack semantics from Tibia/OTBM conventions if the owning Game profile is still missing.
- Do not freeze FlatBuffers, Protobuf, JSON/JSONL, compression, permanent chunk size or floor packing merely because one candidate works.
- Svelte 5 + TypeScript + PixiJS 8 + WebGL2 is the preferred browser proof candidate, not an irreversible architecture decision.
- Do not implement live players, live bridge, production analytics, bounty mutations, full NPC conversation extraction, full Interaction IR, editor writes, full-map conversion or raster retirement.
- Do not create a second authoritative item/NPC/monster/loot database in Atlas.
- No production/staging/deployment/DNS/repository-migration mutation unless a separate task explicitly authorizes it.
- No owner-funded Codex/OpenAI/API usage unless explicitly authorized for that exact use.

## Required preflight

1. Verify exact repository, default-branch head, task branch, active tasks and open PR ownership.
2. Read governing AGENTS/routing/checkpoint rules.
3. Verify the exact current Game -> Atlas export authority and whether canonical spatial/coordinate semantics required by the proof are accepted or still an evidence gap.
4. Pin every external fixture/source by immutable revision/digest.
5. Define the exact Thais Z7 bounding box/selection rule from evidence; do not guess coordinates.
6. Confirm asset rights/provenance before committing any sprite/appearance fixture.
7. Record `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT`; missing evidence stays `UNKNOWN`.

## Implementation shape

Keep the proof replaceable behind semantic boundaries.

Conceptual pipeline:

```text
pinned semantic/static source
 -> bounded projection/compiler adapter
 -> deterministic Atlas manifest/chunks
 -> asset/sprite catalog + deduplicated pixel blobs
 -> browser consumer validation
 -> scene decode/preparation
 -> PixiJS/WebGL2 render candidate
 -> semantic picking/inspector
 -> deterministic static navigation fixture
```

Do not let the physical encoding dictate canonical domain types.

## Required proof content

### Semantic scene

For the bounded Thais Z7 slice, preserve:

- x/y/floor semantics supplied by the owning profile;
- ordered same-position stack/layer semantics;
- stable semantic content/entity references;
- presentation/appearance references;
- provenance sufficient to diagnose source conversion;
- explicit unsupported/unknown records.

### Immutable chunks

Separate logical address from content identity. A local semantic change must change only the expected affected data artifact(s) and aggregate/root manifest identity, not unrelated chunks.

The chosen proof packaging must be deterministic for identical pinned inputs and declared toolchain/profile.

### Sprites

Deduplicate identical decoded pixel content without collapsing semantic appearance/item identity. Preserve metadata required for multi-tile, layer, frame/animation, pattern and displacement/anchor semantics that the proof actually exercises.

GPU page/atlas placement is runtime cache state, not persistent semantic identity.

### Browser renderer

The browser must render the projection rather than parse the legacy map format.

The proof should exercise:

- floor switching within the bounded slice where applicable;
- pan/zoom/deep-link state needed by the test harness;
- batched sprite rendering;
- bounded texture lifecycle;
- selection/hover semantic picking;
- stable inspector output.

### Inspector

At minimum, when the upstream profile supports the facts, expose:

- world position/floor;
- ordered visible/static stack;
- semantic identities/keys;
- relevant public flags/refs;
- appearance/sprite content refs;
- provenance/export revision;
- unsupported/ambiguous state.

### Static navigation

Provide one deterministic route fixture using only static public navigation semantics. The result is advisory; do not claim Game Server movement authority.

## Candidate physical-format comparison

The proof must remain able to compare at least a debuggable canonical text baseline with the selected binary candidate if practical. FlatBuffers may be evaluated as a strong candidate but is not accepted by this prompt.

Record for each tested profile as applicable:

- raw/compressed bytes;
- file/chunk count;
- point/viewport fetch bytes;
- decode/preparation time;
- malformed input behavior;
- local edit invalidation;
- browser implementation/tooling cost observations.

Do not select a winner solely from total compressed size.

## Required measurements

Record exact commands/harness revision and representative environment for:

- input/output bytes;
- semantic record/tile/object counts;
- chunk/artifact counts;
- unique semantic appearances;
- unique deduplicated pixel blobs;
- decode/scene-preparation time;
- GPU upload/texture memory estimate;
- draw calls/batches;
- representative frame time/FPS;
- point/viewport network bytes under tested packaging;
- local edit changed-artifact count;
- diagnostic counts for unsupported/unknown semantics.

Measurements are baseline evidence, not invented production SLOs.

## Acceptance criteria

The task is complete only when all applicable items are proven on the exact final head:

1. same pinned input + compiler/profile/toolchain produces identical declared manifest/chunk identities;
2. Thais Z7 selection/bounds are explicit and reproducible;
3. tile/object/reference counts reconcile with zero silent drops;
4. same-position ordering is explicit and preserved, or the proof stops on the named upstream coordinate/order blocker rather than inventing it;
5. one local change invalidates only expected affected artifact dependencies plus root/manifest identity;
6. identical pixels are deduplicated without semantic identity loss;
7. browser consumes Atlas projection bytes and does not parse OTBM/legacy Game files;
8. tile inspector exposes the proof’s supported semantic identity/provenance fields;
9. unsupported semantics emit deterministic diagnostics;
10. static navigation fixture is deterministic;
11. representative visual/reference parity evidence is named and reproducible;
12. performance/size/locality baseline is recorded with no fabricated threshold;
13. no out-of-scope live/bounty/editor/full-map/raster-cutover work is present;
14. permanent serializer/chunk-size/framework decisions remain deferred unless their separate owning evidence gate has actually been satisfied.

## Validation

Select validation from the actual repository’s current build/test matrix. At minimum:

- schema/contract/fixture validation for generated semantic data;
- deterministic rebuild comparison;
- malformed/oversized/unsupported negative cases appropriate to the implemented parser;
- focused renderer/browser test for Thais Z7;
- exact changed-path review;
- full-diff self-review;
- required exact-head CI on the final candidate;
- no claim of production performance, full-map compatibility or cross-repository runtime compatibility without named evidence.

## Required durable evidence

The implementation task/checkpoint or linked evidence record must retain:

```text
repository + branch + PR
exact final head
base revision
pinned upstream contract/source revisions
fixture/source digests
Thais Z7 selection rule
physical/render candidate(s) tested
validation commands and outcomes
measurement artifact/report identity
known unsupported semantics
PROVEN / DERIVED / UNKNOWN / CONFLICT
exact next action if any blocker remains
```

## Stop conditions

Stop without workaround if:

- required canonical coordinate/floor/stack semantics are still unavailable and the proof cannot truthfully preserve ordering;
- fixture licensing/provenance is unresolved;
- implementing the proof would require writing an unauthorized repository;
- an overlapping task/PR owns the same paths/contracts;
- a proposed shortcut would expose server-only/private state;
- the only path to readiness would bypass repository merge/review/CI governance.

A partial visual demo is not completion if the semantic/determinism/authority gates are not proven.
