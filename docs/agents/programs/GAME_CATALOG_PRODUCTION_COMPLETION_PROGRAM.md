---
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
status: active
owner: cross-repository-contract-coordination
coordination_issue: blakinio/Oteryn-Platform#330
state_mode: live_query_required
repositories_described:
  - blakinio/Oteryn-Platform
  - blakinio/canary
platform_default_authority: blakinio/Oteryn-Platform-only
server_repository_authority: explicit_current_owner_authorization_required
created: 2026-07-29T22:18:00Z
updated: 2026-08-15
historical_platform_main: f90bb8075b300569b7d493c84f0080e6b3295c35
historical_canary_main: 09209bae26b2bb7e14346f08677e2cd8724aa7ae
---

# Game Catalog Production Completion Program

## Mission

Complete the existing Oteryn Game Catalog from deterministic game-domain export of items, creatures and loot to a production-capable, evidence-backed catalogue covering NPCs, NPC shop offers, quests, quest rewards, spawns, raids and other confirmed creation or availability sources, while preserving immutable schemas, explicit unknowns, inactive imports, transactional activation, deterministic rollback and a separately authorized manual exact-snapshot production gate.

This is a specialized decomposition programme of bounded tasks and pull requests. It is not one implementation branch, a live portal scheduler, production authorization, or standing permission to access another repository.

## Repository and authority boundary

This programme describes a conceptual cross-repository producer/consumer dependency graph. **It grants no cross-repository access or mutation authority.**

For work launched from `blakinio/Oteryn-Platform`:

- the default authorized repository scope is `blakinio/Oteryn-Platform` only;
- Canary, Oteryn-v2 and every other server/game repository must not be read, searched, fetched, inspected, audited, branched, edited, reviewed or merged unless the repository owner explicitly authorizes that exact repository scope for the current task;
- the coordination Issue, programme name, historical task names, prior permission, dependency graph or presence of a repository under `repositories_described` does not grant access;
- when required producer evidence exists only in a server/game repository and no current explicit authorization exists, record the exact `BLOCKED` or `DECISION_REQUIRED` state and continue only with independent Platform-safe work;
- if server/game repository authority is explicitly granted later, that repository's current governance and task ownership remain controlling.

Production, staging activation, protected environments, secrets and operator actions remain separately authorized even when repository implementation is complete.

## State interpretation

The dated revisions, PR references, proposed task names and baseline observations below are preserved as historical programme evidence. They are **not** current ownership, queue state or permission evidence.

Before selecting specialized work:

1. let `OTERYN_PORTAL_COMPLETION` determine whether Game Catalog work is currently reachable in portal order;
2. refresh current Platform Issues, tasks, PRs, ownership and protected `main`;
3. inspect a server/game repository only when current explicit owner authorization permits that exact repository scope;
4. reuse the current live Issue/task/PR rather than starting a duplicate;
5. apply the dependency graph below only after live permitted evidence confirms the relevant predecessor state.

## Historical programme baseline

### Canary producer — historical baseline

- PR #991 delivered the export-only deterministic producer for final runtime items, creatures and loot.
- Schemas `1.0.0`, `1.1.0` and `1.2.0` exist and remain immutable compatibility contracts.
- PRs #1005, #1006, #1010, #1012 and #1015 added reviewed metadata boundaries, endpoint integrity, exact runtime loot thresholds and dispatcher-safe loader/export execution.
- The repository-default profile emitted schema `1.2.0` with `verified_content_through_release: null` and `contains_content_through_release: null` at the recorded baseline.
- At that baseline, no reviewed collector exported NPCs, shop offers, quests, rewards, spawns or raids.

These statements preserve the programme's recorded historical evidence. They must not be promoted to current server state without permitted re-verification.

### Platform consumer — historical baseline

