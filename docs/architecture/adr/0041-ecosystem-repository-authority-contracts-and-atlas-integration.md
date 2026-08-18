# ADR 0041 — Ecosystem repository authority, contracts and Atlas integration boundary

## Status

Superseded for ecosystem repository-topology and META coordination authority — 2026-08-18

- Previous status: Accepted — 2026-08-15
- Successor: `Oteryn/Oteryn` ADR 0001 — Oteryn ecosystem repository topology authority
- Successor merge: `a2672baac544ada81c526e92f0517903865a9ad0`
- Superseded scope: ecosystem repository topology and META coordination authority
- Preserved historical value: provider ownership boundaries, migration safety constraints, review evidence and sequencing rationale remain provenance and continue to be referenced where the successor preserves them
- Decision owner: repository owner
- Supersedes: ADR 0040 — Oteryn ecosystem repository topology and Atlas extraction boundary
- Applies historically to: target Oteryn repository topology, cross-repository authority, provider/consumer contract ownership, Game/Atlas data flow, Atlas browser/deployment trust, release independence, migration sequencing and legacy-source classification
- Does not authorize: creating or transferring repositories, changing the GitHub organization, moving Git history, changing runtime code, changing CI/CD, deploying Atlas, changing Synology, changing DNS, changing authentication behavior, production activation or deleting legacy repositories

## Context

ADR 0040 established the first durable target repository topology after the repository owner clarified that the existing OTBM Atlas lives inside the legacy `blakinio/Otheryn` Canary/Crystal lineage and must not make that legacy repository part of the target architecture.

The owner then requested independent review from the target product perspectives. The following merged evidence now exists:

- `blakinio/Oteryn-v2` first-pass topology review, PR #278: `ACCEPT_WITH_CHANGES`;
- `blakinio/Oteryn-v2` senior developer / programmer / project-manager second pass, PR #280: upholds `ACCEPT_WITH_CHANGES` and strengthens sequencing/release constraints;
- `blakinio/Oteryn-Platform` ADR 0040 review, PR #1100: `ACCEPT_WITH_CHANGES` and strengthens authority-transfer, Atlas release/origin and API-boundary rules;
- `blakinio/Otheryn` Atlas extraction audit, PR #407: `EXTRACTABLE_WITH_REFACTOR` and proves the current Atlas namespace mixes future Game-owned legacy/world semantics with future Atlas-owned browser/publication concerns.

The repository owner additionally decided on 2026-08-15 that `blakinio/canary` and `blakinio/otclient` are legacy/transitional/migration-reference repositories only and are **not** normative reviewers of the target Oteryn repository architecture. Their code, tests, fixtures and history may be used as bounded evidence during migration, but their historical architecture cannot veto or redefine the target topology.

The review outcome therefore has no target-product disagreement about the four principal boundaries. The required changes are refinements to authority, contracts, browser trust, release independence and migration execution rather than a different repository split.

This ADR supersedes ADR 0040 for current ecosystem-topology scope. ADR 0040 remains historical provenance for the initial owner decision.

## Decision

### 1. Target permanent repository topology remains four product/architecture repositories

The target ecosystem is:

```text
<future Oteryn GitHub organization>
│
├── .github
│   └── organization-level community/policy/reusable-workflow material
│
├── Oteryn
│   └── thin ecosystem coordination/meta plane
│
├── Oteryn-Game
│   └── native playable game product + canonical game-content toolchain
│
├── Oteryn-Platform
│   └── web/application/control plane
│
└── Oteryn-Atlas
    └── derived browser-map/read-model product
```

The boundaries are logical architecture now. Physical repository creation/rename/transfer remains a separately authorized migration operation.

### 2. Repository-per-bounded-context fragmentation remains rejected

Do **not** create separate permanent repositories now for:

- `Oteryn-Portal`;
- `Oteryn-Identity`;
- `Oteryn-Login`;
- `Oteryn-Gateway`;
- `Oteryn-Client`;
- `Oteryn-Server`;
- `Oteryn-Protocol`.

A bounded context, executable or deployment boundary does not automatically justify a source-repository boundary.

