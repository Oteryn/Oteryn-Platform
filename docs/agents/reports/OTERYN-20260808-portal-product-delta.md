# Oteryn WWW portal product delta — 2026-08-08

## Scope and evidence boundary

This is a focused follow-up to `OTERYN-20260806-portal-player-tools-benchmark.md`. It asks whether current public MMORPG/player-tool patterns expose a material architecture gap that is not already owned by Oteryn Platform.

External websites are untrusted product-research evidence only. They do not define Oteryn authority or implementation truth. Do not copy external code, datasets, prose, layouts, branding, icons or artwork.

Observed sources on 2026-08-08:

- official Tibia website: `https://www.tibia.com/` and current event/manual surfaces;
- TibiaPal: `https://tibiapal.com/`;
- RubinOT: `https://rubinot.net/`;
- Tibia Tracker: `https://www.tibia-tracker.com/`;
- TibiaXplorer: `https://www.tibiaxplorer.com/`;
- Tibijka Tracker: `https://tibijkatracker.pl/`;
- Tibia Tools: `https://www.tibiatools.pl/`;
- TibiaPulse: `https://www.tibiapulse.com/`.

## What changed in the benchmark signal

The 2026-08-06 architecture remains valid. The new evidence does **not** justify a new frontend stack, a separate player-tools website, default microservices or a broad portal rewrite.

The material delta is narrower: successful player portals increasingly combine static/reference tools with a personal routine layer and a compact current-state dashboard.

### 1. `Today` / command-centre composition

Current official/community surfaces repeatedly place time-sensitive signals close to the entry point:

- current boosted creature/boss and online count;
- event calendar and server-save-relative events;
- daily NPC/location signals;
- weekly task cycle/progress;
- world population/activity/quiet-hour signals;
- personalized goals and recent hunt performance.

Oteryn already has the correct owners. A new source-of-truth module is unnecessary.

Recommended composition:

```text
PublicPortal Today view model
  <- Announcements / Events / CMS editorial presentation
  <- LiveOps authoritative current schedules/runtime state
  <- PublicGameData public world/player projections
  <- PlayerCompanion owner-private routines/goals/tracked signals when authenticated
```

`PublicPortal` owns only composition/presentation. It must preserve each source's freshness, confidence, privacy and unavailable semantics and must never convert missing data into `0`, `offline`, `none` or completed state.

### 2. Owner-private tracking and routines

Tibia Tracker, TibiaXplorer, Tibijka and Hunt Analyser all strengthen the product case for persistent player-oriented tracking: weekly tasks, character progression, hunt history, goals, world watchlists and change signals.

Oteryn does not need a standalone `Tracking` microservice. ADR 0025 already gives `PlayerCompanion` progression/goals ownership, and `Notifications` already owns delivery.

Recommended ownership refinement:

```text
PlayerCompanion.ProgressTracker
  owns tracked entities/goals/routines/subscription preferences
  consumes approved PublicGameData / LiveOps / GameAnalytics projections
  produces owner-private change/progress signals

Notifications
  delivers an already-authorized notification
  does not decide what a player follows or whether a game fact changed
```

The first scope should stay owner-private. Public stalking/social graphs are not implied. Tracking may observe only data already public or explicitly authorized for the owner, with bounded refresh, retention and abuse controls.

### 3. Server-specific system definitions need an explicit split

RubinOT exposes server-specific systems such as task systems, battle-pass-like progression, hunt discovery, castles and world-transfer information as named product concepts rather than hiding everything inside generic CMS prose.

Oteryn already has the necessary bounded owners, but the split should be explicit:

```text
GameCatalog
  -> stable typed system definition
  -> version / game profile / ruleset / season applicability
  -> deterministic parameters and entity relations when authoritative

Wiki
  -> explanatory guide, editorial strategy and player-facing prose

LiveOps
  -> current schedule, active rotation, season/runtime state and freshness

PlayerCompanion
  -> calculator/planner/tracker consuming those owned facts
```

This is a `GameCatalog` capability family, not a new top-level deployable module. A system without an authoritative structured source may remain Wiki-only editorial content, but must not be presented as deterministic/live truth.

### 4. World hub is a view, not a new authority

Mature portals expose worlds by region/PvP/profile and increasingly show population/activity trends or watchlists. Oteryn already preserves explicit `world`, profile/ruleset and season dimensions.

A future World Hub should compose:

- configured public world identity/policy;
- PublicGameData population/community projections;
- LiveOps current status/freshness/history;
- optional evidence-backed GameAnalytics trends.

It must not become routing authority and must not infer game availability from stale page/cache data.

### 5. Community hunt evidence is useful but later

Community-submitted hunt reports can enrich HuntAdvisor with real experience/profit/routes, but they introduce sampling bias, privacy, moderation and manipulation risks.

Keep this P2/discovery until an explicit contribution contract exists. Any aggregate must expose observation window, sample size/confidence and provenance; private session logs are never silently promoted into public evidence.

## No-change conclusions

The following existing decisions remain correct:

- Laravel modular monolith + Blade baseline;
- `PlayerCompanion` rather than miscellaneous calculators in Wiki/CMS/controllers;
- `LiveOps` separate from editorial Events/Announcements;
- `PlatformAPI` only as an adapter over existing module services for concrete consumers;
- explicit game profile/ruleset/catalog/world/season/effective-period dimensions;
- interactive maps as a separate later programme;
- market/economy analytics deferred until authoritative data/privacy exist;
- native forum deferred while Discord + Support satisfy the initial community boundary;
- roulette/chance-based commercial systems rejected by default pending explicit product/legal decision.

## Recommended priority adjustment

No P0 replacement is required. Preserve the existing P0 PlayerCompanion order.

Add to P1:

1. owner-private tracked entities, routines and change/progress signals under `PlayerCompanion.ProgressTracker`;
2. a composed `Today`/command-centre experience in `PublicPortal` after LiveOps/current-source contracts exist;
3. a public World Hub once world/status projection sources are mature;
4. structured server-specific system definitions under GameCatalog where authoritative sources exist.

Keep community-contributed hunt evidence and advanced social/public tracking in P2/discovery.

## Architecture verdict

- foundation rewrite: **REJECT**;
- new top-level tracking microservice/module: **REJECT**;
- Today/command-centre composition: **ACCEPT as PublicPortal composition direction**;
- owner-private tracking/routines: **ACCEPT inside PlayerCompanion.ProgressTracker**;
- Notifications as subscription authority: **REJECT**; delivery only;
- server-specific system registry: **ACCEPT as GameCatalog capability family with Wiki/LiveOps split**;
- World Hub: **ACCEPT as future PublicPortal/PublicGameData/LiveOps composition, not routing authority**;
- community hunt-data contribution: **DEFER to bounded P2/discovery**.

These refinements fit ADR 0025 and existing module ownership; no new ADR or deployable service is required.