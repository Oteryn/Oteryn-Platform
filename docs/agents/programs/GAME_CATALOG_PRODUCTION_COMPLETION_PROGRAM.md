---
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
status: active
owner: cross-repository
coordination_issue: blakinio/Oteryn-Platform#330
repositories:
  - blakinio/Oteryn-Platform
  - blakinio/canary
created: 2026-07-29T22:18:00Z
updated: 2026-07-29T22:18:00Z
current_platform_main: f90bb8075b300569b7d493c84f0080e6b3295c35
current_canary_main: 09209bae26b2bb7e14346f08677e2cd8724aa7ae
---

# Game Catalog Production Completion Program

## Mission

Complete the existing Oteryn Game Catalog from deterministic Canary export of items, creatures and loot to a production-capable, evidence-backed catalogue covering NPCs, NPC shop offers, quests, quest rewards, spawns, raids and other confirmed creation or availability sources, while preserving immutable schemas, explicit unknowns, inactive imports, transactional activation, deterministic rollback and a manual exact-snapshot production gate.

This is a programme of bounded tasks and pull requests. It is not one implementation branch or one production authorization.

## Repository boundary

Writes are permitted only in:

- `blakinio/Oteryn-Platform`;
- `blakinio/canary`.

All upstream, donor, wiki, client and external repositories are read-only evidence sources unless the user later expands authorization explicitly.

## Initial state

### Canary producer

- PR #991 delivered the export-only deterministic producer for final runtime items, creatures and loot.
- Schemas `1.0.0`, `1.1.0` and `1.2.0` exist and remain immutable compatibility contracts.
- PRs #1005, #1006, #1010, #1012 and #1015 added reviewed metadata boundaries, endpoint integrity, exact runtime loot thresholds and dispatcher-safe loader/export execution.
- The repository-default profile emits schema `1.2.0` with `verified_content_through_release: null` and `contains_content_through_release: null`.
- No current collector exports NPCs, shop offers, quests, rewards, spawns or raids.

### Platform consumer

- PR #272 delivered schema validation, transactional inactive import, immutable snapshots, candidate activation, rollback, diff, read-only admin inspection and public items/creatures/loot projection.
- PRs #299 and #310 added consumer-first schema `1.1.0` and `1.2.0` support.
- Supported schema hashes are pinned in `config/game-catalog.php`.
- Import never activates automatically.
- Activation requires a validated snapshot, compatible profile and a concrete verified-content boundary; failures preserve the previous active snapshot and projections.
- No current persistence path supports typed NPC entities or shop relations.
- No repository evidence establishes the live staging or production snapshot/profile state.

## Immutable contracts

The following schema versions must not change in bytes or semantics:

| Schema | SHA-256 | Status |
|---|---|---|
| `1.0.0` | `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b` | retained import and rollback compatibility |
| `1.1.0` | `323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac` | nullable verified boundary |
| `1.2.0` | `a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de` | exact runtime loot threshold model |

Every new entity or relation family requires a new schema version. Unsupported consumers must reject it fail closed.

## Proposed schema sequence

| Version | Scope | Status |
|---|---|---|
| `1.3.0` | NPC entities and `npc_buy_offer` / `npc_sell_offer` relations; consumer first | proposed and owned by the first architecture task |
| `1.4.0` | Quest, quest-line and mission entities plus reward and requirement relations | provisional; blocked by quest-authority audit |
| `1.5.0` | Spawn, raid, scripted-creation, summon and location/reachability evidence | provisional; blocked by source and world-index audit |
| `1.6.0` | Evidence/provenance extensions that remain additive to the current document model | provisional |
| `2.0.0` | Reserved only if the historical evidence graph requires incompatible top-level semantics | not selected |

A later task may choose a different version only with an explicit compatibility decision in both repositories. No later number is a promise of complete historical truth.

## Rollout dependency graph

```text
current-state audits and programme records
  -> schema 1.3 architecture and exact proposal bytes
    -> Platform 1.3 inactive consumer implementation
      -> Canary NPC runtime-authority audit
        -> Canary 1.3 final-registry collector
          -> byte-identical schema/fixture parity proof
            -> cross-repository staging import
              -> candidate activation
                -> rollback proof
                  -> separate NPC/shop public projection task

quest authority audit
  -> schema 1.4 consumer
    -> schema 1.4 producer
      -> staging/rollback
        -> separate public projection

spawn/raid/source audit + existing World Index/reachability selection
  -> schema 1.5 consumer
    -> schema 1.5 producer
      -> staging/rollback
        -> separate public projection

historical evidence programme
  -> historically compatible runtime/datapack/assets/configuration bundles
    -> reviewed historical snapshots

reusable immutable artifact transport
  -> staging operations proof
    -> exact-snapshot manual production activation task
```

