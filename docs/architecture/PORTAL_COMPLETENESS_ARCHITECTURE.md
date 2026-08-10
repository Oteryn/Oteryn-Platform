# Oteryn Portal Completeness Architecture

## Status

**CURRENT — focused portal-completeness architecture.**

This document owns the current architectural assessment and completion boundary for the Oteryn web portal. It complements the older phased proposal in `PUBLIC_WEBSITE_EXPANSION_PLAN.md`, current module ownership in `MODULE_CATALOG.md`, the native Account Center boundary in ADR 0030, the player-tools boundary in `PLAYER_COMPANION_ARCHITECTURE.md`, and the public federated-search boundary in ADR 0033 / `FEDERATED_SEARCH_ARCHITECTURE.md`.

Implementation availability, capability completeness, environment evidence and production activation remain separate facts.

## Decision summary

The current foundation is sound:

- retain the Laravel modular monolith;
- retain server-rendered Blade as the baseline public/application UI;
- retain the Platform/game-domain repository and ownership split;
- retain explicit operation-specific contracts for shared writes;
- use ADR 0030's Accounts-owned Character Portfolio boundary for native authenticated Account Center composition;
- do not replace the portal with WordPress, a separate SPA or default microservices;
- extract a service only after measured independent scaling, isolation, lifecycle or ownership need.

The portal architecture is **not globally exhausted or product-complete**. It is complete only at the level of foundational direction and already delivered bounded modules. Remaining completion work must be intentionally implemented, deferred or rejected.

The 2026-08-08 benchmark delta does not change this foundation. It sharpens four composition/ownership directions without adding a new top-level service: a first-party `Today`/command-centre experience, owner-private tracking/routines inside `PlayerCompanion`, typed server-specific system definitions under `GameCatalog`, and a future public World Hub composed from existing world/status owners.

The 2026-08-09 federated-search decision resolves another previous discovery gap without adding a module: public cross-content search is a `PublicPortal` application capability over source-owned public search/query interfaces, while exact-name character lookup remains a separate `PublicGameData` search product and any later dedicated search index remains rebuildable derived infrastructure.

## Current module reconciliation

The codebase contains and the canonical module catalog recognizes:

- `PublicPortal`;
- `Announcements`;
- `Events`;
- `Downloads`;
- `PublicGameData`;
- `CMS`;
- `Wiki`;
- `GameCatalog`;
- `Support`;
- `Identity`, `Accounts` and `Characters`;
- `CharacterProfiles` as an implemented top-level `AVAILABLE` Platform-owned presentation/privacy preference subdomain;
- `Wallet` and `Marketplace`;
- `Admin`, `Audit` and operational boundaries.

ADR 0025 additionally accepts:

- `PlayerCompanion` as the planned player-tools boundary;
- `LiveOps` as the planned time-sensitive world/service-state boundary.

ADR 0030 accepts the native Account Center responsibility split:

- `Accounts` owns authenticated Character Portfolio composition;
- `Characters` owns Platform-side orchestration of approved character commands;
- Oteryn-v2 Character Authority owns canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, character lifecycle and native mutation outcomes;
- `PublicGameData` remains public/general projection rather than authenticated ownership authority;
- `CharacterProfiles` remains presentation/privacy state and targets canonical `CharacterId` after a separately authorized additive migration.

ADR 0033 keeps federated public content search inside `PublicPortal` rather than creating a new Search/Discovery module. Source modules retain search eligibility, publication, localization, source-local ranking and canonical route ownership. `PlatformAPI` remains planned as an adapter over module application services, not a second business-logic implementation.

The accepted federated-search dependency direction is a target, not a claim that all existing homepage composition code is already acyclic. `Announcements` and `Events` currently import `App\PublicPortal\PublicContentState`; that reverse compatibility edge must be removed before those modules are onboarded behind a new PublicPortal federated-search provider dependency.

## Architectural gaps that remain material

### Complete expected-content inventories

Wiki and Game Catalog cannot be called content-complete merely because their engines, routes and sample content exist.

Required machine-readable inventories include, where applicable:

- categories and articles;
- localized slugs and EN/PL parity;
- internal links and references;
- items, creatures, bosses, NPCs, spells, quests and achievements;
- entity fields, relations and media/fallbacks;
- expected counts and supported version/profile boundaries;
- source/provenance and effective dates.

Open Issues #488 and #489 remain evidence that these inventories and related failure/portability coverage are incomplete.

### Wiki/editorial-media failure paths

Issue #365 retains reproduced Wiki publication/thumbnail behavior that must be classified and repaired from current exact evidence. No portal-wide completion claim may hide unexplained HTTP 500 media responses.

### Production and public edge

Repository and staging evidence do not prove production correctness. Portal production completion requires exact evidence for:

- deployed release identity;
- DNS, TLS, redirects, HSTS, WAF and origin restrictions;
- private database/cache/game-service paths;
- mail, queues and schedulers;
- structured logs, metrics, alerts and on-call ownership;
- backup, restore and rollback;
- controlled production smoke/E2E.

Issue #490 and the production go-live gate remain controlling evidence routes.

### Client distribution

The available Download Center is a valid foundation. A mature first-party distribution path should separately decide and implement:

- stable/beta channels;
- minimum supported version;
- mandatory-update policy;
- immutable update manifests;
- publisher signatures or equivalent provenance;
- release withdrawal/rollback;
- compatibility with the Rust client updater.

A SHA-256 checksum proves integrity only after the expected digest is trusted; it does not independently prove publisher origin.

### LiveOps

Typed editorial events and announcements must not impersonate runtime state. `LiveOps` should own authoritative projections for:

- maintenance;
- server save;
- current service/world status and freshness;
- status/maintenance history;
- raid or boss schedules when an authoritative producer exists;
- rotating boosted creature/boss or similar systems;
- explicit zero/offline/maintenance/stale/unavailable distinctions.

### `Today` / command-centre composition

A mature portal benefits from one compact first-party surface that answers “what matters now?” without forcing the player to visit unrelated pages. This is a **PublicPortal composition**, not a new source-of-truth module.

Conceptual composition:

```text
PublicPortal Today view model
  <- Announcements / Events / CMS editorial presentation
  <- LiveOps current schedules, rotations, maintenance and runtime freshness
  <- PublicGameData public world/player/community projections
  <- PlayerCompanion owner-private routines, goals and tracked signals when authenticated
```

Rules:

- every card/signal retains the source owner's freshness, applicability and confidence semantics;
- an unavailable dependency is rendered as unavailable/stale, never fabricated `0`, `offline`, `none` or “completed”;
- personalized cards remain owner-private and are omitted rather than inferred for guests;
- PublicPortal may prioritize and arrange data, but cannot reinterpret an editorial item as runtime truth or a recommendation as deterministic fact;
- the homepage/template system may progressively expose this composition without creating a second dashboard domain.

#### Today representation privacy and cache isolation

A correct owner check at composition time is necessary but not sufficient. Privacy must also survive materialization and cache reuse.

- a guest-only representation containing only public inputs is `PUBLIC_GUEST`;
- if any owner-private PlayerCompanion routine, goal, tracking preference, private derived signal or other owner-private input is included **or influences bytes, inclusion, ordering, counts, badges, recommendations, suppression or eligibility**, the complete materialized Today representation is `PRIVATE_PERSONALIZED`;
- if the required owner/privacy/authorization context cannot be proven, personalized composition fails closed instead of falling back to a possibly stale private representation;
- `PRIVATE_PERSONALIZED` Today output is not eligible for shared/public response caches, CDN/proxy shared caches or anonymous fragment caches; later implementation must use shared-cache bypass plus private/non-shareable semantics, with `no-store` or an equivalently strong policy when no deliberately owner-scoped representation cache exists;
- guest and authenticated/private variants do not share a cache identity merely because route, query, world, profile or public cards match;
- anonymous requests never inherit private fragments or private-influenced presentation from a prior authenticated request;
- cache metadata such as a generic authenticated flag, role name or `Vary` dimension is not by itself sufficient proof of owner isolation.

If a future private server-side Today representation cache is adopted, its key/fence must bind the authenticated owner and every revision required to prove that cached authorization/privacy remains equivalent. The semantic identity must cover, where applicable:

```text
owner_identity
session/authentication_generation or equivalent session fence
authorization_revision
privacy_revision
account/character ownership revision
private PlayerCompanion/tracking/routine revision
world/profile/season/applicability dimensions
representation/schema version
```

Route/query/world/profile-only cache identity, a generic `authenticated=true` bit, or public-source revisions alone are forbidden for private output.

Prior personalized representations must be fenced/invalidate-before-reuse on at least:

- logout or authentication/session invalidation;
- session replacement/authentication-generation change;
- account/character ownership change affecting private context;
- authorization or privacy tightening;
- deletion/change of owner-private routines, goals, tracking preferences or signals represented in the output;
- representation/schema incompatibility or applicability change.

Public sub-fragments may be cached independently only when their composition boundary proves that owner-private inputs cannot alter the public fragment's bytes, inclusion, ordering, counts, semantic meaning, cache key or cache eligibility. Combining such a public fragment with any owner-private fragment still yields a `PRIVATE_PERSONALIZED` combined response; shared-cache eligibility never propagates upward from the public fragment.

Before any personalized Today implementation may be called complete, exact negative-path evidence must prove at least:

- User A → User B equivalent-cache request isolation;
- authenticated → guest transition isolation;
- guest → authenticated transition isolation;
- logout and session replacement fencing;
- account/character ownership-change fencing;
- stale private fragment after tracking/goal/signal deletion or revision;
- privacy/authorization tightening while an older personalized representation exists;
- CDN/reverse-proxy/shared-cache simulation showing no private bytes cross principals;
- public-subfragment + private-fragment composition preserving private classification of the final response.

ADR 0032 is the controlling focused decision. This architecture does not claim a current Today route/cache leak and does not authorize cache middleware, CDN/proxy configuration, response headers, cache storage or production activation.

### Native Character Portfolio / Account Center v2

The architectural owner is resolved by ADR 0030; runtime implementation remains intentionally separate.

The native target must replace new reliance on Canary numeric identifiers with canonical cross-boundary identities and an authorized game-owned projection/command boundary:

```text
Platform Identity / AccountId
  -> Accounts / Character Portfolio composition
       -> authorized Oteryn-v2 Character Authority projection
       -> Platform CharacterProfiles preferences
       -> effective Account Center state

Characters
  -> versioned Oteryn-v2 Character Authority commands
```

Required implementation decisions and evidence before native activation include:

- the concrete `Accounts` application interface/read model for Character Portfolio;
- typed portfolio success/empty/stale/unavailable/ambiguous/incompatible semantics;
- authoritative revision/observation and freshness policy;
- the versioned Character Authority projection adapter;
- command orchestration and idempotent mutation receipt/result handling;
- additive `canary_player_id` -> `CharacterId` preference migration, backfill, rollback and removal gates;
- game-owned versus Platform-owned capability/denial semantics;
- entitlement-to-game capability exchange if product policy needs it;
- integration, migration, negative-path and real E2E proof before activation.

PlayerCompanion P0 should not create a parallel Canary-numeric ownership model while this native boundary is unimplemented. It should consume owned-character context through `Accounts` and consume game facts/rules through its accepted GameCatalog/PublicGameData/LiveOps/GameAnalytics dependencies.

### Multi-world, profiles and seasons

The launch may use one world, but architecture must preserve explicit dimensions for:

- game profile;
- ruleset version;
- catalog snapshot;
- world;
- season;
- effective period.

These dimensions apply to portal URLs, cache keys, events, LiveOps, Game Catalog and PlayerCompanion. A single-world launch must not create irreversible global assumptions.

`ChannelId` is not a durable Character Portfolio identity dimension under ADR 0030. Channel is topology/runtime placement and belongs only in a separately justified runtime/session projection.

### World Hub

A future public World Hub is justified when multiple world/profile choices or meaningful historical population/activity signals exist. It is a view/composition, not a routing authority.

It may combine:

- public world identity and configured public policy/presentation;
- `PublicGameData` population/community projections;
- `LiveOps` current status, maintenance and service-history projections;
- optional evidence-backed `GameAnalytics` trends such as population/activity distributions.

