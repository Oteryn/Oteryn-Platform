# ADR 0042 — Non-native reference-content boundary

## Status

Accepted — 2026-08-16

- Decision owner: repository owner
- Decision Issue: #1121
- Decision PR: #1122
- Applies to: provenance-pinned, non-native, non-executable reference content used for offline reconciliation, structured Wiki reference and PlayerCompanion inputs
- Does not authorize: Oteryn-v2 or Canary access, runtime implementation, database migration, deployment, production/staging mutation, native GameCatalog activation, external producer implementation, or publication of third-party prose/dialogue/maps/media

## Context

ADR 0034 already settles the authoritative game-content boundary: Oteryn-v2 owns native gameplay-content truth and Platform `GameCatalog` owns validated immutable catalogue lifecycle/projections rather than executable game truth. Legacy Canary Compatibility remains a separate anti-corruption boundary.

The content-completion programme has an owner-supplied `crystalserver-main.zip` archive pinned by SHA-256 `920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26`. The archive contains useful structured and transformable source material, but neither possession of the archive nor its root license text makes it native Oteryn authority, Legacy Canary Compatibility authority, current-runtime evidence, or a blanket publication-rights grant.

Issue #1121 therefore asks only whether a third source class may exist as explicitly non-authoritative reference evidence for research and player-facing reference/tooling without weakening ADR 0034.

## Options considered

### Option A — reject or defer all durable third-party reference use

Keep the archive as ad hoc audit material only. This minimizes new architecture, but forces every Wiki/tool/reconciliation task to reinvent provenance and fail-closed rules and leaves no durable distinction between reviewed reference facts and arbitrary research notes.

### Option B — add a reference-only `GameCatalog` authority/profile

Reuse the existing immutable snapshot/profile machinery but mark the source non-authoritative. This appears operationally convenient, but `GameCatalog` profile activation, visibility and authority semantics are deliberately tied to native or explicit Legacy Canary Compatibility source profiles. A third profile class creates avoidable risk of accidental fallback, mixed authority and public presentation as current game truth.

### Option C — separate `ReferenceContent` read-model boundary

Create a separate logical Platform boundary for immutable, provenance-pinned, non-executable reference snapshots. It may reuse generic implementation primitives later, but it does not share `GameCatalog` authority-profile activation semantics. Consumers must opt in and preserve the reference classification.

## Decision

The repository owner accepted **Option C** by instructing Architecture Review to resolve Issue #1121 autonomously under the existing authority constraints.

### 1. `ReferenceContent` is a separate logical boundary

`ReferenceContent` owns immutable reference snapshots, deterministic extraction/normalization metadata, review state, source-local identity and consumer-safe reference projections.

It is part of the existing Laravel modular-monolith architecture. This decision does not create a microservice or authorize implementation.

A `ReferenceContent` snapshot is never:

- a native `GameCatalog` authority profile;
- a `legacy-canary` profile;
- an active gameplay-content fallback;
- evidence that source definitions execute in Oteryn-v2;
- evidence of runtime availability, reachability, spawn state, schedule state or current balance applicability.

### 2. Provenance is mandatory and immutable

Every reference snapshot binds at least:

```text
reference_contract_version
reference_snapshot_id
source_kind
source_artifact_name
source_artifact_sha256
source_profile_id
fact_family
source_paths
extractor_id
extractor_version
transformation_digest
extracted_at
coverage_scope
completeness_state
semantic_state
review_state
payload_digest
```

For the supplied CrystalServer archive, `data-global` and `data-crystal` are distinct `source_profile_id` values. They cannot be unioned, overlaid or used as fallback for one another unless a later evidence-backed compatibility decision explicitly permits it.

`extracted_at` records processing time only. It is not evidence that the source is current for Oteryn.

### 3. Reference identity never becomes native identity by inference

Reference entities use source-local, namespaced identity scoped by reference source, pinned artifact, profile and fact family. Names, client IDs, server IDs, paths and hashes are not native Oteryn identities.

A reference-to-authority crosswalk may record `CANDIDATE`, `VERIFIED` or `REJECTED` mapping evidence. `VERIFIED` means the mapping was reconciled against an independently authoritative target identity; the reference source cannot mint that target identity or make itself authoritative.

### 4. Allowed uses are explicit

`ReferenceContent` may be consumed for:

- deterministic **offline comparison/reconciliation** against an independently identified authoritative or compatibility snapshot;
- source-family inventory, transformation validation, gap analysis and migration planning;
- **structured Wiki reference** surfaces that visibly retain source/provenance/reference status and do not claim current/native authority;
- **PlayerCompanion** reference/planning inputs where the result exposes the non-authoritative evidence class and limitations;
- candidate crosswalk generation for later authoritative reconciliation.

Public or user-facing use does not erase the reference label. A consumer cannot copy a reference value into an authoritative `GameCatalog` row merely because the value is useful.

### 5. PlayerCompanion result certainty and source authority remain orthogonal

Existing `DETERMINISTIC | SIMULATION | RECOMMENDATION` result classification remains unchanged. A new source-evidence dimension records `NON_AUTHORITATIVE_REFERENCE` when a result materially depends on `ReferenceContent`.