A later extraction requires concrete evidence such as durable independent ownership, materially independent lifecycle/release cadence, security/compliance isolation, multiple independent consumers, build/CI isolation that cannot be achieved path-locally, or an independently governed public contract whose value exceeds atomic-change safety.

### 3. `Oteryn-Game` is the target boundary for the native game product and canonical content toolchain

The current `blakinio/Oteryn-v2` repository is the target source lineage for `Oteryn-Game`.

`Oteryn-Game` owns:

- native Rust Client;
- authoritative Rust Game Server / GameNode;
- `protocol-oteryn` schemas, codecs and golden fixtures;
- shared native game/domain identifiers and semantic crates;
- native client/server/headless protocol E2E mechanics and game-domain scenarios;
- canonical native World/Content schema and model;
- editable World Project semantics;
- deterministic world/content compiler;
- validation and immutable World Bundle semantics;
- bounded legacy OTBM parser/importer and Legacy IR;
- project-owned authoring APIs and Oteryn Studio;
- Game-owned safe public projection/export semantics used by Atlas.

Client + Server + `protocol-oteryn` remain together because atomic protocol/shared-type evolution and one exact source revision currently provide more safety than independent repository version coordination.

Canonical world/compiler/runtime code must never depend on OTBM/Legacy IR types, Atlas consumer types, Platform transport adapters or Studio UI shells.

### 4. One Game source repository does not imply one release version

`Oteryn-Game` is one source repository with multiple independently identifiable compatibility/release units.

At minimum, release evidence must be able to distinguish:

- Client build/version;
- Server build/version;
- `protocol-oteryn` major/schema revision;
- canonical World/Content schema revision;
- World Bundle format/artifact digest;
- Studio build/version;
- Atlas export schema/artifact revision.

A repository commit/tag is source provenance, not a substitute for all compatibility identities.

Studio remains in `Oteryn-Game` now but must be a dependency/build/release island:

```text
Studio UI/shell
  -> headless editor/preview APIs
  -> canonical world/content/validation/compiler crates

canonical crates -X-> Studio UI/Tauri/web shell
```

### 5. `Oteryn-Platform` retains Portal, Identity and Gateway ownership

`Oteryn-Platform` owns:

- PublicPortal and ordinary web/application composition;
- Identity, authentication/security policy, OAuth/PKCE, MFA/recovery and Platform sessions;
- Accounts and Platform-owned business/application state;
- GameAuth / Game Login Ticket lifecycle;
- World Registry control-plane policy within its accepted boundary;
- Game Gateway semantics and source, even though Gateway is independently deployable;
- other Platform modules and Platform-local projections/contracts.

No current evidence justifies separate Identity or Gateway repositories.

Existing GameAuth, internal Gateway transports and operational probes remain specialized bounded contracts. They do **not** imply activation of the deferred general PlatformAPI from ADR 0036.

### 6. `Oteryn-Atlas` is an independent derived product, not a second world authority

`Oteryn-Atlas` owns:

- browser map viewer/runtime;
- map navigation, floors, zoom and deep-link state;
- map-specific search/details;
- layers/overlays;
- POI/spawn/NPC presentation of approved derived facts;
- Atlas-specific indexing/spatial partitioning/cache;
- Atlas application assets and packaging;
- Atlas runtime/deployment artifacts;
- consumer-side validation of the Game-owned Atlas export contract;
- Atlas release and rollback lifecycle.

`Oteryn-Atlas` does **not** own:

- OTBM parsing;
- Legacy IR;
- canonical World/Content schema;
- authoritative game content identities/rules;
- Game persistence;
- World Bundle compilation;
- Crystal/Canary source-tree interpretation;
- a second canonical copy of Game-owned schemas.

### 7. Existing Otheryn Atlas is extractable only after responsibility separation

Merged Otheryn PR #407 establishes `EXTRACTABLE_WITH_REFACTOR`.

The current `tools/otbm_atlas/**` / `tools/otbm_atlas_facts/**` implementation mixes responsibilities. A wholesale subtree move/filter is therefore rejected.

Examples of future Game-owned legacy/world concerns include:

- OTBM node framing/semantic decoding;
- spawn/house/mechanics interpretation;
- Crystal Lua/NPC/monster/raid semantic extraction;
- canonical factual normalization;
- legacy composition/provenance needed to produce canonical native content.

