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
---

# Game Catalog Production Completion Program

## Mission

Complete the existing Oteryn Game Catalog through evidence-backed, versioned producer/consumer contracts covering items, creatures, loot, NPCs, shops, quests, rewards, spawns, raids and other confirmed creation/availability sources, while preserving immutable schemas, explicit unknowns, inactive imports, transactional activation, deterministic rollback and a separately authorized exact-snapshot production gate.

This is a specialized decomposition programme of bounded tasks and pull requests. It is **not** a live portal scheduler, one implementation branch, production authorization, or standing permission to access another repository.

## Repository and authority boundary

This programme describes a conceptual cross-repository dependency graph. **It grants no cross-repository access or mutation authority.**

For an invocation launched from `blakinio/Oteryn-Platform`:

- default authorized repository scope is `blakinio/Oteryn-Platform` only;
- Canary, Oteryn-v2 and every other server/game repository must not be read, searched, fetched, inspected, audited, branched, edited, reviewed or merged unless the repository owner explicitly authorizes that exact repository scope for the current task;
- the coordination Issue, programme name, old task name, historical permission, dependency graph or presence of a repository in `repositories_described` does **not** grant access;
- when required producer evidence exists only in a server/game repository and no current explicit authorization exists, record the exact candidate `BLOCKED`/`DECISION_REQUIRED` state and continue only with independent Platform-safe work;
- if explicit server-repository authority is later granted, that repository's own governance and task ownership control any read/write operation.

Production, staging activation, protected environments, secrets and operator actions remain separately authorized even when repository implementation is complete.

## State and routing rule

All dated repository revisions, task names and baseline observations below are historical planning/evidence. They are not current ownership or queue state.

Before selecting specialized work:

1. let `OTERYN_PORTAL_COMPLETION` determine whether the Game Catalog candidate is currently reachable in portal order;
2. refresh current Platform Issues, tasks, PRs, ownership and exact protected `main`;
3. inspect server/game state **only** if current owner authorization explicitly permits that repository access;
4. reuse the current live Issue/task/PR rather than starting a duplicate;
5. apply the rollout dependency graph below only after live evidence confirms the relevant predecessor state.

## Historical programme baseline

The following baseline was established when this programme was created and is retained as provenance only. It must not be treated as current server state without permitted re-verification.

### Producer-side baseline

- A deterministic export-only producer for final runtime items, creatures and loot existed in the historical Canary programme state.
- Schemas `1.0.0`, `1.1.0` and `1.2.0` were established as immutable compatibility contracts.
- Historical producer work added metadata boundaries, endpoint integrity, exact runtime loot thresholds and dispatcher-safe loader/export execution.
- At that baseline, no reviewed collector exported NPCs, shop offers, quests, rewards, spawns or raids.

### Platform consumer baseline

- Platform delivered schema validation, transactional inactive import, immutable snapshots, candidate activation, rollback, diff, read-only admin inspection and public items/creatures/loot projection.
- Platform later added consumer-first schema `1.1.0` and `1.2.0` support.
- Supported schema hashes are pinned in `config/game-catalog.php`.
- Import does not activate automatically.
- Activation requires a validated snapshot, compatible profile and a concrete verified-content boundary; failures preserve the previous active snapshot and projections.
- Later Platform work may supersede this baseline; current implementation must be read from current `main` and live PRs before acting.

## Immutable contracts

The following schema versions are historical accepted compatibility contracts and must not change in bytes or semantics:

| Schema | SHA-256 | Status |
|---|---|---|
| `1.0.0` | `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b` | retained import and rollback compatibility |
| `1.1.0` | `323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac` | nullable verified boundary |
| `1.2.0` | `a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de` | exact runtime loot threshold model |

Every new entity or relation family requires a new schema version. Unsupported consumers must reject it fail closed. Exact current support and any later accepted schema version must be verified from current Platform authority rather than inferred from this dated programme record.

## Planned schema sequence

These identifiers are dependency planning, not proof that work is currently unowned or authorized:

| Version | Scope | Planning state |
|---|---|---|
| `1.3.0` | NPC entities and `npc_buy_offer` / `npc_sell_offer` relations; consumer first | planned/implementation state must be resolved live |
| `1.4.0` | Quest, quest-line and mission entities plus reward and requirement relations | provisional; authority evidence required |
| `1.5.0` | Spawn, raid, scripted-creation, summon and location/reachability evidence | provisional; source/world-index evidence required |
| `1.6.0` | Evidence/provenance extensions additive to the current document model | provisional |
| `2.0.0` | Reserved only if evidence graph requires incompatible top-level semantics | not selected |

A later task may choose a different version only through an explicit compatibility decision. No later number is a promise of complete historical truth.

## Rollout dependency graph

