# ADR 0025: Player Companion and portal-tools boundary

- Status: Accepted
- Date: 2026-08-06
- Decision owner: repository owner
- Decision record: task `OTERYN-20260806-player-companion-portal-architecture` / PR #667
- Accepted option: A

## Context

Oteryn Platform already has a mature Laravel modular-monolith foundation and broad portal modules. A product comparison with the official Tibia website, RubinOT and TibiaPal identified two separate questions:

1. whether the portal foundation should be replaced or substantially decomposed;
2. where player-useful calculators, build planners, hunt guidance, session analysis and progression tracking should live.

The existing foundation is not the problem. Platform Identity, account and character operations, PublicGameData, CMS, Wiki, GameCatalog, Support, Wallet, Marketplace, Admin, Audit, operational boundaries and the server-rendered frontend already benefit from one deployable application and shared authorization policy.

The missing boundary is a player-facing companion layer. Putting calculators and planners directly into Wiki, CMS, Blade templates, JavaScript helpers or arbitrary controllers would:

- duplicate formulas and game data;
- mix editorial content with deterministic rules;
- obscure ruleset/version compatibility;
- make web, future API and client results drift;
- create unclear privacy ownership for session logs and saved plans;
- turn broad modules into generic dumping grounds.

TibiaPal demonstrates product demand for loot splitting, hunting-place discovery, equipment references, proficiency/build planning, quest/bestiary guidance and training/economy calculators. It also retains explicitly outdated hunting data after major game changes, demonstrating the need for first-class ruleset/version and freshness semantics.

## Options considered

### Option A — dedicated PlayerCompanion module inside the modular monolith

Create a bounded `PlayerCompanion` module that owns calculator execution, build plans, hunt-advisor orchestration, session analysis, progression goals, shareable plans and explainable recommendations.

It consumes:

- versioned entities, relations, rules and formula evidence from `GameCatalog`;
- guides and editorial explanations from `Wiki`;
- public or authorized character/world projections from `PublicGameData`;
- measured aggregates and confidence from future `GameAnalytics`;
- time-sensitive world/service state from `LiveOps`.

It does not own canonical game entities, raw Canary data access, game mutations, payment settlement or balance decisions.

### Option B — distribute tools across Wiki, CMS and PublicGameData

Each tool would be implemented near the content or data it appears to use.

This reduces the number of named modules initially, but creates mixed ownership, duplicated formulas, inconsistent privacy rules and no coherent player workspace or recommendation boundary.

### Option C — separate player-tools service or SPA

Build an independently deployed frontend/service for calculators and planning.

This may become useful after proven independent scaling or lifecycle needs, but currently duplicates authentication, authorization, localization, observability and deployment concerns without evidence that extraction is required.

## Decision

The repository owner accepted **Option A** through the 2026-08-06 instruction to persist the complete portal and TibiaPal architecture discussion.

The durable decision is:

- retain Laravel modular monolith as the default deployment and ownership model;
- add `PlayerCompanion` as a bounded module, not a second website;
- keep `GameCatalog`, `Wiki`, `PublicGameData`, `GameAnalytics` and `LiveOps` as separate sources/boundaries;
- classify outputs as `DETERMINISTIC`, `SIMULATION` or `RECOMMENDATION`;
- bind formulas, plans and recommendations to explicit game profile, ruleset, catalog snapshot, world/season applicability and effective dates;
- keep saved builds, goals and session analyses private by default;
- use validated, bounded and non-identifying representations for shareable builds/summaries;
- implement formulas once in reusable versioned domain services;
- expose the same application services to the web UI and any future Platform API or approved client integration;
- deliver player tools as small complete vertical slices;
- prioritize loot/session analysis, hunt discovery, equipment/build planning, charm/perk/proficiency planning, quest/access tracking and EXP/training tools before advanced maps, market analytics or public social features.

The decision also adopts `LiveOps` as a planned boundary for maintenance, server save, service status/history, raid schedules and other authoritative time-sensitive world state. CMS/Events may provide editorial presentation but must not impersonate runtime truth.

## Consequences

### Positive

- Player tools receive one coherent product, privacy and versioning boundary.
- Game data and formulas are reused rather than copied into each tool.
- Web, future API and client consumers can share application/domain services.
- Ruleset transitions can invalidate or reclassify stale results explicitly.
- Session analysis and saved plans receive owner-scoped privacy controls.
- Recommendation uncertainty and evidence become visible.
- The existing deployment and security model remains simple.

### Negative

- `PlayerCompanion` introduces a broad product area that must be kept internally decomposed.
- Many useful tools depend on Game Catalog formula/content completeness that is not yet proven.
- Personalized recommendations require future Game Analytics contracts and privacy review.
- Versioned saved plans require migration/reproducibility policy after ruleset changes.
- Full P0 scope is too large for one implementation task or PR.

## Rejected shortcuts

- Treat every benchmark feature as mandatory for launch.
- Copy external portal code, text, data, artwork or layouts.
- Put formulas directly in Blade templates or duplicate them independently in JavaScript and clients.
- Present editorial recommendations as deterministic optimal answers.
- Infer character ownership from browser-provided identifiers.
- Store or publish raw party-session logs by default.
- Hard-code an irreversible single-world assumption.
- Extract microservices before measured operational need.

## Implementation and activation boundary

This ADR and `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` define accepted architecture only.

They do not authorize:

- implementation of the full capability inventory in one task;
- production activation;
- cross-repository Canary/Otheryn or client mutations;
- collection of new telemetry;
- public exposure of private character/session data;
- payment, market or game economy mutation;
- generative recommendations without source/evidence constraints.

Each player tool requires its own bounded task, source contract, privacy classification, complete user-facing vertical slice, tests, real E2E and exact-head merge gate.

## References

- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- `docs/agents/reports/OTERYN-20260806-portal-player-tools-benchmark.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/SYSTEM_ARCHITECTURE.md`
- `docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md`
- `docs/architecture/GAME_CATALOG_ARCHITECTURE.md`
- ADR 0001 — Laravel modular monolith
- ADR 0008 — frontend information and shell architecture
- ADR 0016 — versioned Game Catalog snapshots
- PR #667