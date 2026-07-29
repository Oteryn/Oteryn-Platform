# Game Catalog Current-State Audit

Date: 2026-07-29 UTC  
Programme: `GAME-CATALOG-PRODUCTION-COMPLETION`  
Scope: repository state only; no live environment access

## Executive result

The existing Game Catalog is a working first vertical slice, not a complete production catalogue.

Canary produces deterministic export-only schema `1.2.0` snapshots for final runtime items, creatures and creature loot. Platform can validate and transactionally import supported snapshots inactive, compare candidates, activate a validated compatible snapshot, roll back through the same checks, expose read-only administrator inspection and serve public items/creatures/loot only from an enabled active profile.

NPCs, shop offers, quests, rewards, spawns, raids and broader creation/availability evidence are not implemented in the current producer or consumer contract. The next compatible schema must be consumer first. Live staging and production catalogue contents are unknown.

## Verified repository heads

| Repository | `main` at audit | Drift after latest catalogue merge |
|---|---|---|
| `blakinio/canary` | `09209bae26b2bb7e14346f08677e2cd8724aa7ae` | two later commits; no Game Catalog source/schema/workflow changes |
| `blakinio/Oteryn-Platform` | `f90bb8075b300569b7d493c84f0080e6b3295c35` | three later commits; no Game Catalog source/schema/workflow changes except archival/governance references |

No open Game Catalog PR or matching `game-catalog` branch was found in either authorized repository during preflight.

The commit-status and PR-workflow lookup for both current `main` heads returned no attached workflow runs. Current catalogue confidence therefore comes from unchanged paths plus the exact merged PR evidence below, not from an invented current-main CI result.

## Relevant merged evidence

### Canary

| PR | Delivered |
|---|---|
| #991 | deterministic export-only item, creature and loot producer; JSON plus SHA-256; no normal world, DB or network startup |
| #1005/#1006 | reviewed metadata seed and schema `1.1.0` nullable verified boundary |
| #1010 | loot endpoint integrity and complete default-datapack relation validation |
| #1012 | schema `1.2.0` exact runtime threshold and roll-maximum model |
| #1015 | complete exporter execution serialized on the dispatcher with telemetry-off/on stability evidence |

### Platform

| PR | Delivered |
|---|---|
| #271 | architecture and initial immutable contract |
| #272 | importer, immutable inactive snapshots, activation/candidate activation, rollback, diff, admin inspection, public item/creature/loot projection and bounded staging proof |
| #299 | consumer-first schema `1.1.0` support |
| #310 | consumer-first schema `1.2.0` threshold support |

## Schema integrity

The current Canary and Platform Git blob SHA values match for all three schema paths, proving byte-identical files at the audited heads:

| Schema | Git blob SHA | Pinned content SHA-256 |
|---|---|---|
| `1.0.0` | `a3c239a6d61385edde0b06f72cdf781f4ce58df3` | `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b` |
| `1.1.0` | `c85a1c3569276457f86ea76d4a8ee9af641aa486` | `323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac` |
| `1.2.0` | `fcb2a51d8c1b0245bbcc996d62d9d6ba110784ab` | `a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de` |

No task in this programme may modify those files or reinterpret their semantics.

## Canary producer inventory

### Export lifecycle

The producer uses the dedicated `--export-game-catalog-only` path. The current workflow:

- builds a dedicated binary;
- loads isolated definition inputs;
- disables database-backed startup, map download, metrics, backups and unrelated scheduling;
- runs two fixed-input exports;
- traces `connect`, `bind`, `listen` and accept syscalls;
- validates snapshots and lowercase SHA-256 sidecars;
- checks deterministic output after removing the intentionally different timestamp;
- exercises full default-datapack export and loader stability.

The complete export remains one dispatcher-thread operation so definition loading and Lua-scheduled work cannot race catalogue collection.

### Exported entities and relations

Current schema `1.2.0` supports only:

- entity `item`;
- entity `creature`;
- relation `creature_loot`.

It does not support NPC, quest, mission, area, spawn, raid or shop relation payloads.

### Current metadata state

The repository-default profile declares:

```json
{
  "schema_version": "1.2.0",
  "runtime_release": "15.25",
  "content_target_release": "15.25",
  "verified_content_through_release": null,
  "contains_content_through_release": null,
  "datapack_commit_sha": null,
  "producer_build_id": null
}
```

This is valid inactive review output, not activation evidence. It does not prove datapack-wide completeness through any release.

### Confirmed NPC runtime authority

Current Canary source provides a final `Npcs` registry backed by a private ordered map from lowercase registry key to `NpcType`.

`NpcType` contains final loaded runtime fields including:

- `name`;
- lowercase name;
- `typeName`;
- `nameDescription`;
- outfit, health, speed, walk and respawn data;
- `currencyId`;
- registered scripts/callbacks;
- ordered `shopItemVector`.

NPC Lua registration resolves through `g_npcs().getNpcType(name, true)` and mutates the resulting `NpcType`. Shop additions call `NpcType::loadShop`, which deduplicates equal `ShopBlock` values before storing them in the final vector.