A World Hub must preserve region/PvP/ruleset/profile/season applicability where relevant, show observation age, and never derive current admission/routing authority from cached portal data.

### Platform API

A versioned first-party API becomes justified by concrete consumers such as:

- Rust client;
- launcher/updater;
- first-party player tools;
- approved administrative tools;
- approved Discord/community integrations.

The API must reuse module application services, authorization, validation and version/freshness semantics. It must not serialize raw database models or recreate formulas and domain decisions.

For owned-character consumers the API must reuse the ADR 0030 Accounts Character Portfolio service rather than exposing `canary_account_id`, `canary_player_id` or raw game-owned tables as a permanent contract.

For federated public content search, PlatformAPI reuses the ADR 0033 `PublicPortal` federated-search application service instead of independently fan-out querying CMS/Wiki/GameCatalog/etc. or recreating cross-source grouping/ranking policy.

### Search and discoverability

ADR 0033 resolves the ownership architecture for first-party federated public content search.

The accepted capability is:

```text
PublicPortal FederatedSearch
  -> CMS public news/pages
  -> Announcements public eligible records after reverse-edge cleanup
  -> Events public eligible records after reverse-edge cleanup
  -> Wiki published localized search
  -> GameCatalog active/verified/public-safe entities
  -> later explicitly public PlayerCompanion artefacts only after a separate indexability contract
```

Rules:

- `PublicPortal` owns the public query contract, provider orchestration, deterministic grouping/interleaving, normalized result envelope, partial-failure semantics and public search UX;
- each source module owns public eligibility/publication, localization, source-local relevance and canonical source URL semantics;
- target provider adapters call bounded source-module application queries and source modules must not depend on PublicPortal search/presentation types;
- current Announcements/Events imports of `PublicPortal\PublicContentState` are explicit compatibility debt; their removal is a provider-onboarding prerequisite, and a new opposite PublicPortal search dependency must not be added until the reverse edge is gone;
- raw cross-module model/table access is forbidden;
- PublicGameData exact-name character search remains a separate search product/vertical and is not silently converted into fuzzy people discovery;
- Marketplace search/filtering remains Marketplace-owned;
- private PlayerCompanion, Support, Admin, Audit, Identity and Accounts data is excluded from public federation;
- raw provider relevance scores are not treated as globally comparable; initial delivery should prefer grouped verticals or deterministic rank-position/source-quota interleaving;
- provider outage differs from zero healthy results, and `COMPLETE`, `PARTIAL`, `UNAVAILABLE` and invalid-query states remain distinct;
- raw search text is not an ordinary log field or metric label;
- arbitrary search-result pages are normally `noindex`; canonical source pages remain the indexable content;
- no external search engine is an architecture prerequisite.

A later dedicated search engine/index is allowed only as a rebuildable derived projection carrying source identity/revision/locale and explicit index generation/tombstone/stale-lag semantics. It never becomes source truth.

Focused details are in `FEDERATED_SEARCH_ARCHITECTURE.md`.

### Typed server-specific systems

Server-specific systems should be discoverable as typed product concepts rather than buried inside arbitrary CMS prose when they carry deterministic rules, version applicability or live state.

The ownership split is:

```text
GameCatalog
  -> stable system definition, version/applicability and authoritative deterministic relations/parameters

Wiki
  -> explanatory/editorial guide and strategy content

LiveOps
  -> current schedule, active rotation, season/runtime state and freshness

PlayerCompanion
  -> planner/calculator/tracker consuming the owned facts above
```

Examples may include task/progression systems, seasonal systems, server-specific progression mechanics, rotating bonuses or event-like systems. This does not create a generic plugin framework or a new `GameSystems` deployable module.

If no authoritative structured source exists, a topic may remain Wiki/editorial only. The portal must not promote prose or manually copied values to deterministic/live truth.

### Community scope

Tibia and mature OTS portals demonstrate possible surfaces such as:

- houses;
- guild wars;
- broader leaderboards and kill statistics;
- polls;
- forums;
- team/staff pages;
- service history.

