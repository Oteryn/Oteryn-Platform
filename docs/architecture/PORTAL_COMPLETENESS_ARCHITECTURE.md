# Oteryn Portal Completeness Architecture

## Status

**CURRENT — focused portal-completeness architecture.**

This document owns the current architectural assessment and completion boundary for the Oteryn web portal. It complements the older phased proposal in `PUBLIC_WEBSITE_EXPANSION_PLAN.md`, current module ownership in `MODULE_CATALOG.md`, the native Account Center boundary in ADR 0030, and the player-tools boundary in `PLAYER_COMPANION_ARCHITECTURE.md`.

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

`PlatformAPI` remains planned as an adapter over module services, not a second business-logic implementation.

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

### Search and discoverability

Character search remains a separate exact-name game-data function. A later federated content search may cover:

- news and managed pages;
- announcements/events;
- Wiki;
- Game Catalog;
- approved player-tool references.

The search adapter does not take ownership of source content and must preserve localization, publication, permissions and dependency-failure behavior.

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
| PublicPortal Today / command-centre composition | PLANNED P1 composition over existing owners; no new truth module |
| Native Character Portfolio / Account Center v2 | ARCHITECTURE ACCEPTED; implementation/migration gated |
| Wiki and structured Game Catalog | KEEP/EVOLVE; close expected-content inventories |
| Typed server-specific system definitions | DISCOVERY/EVOLVE under GameCatalog with Wiki/LiveOps ownership split |
| PlayerCompanion P0 tools | PLANNED; deliver as separate vertical slices after the required ownership/context boundary is available |
| PlayerCompanion owner-private tracking/routines | PLANNED P1 inside ProgressTracker; Notifications delivery-only |
| LiveOps and service history | PLANNED |
| Public World Hub | PLANNED P1 when authoritative world/status/history inputs exist |
| Platform API for concrete first-party consumers | PLANNED |
| Houses, guild wars and expanded leaderboards | DISCOVERY |
| Federated content search | DISCOVERY |
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

The same minimum-module rule applies to the 2026-08-08 refinements: `Today` remains PublicPortal composition, personal tracking remains PlayerCompanion, server-system definitions remain GameCatalog, and the World Hub remains a composition of world/status projections. None justifies a new deployable service by itself.

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
12. typed server-specific systems do not blur GameCatalog deterministic definition, Wiki editorial explanation and LiveOps current-state ownership;
13. exact production edge, observability, backup/restore/rollback and smoke evidence passes when claiming production readiness;
14. required exact-head CI passes and all related PR/task ownership is terminal.

## Current verdict

- **Foundation quality:** sound and scalable.
- **Need for architectural rewrite:** no.
- **Need for architectural improvement:** yes, through bounded composition/ownership refinements and completion gates rather than new services.
- **Native Character Portfolio ownership:** accepted through ADR 0030; runtime implementation not yet authorized by this architecture package.
- **Today/command-centre ownership:** PublicPortal composition over existing bounded sources; no new truth module.
- **Owner-private tracking ownership:** PlayerCompanion.ProgressTracker; Notifications remains delivery-only.
- **Server-specific system definition ownership:** GameCatalog for structured deterministic definition, Wiki for editorial explanation, LiveOps for current state.
- **Portal topic globally complete:** no.
- **Player-tools architecture defined:** yes, through ADR 0025 and `PLAYER_COMPANION_ARCHITECTURE.md`.
- **Player tools implemented:** no claim; module status remains `PLANNED`.

## References

- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/DATA_OWNERSHIP.md`
- `docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- ADR 0025
- ADR 0030
- `docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md`
- `docs/agents/reports/OTERYN-20260808-portal-product-delta.md`
- Issues #302, #365, #488, #489 and #490