- PR #272 delivered schema validation, transactional inactive import, immutable snapshots, candidate activation, rollback, diff, read-only admin inspection and public items/creatures/loot projection.
- PRs #299 and #310 added consumer-first schema `1.1.0` and `1.2.0` support.
- Supported schema hashes are pinned in `config/game-catalog.php`.
- Import never activates automatically.
- Activation requires a validated snapshot, compatible profile and a concrete verified-content boundary; failures preserve the previous active snapshot and projections.
- At the recorded baseline, no persistence path supported typed NPC entities or shop relations.
- No baseline evidence established the live staging or production snapshot/profile state.

Current Platform implementation and live ownership must be resolved from current `main` and current PR/task state before acting.

## Immutable contracts

The following schema versions must not change in bytes or semantics:

| Schema | SHA-256 | Status |
|---|---|---|
| `1.0.0` | `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b` | retained import and rollback compatibility |
| `1.1.0` | `323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac` | nullable verified boundary |
| `1.2.0` | `a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de` | exact runtime loot threshold model |

Every new entity or relation family requires a new schema version. Unsupported consumers must reject it fail closed. Exact current support for later schemas must be verified from current Platform authority rather than inferred from this dated programme record.

## Proposed schema sequence

The statuses below preserve the original dependency plan; they are not current task ownership or implementation state.

| Version | Scope | Historical planning status |
|---|---|---|
| `1.3.0` | NPC entities and `npc_buy_offer` / `npc_sell_offer` relations; consumer first | proposed and assigned to the first architecture task at programme creation; resolve live state before acting |
| `1.4.0` | Quest, quest-line and mission entities plus reward and requirement relations | provisional; dependent on quest-authority evidence |
| `1.5.0` | Spawn, raid, scripted-creation, summon and location/reachability evidence | provisional; dependent on source and world-index evidence |
| `1.6.0` | Evidence/provenance extensions that remain additive to the current document model | provisional |
| `2.0.0` | Reserved only if the historical evidence graph requires incompatible top-level semantics | not selected |

A later task may choose a different version only through an explicit compatibility decision under current authority. No later number is a promise of complete historical truth.

## Rollout dependency graph

```text
current-state audits and programme records
  -> schema 1.3 architecture and exact proposal bytes
    -> Platform 1.3 inactive consumer implementation
      -> separately authorized Canary NPC runtime-authority audit
        -> separately authorized Canary 1.3 final-registry collector
          -> byte-identical schema/fixture parity proof
            -> separately authorized cross-repository staging import
              -> candidate activation
                -> rollback proof
                  -> separate NPC/shop public projection task

quest authority audit
  -> schema 1.4 consumer
    -> separately authorized schema 1.4 producer
      -> staging/rollback
        -> separate public projection

spawn/raid/source audit + existing World Index/reachability selection
  -> schema 1.5 consumer
    -> separately authorized schema 1.5 producer
      -> staging/rollback
        -> separate public projection

historical evidence programme
  -> historically compatible runtime/datapack/assets/configuration bundles
    -> reviewed historical snapshots

reusable immutable artifact transport
  -> staging operations proof
    -> exact-snapshot manual production activation task
```

No producer schema may merge before a compatible inactive consumer exists when the contract is `atomic-required`. This ordering rule does not itself authorize producer-repository access, writes, staging or production operations.

## Evidence model

Every task must maintain distinct lists:

- `PROVEN`: direct permitted repository, workflow, artifact, runtime or environment evidence;
- `DERIVED`: a conclusion that follows from proven facts;
- `UNKNOWN`: no sufficient evidence;
- `CONFLICT`: authoritative evidence disagrees.

Historical and availability facts additionally require source type, exact revision or URL, acquisition date, claim scope, evidence level, review status, profile/datapack scope and whether the claim concerns definition, loading, availability, mechanics or public visibility.

Missing evidence remains `null`, `unknown` or `unverified`. It is never converted to false, complete, obtainable or encounterable.

## Source hierarchy for historical facts

Use a source only when the current task is authorized to inspect it.

1. Runnable historical Canary/runtime plus matching datapack.
2. Exact historical source revisions and assets.
3. Repository migrations and changelogs.
4. Official data or reproducible maintained-client observation.
5. External wikis only as research leads, never sole authority.