These are product inventory inputs, not automatic launch requirements.

Recommended baseline:

- retain Discord plus the implemented Support module initially;
- defer a native forum until durable discussion/search/moderation need is proven;
- treat Polls as a separate bounded module;
- require contracts before guild/house mutations;
- keep public read expansions allowlisted, privacy-aware and read-only.

## Player utility direction

The portal should become more than an account website and Wiki. The accepted `PlayerCompanion` direction combines:

- TibiaPal-like player utilities;
- automatic authorized character context through the ADR 0030 Accounts Character Portfolio boundary;
- Oteryn-specific versioned rules and catalog data;
- optional Game Analytics evidence;
- shared application services for portal and client.

P0 candidate slices are:

1. Loot Split and private Session Analyzer.
2. Hunt Finder/Advisor.
3. Equipment Explorer/Comparison.
4. Character Build Planner.
5. Charm/Perk/Proficiency Planner.
6. Quest and Access Tracker.
7. EXP and Training calculators.
8. Validated shareable builds.

The 2026-08-08 benchmark delta adds an explicit P1 direction for owner-private tracked entities/routines/change signals under `PlayerCompanion.ProgressTracker`. `Notifications` remains a delivery boundary rather than subscription/rule authority. Community-contributed hunt evidence remains P2/discovery because provenance, sampling bias, moderation and privacy need a separate contract.

P1 and P2 scope is defined in `PLAYER_COMPANION_ARCHITECTURE.md` and the dated benchmark reports.

## Product disposition baseline

The following statuses are planning decisions, not implementation authority.

| Capability family | Disposition |
|---|---|
| Existing public/account/admin foundations | KEEP/EVOLVE |
| PublicPortal Today / command-centre composition | PLANNED P1 composition over existing owners; private/mixed output requires ADR 0032 cache isolation before implementation |
| Native Character Portfolio / Account Center v2 | ARCHITECTURE ACCEPTED; implementation/migration gated |
| Wiki and structured Game Catalog | KEEP/EVOLVE; close expected-content inventories |
| Typed server-specific system definitions | DISCOVERY/EVOLVE under GameCatalog with Wiki/LiveOps ownership split |
| PlayerCompanion P0 tools | PLANNED; deliver as separate vertical slices after the required ownership/context boundary is available |
| PlayerCompanion owner-private tracking/routines | PLANNED P1 inside ProgressTracker; Notifications delivery-only |
| LiveOps and service history | PLANNED |
| Public World Hub | PLANNED P1 when authoritative world/status/history inputs exist |
| Platform API for concrete first-party consumers | PLANNED |
| Houses, guild wars and expanded leaderboards | DISCOVERY |
| Federated content search | ARCHITECTURE ACCEPTED / PLANNED under PublicPortal; Announcements/Events onboarding gated on reverse-edge cleanup; no new module or mandatory external search engine |
| Interactive map and route planning | DEFER to separate programme |
| Community-contributed hunt evidence | DEFER to bounded P2/discovery with provenance/privacy/moderation contract |
| Market-price/economy analytics | DEFER until authoritative data and privacy policy exist |
| Public build profiles/social comparison | DEFER |
| Native forum | DEFER; Discord + Support is the initial recommendation |
| Polls | DEFER to bounded module |
| Provider payments, webshop and entitlements | Separate gated commerce programme |
| Roulette/chance-based commercial systems | REJECT BY DEFAULT pending explicit product/legal decision |

## Code organization direction

New module implementation should converge on a consistent physical convention rather than mixing indefinitely:

```text
app/<Module>/Domain
app/<Module>/Application
app/<Module>/Infrastructure
app/<Module>/Http
routes/modules/<module>.php
resources/views/<module>
tests/<appropriate layer>/<Module>
```

This is a forward convention, not authority for an unrelated repository-wide move. Existing paths should be migrated only through bounded tasks with proven benefit, stable namespace compatibility and complete tests.

Cross-module access should use application interfaces/query objects rather than arbitrary model imports or raw table access.