A calculation may be mechanically reproducible from a reference snapshot, but it must not be presented as a `DETERMINISTIC` current Oteryn gameplay result when its material gameplay rule/parameter is supported only by non-authoritative reference evidence. Such a workflow must either:

- obtain the required authoritative GameCatalog/ruleset evidence; or
- present the output as simulation/reference/recommendation with the pinned source, assumptions and limitations visible.

### 6. Wiki editorial authority remains separate

Wiki may render structured reference facts or links from `ReferenceContent`, but Wiki editorial articles, guides and localized prose remain governed by Wiki publication lifecycle. Reference facts do not become editorial prose automatically.

This ADR does **not** decide rights to republish third-party descriptions, dialogue, maps, images, media or other expressive material. ADR 0026 and `THIRD_PARTY_NOTICES.md` continue to fail closed for unknown/incompatible rights. Bulk copying remains prohibited.

### 7. Precedence and conflict handling are fail closed

For native gameplay-content claims:

1. accepted Oteryn-v2 authority evidence wins;
2. an exact Legacy Canary Compatibility profile remains compatibility evidence only within its contract;
3. `ReferenceContent` remains non-authoritative comparison/reference evidence.

Conflicting reference sources or profiles are retained as separate evidence. Platform does not pick a winner by recency, name match, record count or convenience.

When reference evidence disagrees with native authority, the native fact remains authoritative and the disagreement may be retained as reconciliation evidence. When native authority is absent or unavailable, Platform reports that absence/unavailability; it does not promote reference data as fallback truth.

### 8. Completeness and freshness do not imply authority

Reference completeness is scoped only to the declared artifact/profile/fact family and may be:

- `COMPLETE_FOR_DECLARED_SCOPE`;
- `PARTIAL`;
- `UNKNOWN`.

Missing reference data never proves authoritative absence. A complete extraction of a pinned archive does not prove Oteryn gameplay completeness or currentness.

Semantic review is independently classified. Data requiring transformation or exposing only partial implementation semantics cannot be upgraded merely because parsing succeeded.

### 9. Failure semantics

A reference artifact is rejected or unavailable for the affected consumer when:

- the source artifact hash does not match the pinned manifest;
- profile/path bounds are violated;
- extraction is nondeterministic or transformation identity is missing;
- executable code would need to be evaluated rather than statically parsed;
- required semantics are partial/unknown for the requested use;
- conflicting records cannot be represented without blending;
- consumer policy would hide the non-authoritative status;
- the requested use would require unresolved publication rights.

Failure of native/compatibility authority never triggers automatic `ReferenceContent` fallback.

## Consequences

### Positive

- The content-completion programme can use owner-supplied source material without weakening native authority.
- Wiki and PlayerCompanion gain a durable, testable source classification instead of ad hoc source copying.
- Offline reconciliation can preserve useful disagreements and candidate mappings before native content is available.
- `GameCatalog` authority-profile activation remains semantically narrow and fail closed.

### Negative

- Reference ingestion/projection requires separate persistence/application boundaries if implemented.
- Consumers must carry an additional source-evidence dimension and explicit UI labeling.
- Some attractive tools cannot claim deterministic/current Oteryn results until authoritative content exists.
- Publication-rights review remains a separate gate rather than being solved by this architecture decision.

## Rejected shortcuts

- Rename CrystalServer data as native Oteryn content.
- Reuse the `legacy-canary` authority name or schema semantics for CrystalServer.
- Activate a reference snapshot as an authoritative `GameCatalog` profile.
- Merge native/Canary/reference fields into one apparently authoritative record.
- Infer native identity from matching names, source-local numeric IDs or source paths.
- Treat a successful parser as proof of semantic completeness.
- Execute source Lua or other code to derive facts inside the Platform reference pipeline.
- Treat archive age, Git-like metadata or extraction time as current Oteryn freshness.
- Bulk-publish third-party prose, dialogue, maps, images or media under this decision.

## Delivery handoff

After this ADR is accepted on `main`:

- `SOURCE-PIPELINE` is architecture-`READY` for a bounded deterministic extraction/normalization task targeting `ReferenceContent`, with no automatic publication or GameCatalog activation.
- `WIKI-REFERENCE` is architecture-`READY` for a bounded structured-reference slice that preserves `NON_AUTHORITATIVE_REFERENCE` provenance. Public publication of any material whose rights are not proven must still fail closed under ADR 0026; copied prose/assets remain blocked outside this lane.
- `PLAYER-COMPANION` is architecture-`READY` for one bounded reference-aware vertical slice. A slice that requires current deterministic gameplay truth remains dependent on authoritative GameCatalog evidence rather than reference fallback.

These states resolve the architecture dependency only. `CONTENT-COORD` must still refresh live ownership/path locks and create separate implementation tasks before dispatch.

## Related records

- ADR 0025 — PlayerCompanion boundary
- ADR 0026 — proprietary repository licensing policy
- ADR 0034 — native Game Catalog content ownership
- `docs/contracts/NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- Issue #1115 — OTERYN-CONTENT-COMPLETION
- Issue #1121 — bounded architecture decision