Examples of future Atlas-owned concerns include:

- browser viewer/runtime;
- URL state;
- spatial/search indexing;
- overview/publication rendering;
- Atlas-specific cache/publication verification;
- browser animation/presentation after consuming canonical exported facts.

Mixed orchestrators/render/factual-layer code must be split or rewritten around the new Game -> Atlas contract before history extraction is claimed complete.

Generated `build/**` products are regenerated from immutable inputs; they are not migrated as source history.

### 8. Game -> Atlas is artifact-first and producer-owned

The primary target flow is:

```text
legacy OTBM / legacy world content
        |
        v
Oteryn-Game bounded legacy importer
        |
        v
canonical Oteryn World/Content Model
        |
        +--> Game runtime / World Bundles
        |
        +--> Oteryn Studio
        |
        +--> Game-owned public Atlas projection/export
                  |
                  v
          immutable versioned artifact
                  |
                  v
             Oteryn-Atlas
        ingestion/index/cache/render
                  |
                  v
              browser map
```

The canonical Game -> Atlas payload is an immutable artifact/snapshot, not a synchronous Game Server API.

`Oteryn-Game` canonically owns:

- Atlas export schema;
- public-field allowlist/classification at the Game semantic boundary;
- deterministic exporter;
- producer validation and golden fixtures;
- source World/Content revision identity;
- export revision and artifact digest/provenance.

`Oteryn-Atlas` owns:

- parser/consumer validation;
- resource/size/limit handling;
- indexing/spatial projection;
- derived caches;
- browser/publication artifacts;
- consumer compatibility evidence.

The future `Oteryn` meta plane may mark a Game-export / Atlas-consumer pair ecosystem-supported only after both producer and consumer evidence exists. Producer ownership is not permission to break consumers silently.

Start with complete deterministic snapshots. Incremental/delta exports are deferred until measured size/build/publication evidence justifies them; any future delta must bind exact base and target digests and retain full-snapshot recovery.

### 9. Atlas export is an explicit public-safe projection

Atlas must never receive the canonical World Project wholesale merely because it can render map data.

The export is allowlist-based and may include only fields explicitly classified for Atlas/public use.

Server-only/editor-only/unreleased/security-sensitive fields remain absent by default, including hidden mechanics or admin metadata unless a later explicit public-data decision allows them.

Atlas may not use undocumented Game database tables, live GameNode memory, OTBM files or Platform as a transit owner to reconstruct missing authority.

Every published Atlas dataset must identify enough immutable provenance to reproduce and roll back it, including source world/content revision, producer/export revision and artifact digest.

### 10. Meta authority is narrow, neutral and single-source

`Oteryn` is a coordination/contract plane, not a giant monorepo, source aggregator or schema mirror.

It may own:

- ecosystem repository topology;
- cross-repository ADRs whose authority genuinely spans multiple products;
- machine-readable repository/release manifests;
- compatibility matrices;
- ecosystem release manifests pinning exact SHAs/tags/artifact/image digests;
- cross-repository release/integration E2E orchestration;
- global governance that genuinely applies across repositories.

It must not own or copy as normative duplicates:

- `protocol-oteryn` IDL/schema owned by Game;
- Platform API/GameAuth/Gateway provider schemas owned by Platform;
- Atlas browser/runtime implementation;
- Game canonical world/content schema;
- component-local architecture or CI implementation.

Provider schemas stay with their provider. Meta records discovery, exact version identities and compatible combinations.

Git submodules are not the canonical composition mechanism. Use immutable SHAs, versions and artifact/image digests in manifests.

The reviews recommend demand-triggered physical creation of the meta repository. That trigger is now materially present: ecosystem topology, compatibility/release manifests, Game <-> Atlas contract coordination and cross-repository E2E all require a neutral future home. **Meta creation is therefore architecture-ready but remains a separately authorized repository/organization migration operation.**

### 11. Platform -> Atlas browser integration preserves release and trust independence

Platform owns public discovery/entry policy for the Map capability. Atlas owns its application/runtime/assets/release.

A Platform build must not normally vendor or rebuild an independently released Atlas application merely to expose `/map`.