No producer schema may merge before a compatible inactive consumer exists when the contract is `atomic-required`.

## Evidence model

Every task must maintain distinct lists:

- `PROVEN`: direct repository, workflow, artifact, runtime or environment evidence;
- `DERIVED`: a conclusion that follows from proven facts;
- `UNKNOWN`: no sufficient evidence;
- `CONFLICT`: authoritative evidence disagrees.

Historical and availability facts additionally require source type, exact revision or URL, acquisition date, claim scope, evidence level, review status, profile/datapack scope and whether the claim concerns definition, loading, availability, mechanics or public visibility.

Missing evidence remains `null`, `unknown` or `unverified`. It is never converted to false, complete, obtainable or encounterable.

## Source hierarchy for historical facts

1. Runnable historical Canary/runtime plus matching datapack.
2. Exact historical source revisions and assets.
3. Repository migrations and changelogs.
4. Official data or reproducible maintained-client observation.
5. External wikis only as research leads, never sole authority.

A credible historical snapshot requires a compatible runtime revision, datapack, appearances/assets, configuration, protocol profile, schema version, provenance and successful exporter execution. Filtering a modern snapshot is not historical runtime evidence.

## Ownership map

| Concern | Owner | Required reuse |
|---|---|---|
| Final item, creature, NPC and runtime relation facts | Canary | existing export-only lifecycle and final registries |
| NPC registry iteration | Canary | `Npcs` / `NpcType`; do not parse Lua as a substitute |
| Shop offer values | Canary | final `NpcType::info.shopItemVector` / `ShopBlock` state |
| Quest authority | Canary plus reviewed manifest only if required | do not infer from file names, storage IDs or wiki text |
| Map placement and reachability | Canary | Unified OTBM World Index and existing reachability tooling |
| Schema pinning and supported-version decision | Platform first | exact bytes and SHA-256 |
| Validation, inactive import and persistence | Platform | existing `GameCatalog` module |
| Candidate review, diff, activation, rollback and audit | Platform | existing snapshot/profile lifecycle |
| Public visibility and UI | Platform | active profile projections and exact allowlists |
| Artifact creation | Canary | immutable JSON plus lowercase SHA-256 sidecar |
| Artifact intake and environment activation | Platform/operator | no public unauthenticated endpoint |

## Completeness gate

The programme is complete only when all applicable entity families have:

- an explicit authority;
- an immutable schema version and exact hash;
- a shared fixture with exact hash;
- consumer-first inactive import support;
- a final-runtime Canary collector or an explicitly reviewed authority where runtime has none;
- collision, duplicate and dangling-reference failure tests;
- deterministic bytes and sidecar proof;
- transactional import and persisted-count proof;
- admin candidate review and diff;
- staging activation, public projection smoke and deterministic rollback;
- compatibility matrix entries with exact producer and consumer commits;
- no unresolved conflict at the requested activation boundary.

Production readiness additionally requires exact deployed commits, exact artifact digest, exact schema hash, confirmed profile/datapack, import PASS, semantic validation PASS, staging activation PASS, staging public projection PASS, rollback PASS, backup of the previous active snapshot, rollback target, monitoring and operator approval.

## Bounded backlog

### Stage 0 — verified baseline

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-program-audit` | Platform | current-state report, programme, backlog and matrices only | issue #330 |
| `CAN-20260730-game-catalog-program-registration` | Canary | mirror programme ownership and reconcile the existing Canary completeness subprogramme | Platform issue #330 |

### Stage 1 — schema-next architecture

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-architecture` | Platform | exact NPC/shop schema and fixture proposal, compatibility and rollout; no support registration or product mutation | Stage 0 evidence |

### Stage 2 — NPC/shop vertical slice

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-consumer` | Platform | parser, validation, typed persistence, inactive import, rollback guard and admin preview; no public activation | schema architecture |
| `CAN-20260730-game-catalog-npc-runtime-authority` | Canary | prove the final registry boundary and safe deterministic iteration API; no export fields yet | schema architecture |
| `CAN-20260730-game-catalog-schema-1-3-producer` | Canary | byte-identical schema/fixture and final-registry NPC/shop collector | compatible Platform consumer + authority audit |
| `OTERYN-20260730-game-catalog-npc-shop-staging` | Platform | exact artifact intake, MariaDB lifecycle, candidate activation and rollback | both producer and consumer |
| `OTERYN-20260730-game-catalog-npc-shop-public` | Platform | separately gated public NPC/shop projection and authorization/UI acceptance | staging proof and reviewed availability |

### Stage 3 — quests and rewards

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-quest-authority-audit` | Canary | inventory canonical runtime authority; design reviewed registry only if needed | NPC/shop slice independent |
| `OTERYN-20260730-game-catalog-schema-1-4-consumer` | Platform | quest/mission/reward inactive consumer | authority decision |
| `CAN-20260730-game-catalog-schema-1-4-producer` | Canary | quest/mission/reward producer | compatible consumer |
| `OTERYN-20260730-game-catalog-quest-staging` | Platform | staging import/activation/rollback | both sides |
| `OTERYN-20260730-game-catalog-quest-public` | Platform | public projection; Wiki walkthroughs remain editorial links | staging and evidence |

