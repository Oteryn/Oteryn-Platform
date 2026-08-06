# Portal and player-tools benchmark

Date: 2026-08-06
Repository: `blakinio/Oteryn-Platform`
Task: `OTERYN-20260806-player-companion-portal-architecture`
PR: #667

## Purpose and evidence boundary

This report records product research requested by the repository owner. It compares the current Oteryn Platform architecture with public surfaces visible on Tibia, RubinOT and TibiaPal.

The websites are untrusted external research evidence, not architecture authority or implementation proof. Oteryn must not copy external code, text, layouts, branding, icons, artwork or proprietary datasets. Exact Oteryn decisions are recorded separately in accepted ADRs and focused canonical architecture documents.

Evidence was observed on 2026-08-06 from:

- `https://www.tibia.com/`;
- `https://rubinot.net/` and `https://www.rubinot.net/`;
- `https://tibiapal.com/`, including its current tool pages and dated change announcements.

## Current Oteryn baseline

The current repository already provides a broad first-party platform foundation:

- Laravel modular monolith and server-rendered public shell;
- Platform Identity, sessions, recovery and MFA;
- account provisioning and character creation;
- public characters, guilds, highscores, online state, servers and deaths;
- CMS news and pages;
- announcements and events;
- Download Center;
- Wiki and versioned Game Catalog foundations;
- Support/Moderation;
- Wallet and Character Bazaar;
- Admin/RBAC/Audit;
- operations, public-edge and E2E boundaries.

The implementation contains separate route modules for `PublicPortal`, `Announcements`, `Downloads` and `Events`. The canonical module table did not list those boundaries at task start, creating documentation-to-code drift.

## Tibia public portal observations

The official Tibia website exposes a wide information architecture rather than only account creation and highscores. The observed navigation includes:

- latest news, archive and event schedule;
- game introduction, screenshots, features and premium information;
- quickstart, manual and security hints;
- creatures, boostable bosses, spells, achievements, world quests, experience table, maps, genesis and soundtrack;
- characters, worlds, highscores, leaderboards, Wheel of Destiny planner, kill statistics, houses, guilds, polls and feedback;
- world, trade, community, support and guild boards;
- account management, account creation, client download, webshop and account recovery;
- current and historical Character Bazaar auctions plus owner-scoped bids, listings and watches;
- help, rules and legal documents.

This breadth is useful as an inventory of possible product surfaces. It is not a requirement to reproduce every Tibia feature. In particular, forum boards, resellers, fansite programmes and every commerce surface require explicit Oteryn product decisions.

## RubinOT public portal observations

RubinOT demonstrates the breadth commonly expected from a mature OTS portal. Its observed navigation includes:

- news, 2FA, event schedule and Wiki;
- server information, raid calendar and VIP/Loyalty information;
- characters, highscores, last deaths, houses, guilds, guild wars, banishments, wheel planner and polls;
- account management, account creation and account recovery;
- server-specific systems such as linked tasks and castle information;
- auction catalogue, history, bids, seller listings and watches;
- rules, staff, social links and client download.

The useful architectural lesson is to make server-specific systems discoverable and typed. They must not be hidden inside arbitrary CMS pages when they require runtime state, scheduling, permissions or deterministic rules.

## TibiaPal player-tools observations

TibiaPal describes itself as an extension of TibiaLootSplit and focuses on tools that enhance the player's game experience. The current site and dated announcements evidence several categories.

### Planning and build references

Observed capabilities include:

- Weapon Proficiency Planner with perk refinement, shaping/reshaping and URL-encoded shareable builds;
- Proficiency Refinement recommendations by vocation and weapon;
- curated Wheel of Destiny build references and planner links;
- charm planning and charm-damage calculations;
- vocation equipment browsing and filtering, including protection and weapon damage types.

### Hunt and progression guidance

Observed capabilities include:

- hunting-place filtering by vocation, level and relevant weapon/rune/mastery dimensions;
- solo, duo and team-hunt references;
- quest reference filtering by level, solo/team suitability, boss access and hunting-ground access;
- bestiary task/creature guidance with suggested locations;
- weekly task and weekly delivery references;
- boss timers and other recurring-progression helpers.

### Calculators and economy helpers

Observed capabilities include:

- experience and shared-experience calculations;
- exercise-weapon and online/offline training calculations;
- stamina, imbuement and leech calculations;
- loot splitting from session logs;
- profit per hour, damage contribution and saved analysis history;
- house-auction and daily-NPC helpers.

### Maintenance lesson

TibiaPal keeps an explicit `hunting-old` surface labelled as outdated after the 2026 vocation rebalance and earlier update changes. This is strong evidence that player tools need:

- ruleset and game-version binding;
- source and effective-date metadata;
- explicit current/stale/experimental/deprecated status;
- deterministic invalidation after balance changes;
- separation between calculated facts and editorial recommendations.

## Architectural conclusions

### Retain the current foundation

The current Laravel modular monolith remains appropriate. A separate SPA, WordPress deployment or default microservice decomposition would increase operational and authorization complexity without proven need.

### Portal architecture is not globally complete

The foundation is mature, but the complete MMORPG portal subject remains open until:

- the canonical module inventory matches actual modules;
- every benchmark capability is intentionally classified as implement, defer or reject;
- Wiki and Game Catalog expected-content inventories are complete and versioned;
- production edge, restore, observability and exact-environment proof are closed separately;
- player tools, LiveOps, client distribution, Platform API and multi-world/seasons have explicit boundaries;
- open material audit findings are closed or intentionally deferred.

### Adopt a dedicated PlayerCompanion boundary

Player calculators and planning tools must not become miscellaneous Wiki, CMS or controller logic. A dedicated `PlayerCompanion` module should orchestrate:

- deterministic calculators;
- character/build planning;
- hunt discovery and recommendation;
- session analysis;
- progression and goal tracking;
- shareable plans;
- personalized recommendations.

It consumes versioned rules and entities from `GameCatalog`, explanatory content from `Wiki`, public/authorized character projections from `PublicGameData` and evidence-backed aggregates from future `GameAnalytics`. It does not own those sources of truth.

### Add a LiveOps boundary

Maintenance state, server save, service health/history, raid schedules, runtime events, boosted creature/boss and other time-sensitive world state require a separate `LiveOps` boundary. CMS may explain an event but must not impersonate authoritative runtime state.

### Strengthen client distribution

The existing `Downloads` module should evolve from metadata and checksums toward:

- stable/beta channels;
- minimum supported version and mandatory-update policy;
- immutable manifests;
- publisher signatures or an equivalent provenance mechanism;
- rollback/withdrawal of a defective release;
- compatibility with the client updater.

A checksum alone proves byte integrity after the expected value is trusted; it does not independently prove publisher origin.

### Keep API as an adapter

A future versioned `PlatformAPI` may serve the Rust client, launcher, official tools, bots and approved integrations. It must delegate to module application services and must not duplicate domain rules.

### Preserve multi-world and ruleset dimensions

Player tools and portal projections must be keyed where applicable by:

```text
game_profile
ruleset_version
catalog_snapshot
world
season
effective_from
effective_until
```

Single-world launch configuration must not hard-code assumptions that make later worlds, seasonal rules or test realms unsafe to introduce.

## Product disposition matrix

The following is the accepted planning baseline. `PLANNED` does not authorize implementation or production activation.