Default browser trust boundary for independently released Atlas executable code is a distinct origin/subdomain, for example conceptually:

```text
Platform origin -> Portal / Identity / Account / GameAuth
Atlas origin    -> independently released Atlas application
```

The exact hostname is deferred.

Platform may expose a stable `/map` entry/alias/redirect/deep-link gateway to that Atlas origin.

Same-origin Atlas JavaScript is allowed only by an explicit future decision that treats Atlas executable code as fully trusted Platform-origin application code and subjects it to equivalent application-security, CSP, dependency, release, incident-response and review governance. Reverse-proxy credential/header stripping alone is **not** browser security isolation for same-origin JavaScript.

### 12. Platform and Atlas remain independent release/failure domains

Atlas and Platform must be independently releasable and independently rollbackable.

Atlas publication failure, incompatible Atlas release or Atlas outage must not make Portal, Identity, Accounts or GameAuth unavailable.

Platform may truthfully degrade/redirect/disable the Map entry when Atlas is unavailable, but must not fabricate availability or couple core Platform startup to an Atlas build.

### 13. Cross-repository E2E is risk-proportional, not a tax on every local PR

Recommended validation placement:

- component-local PR: local focused/unit/integration/E2E selected by changed risk;
- contract-affecting PR: producer/consumer fixtures and targeted cross-repository compatibility proof;
- protected `main` / scheduled: selected wider ecosystem journeys;
- release candidate: complete named ecosystem manifest and full required cross-repository E2E.

Game retains native Client/Server/protocol test harness ownership. Platform retains Platform-local contract/integration evidence. Atlas retains Atlas-local ingestion/UI evidence. Meta composes immutable product evidence; it does not relocate every test implementation.

### 14. Canary and otclient are explicitly non-normative legacy sources

For target repository architecture:

```text
blakinio/canary   -> LEGACY / TRANSITIONAL / MIGRATION REFERENCE
blakinio/otclient -> LEGACY / MIGRATION / REFERENCE
```

They are excluded from architecture approval/quorum and cannot veto the four-repository target.

They may still supply bounded migration evidence such as tests, fixtures, behavior knowledge, compatibility observations or implementation ideas. Every reused element must be revalidated against the target owner and target contract; legacy placement is never itself evidence of target ownership.

After Atlas extraction and remaining dependency reconciliation, `blakinio/Otheryn` also becomes legacy/migration/reference rather than a target product repository.

### 15. Physical migration must not become a competing product programme

Logical ownership is frozen by this ADR, but physical repository operations must be demand-driven and bounded.

Do not make repository renames/transfers/extraction a blocker for the first complete native Game vertical slice unless the current repository coordinate itself prevents safe delivery.

Preferred migration waves:

#### Wave 0 — architecture freeze; no physical movement

- this ADR is canonical temporary cross-repository topology authority;
- define/maintain extraction seams and provider ownership;
- keep product implementation moving.

#### Wave 1 — make Game internally extraction-safe

- enforce dependency directions around protocol/domain/world/compiler/Studio;
- preserve path-proportional CI;
- preserve independent release identities;
- inventory repository-coordinate references and provenance before rename.

#### Wave 2 — bootstrap organization/meta when separately authorized

- create organization/team/governance baseline;
- create `Oteryn` meta only with its real manifest/compatibility/ADR workload;
- do not duplicate Platform ADR 0041 as a second Accepted authority.

The first accepted meta topology ADR must explicitly state something equivalent to:

```text
Supersedes: blakinio/Oteryn-Platform ADR 0041
```

and Platform ADR 0041 must then be marked superseded for ecosystem scope.

#### Wave 3 — transfer/rename current product repositories when bounded

- transfer/rename `blakinio/Oteryn-v2` -> `Oteryn-Game` while preserving history and provenance;
- transfer `Oteryn-Platform` without splitting Portal/Identity/Gateway;
- update exact owner/repository references, Actions/reusable-workflow references, package/image/release metadata and manifests deliberately.

Do not combine these moves with unrelated runtime refactors.

#### Wave 4 — refactor legacy Atlas ownership before extraction

Using merged Otheryn PR #407 as evidence:

- isolate Game-owned OTBM/legacy/content interpretation;
- isolate Atlas-owned viewer/index/publication code;
- split or rewrite mixed boundary hotspots;
- preserve exact source-path/history mapping.

#### Wave 5 — create `Oteryn-Atlas` through selective history-preserving extraction

Prefer history preservation for Atlas-owned source where feasible, but do not import Game-owned legacy/world semantics merely to preserve a convenient subtree.

#### Wave 6 — prove Game -> Atlas v1

- deterministic full export;
- public allowlist;
- producer fixtures;
- consumer parsing/limit proof;
- exact digest/provenance;
- rollback to a previous immutable dataset.

#### Wave 7 — integrate the public Map product

Only after Atlas has an independent releasable runtime and stable consumer contract should Platform activate the chosen browser-origin/entry integration.

### 16. Large binary storage is a storage decision, not a repository-ownership decision

Game owning canonical content/assets does not require every large binary source, generated bundle, cache, audio/image artifact or release package to live in ordinary Git history.

Do not create `Oteryn-Content` merely to solve storage mechanics.

Keep canonical schema, manifests, structured authored source relationships, provenance and validation authority with Game; choose Git/LFS/object storage/package registry/release artifacts later based on measured size, authoring and licensing requirements.

### 17. Future repository extraction is evidence-triggered

Potential future splits such as Client, Protocol, Studio, Content, Identity or Gateway require a new accepted decision based on observed friction/ownership/lifecycle evidence.

Design stable seams now; do not pre-split source repositories in anticipation of hypothetical teams or services.

## Reconciled decision matrix

| Proposed repository / source | Current disposition | Rationale |
|---|---|---|
| `Oteryn` | TARGET META; create when separately authorized | Neutral ecosystem coordination/manifest/compatibility/ADR workload now exists. |
| current `Oteryn-v2` | RENAME/TRANSFER -> `Oteryn-Game` when migration is bounded | Canonical native Game product lineage; high Client/Server/Protocol/world cohesion. |
| `Oteryn-Platform` | KEEP / transfer when organization migration is authorized | Correct web/application/control-plane boundary. |
| `Oteryn-Atlas` | CREATE via refactor-first selective extraction | Real independent derived map product; current legacy code is mixed. |
| `Oteryn-Portal` | DO NOT CREATE | Portal remains Platform module. |
| `Oteryn-Identity` | DO NOT CREATE | Identity remains Platform module. |
| `Oteryn-Login` | DO NOT CREATE | No separate password/login-server target; GameAuth/Gateway/admission responsibilities are already split semantically. |
| `Oteryn-Gateway` | DO NOT CREATE NOW | Separate deployable does not justify source-repo split. |
| `Oteryn-Client` | DO NOT CREATE NOW | Atomic Client/Server/Protocol changes remain high value. |
| `Oteryn-Server` | DO NOT CREATE NOW | Native authoritative Server remains in Game source repo. |
| `Oteryn-Protocol` | DO NOT CREATE NOW | Highest drift risk if prematurely split. |
| `blakinio/canary` | LEGACY ONLY | Transitional/reference source; non-normative for target topology. |
| `blakinio/otclient` | LEGACY ONLY | Migration/reference source; non-normative for target topology. |
| `blakinio/Otheryn` | MIGRATION SOURCE, then LEGACY | Source of current Atlas and legacy semantics; not a target repository after extraction. |

## Consequences

### Positive

- every target product perspective converges on one durable repository topology;
- Client/Server/Protocol/world atomicity is preserved without forcing one release version;
- Platform retains coherent Identity/Gateway/security ownership;
- Atlas gains a real independent release/product boundary without becoming a second world authority;
- Game -> Atlas becomes deterministic, public-safe and reproducible;
- future meta authority is narrow and cannot silently duplicate provider schemas;
- Atlas browser security is based on origin/trust boundaries rather than repository names or proxy assumptions;
- legacy Canary/otclient architecture no longer creates false governance vetoes;
- physical migration can proceed without starving native runtime delivery.

### Costs

- Game and Atlas require an explicit new export contract and producer/consumer validation;
- current legacy Atlas code requires responsibility refactoring before extraction;
- independent release identities and ecosystem manifests add release engineering work;
- a separate Atlas browser origin can require explicit auth/deep-link/CORS/CSP design if future authenticated Atlas capabilities are introduced;
- meta handoff requires one deliberate supersession event rather than copying current Platform architecture files.

