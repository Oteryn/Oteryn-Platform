# ADR 0040 — Oteryn ecosystem repository topology and Atlas extraction boundary

## Status

Superseded — 2026-08-15 by ADR 0041

- Superseded by: `0041-ecosystem-repository-authority-contracts-and-atlas-integration.md`

This record is retained as the initial ecosystem-topology decision and provenance. ADR 0041 — `0041-ecosystem-repository-authority-contracts-and-atlas-integration.md` — is canonical for current ecosystem topology, authority, contract and Atlas-integration scope.

- Decision owner: repository owner
- Applies to: target Oteryn repository topology, temporary cross-repository architecture authority, Atlas extraction/ownership, canonical world versus Atlas product boundaries, Platform integration ownership, legacy repository disposition and future meta-repository handoff
- Does not authorize: creating or transferring repositories, changing GitHub organizations, reading or mutating external/server repositories, moving Git history, changing runtime code, changing Synology, deploying Atlas, changing DNS, activating `/map`, deleting legacy repositories, or removing Canary compatibility

## Context

Oteryn currently has architecture and implementation concerns spread across multiple repositories and legacy lineages. The target architecture needs clear repository ownership without creating one repository per bounded context and without preserving historical server repositories as permanent product boundaries.

The repository owner supplied the following correction on 2026-08-15:

- the current OTBM Atlas lives in the legacy `blakinio/Otheryn` project;
- that project is an old Canary/Crystal Server lineage;
- Atlas should be moved out of that legacy project rather than making the legacy project part of the target Oteryn architecture.

This owner decision supersedes the premise of closed draft PR #1065, which proposed keeping Otheryn as the canonical Atlas producer/source of truth. Proposed ADR 0038 from that PR was never merged or accepted and has no canonical authority.

Accepted Platform ADR 0031 already separates target native Oteryn-v2 game-domain ownership from Legacy Canary Compatibility. This decision applies the same principle at repository/product level: legacy server lineage may remain a migration source, compatibility source or historical reference, but it must not silently define target native repository ownership.

The future `Oteryn` meta repository does not yet exist. Until it exists, this ADR is the temporary highest-scope cross-repository record for the decisions below. Component-local architecture remains owned by each component repository.

## Decision

### 1. Target permanent repository topology

The target ecosystem starts with four permanent architectural/product repositories plus the normal organization-level `.github` repository:

```text
<future Oteryn GitHub organization>
│
├── .github
│   └── organization-level community, policy and reusable workflow material
│
├── Oteryn
│   └── meta repository only
│       ├── global/cross-repository architecture
│       ├── cross-repository ADRs
│       ├── repository manifest
│       ├── compatibility matrix
│       ├── release manifests
│       ├── cross-repository integration/E2E orchestration
│       └── global agent/governance policy
│
├── Oteryn-Game
│   └── target name for the current native Oteryn-v2 product repository
│       ├── native client
│       ├── native game server
│       ├── protocol-oteryn
│       ├── shared native game/domain crates
│       ├── canonical world/content model
│       ├── world compiler/bundles/validation
│       ├── bounded legacy OTBM import/migration tooling
│       └── Oteryn Studio / project-owned authoring tooling
│
├── Oteryn-Platform
│   └── web/application platform
│       ├── PublicPortal
│       ├── Identity
│       ├── Accounts
│       ├── GameAuth
│       ├── Game Gateway
│       └── other Platform modules
│
└── Oteryn-Atlas
    └── first-party browser map product
        ├── map viewer/runtime
        ├── map search
        ├── layers and overlays
        ├── POI / spawn / NPC presentation
        ├── map-specific navigation/discovery UX
        └── derived map-data ingestion/publishing pipeline
```

The organization handle itself is not selected or proven available by this ADR. The structure is conceptual until a separately authorized organization/repository migration task executes it.

### 2. Do not create repository-per-bounded-context fragmentation

Do **not** create permanent standalone repositories now for:

- `Oteryn-Portal`;
- `Oteryn-Identity`;
- `Oteryn-Login`;
- `Oteryn-Gateway`;
- `Oteryn-Client`;
- `Oteryn-Server`;
- `Oteryn-Protocol`.

Portal, Identity and Gateway remain Platform-owned concerns even when Gateway is independently deployable. Client, Server, native protocol, native shared domain/world code and their cross-component compatibility remain grouped in `Oteryn-Game` unless a later accepted ADR proves a stronger lifecycle/security/scaling/ownership reason to split them.

Bounded contexts define responsibility and dependency direction; they do not automatically require separate Git repositories.

### 3. `Oteryn` is a meta repository, not a product monorepo

The future `Oteryn` repository must not become a copy of every component or a Git-submodule umbrella.

It owns cross-repository truth that otherwise has no single component owner:

- ecosystem topology and cross-repository ADRs;
- an explicit machine-readable repository/release manifest;
- compatibility matrices;
- exact release manifests pinning repository SHAs/tags and artifact/image digests;
- cross-repository integration/release E2E orchestration;
- global governance that genuinely applies across components.