```text
current-state audits and accepted programme records
  -> schema-next architecture / exact proposal bytes when still required
    -> Platform inactive consumer implementation
      -> separately authorized producer runtime-authority audit
        -> producer collector implementation
          -> byte-identical schema/fixture parity proof
            -> separately authorized cross-repository staging import
              -> candidate activation
                -> rollback proof
                  -> separate public projection task

quest authority evidence
  -> Platform quest consumer
    -> authorized producer
      -> staging/rollback
        -> separate public projection

spawn/raid/source authority evidence + world/reachability model
  -> Platform creation/location consumer
    -> authorized producer
      -> staging/rollback
        -> separate public projection

historical evidence work
  -> compatible runtime/datapack/assets/configuration evidence
    -> reviewed historical snapshots

reusable immutable artifact transport
  -> staging operations proof
    -> exact-snapshot manual production activation
```

When the compatibility contract is `atomic-required`, a producer schema must not become active before a compatible inactive consumer exists. This dependency rule does not itself authorize producer-repository access or writes.

## Evidence model

Every task must maintain distinct lists:

- `PROVEN`: direct permitted repository, workflow, artifact, runtime or environment evidence;
- `DERIVED`: a conclusion that follows from proven facts;
- `UNKNOWN`: insufficient evidence;
- `CONFLICT`: authoritative evidence disagrees.

Historical and availability facts additionally require source type, exact revision or URL, acquisition date, claim scope, evidence level, review status, profile/datapack scope and whether the claim concerns definition, loading, availability, mechanics or public visibility.

Missing evidence remains `null`, `unknown` or `unverified`. It is never converted to false, complete, obtainable or encounterable.

## Source hierarchy for historical facts

Use this hierarchy only when the current task is authorized to inspect the named source:

1. Runnable historical authoritative runtime plus matching datapack.
2. Exact historical source revisions and assets.
3. Repository migrations and changelogs.
4. Official data or reproducible maintained-client observation.
5. External wikis only as research leads, never sole authority.

A credible historical snapshot requires a compatible runtime revision, datapack, appearances/assets, configuration, protocol profile, schema version, provenance and successful exporter execution. Filtering a modern snapshot is not historical runtime evidence.

## Ownership map

The table describes domain responsibility, not repository-access permission:

| Concern | Domain owner | Required reuse |
|---|---|---|
| Final item, creature, NPC and runtime relation facts | game-domain producer | authoritative final registries; exact implementation must be verified under permitted access |
| NPC registry iteration | game-domain producer | deterministic final-registry iteration; do not infer from script filenames |
| Shop offer values | game-domain producer | final runtime shop state |
| Quest authority | game-domain producer plus reviewed manifest only if required | do not infer from filenames, storage IDs or wiki text |
| Map placement and reachability | game-domain producer | accepted world-index/reachability evidence when authorized |
| Schema pinning and supported-version decision | Platform consumer first | exact bytes and SHA-256 |
| Validation, inactive import and persistence | Platform | existing `GameCatalog` module |
| Candidate review, diff, activation, rollback and audit | Platform | existing snapshot/profile lifecycle |
| Public visibility and UI | Platform | active profile projections and exact allowlists |
| Artifact creation | game-domain producer | immutable document plus digest/provenance |
| Artifact intake and environment activation | Platform/operator | protected authenticated intake; no public unauthenticated activation endpoint |

## Completeness gate

The specialized programme is complete only when all entity families promoted into accepted scope have:

- explicit authority;
- immutable schema version and exact hash;
- shared fixture with exact hash;
- consumer-first inactive import support where required;
- an authoritative producer or explicitly accepted alternate authority;
- collision, duplicate and dangling-reference failure tests;
- deterministic artifact bytes/digest proof;
- transactional import and persisted-count proof;
- admin candidate review/diff;
- staging activation, public projection smoke and deterministic rollback where applicable;
- compatibility matrix entries with exact permitted producer/consumer revisions;
- no unresolved `CONFLICT` at the requested activation boundary.

Production readiness additionally requires exact deployed revisions/artifact, backup, rollback target, monitoring, operator approval and separately authorized production evidence. Green repository CI never implies production activation.

## Bounded backlog model

The following task identifiers preserve the intended dependency decomposition. They are planning identities only; **before using one, query live state and current authority**.

### Stage 0 — baseline and programme registration

| Planning task | Repository domain | Scope |
|---|---|---|
| `OTERYN-20260730-game-catalog-program-audit` | Platform | current-state report/programme/backlog/matrices |
| `CAN-20260730-game-catalog-program-registration` | server/game | mirror ownership only when explicitly authorized |

### Stage 1 — schema-next architecture

| Planning task | Repository domain | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-architecture` | Platform | exact NPC/shop schema/fixture compatibility boundary; no product activation | current permitted evidence |

### Stage 2 — NPC/shop vertical slice

| Planning task | Repository domain | Scope | Depends on |
|---|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-consumer` | Platform | parser, validation, typed persistence, inactive import, rollback guard/admin preview | accepted schema architecture |
| `CAN-20260730-game-catalog-npc-runtime-authority` | server/game | prove authoritative final registry/iteration | explicit server access + accepted schema |
| `CAN-20260730-game-catalog-schema-1-3-producer` | server/game | byte-identical schema/fixture and authoritative collector | explicit server authority + compatible Platform consumer |
| `OTERYN-20260730-game-catalog-npc-shop-staging` | Platform/operator | exact artifact intake, lifecycle, candidate activation/rollback | both sides + staging authority |
| `OTERYN-20260730-game-catalog-npc-shop-public` | Platform | separately gated public projection/UI | staging proof + reviewed availability |

