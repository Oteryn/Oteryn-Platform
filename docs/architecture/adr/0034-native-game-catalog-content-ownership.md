# ADR 0034 — Native Game Catalog content ownership

## Status

Accepted — 2026-08-13

- Decision owner: repository owner
- Decision Issue: #1033
- Applies to: native game-content authority, Platform GameCatalog projections and Legacy Canary Compatibility importers
- Does not authorize: runtime implementation, database migration, external-repository access or writes, deployment, production activation, or final external transport/IDL bytes

## Context

The delivered Game Catalog consumes immutable Canary snapshots. Canary final registries and the `oteryn.game-catalog` schemas are valid current compatibility evidence, but ADR 0031 forbids new native capabilities from inheriting Canary identifiers, persistence shape or loader semantics as their target domain model.

Without a durable split, Platform snapshot ownership could be mistaken for gameplay-content authority, or a legacy importer could silently become the native producer contract.

## Decision

1. **Oteryn-v2 game-domain authority owns native gameplay content truth.** This includes canonical native content identity, executable definitions and relations that affect gameplay, source revision/epoch, ruleset/profile applicability, availability and authoritative deletion or replacement semantics.
2. **Platform `GameCatalog` owns the catalogue lifecycle, not executable gameplay truth.** Platform validates, persists and activates immutable imported snapshots; maintains profiles and rebuildable projections; owns editorial presentation/localization and catalogue-to-Wiki links; and exposes explicit provenance, completeness, freshness and degraded state.
3. **Legacy Canary Compatibility importers remain anti-corruption adapters.** Supported Canary schemas `1.0.0`–`1.2.0`, numeric/runtime identifiers and final-registry assumptions remain scoped to compatibility snapshots. Proposed schema `1.3.0` remains non-authoritative until its separate consumer/producer gate is terminal. None can define native canonical identity, native schema evolution or native producer capability.
4. **No dual authority or field-by-field source blending.** A snapshot has one declared authority profile. Native and Canary records are not merged into an apparently authoritative row. Reviewed editorial metadata may supplement presentation-only fields when field-level provenance and override rules are explicit; it cannot override executable facts.
5. **Native exchange is immutable, versioned and fail closed.** A later producer contract must bind contract/schema version, snapshot identity, content authority revision/epoch, ruleset/profile, deterministic digest, completeness/capabilities, tombstones, and stable typed entity/relation identities. Exact bytes, transport and producer implementation remain separately owned.
6. **Activation is transactional and reversible.** Import is inactive by default. Validation, expected inventory/capability checks and compatibility gates precede atomic activation. Rollback selects an earlier validated snapshot without mutating snapshot bytes or lowering accepted authority revision silently.
7. **Migration is consumer-first and authority-explicit.** Native support may run in shadow/inactive comparison with Canary compatibility. Cutover changes the declared authority for an exact profile only after mixed-version and rollback evidence. It does not dual-write or silently fall back from unavailable native authority to Canary.

## Consequences

- Current programme #330 and draft PR #338 remain the Canary compatibility track and are not invalidated.
- Platform can reuse its immutable snapshot/activation machinery for native content without treating imported rows as game-owned truth.
- Native content identifiers and revisions require an external producer contract before implementation; Platform must not invent them from Canary keys.
- Unknown, stale, incomplete, conflicting or unsupported evidence remains unavailable for authoritative claims and production activation.

## Rejected alternatives

- **Make Platform the native content authority:** rejected because Platform does not execute gameplay definitions and cannot prove runtime applicability.
- **Promote Canary schemas to the native domain model:** rejected because compatibility layout and identifiers would become permanent coupling.
- **Merge native, Canary and editorial fields opportunistically:** rejected because provenance and rollback become ambiguous.
- **Read game persistence directly:** rejected because shared-schema coupling bypasses producer validation and authority revisions.

## Related records

- ADR 0016 — versioned immutable Game Catalog snapshots
- ADR 0018 — explicit unknown verified boundary
- ADR 0031 — native Oteryn-v2 integration and Legacy Canary Compatibility boundary
- `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`
- `docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md`