This proves a final runtime authority exists. It does not prove a safe exporter iteration API: the registry map is private and exposes only lookup by name.

### Confirmed shop runtime values

`ShopBlock` currently preserves:

- `itemId`;
- `itemName`;
- `itemSubType`;
- `itemBuyPrice`;
- `itemSellPrice`;
- `itemStorageKey`;
- `itemStorageValue`;
- nested `childShop` records.

`NpcType` separately supplies the exact runtime currency item ID. Prices are per NPC/offer and cannot be treated as globally unique item facts.

The schema-next producer must collect these final objects. It must not parse selected Lua scripts independently and claim runtime equivalence.

## Platform consumer inventory

### Supported schemas

`config/game-catalog.php` registers exact paths and hashes for `1.0.0`, `1.1.0` and `1.2.0`. An unsupported version has no schema contract and fails closed.

### Validation and import

`CatalogImportService` currently:

1. validates the file and optional expected artifact hash;
2. records rejected validation findings when possible;
3. deduplicates exact content hashes;
4. starts an import run;
5. transactionally persists releases, snapshot, entities and relations;
6. verifies persisted counts;
7. marks the new snapshot `validated`;
8. leaves the current profile unchanged.

The import is inactive by default. A failure inside persistence rolls back the transaction and records a rejected import run rather than publishing partial catalogue state.

### Confirmed schema-next blockers in persistence

The current importer has two hard type assumptions:

- an entity is persisted as an item when `type === item`; every other accepted entity is persisted as a creature;
- every accepted relation is persisted into `game_catalog_loot_snapshots`.

The persisted-count verification also derives creature count as `total entities - items` and loot count as `all relations`.

Therefore schema `1.3.0` cannot be safely added by producer-only changes or schema-only registration. The Platform consumer requires typed NPC and shop persistence, dispatch by explicit type, typed count verification and fail-closed unknown-type handling before Canary may emit the new version.

### Snapshot lifecycle

`CatalogActivationService`:

- locks the target profile;
- requires an existing snapshot with status `validated`;
- requires a supported contract/schema;
- checks profile protocol compatibility;
- requires content and verified boundaries through the target release;
- verifies persisted entity and relation counts;
- rebuilds visibility projections;
- changes `active_snapshot_id` and appends an audit event in one transaction.

Any exception preserves the prior active snapshot and projections.

`game-catalog:rollback` reactivates an earlier validated immutable snapshot through the same checks. It is not a destructive database rollback.

### Administrative surfaces

Current browser administration is read-only:

- overview;
- profiles and profile detail;
- snapshots and snapshot detail;
- validation findings;
- snapshot diff.

Routes require authenticated, confirmed-MFA administrator context and exact catalogue permissions. The current repository does not expose browser upload, browser activation or browser rollback routes.

Operator CLI commands own import, activation, rollback, diff and verification.

### Public projection

Current public routes expose catalogue overview, item list/detail and creature list/detail. Public context exists only when:

- the configured profile exists;
- `public_enabled` is true;
- it has an active snapshot;
- the snapshot status is `validated`;
- the verified boundary exists.

Database schema defaults `public_enabled` to false and `active_snapshot_id` to null.

The repository proves this policy and code path. It does not prove that a live environment currently has such a profile or snapshot.

## Existing snapshot and environment status

| Question | Repository result |
|---|---|
| Which snapshot rows exist in staging? | `UNKNOWN` — requires staging database/operator evidence |
| Is a staging profile active? | `UNKNOWN` |
| Which snapshot rows exist in production? | `UNKNOWN` |
| Is a production profile active/public? | `UNKNOWN` |
| Does production currently render the catalogue? | `UNKNOWN` |
| Which Canary/Platform commits are deployed? | `UNKNOWN` |
| Is a rollback target backed up in production? | `UNKNOWN` |

No repository-only statement may replace these unknowns.

## Transport audit

The durable contract currently defines operator/deployment files:

```text
game-catalog.json
game-catalog.json.sha256
```

There is no network push and no browser upload in the current version.

PR #272 additionally proved one bounded cross-repository staging lifecycle using a specific Canary workflow artifact, a staging branch containing chunked base64 payload data, exact hashes and a MariaDB import/activate/candidate/rollback test. The current workflow is hard-coded to Platform PR #272 and schema `1.0.0` evidence.

This is historical proof that the lifecycle can work. It is not a reusable current staging or production transport mechanism for schema `1.2.0` or future schemas.

A later transport task must define an immutable artifact manifest with exact Canary commit, datapack revision, schema hash, artifact digest, generation timestamp and profile key, then import it as an inactive candidate without using a public unauthenticated endpoint.

## Public versus administrative facts

### Public today, when an active public profile exists

- allowlisted item fields and item availability summary;
- allowlisted creature fields;
- visible creature loot relations;
- simple active profile/release context.

### Administrator-only today

- schema version and content hash;
- producer/provenance fields;
- validation findings and summaries;
- completeness and availability diagnostics;
- profile visibility reasons;
- snapshot diff;
- import and activation audit events.