### Stage 3 — quests and rewards

| Planning task | Repository domain | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-quest-authority-audit` | server/game | canonical authority audit | explicit server authority |
| `OTERYN-20260730-game-catalog-schema-1-4-consumer` | Platform | quest/mission/reward inactive consumer | accepted authority decision |
| `CAN-20260730-game-catalog-schema-1-4-producer` | server/game | authoritative producer | explicit server authority + compatible consumer |
| `OTERYN-20260730-game-catalog-quest-staging` | Platform/operator | staging import/activation/rollback | both sides + staging authority |
| `OTERYN-20260730-game-catalog-quest-public` | Platform | public projection; Wiki walkthroughs remain editorial | staging/evidence |

### Stage 4 — spawns, raids and availability

| Planning task | Repository domain | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-creation-source-audit` | server/game | map spawns/raids/scripts/summons/instances/admin-test source taxonomy | explicit server authority |
| `OTERYN-20260730-game-catalog-schema-1-5-consumer` | Platform | creation/location/reachability inactive consumer | accepted source taxonomy |
| `CAN-20260730-game-catalog-schema-1-5-producer` | server/game | authoritative source collectors/evidence relations | explicit server authority + compatible consumer |
| `OTERYN-20260730-game-catalog-availability-staging` | Platform/operator | staging import/activation/rollback | both sides + staging authority |
| `OTERYN-20260730-game-catalog-availability-public` | Platform | public encounterability/obtainability projection | staging/reviewed reachability |

### Stage 5 — history, transport and activation

| Planning task | Repository domain | Scope | Depends on |
|---|---|---|---|
| `CAN-20260730-game-catalog-historical-evidence-program` | server/game | evidence hierarchy/reviewed historical bundles | explicit server authority + relevant schemas |
| `CAN-20260730-game-catalog-artifact-manifest` | server/game | immutable transport manifest/provenance | explicit server authority + stable producer |
| `OTERYN-20260730-game-catalog-artifact-intake` | Platform | auditable candidate intake/manifest verification | accepted transport contract |
| `OTERYN-20260730-game-catalog-reusable-staging` | Platform/operator | reusable non-production lifecycle | artifact intake + staging authority |
| `OTERYN-YYYYMMDD-game-catalog-production-activation` | Platform/operator | one exact manually approved snapshot | all production gates + explicit production authority |

## Validation matrix

| Boundary | Required evidence |
|---|---|
| Contract | valid schema; exact bytes/hash across every authorized participant; shared fixture; immutable old schemas; unsupported version rejection |
| Producer unit/runtime | exact field values, duplicate/collision/dangling-endpoint failures, fixed-input determinism, atomic publication and previous-output preservation when producer work is authorized |
| Platform validation | schema/hash/artifact/count/uniqueness/range/endpoint checks; unknown preservation |
| Platform persistence | transactional inactive import; no partial writes; typed counts; prior active snapshot preservation |
| Platform lifecycle | admin preview/diff; candidate activation; rollback; audit; RBAC/MFA; public projection only in separately selected task |
| Cross-repository | exact permitted revisions, artifact digest, lifecycle import/activation/rollback/public smoke, compatibility registry update |
| Production | exact deployed revisions/artifact, backup, rollback target, monitoring, operator approval and explicit protected-environment authority |

## Principal risks

- A Platform importer may lag newly introduced entity/relation families; verify current code rather than relying on the historical baseline.
- Producer registry/runtime iteration may be private or nondeterministic; do not invent an iteration authority.
- Nested shop conditions or other runtime semantics can be lost by flattening.
- Definition/registration does not prove map placement, reachability, obtainability or public availability.
- Historical transport workflows can be obsolete and must not be reused as current deployment authority without revalidation.
- Missing verified-content/profile boundaries must remain unavailable rather than being guessed.
- Live staging/production state, deployed profiles, operator permissions, secrets, routing and monitoring require exact current evidence.
- Any legacy Issue/programme assumption that conflicts with current accepted architecture/governance is subordinate and must be reconciled before execution.

## Manual production gate

Production activation is never implied by a green repository PR or staging test. It requires explicit authorization for one exact snapshot after all applicable environment evidence is recorded. On failure, the previous active snapshot remains active; the candidate remains inactive/failed; no partial public catalogue may exist.

## Current execution routing

`live_query_required`.

Do not use a dated planning task in this file as the current next action. `OTERYN_PORTAL_COMPLETION` selects whether Game Catalog work is currently reachable; then the specialized worker resolves the current Platform Issue/task/PR and only inspects a server/game repository if the owner has explicitly authorized that exact repository scope for the current task.