## Explicitly deferred decisions

This ADR intentionally leaves the following to bounded follow-up work:

- exact GitHub organization handle and plan;
- exact date of repository transfer/rename operations;
- exact Game -> Atlas schema bytes, encoding, compression and manifest layout;
- exact Atlas hostname/domain/CDN/object-storage strategy;
- exact Atlas language/framework after migration;
- exact legacy source path set and `git filter-repo` command sequence;
- exact large-binary storage technology;
- exact authentication model for any future private/personalized Atlas feature;
- exact numerical cache/retention/size limits;
- exact retirement dates of legacy repositories.

## Rejected alternatives

- require Canary or otclient approval before the target architecture can proceed;
- use legacy repository layout as target ownership proof;
- split Client, Server or `protocol-oteryn` now;
- split Identity or Gateway only because they are security-sensitive or separately deployable;
- make future meta the normative copy of provider schemas;
- use Git submodules as the primary ecosystem composition model;
- make Atlas consume OTBM, canonical World Project or Game database tables directly;
- use a synchronous live Game API as the primary Atlas dataset path;
- copy/filter the entire current `tools/otbm_atlas` namespace into Atlas unchanged;
- make same-origin independently released Atlas JavaScript the default security model;
- force one SemVer/release train across all deliverables in `Oteryn-Game`;
- perform all repository moves before native runtime work may continue.

## Supersession and future authority handoff

ADR 0040 is superseded by this ADR for ecosystem repository topology and Atlas boundary scope.

This ADR is stored in `blakinio/Oteryn-Platform` only because the future neutral meta repository does not yet exist.

When the future `Oteryn` meta repository becomes canonical, it must take authority through explicit supersession, not duplication:

1. create an accepted meta ADR with a stable canonical identity;
2. state that it supersedes `blakinio/Oteryn-Platform` ADR 0041;
3. preserve links to ADR 0040, ADR 0041 and the merged review evidence;
4. mark Platform ADR 0041 superseded for ecosystem scope;
5. leave Platform-local architecture and provider contracts in Platform;
6. leave Game-local architecture/protocol/world contracts in Game;
7. leave Atlas-local runtime/consumer architecture in Atlas.

There must never be two normative copies of the same ecosystem ADR.

## References

### Current temporary ecosystem authority

- ADR 0040 — superseded initial topology decision
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/adr/README.md`

### Oteryn-v2 / future Game review evidence

- `blakinio/Oteryn-v2` PR #278 — first-pass ecosystem topology review (`ACCEPT_WITH_CHANGES`)
- `blakinio/Oteryn-v2` PR #280 — senior developer/programmer/project-manager second pass (upholds `ACCEPT_WITH_CHANGES`)
- `docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_REVIEW_2026-08-15.md` in Oteryn-v2
- `docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_SENIOR_DEV_PM_SECOND_PASS_2026-08-15.md` in Oteryn-v2
- Oteryn-v2 ADR-0002 — repository ownership/native client migration
- Oteryn-v2 ADR-0005 — native world format, Studio and legacy conversion

### Platform review evidence

- `blakinio/Oteryn-Platform` PR #1100 — ADR 0040 Platform review (`ACCEPT_WITH_CHANGES`)
- `docs/architecture/reviews/OTERYN_ECOSYSTEM_REPOSITORY_TOPOLOGY_PLATFORM_REVIEW_2026-08-15.md`
- ADR 0031 — native Oteryn-v2 integration and Legacy Canary compatibility boundary
- ADR 0036 — deferred general PlatformAPI activation

### Legacy Atlas extraction evidence

- `blakinio/Otheryn` PR #407 — OTBM Atlas extraction review (`EXTRACTABLE_WITH_REFACTOR`)
- `docs/architecture/OTERYN_ATLAS_EXTRACTION_REVIEW_2026-08-15.md` in `blakinio/Otheryn`

### Owner classification

- repository-owner decision, 2026-08-15: Canary and otclient are legacy/reference only and are excluded from normative target-topology review