It must not duplicate component-local architecture, source code, component CI internals, or canonical protocol/API schemas that have a clear producer owner.

### 4. No Git submodules for product composition

Do not use Git submodules as the canonical composition model for these product repositories.

Cross-repository composition should use explicit manifests and immutable identities such as:

- repository commit SHA;
- component release/tag identity;
- package/container digest;
- schema/protocol version.

A release manifest in the future meta repository should make one ecosystem release reproducible without nesting component working trees.

### 5. Legacy `blakinio/Otheryn` is a migration source, not target architecture

The legacy `blakinio/Otheryn` repository must not be renamed wholesale to `Oteryn-Atlas` and must not become the future Oteryn meta repository or future native game repository merely because Atlas currently lives there.

Its target classification is:

```text
LEGACY / MIGRATION SOURCE / HISTORICAL REFERENCE
```

It may continue to exist during migration for provenance, rollback/reference and any still-active legacy dependency. A later migration task may archive it only after all required consumers and history have been reconciled. This ADR does not authorize deletion.

The exact current Atlas paths and history in that external repository remain unverified by this Platform-only task and must be inspected in a separately authorized migration task before extraction.

### 6. Create `Oteryn-Atlas` as a separate first-party product repository

Atlas is a real product boundary and should become a standalone `Oteryn-Atlas` repository when repository migration is authorized.

`Oteryn-Atlas` owns browser-map concerns such as:

- rendering/runtime for the web map;
- viewport/floor/zoom interaction;
- search and details;
- map-specific overlays and layers;
- map presentation of POIs, spawns, NPCs and other approved derived facts;
- map-specific URL/deep-link state;
- derived map-data ingestion, indexing, caching and publication;
- optional route/discovery algorithms whose source data contract is explicit.

Atlas does **not** become a second canonical owner of the game world's authoritative model merely because it visualizes it.

### 7. Canonical world and legacy OTBM migration ownership belongs to `Oteryn-Game`

Target native world ownership remains with the game/world domain, not the browser Atlas product.

`Oteryn-Game` owns, at target architecture level:

- canonical native world/content schema and model;
- authoritative world compiler/runtime bundle semantics;
- world validation and authoring semantics;
- the project-owned Studio/editor boundary;
- bounded legacy OTBM import/migration into the canonical native world model;
- any legacy intermediate representation needed to prevent OTBM bytes or Canary/Crystal assumptions from becoming native runtime truth.

OTBM is therefore a **legacy import format**, not the target shared runtime contract between Game and Atlas.

During actual extraction, code currently located inside the legacy Atlas tree must be classified by responsibility rather than copied blindly:

- OTBM/world semantics required to create or validate canonical native world state -> `Oteryn-Game`;
- viewer/search/layers/presentation and consumption of a derived map artifact -> `Oteryn-Atlas`;
- direct Canary/Crystal runtime coupling -> legacy adapter, rewrite candidate or removal candidate, subject to migration evidence.

### 8. Game-to-Atlas data flow uses a versioned derived artifact or contract

The target flow is:

```text
legacy .otbm / legacy world source
          |
          v
Oteryn-Game bounded legacy importer
          |
          v
canonical Oteryn World/Content Model
          |
          +------> native Game runtime / bundles
          |
          +------> Oteryn Studio
          |
          +------> versioned Atlas export
                        |
                        v
                  Oteryn-Atlas
                        |
                        v
                 browser map product
```

The Game-to-Atlas boundary must be explicit and immutable/versioned. The exact schema/transport is deferred to a producer/consumer contract task, but the following invariants are accepted now:

- Atlas does not read undocumented native game database tables as its source contract;
- Platform does not become a transit owner of canonical world data;
- Atlas does not parse arbitrary live game persistence to reconstruct authority;
- published Atlas data identifies the source world/content revision and producer build;
- rollback/cache behavior can identify the exact immutable Atlas dataset being served.

### 9. Platform integrates Atlas but does not own the Atlas domain

`Oteryn-Platform` remains the public application/composition layer.

Its Atlas responsibilities may include:

- the player-facing navigation entry, currently intended as `Map` / `Mapa`;
- a public `/map` entry point or equivalent same-product route;
- authentication/authorization composition if a future Atlas feature needs Platform identity;
- deployment/static-serving/reverse-proxy integration where appropriate;
- links/composition with Wiki, Game Catalog, Player Companion or other Platform surfaces through explicit contracts.

Platform must not copy the Atlas generator/viewer solely to make `/map` exist, must not become the canonical OTBM parser owner, and must not create a second authoritative world model.

The exact browser mount/static/CDN/reverse-proxy design is deliberately deferred until `Oteryn-Atlas` exists and exposes a stable consumer contract. Closed PR #1065's detailed producer/mount assumptions are not accepted by this ADR.

### 10. Preserve Atlas history when extraction is performed

When the legacy source is audited and extraction is authorized, prefer history-preserving extraction for the Atlas-owned subtree rather than copy/paste or repository reinitialization.

A likely technique is a bounded `git filter-repo --path ...` extraction when the source tree permits it. This ADR selects the preservation goal, not the exact command/path set.