A credible historical snapshot requires a compatible runtime revision, datapack, appearances/assets, configuration, protocol profile, schema version, provenance and successful exporter execution. Filtering a modern snapshot is not historical runtime evidence.

## Ownership map

This table describes domain responsibility and expected reuse; it does **not** grant repository-access permission.

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

The programme is complete only when all applicable entity families promoted into accepted scope have:

- an explicit authority;
- an immutable schema version and exact hash;
- a shared fixture with exact hash;
- consumer-first inactive import support;
- a final-runtime producer or an explicitly reviewed authority where runtime has none;
- collision, duplicate and dangling-reference failure tests;
- deterministic bytes and sidecar proof;
- transactional import and persisted-count proof;
- admin candidate review and diff;
- staging activation, public projection smoke and deterministic rollback where applicable;
- compatibility matrix entries with exact permitted producer and consumer commits;
- no unresolved conflict at the requested activation boundary.

Production readiness additionally requires exact deployed commits, exact artifact digest, exact schema hash, confirmed profile/datapack, import PASS, semantic validation PASS, staging activation PASS, staging public projection PASS, rollback PASS, backup of the previous active snapshot, rollback target, monitoring, operator approval and separately authorized production evidence.

## Bounded backlog

The task identifiers below are planning identities, not proof that implementation is currently unowned, reachable or authorized. Resolve live state before using any of them; server/game tasks additionally require explicit current repository authority.

### Stage 0 — verified baseline

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-program-audit` | Platform | current-state report, programme, backlog and matrices only | issue #330 |
| `CAN-20260730-game-catalog-program-registration` | Canary | mirror programme ownership and reconcile the existing Canary completeness subprogramme | Platform issue #330 plus explicit current Canary authority |

### Stage 1 — schema-next architecture

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-architecture` | Platform | exact NPC/shop schema and fixture proposal, compatibility and rollout; no support registration or product mutation | Stage 0 evidence |

### Stage 2 — NPC/shop vertical slice

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-consumer` | Platform | parser, validation, typed persistence, inactive import, rollback guard and admin preview; no public activation | schema architecture |
| `CAN-20260730-game-catalog-npc-runtime-authority` | Canary | prove the final registry boundary and safe deterministic iteration API; no export fields yet | schema architecture + explicit current Canary authority |
| `CAN-20260730-game-catalog-schema-1-3-producer` | Canary | byte-identical schema/fixture and final-registry NPC/shop collector | compatible Platform consumer + authority audit + explicit current Canary authority |
| `OTERYN-20260730-game-catalog-npc-shop-staging` | Platform | exact artifact intake, MariaDB lifecycle, candidate activation and rollback | both producer and consumer + staging authority |
| `OTERYN-20260730-game-catalog-npc-shop-public` | Platform | separately gated public NPC/shop projection and authorization/UI acceptance | staging proof and reviewed availability |

### Stage 3 — quests and rewards

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-quest-authority-audit` | Canary | inventory canonical runtime authority; design reviewed registry only if needed | explicit current Canary authority |
| `OTERYN-20260730-game-catalog-schema-1-4-consumer` | Platform | quest/mission/reward inactive consumer | authority decision |
| `CAN-20260730-game-catalog-schema-1-4-producer` | Canary | quest/mission/reward producer | compatible consumer + explicit current Canary authority |
| `OTERYN-20260730-game-catalog-quest-staging` | Platform | staging import/activation/rollback | both sides + staging authority |
| `OTERYN-20260730-game-catalog-quest-public` | Platform | public projection; Wiki walkthroughs remain editorial links | staging and evidence |

