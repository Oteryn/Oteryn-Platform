# Oteryn Portal Completeness Architecture

## Status

**CURRENT — focused portal-completeness architecture.**

This document owns the current architectural assessment and completion boundary for the Oteryn web portal. It complements the older phased proposal in `PUBLIC_WEBSITE_EXPANSION_PLAN.md`, current module ownership in `MODULE_CATALOG.md`, and the player-tools boundary in `PLAYER_COMPANION_ARCHITECTURE.md`.

Implementation availability, capability completeness, environment evidence and production activation remain separate facts.

## Decision summary

The current foundation is sound:

- retain the Laravel modular monolith;
- retain server-rendered Blade as the baseline public/application UI;
- retain the Platform/Canary repository and ownership split;
- retain explicit operation-specific contracts for shared writes;
- do not replace the portal with WordPress, a separate SPA or default microservices;
- extract a service only after measured independent scaling, isolation, lifecycle or ownership need.

The portal architecture is **not globally exhausted or product-complete**. It is complete only at the level of foundational direction and already delivered bounded modules. Remaining completion work must be intentionally implemented, deferred or rejected.

## Current module reconciliation

The codebase contains and the canonical module catalog now recognizes:

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
- `Wallet` and `Marketplace`;
- `Admin`, `Audit` and operational boundaries.

ADR 0025 additionally accepts:

- `PlayerCompanion` as the planned player-tools boundary;
- `LiveOps` as the planned time-sensitive world/service-state boundary.

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

### Multi-world, profiles and seasons

The launch may use one world, but architecture must preserve explicit dimensions for:

- game profile;
- ruleset version;
- catalog snapshot;
- world;
- season;
- effective period.

These dimensions apply to portal URLs, cache keys, events, LiveOps, Game Catalog and PlayerCompanion. A single-world launch must not create irreversible global assumptions.

### Platform API

A versioned first-party API becomes justified by concrete consumers such as:

- Rust client;
- launcher/updater;
- first-party player tools;
- approved administrative tools;
- approved Discord/community integrations.

The API must reuse module application services, authorization, validation and version/freshness semantics. It must not serialize raw database models or recreate formulas and domain decisions.

### Search and discoverability

Character search remains a separate exact-name game-data function. A later federated content search may cover:

- news and managed pages;
- announcements/events;
- Wiki;
- Game Catalog;
- approved player-tool references.

The search adapter does not take ownership of source content and must preserve localization, publication, permissions and dependency-failure behavior.

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
- automatic authorized character context;
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

P1 and P2 scope is defined in `PLAYER_COMPANION_ARCHITECTURE.md` and the dated benchmark report.

## Product disposition baseline

The following statuses are planning decisions, not implementation authority.

| Capability family | Disposition |
|---|---|
| Existing public/account/admin foundations | KEEP/EVOLVE |
| Wiki and structured Game Catalog | KEEP/EVOLVE; close expected-content inventories |
| PlayerCompanion P0 tools | PLANNED; deliver as separate vertical slices |
| LiveOps and service history | PLANNED |
| Platform API for concrete first-party consumers | PLANNED |
| Houses, guild wars and expanded leaderboards | DISCOVERY |
| Federated content search | DISCOVERY |
| Interactive map and route planning | DEFER to separate programme |
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

## Portal completion gate

The architecture subject may be considered closed for a named release scope only when all of the following are true:

1. the release has an explicit public, account, admin, player-tools and non-UI capability inventory;
2. every benchmark capability is `IMPLEMENT`, `DEFER` or `REJECT`, with an owner and rationale;
3. `MODULE_CATALOG.md` and actual routes/modules agree;
4. Wiki and Game Catalog expected-content inventories pass;
5. no critical/high/material-medium portal audit finding remains open for the release scope;
6. each implemented user-facing capability has backend, frontend, real integration, states, localization, accessibility/responsive evidence and real E2E;
7. client distribution has an accepted provenance/update policy for the release;
8. LiveOps sources and stale/unavailable semantics are explicit for every displayed runtime claim;
9. multi-world/profile/season applicability is explicit or validly not applicable;
10. exact production edge, observability, backup/restore/rollback and smoke evidence passes when claiming production readiness;
11. required exact-head CI passes and all related PR/task ownership is terminal.

## Current verdict

- **Foundation quality:** sound and scalable.
- **Need for architectural rewrite:** no.
- **Need for architectural improvement:** yes, through the accepted bounded additions and completion gates above.
- **Portal topic globally complete:** no.
- **Player-tools architecture defined:** yes, through ADR 0025 and `PLAYER_COMPANION_ARCHITECTURE.md`.
- **Player tools implemented:** no claim; module status remains `PLANNED`.

## References

- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- `docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md`
- ADR 0025
- Issues #365, #488, #489 and #490