Expected migration evidence includes:

- exact source repository SHA;
- exact extracted path set;
- retained authors/timestamps where Git permits;
- an explicit mapping from legacy source to new repository history;
- classification of files moved to Atlas, moved/reimplemented in Game, retained as legacy adapters or intentionally excluded;
- no claim that rewritten filtered SHAs are identical to legacy commit SHAs.

### 11. Transitional repositories remain transitional

The target permanent topology does not turn current compatibility/reference repositories into permanent first-class product repositories.

- Canary remains a legacy/transitional game-runtime dependency until native replacement/cutover criteria are separately proven.
- `otclient` remains migration/reference material for the native client unless a later decision proves another target role.
- legacy `blakinio/Otheryn` remains migration/reference material after Atlas extraction until its remaining dependencies reach terminal disposition.

Do not rename Canary to `Oteryn-Server` or the legacy Otheryn repository to `Oteryn-Atlas` as a shortcut around real responsibility extraction.

### 12. Temporary authority and future handoff

Because the future `Oteryn` meta repository does not yet exist, this accepted ADR is temporarily stored in `blakinio/Oteryn-Platform`.

When the meta repository is created, the migration must avoid two active cross-repository sources of truth. The meta repository should adopt this decision by one explicit migration/supersession step that:

1. preserves a link to ADR 0040 and its Git history;
2. records the new canonical meta-repository path/decision identity;
3. marks this Platform ADR superseded for cross-repository topology scope if appropriate;
4. leaves Platform-local implementation architecture in Platform.

Until that happens, this ADR is authoritative for the target repository topology described here because accepted ADRs outrank unmerged PRs, Issues and historical planning in Platform architecture precedence.

## Consequences

### Positive

- Atlas leaves an obsolete game-server lineage instead of making that lineage permanent architecture;
- browser-map evolution can have its own product lifecycle without owning native world authority;
- canonical world/OTBM migration semantics stay close to the native game/world compiler and Studio;
- client/server/protocol/world changes can remain atomic inside the native Game repository where shared compatibility matters;
- Platform stays focused on web/application composition instead of becoming an Atlas implementation monorepo;
- the future meta repository has a clear role and does not require submodules.

### Costs

- Atlas extraction requires a deliberate history/code responsibility audit rather than a simple repository rename;
- Game and Atlas need a new versioned derived-data contract;
- existing legacy Atlas code may need splitting or rewriting where viewer and Canary/Crystal/OTBM concerns are mixed;
- cross-repository release/compatibility manifests become required as the ecosystem matures;
- the temporary cross-repository ADR must later be handed off cleanly to the meta repository.

## Explicitly deferred decisions

This ADR does not decide:

- exact GitHub organization handle or billing plan;
- exact date/order of repository transfers;
- exact legacy Atlas directory paths and extraction command;
- exact Atlas implementation language/framework;
- exact Game-to-Atlas artifact/schema format;
- exact Atlas hosting/CDN/object-storage strategy;
- exact public `/map` serving mechanism;
- exact Atlas routing-graph semantics;
- exact retirement date of Canary, `otclient` or legacy Otheryn;
- production activation or DNS changes.

These require bounded follow-up work with exact repository/environment evidence.

## Rejected alternatives

- keep Atlas permanently inside the legacy Otheryn/Canary/Crystal project;
- rename the whole legacy Otheryn repository to `Oteryn-Atlas`;
- copy Atlas into Oteryn Platform and make Platform own the map implementation;
- create separate permanent Client, Server and Protocol repositories by default;
- create separate permanent Portal, Identity, Login or Gateway repositories by default;
- make the future `Oteryn` repository a Git-submodule umbrella or giant source monorepo;
- let Atlas consume undocumented game database tables as its canonical API;
- make OTBM the target native world format merely because the current Atlas consumes legacy OTBM data.

## Migration direction

The target migration sequence is:

1. create the future Oteryn organization/governance and meta repository under separately authorized work;
2. transfer/rename current native Oteryn-v2 to `Oteryn-Game` while preserving history/settings/PR provenance according to the migration plan;
3. transfer `Oteryn-Platform` without splitting Portal/Identity/Gateway repositories;
4. audit the exact legacy Atlas subtree and mixed dependencies;
5. create `Oteryn-Atlas` through history-preserving extraction of Atlas-owned material where feasible;
6. move/reimplement canonical OTBM/world migration semantics under `Oteryn-Game` rather than duplicating them in Atlas;
7. define and prove the immutable versioned Game-to-Atlas export contract;
8. integrate the resulting Atlas product into Platform public navigation/route/deployment;
9. retain legacy repositories until replacement, rollback/provenance and active-consumer checks allow a separate archive decision.

No step above is executed by this ADR.

## References

- repository-owner decision, 2026-08-15
- closed superseded PR #1065 — historical public Map/Otheryn-producer draft; never accepted
- Issue #302 — prior optional map/discovery disposition
- ADR 0031 — Native Oteryn-v2 integration and legacy Canary compatibility boundary
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/adr/README.md`