| Capability | Disposition | Priority | Architectural owner |
|---|---|---:|---|
| Production homepage and public shell | KEEP/EVOLVE | P0 | PublicPortal |
| News, managed pages and legal content | KEEP/EVOLVE | P0 | CMS / LegalCommerce where applicable |
| Announcements and editorial events | KEEP/EVOLVE | P0 | Announcements / Events |
| Client downloads and release metadata | KEEP/EVOLVE | P0 | Downloads |
| Characters, guilds, highscores, online, deaths | KEEP/EVOLVE | P0 | PublicGameData |
| Wiki and structured item/creature catalogue | KEEP/EVOLVE | P0 | Wiki / GameCatalog |
| Loot split and session analyser | PLANNED | P0 | PlayerCompanion.SessionAnalysis |
| Hunt finder/advisor | PLANNED | P0 | PlayerCompanion.HuntAdvisor |
| Equipment explorer and comparison | PLANNED | P0 | PlayerCompanion.BuildPlanner + GameCatalog |
| Character build planner | PLANNED | P0 | PlayerCompanion.BuildPlanner |
| Charm/perk/proficiency planner | PLANNED | P0 | PlayerCompanion.BuildPlanner |
| Quest and access tracker | PLANNED | P0 | PlayerCompanion.ProgressTracker |
| EXP and training calculators | PLANNED | P0 | PlayerCompanion.Calculators |
| Shareable builds | PLANNED | P0 | PlayerCompanion |
| Bestiary/Bosstiary planner | PLANNED | P1 | PlayerCompanion.ProgressTracker |
| Forge/upgrade calculator | PLANNED | P1 | PlayerCompanion.Calculators |
| Imbuement and sustain calculators | PLANNED | P1 | PlayerCompanion.Calculators |
| Team-hunt composition | PLANNED | P1 | PlayerCompanion.HuntAdvisor |
| Weekly-task planner and personal goals | PLANNED | P1 | PlayerCompanion.ProgressTracker |
| Damage/resistance/build simulation | DISCOVERY | P1 | PlayerCompanion + versioned rules engine |
| Personalized next-action recommendations | DISCOVERY | P1 | PlayerCompanion.Recommendations + GameAnalytics |
| Houses, guild wars and broader leaderboards | DISCOVERY | P1 | PublicGameData plus operation-specific contracts |
| Maintenance/service history and raid calendar | PLANNED | P1 | LiveOps |
| Federated content search | DISCOVERY | P1 | PublicPortal/Search adapter over bounded modules |
| Platform API | PLANNED | P1 | PlatformAPI |
| Interactive map and route planning | DEFER | P2 | Separate map programme + PlayerCompanion consumer |
| Market price trends | DEFER | P2 | Economy analytics with authoritative data source |
| Public build profiles and comparisons | DEFER | P2 | PlayerCompanion |
| Own forum | DEFER | P2 | Separate Community decision |
| Polls | DEFER | P2 | Separate bounded Polls module |
| Webshop/provider payments | DEFER | separate gated programme | Payments / ProductsEntitlements / LegalCommerce |
| Roulette or chance-based commercial systems | REJECT BY DEFAULT | n/a | requires separate legal/product decision |

## Explicit design rules

1. `DETERMINISTIC` results are calculated from a pinned ruleset and declare inputs, formula version and precision.
2. `SIMULATION` results declare assumptions and uncertainty.
3. `RECOMMENDATION` output identifies whether it is editorial, analytics-derived or personalized.
4. Every tool exposes ruleset/catalog freshness and fails closed when required data is unknown or incompatible.
5. Character association is resolved server-side; browser-provided account or character ownership claims are never trusted.
6. Session logs and saved plans are private by default, retention-bounded and explicitly shared only through revocable or non-identifying representations.
7. URL-shareable builds contain validated bounded configuration, never credentials, private account identifiers or raw session logs.
8. Formulas belong in reusable versioned domain services, not duplicated across Blade, JavaScript, API and client implementations.
9. The portal remains usable without client-side JavaScript for baseline content; complex planners may progressively enhance this baseline.
10. External benchmark sites inspire inventory and usability decisions only.

## Completion position

The architectural foundation is judged sound and scalable. The portal architecture is not considered globally exhausted or product-complete. This package closes the missing architecture decision for player tools and records the remaining bounded decisions; it does not claim those tools are implemented.