### Stage 4 — spawns, raids and availability

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-creation-source-audit` | Canary | map spawns, raids, scripts, summons, instances and admin/test source taxonomy | existing World Index and reachability tooling |
| `OTERYN-20260730-game-catalog-schema-1-5-consumer` | Platform | creation/location/reachability inactive consumer | source taxonomy |
| `CAN-20260730-game-catalog-schema-1-5-producer` | Canary | source collectors and exact evidence relations | compatible consumer |
| `OTERYN-20260730-game-catalog-availability-staging` | Platform | staging import/activation/rollback | both sides |
| `OTERYN-20260730-game-catalog-availability-public` | Platform | public encounterability/obtainability projection | staging and reviewed reachability |

### Stage 5 — history, transport and activation

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-historical-evidence-program` | Canary | evidence hierarchy, reviewed claims and historical bundle criteria | entity schemas |
| `CAN-20260730-game-catalog-artifact-manifest` | Canary | immutable deployment manifest/signature option and provenance | stable producer |
| `OTERYN-20260730-game-catalog-artifact-intake` | Platform | auditable candidate intake and manifest verification | transport contract |
| `OTERYN-20260730-game-catalog-reusable-staging` | Platform | reusable non-production staging lifecycle | artifact intake |
| `OTERYN-YYYYMMDD-game-catalog-production-activation` | Platform | one exact manually approved snapshot only | all production gates |

Task identifiers after the first architecture task are reserved planning names, not proof that implementation has started.

## Validation matrix

| Boundary | Required evidence |
|---|---|
| Contract | valid JSON Schema; exact bytes/hash both repos; shared fixture; immutable old schemas; unsupported version rejection |
| Canary unit | exact field values; duplicates; canonical-key collisions; nested offers; dangling item/currency endpoints; malformed metadata |
| Canary runtime | fixed-timestamp determinism; two complete exports; atomic publication; previous-output preservation; valid sidecar; no DB/network endpoints; telemetry-off/on stability when Lua loading is involved |
| Platform validation | schema/hash/artifact/count/uniqueness/range/endpoint checks; unknown preservation |
| Platform persistence | transactional inactive import; no partial writes; typed counts; prior active snapshot preservation |
| Platform lifecycle | admin preview/diff; candidate activation; rollback; audit; RBAC/MFA; public projection only in separately authorized task |
| Cross-repository | exact commits; exact artifact digest; MariaDB lifecycle; import; candidate activation; rollback; public smoke; compatibility registry update |
| Production | exact deployed revisions and artifact; backup; rollback target; monitoring; operator approval; explicit user approval |

## Principal risks

- The current Platform importer assumes every non-item entity is a creature and every relation is loot.
- The Canary NPC registry map is private and has no proven safe deterministic iteration API for the exporter yet.
- Shop offers include nested `childShop` entries and storage conditions; flattening or dropping them would lose runtime semantics.
- NPC registration does not prove map placement, reachability or public encounterability.
- An NPC shop relation does not by itself prove that an item is player-obtainable unless the NPC, location, currency and requirements are also available.
- The existing cross-repository staging workflow is historical, schema-`1.0.0` and PR-#272-specific; it is not a reusable current transport.
- The repository-default profile has no verified-content boundary and cannot activate.
- Live staging/production state, deployed profiles, operator permissions, secrets, routing and monitoring are unknown.
- Issue #301 contains rollout and authorization assumptions that conflict with this programme and must not govern implementation without reconciliation.

## Manual production gate

Production activation is never implied by a green repository PR or staging test. It requires explicit user approval for one exact snapshot after all listed environment evidence is recorded. On any failure, the previous active snapshot remains active; the candidate remains inactive or failed; no partial public catalogue may exist.

## Current next action

Complete and review `OTERYN-20260730-game-catalog-schema-1-3-architecture` as an independent consumer-first contract task.