ADR 0030 specifically keeps Character Portfolio as an `Accounts` application/read boundary rather than creating a new `CharacterPortfolio` deployable module. `CharacterProfiles` remains a small Platform-owned presentation/privacy subdomain; its top-level module-catalog classification is reconciled without forcing a repository-wide namespace rename. Canonical `CharacterId` preference migration remains separately authorized and pending under ADR 0030.

The same minimum-module rule applies to the 2026-08-08/09 refinements: `Today` remains PublicPortal composition, personal tracking remains PlayerCompanion, server-system definitions remain GameCatalog, World Hub remains a composition of world/status projections, and federated public content search remains a PublicPortal application capability. None justifies a new deployable service by itself.

## Portal completion gate

The architecture subject may be considered closed for a named release scope only when all of the following are true:

1. the release has an explicit public, account, admin, player-tools and non-UI capability inventory;
2. every benchmark capability is `IMPLEMENT`, `DEFER` or `REJECT`, with an owner and rationale;
3. `MODULE_CATALOG.md` and actual routes/modules agree;
4. Wiki and Game Catalog expected-content inventories pass;
5. no critical/high/material-medium portal audit finding remains open for the release scope;
6. each implemented user-facing capability has backend, frontend, real integration, states, localization, accessibility/responsive evidence and real E2E;
7. client distribution has an accepted provenance/update policy for the release;
8. LiveOps sources and stale/unavailable semantics are explicit for every displayed runtime claim, including composed Today/World Hub cards;
9. multi-world/profile/season applicability is explicit or validly not applicable;
10. native Character Portfolio consumers use the accepted AccountId/CharacterId boundary or are explicitly compatibility-scoped; no new permanent Canary-numeric ownership contract is introduced;
11. personalized tracking/subscription behavior, when implemented, has explicit source, privacy, retention, refresh, abuse and notification-delivery semantics;
12. personalized Today, when implemented, classifies any private-influenced mixed response as owner-private, bypasses shared/CDN/anonymous caches, uses owner/security-revision fencing for any private representation cache, fences logout/session/ownership/privacy/tracking transitions and passes two-user/auth-transition/CDN/private-fragment negative-path evidence;
13. typed server-specific systems do not blur GameCatalog deterministic definition, Wiki editorial explanation and LiveOps current-state ownership;
14. federated search, when implemented, preserves source publication/localization/privacy/canonical identity, distinguishes partial failure from zero results, keeps character enumeration separate, has bounded query privacy/rate/cache/index semantics, and does not introduce a PublicPortal/provider module cycle;
15. exact production edge, observability, backup/restore/rollback and smoke evidence passes when claiming production readiness;
16. required exact-head CI passes and all related PR/task ownership is terminal.

## Current verdict

- **Foundation quality:** sound and scalable.
- **Need for architectural rewrite:** no.
- **Need for architectural improvement:** yes, through bounded composition/ownership refinements and completion gates rather than new services.
- **Native Character Portfolio ownership:** accepted through ADR 0030; runtime implementation not yet authorized by this architecture package.
- **Today/command-centre ownership:** PublicPortal composition over existing bounded sources; mixed owner-private output is `PRIVATE_PERSONALIZED` and requires ADR 0032 cache isolation; no new truth module.
- **Owner-private tracking ownership:** PlayerCompanion.ProgressTracker; Notifications remains delivery-only.
- **Server-specific system definition ownership:** GameCatalog for structured deterministic definition, Wiki for editorial explanation, LiveOps for current state.
- **Federated content search ownership:** PublicPortal application capability over source-owned public queries; architecture accepted through ADR 0033, with Announcements/Events provider onboarding gated on existing reverse-edge cleanup; runtime not yet implemented.
- **Portal topic globally complete:** no.
- **Player-tools architecture defined:** yes, through ADR 0025 and `PLAYER_COMPANION_ARCHITECTURE.md`.
- **Player tools implemented:** no claim; module status remains `PLANNED`.

## References

- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/DATA_OWNERSHIP.md`
- `docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`
- ADR 0025
- ADR 0030
- ADR 0032
- ADR 0033
- `docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md`
- `docs/agents/reports/OTERYN-20260808-portal-product-delta.md`
- Issues #365, #488, #489, #490, #935 and #941