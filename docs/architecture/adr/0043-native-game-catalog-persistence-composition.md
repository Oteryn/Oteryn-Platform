# ADR 0043 — Native GameCatalog persistence composition

## Status

Proposed — 2026-10-06

- Decision ID: `ARCH-DEC-0006`
- Decision owner: repository owner
- Decision Issue: #1460
- Programme: `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`
- Applies to: Platform persistence composition for validated `oteryn.game-platform-catalog` native snapshots under ADR 0034
- Does not authorize: runtime implementation, database migration, native snapshot activation/publication, external Game mutation, production/protected-environment work, secrets or merge bypass

## Context

ADR 0034 already fixes the authority split: Oteryn-v2 owns native gameplay-content truth; Platform owns validated immutable GameCatalog snapshot/profile/projection lifecycle; Legacy Canary Compatibility remains an explicit anti-corruption boundary. The accepted `OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md` also requires native snapshot provenance, authority epochs, completeness semantics, tombstones and fail-closed activation.

Platform protected main at the review start (`a834ef54b154c48b119da20c3bdb6fca74178750`) has two materially different persistence contracts:

1. The delivered Legacy Canary Compatibility lifecycle stores Canary-specific facts directly in `game_catalog_snapshots`: `canary_commit_sha`, `protocol_profile`, runtime/content release foreign keys, `appearances_sha256` and release-boundary semantics. `game_catalog_profiles`, `CatalogActivationService`, `PublicCatalogContextResolver`, entity snapshots and typed item/creature payloads also embed Legacy Canary release/runtime assumptions.
2. The integrated native envelope validator from PR #1229 validates locked `oteryn.game-platform-catalog@1.0.0` evidence carrying `content_authority_id`, `authority_epoch`, `source_revision`, `ruleset_id`, `content_profile_id`, capability/completeness manifests, typed entities/relations, tombstones and `payload_digest`, but intentionally persists nothing.

Using dummy Canary values for native evidence would create false provenance. Making legacy fields nullable in place would weaken database invariants while leaving activation, public-read and entity identity assumptions unresolved. A completely separate native lifecycle would avoid legacy columns but duplicate the Platform-owned profile/activation authority that ADR 0034 requires to remain singular.

## Invariants

- No native snapshot may inherit fabricated Canary commit, release, appearance or availability provenance.
- Existing Legacy Canary Compatibility rows and activation behavior must remain semantically unchanged during migration.
- One Platform profile binds to exactly one declared authority source/profile at a time.
- Native partial/unknown/unsupported capabilities never become authoritative absence.
- Native authority epoch/replay fencing must prevent delayed or retired evidence from lowering accepted authority state.
- Persistence of an inactive native candidate does not authorize activation or public publication.
- Platform must not invent Game-owned content identity or mutate Oteryn-Game.

## Options considered

### Option A — generalize the existing Legacy snapshot/profile tables in place

Add an authority discriminator to `game_catalog_snapshots`, make Canary-only columns conditionally nullable, add native provenance columns to the same row and refactor activation/public reads to branch on authority.

Trade-offs:

- **Correctness/security:** one physical table can still represent one lifecycle, but critical validity moves from current NOT NULL/FK guarantees into authority-conditional application checks.
- **Migration:** highest blast radius because existing snapshot, profile, activation, public-read and entity payload assumptions all change together.
- **Operability:** fewer top-level tables, but every query must understand the union of unrelated authority-specific fields.
- **Reversibility:** difficult once native rows exist because restoring strict legacy constraints requires proving and removing all native variants.

### Option B — shared source-neutral lifecycle root plus authority-specific details

Add a small immutable source-neutral snapshot root used by one profile/activation/audit lifecycle. Bind each existing Legacy Canary snapshot 1:1 to that root without weakening its current constraints. Add a separate native authority detail and native authority-scoped entity/relation persistence keyed from the same root.

Trade-offs:

- **Correctness/security:** preserves strict Legacy Canary invariants while giving native evidence its own exact schema and authority fencing.
- **Migration:** additive first; existing profiles can be backfilled to roots and dual-read/equivalence tested before any native persistence is enabled.
- **Operability:** introduces one additional join/dispatch seam, but keeps authority-specific validation localized.
- **Reversibility:** strongest option because legacy behavior can remain on the old pointer until root equivalence is proven; native inactive candidates can be removed/disabled without rewriting legacy rows.
- **Complexity:** requires a controlled transition period with legacy binding, dual pointers and explicit authority-specific projection adapters.

### Option C — create a fully separate native snapshot/profile/activation subsystem

Leave every Legacy Canary table untouched and introduce independent native snapshot, profile, activation, projection and audit tables/services.