### Stage 4 — spawns, raids and availability

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-creation-source-audit` | Canary | map spawns, raids, scripts, summons, instances and admin/test source taxonomy | existing World Index and reachability tooling + explicit current Canary authority |
| `OTERYN-20260730-game-catalog-schema-1-5-consumer` | Platform | creation/location/reachability inactive consumer | source taxonomy |
| `CAN-20260730-game-catalog-schema-1-5-producer` | Canary | source collectors and exact evidence relations | compatible consumer + explicit current Canary authority |
| `OTERYN-20260730-game-catalog-availability-staging` | Platform | staging import/activation/rollback | both sides + staging authority |
| `OTERYN-20260730-game-catalog-availability-public` | Platform | public encounterability/obtainability projection | staging and reviewed reachability |

### Stage 5 — history, transport and activation

| Task | Repository | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-historical-evidence-program` | Canary | evidence hierarchy, reviewed claims and historical bundle criteria | entity schemas + explicit current Canary authority |
| `CAN-20260730-game-catalog-artifact-manifest` | Canary | immutable deployment manifest/signature option and provenance | stable producer + explicit current Canary authority |
| `OTERYN-20260730-game-catalog-artifact-intake` | Platform | auditable candidate intake and manifest verification | transport contract |
| `OTERYN-20260730-game-catalog-reusable-staging` | Platform | reusable non-production staging lifecycle | artifact intake + staging authority |
| `OTERYN-YYYYMMDD-game-catalog-production-activation` | Platform | one exact manually approved snapshot only | all production gates + explicit production authority |

Task identifiers after the first architecture task remain reserved planning names, not proof that implementation has started.

## Validation matrix

| Boundary | Required evidence |
|---|---|
| Contract | valid JSON Schema; exact bytes/hash for every authorized participant; shared fixture; immutable old schemas; unsupported version rejection |
| Canary unit | exact field values; duplicates; canonical-key collisions; nested offers; dangling item/currency endpoints; malformed metadata when Canary work is authorized |
| Canary runtime | fixed-timestamp determinism; two complete exports; atomic publication; previous-output preservation; valid sidecar; no DB/network endpoints; telemetry-off/on stability when Lua loading is involved and Canary work is authorized |
| Platform validation | schema/hash/artifact/count/uniqueness/range/endpoint checks; unknown preservation |
| Platform persistence | transactional inactive import; no partial writes; typed counts; prior active snapshot preservation |
| Platform lifecycle | admin preview/diff; candidate activation; rollback; audit; RBAC/MFA; public projection only in separately authorized task |
| Cross-repository | exact permitted commits; exact artifact digest; MariaDB lifecycle; import; candidate activation; rollback; public smoke; compatibility registry update |
| Production | exact deployed revisions and artifact; backup; rollback target; monitoring; operator approval; explicit production authority |

## Principal risks

The following risks were recorded by the programme baseline and must be revalidated against current Platform state before being treated as current defects. Server/game-specific facts may be revalidated only under current explicit repository authority.

- The recorded Platform importer assumed every non-item entity was a creature and every relation was loot.
- The recorded Canary NPC registry map was private and had no proven safe deterministic iteration API for the exporter.
- Shop offers included nested `childShop` entries and storage conditions; flattening or dropping them would lose runtime semantics.
- NPC registration did not prove map placement, reachability or public encounterability.
- An NPC shop relation did not by itself prove that an item was player-obtainable unless the NPC, location, currency and requirements were also available.
- The recorded cross-repository staging workflow was historical, schema-`1.0.0` and PR-#272-specific; it was not a reusable current transport.
- The recorded repository-default profile had no verified-content boundary and could not activate.
- Live staging/production state, deployed profiles, operator permissions, secrets, routing and monitoring require current evidence.
- Issue #301 contained rollout and authorization assumptions that conflicted with this programme baseline and must not govern implementation without current reconciliation.

## Manual production gate

Production activation is never implied by a green repository PR or staging test. It requires explicit current authorization for one exact snapshot after all listed environment evidence is recorded. On any failure, the previous active snapshot remains active; the candidate remains inactive or failed; no partial public catalogue may exist.

## Current execution routing

`live_query_required`.

Do not use a dated planning task above as the current next action. `OTERYN_PORTAL_COMPLETION` selects whether Game Catalog work is currently reachable; then the specialized worker resolves the current Platform Issue/task/PR and only inspects a server/game repository when the owner has explicitly authorized that exact repository scope for the current task.