### Not implemented

- NPC list/detail;
- shop offers;
- quest/mission/reward pages;
- spawn/raid/source presentation;
- schema-next candidate counts and typed preview;
- reusable artifact intake UI/operation;
- environment-gated production activation workflow.

## Gap register

| Gap | Evidence state | Required task family |
|---|---|---|
| No NPC entity in schemas `1.0`–`1.2` | PROVEN | schema `1.3` consumer first |
| No shop relations in schemas `1.0`–`1.2` | PROVEN | schema `1.3` consumer first |
| Platform type dispatch assumes item/creature and loot only | PROVEN | typed consumer persistence |
| Canary NPC registry has no proven iteration API | PROVEN | NPC runtime-authority task |
| NPC location/reachability | UNKNOWN | separate schema `1.5` source/reachability tasks |
| Quest canonical authority | UNKNOWN | quest authority audit before schema `1.4` |
| Full historical introduced/removed metadata | UNKNOWN | historical evidence programme |
| Current default verified-content boundary | UNKNOWN/null | reviewed evidence; activation remains blocked |
| Reusable transport and staging operation | missing | artifact manifest/intake/staging tasks |
| Live staging state | UNKNOWN | environment evidence |
| Live production state and approval | UNKNOWN/BLOCKED | exact-snapshot manual activation task |

## Architecture decisions for the first next schema

The first schema-next task must propose `1.3.0` for NPCs and shop offers only.

- NPC entities come from the final `Npcs` registry.
- Shop offers are relations from NPC to item, not text fields embedded in NPC data.
- Buy and sell are distinct relation types.
- Currency is an exact runtime item endpoint/value, not an assumed universal gold price.
- Item amount, price, subtype, storage requirement and nested offer path must be preserved.
- Duplicate runtime offers require deterministic identity; collisions fail closed.
- Every NPC, offered item and currency item endpoint must resolve inside the snapshot when represented as a catalogue endpoint.
- `introduced_in` and `removed_in` remain nullable.
- NPC registration does not prove encounterability.
- A shop offer does not automatically promote item obtainability without NPC/location/currency/requirement evidence.
- Location references are deferred to the spawn/map slice rather than encoded as unverified NPC text.
- Import/admin preview may be implemented before public projection; no schema `1.3.0` activation is implied by support.

## Proposed rollout

1. Merge exact Platform schema `1.3.0` architecture and fixture proposal without registering support.
2. Implement Platform parser, semantic validation, typed persistence, inactive import, admin preview and rollback preservation.
3. Prove Canary final NPC registry iteration and stable offer identity.
4. Pin byte-identical schema and fixture in Canary and implement the collector.
5. Generate one exact immutable candidate artifact and sidecar.
6. Run MariaDB cross-repository import, candidate activation and rollback in staging-only context.
7. Review unknown/unverified counts and endpoint integrity.
8. Implement public NPC/shop projection in a separate task only after availability policy is approved.
9. Keep production activation in a later exact-snapshot manual task.

## Risks and blockers

- A private registry cannot be exposed casually; the producer task must add a bounded read-only iteration boundary that preserves dispatcher/Lua concurrency assumptions.
- `childShop` nesting and storage conditions can be lost by an oversimplified flat offer model.
- Runtime display name, type name and registry key are distinct fields; display text cannot be the sole identity.
- Item global `buyPrice`/`sellPrice` fields are maxima updated during shop load and are not authoritative per-NPC offer prices.
- Current profile history fields are incomplete and must remain null.
- The historical one-off staging workflow must not be mistaken for an operational production pipeline.
- Production environment access, routing, credentials, secrets, operator permissions, backups and monitoring are unavailable in this audit.

## Evidence summary

### PROVEN

- The three existing schemas are byte-identical across repositories and pinned by hash.
- Canary exports final runtime items, creatures and loot only.
- Canary has a final NPC registry and final per-NPC shop vector with exact price, subtype, storage and nesting fields.
- Platform supports inactive transactional import, activation, rollback, diff, admin read-only inspection and public item/creature/loot projection.
- Platform current persistence cannot dispatch NPC/shop types safely.
- The default Canary profile has a null verified boundary.
- The existing cross-repository staging workflow is historical and PR-#272/schema-`1.0.0` specific.

### DERIVED

- Schema `1.3.0` must be consumer first.
- NPC/shop work must remain separate from quest and spawn work.
- Public NPC/shop visibility must remain blocked until a separate projection and availability task.

### UNKNOWN

- Complete NPC aliases and public display policy.
- NPC location and reachability.
- Quest canonical authority.
- Full historical version ranges and datapack-wide completeness.
- Live staging and production catalogue state.

### CONFLICT

- Platform issue #301 records producer-before-consumer and Canary-read-only assumptions; current programme issue #330 and explicit user authorization require consumer-first work with writes allowed in both repositories.

## Next action

Start `OTERYN-20260730-game-catalog-schema-1-3-architecture` on an independent branch and PR, limited to the exact contract/schema/fixture proposal and rollout decomposition.