Trade-offs:

- **Correctness/security:** clean isolation of data shapes.
- **Delivery:** can be initially simple because no legacy migration is required.
- **Architecture:** duplicates profile selection, activation/rollback and audit authority, creating two co-authoritative Platform catalogue lifecycles contrary to ADR 0034's single lifecycle intent.
- **Operability:** administration, rollback, public projection and incident handling must reconcile two control planes.
- **Long-term coupling:** highest risk of semantic drift between legacy and native activation rules.

## Recommendation — non-authoritative until owner acceptance

Recommend **Option B**.

### Shared snapshot root

A new immutable root (working name `game_catalog_snapshot_roots`) should contain only source-neutral lifecycle facts such as root identity, declared authority kind/profile, contract/schema identity, artifact digest, generated/imported timestamps, lifecycle state, counts and validation-summary linkage. It must not contain Canary release ids or native authority-epoch fields.

### Legacy binding

Every existing `game_catalog_snapshots.id` should gain exactly one 1:1 binding to a shared root. Existing Legacy Canary columns, foreign keys and validation remain strict and unchanged. The migration must backfill all historical snapshots before any legacy profile switches to the root pointer.

### Native detail and payload family

A 1:1 native detail keyed by the shared root should persist at least `content_authority_id`, `authority_epoch`, `source_revision`, `ruleset_id`, `content_profile_id`, `payload_digest`, capability manifest, completeness manifest and exact producer-contract identity. Tombstones remain first-class evidence.

Native entity identity must be authority-scoped (declared authority + type + native content key). The first native persistence slice should not reuse the globally unique Legacy `canonical_key` namespace or legacy release/range/runtime-present payload assumptions.

### One profile and activation lifecycle

`game_catalog_profiles` should evolve additively to bind a declared authority and an `active_snapshot_root_id`. Existing `active_snapshot_id` and target-release semantics remain authoritative for legacy profiles during migration and must agree 1:1 with the root binding before cutover.

A generalized activation transaction should lock the profile, require authority match, invoke authority-specific validation/projection rules, atomically switch the shared root pointer and write one common audit event. No cross-authority fallback is allowed.

### Native epoch/replay fence

Native profiles require durable accepted authority epoch/high-water state separate from Canary release ordering. Ordinary rollback may select only retained snapshots allowed by the current accepted epoch. Re-entry into a retired epoch requires an explicit audited recovery decision; delayed artifacts never lower accepted authority state.

### Public/admin projection boundary

Public and admin reads should ultimately consume profile projections bound to the shared root. Legacy read code may remain an authority-specific adapter during migration. Native inactive-candidate persistence must not automatically create a public projection.

## Migration and rollback

If Option B is accepted, the first implementation package is intentionally limited to:

1. add shared root, Legacy 1:1 binding and native detail/identity/relation persistence;
2. backfill exactly one root per existing Legacy Canary snapshot;
3. prove row counts, hashes, active-profile mapping and legacy public output are unchanged;
4. add/backfill `active_snapshot_root_id` while retaining the legacy active pointer;
5. prove dual-read equivalence for Legacy Canary;
6. enable persistence of validated native snapshots only as inactive candidates;
7. stop before native activation/public publication.

Rollback remains additive while only legacy bindings exist. Once native evidence is persisted, a destructive down-migration must fail closed unless an explicit audited evidence-removal/migration plan exists.

## Rejected shortcuts

- Dummy Canary revision/release/appearance values for native snapshots.
- A nullable union that relies only on application code to enforce authority-specific invariants.
- A native sidecar whose parent is still a fabricated Legacy Canary snapshot row.
- A second co-authoritative native profile/activation subsystem.
- Importing raw Game `content/manifest.json` as the locked cross-repository export envelope.
- Reusing globally unique Legacy canonical keys for native identity without authority scope.
- Inferring tombstones or authoritative absence from partial/unknown capability payloads.

## Owner decision required

Select **Option A, B or C** for the durable persistence composition. The recommendation is Option B. Until that choice is explicitly accepted, this ADR remains Proposed and grants no implementation authority.

## Conditional implementation handoff

If Option B is accepted, the first remediation Issue/task owns only additive schema, Legacy backfill/equivalence and native inactive-candidate persistence. Activation, publication, Game capability-adapter work, production transport and protected-environment actions remain separate tasks/gates.

## Related records

- ADR 0034 — native Game Catalog content ownership
- `docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md`
- `docs/architecture/GAME_CATALOG_ARCHITECTURE.md`
- Issue #489 — Game Catalog audit owner
- Issue #1460 — persistence-composition decision owner
- PR #1229 — native envelope validator (validation only)
