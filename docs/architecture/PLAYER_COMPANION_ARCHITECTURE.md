# Oteryn Player Companion Architecture

## Status

**CURRENT — accepted architecture boundary.**

Accepted by ADR 0025 and extended by ADR 0042 for explicitly non-authoritative reference inputs. This document owns the focused architecture for player calculators, planning, hunt guidance, session analysis, progression tracking and personalized recommendations inside Oteryn Platform.

It defines ownership and future delivery constraints. It does not claim that any listed `PLANNED` capability is implemented or production-activated.

## Purpose

`PlayerCompanion` turns versioned game knowledge and authorized character data into useful player workflows:

- calculate deterministic outcomes;
- plan builds and upgrades;
- discover appropriate hunts;
- analyse sessions and party economics;
- track quests, accesses and progression;
- track owner-selected routines, entities and change/progress signals;
- save goals and plans;
- provide clearly labelled recommendations.

It is a bounded application/domain module inside the existing Laravel modular monolith. It is not a second website, generic Wiki extension, arbitrary CMS plugin system or default microservice.

## Product principles

1. **Useful before clever** — prioritize common player decisions over speculative AI features.
2. **Versioned evidence** — every formula and recommendation is bound to explicit authoritative ruleset/catalog evidence or to an explicitly non-authoritative reference snapshot whose limitations are preserved.
3. **No hidden assumptions** — unknown or stale data is visible and may disable a calculation.
4. **Facts differ from advice** — deterministic results, simulations and recommendations are labelled separately.
5. **Private by default** — character plans, tracking preferences and session logs are not public unless the owner deliberately shares a bounded representation.
6. **One domain implementation** — formulas are reusable by web UI, future Platform API and approved client integrations.
7. **Server-side ownership** — account and character association is resolved from trusted bindings, never browser claims.
8. **Progressive delivery** — small vertical slices with complete backend, frontend, states, tests and real E2E.

## Boundary map

```text
GameCatalog ----------------------+
  authoritative/versioned facts, |
  relations, formulas, rulesets   |
                                  |
ReferenceContent -----------------+  NON_AUTHORITATIVE_REFERENCE only
  pinned reference facts,         |
  provenance and limitations      v
Wiki ----------------------> PlayerCompanion <---------------- PublicGameData
  explanations, guides,       |   |   |                         public/authorized
  editorial recommendations   |   |   |                         character/world views
                               |   |   |
GameAnalytics --------------------+   +------------------------- LiveOps
  measured aggregates, confidence     maintenance, schedules,
  and balance evidence                 runtime events/freshness

                               |
                               v
                      Web UI / future PlatformAPI
                      / approved client consumers
```

`ReferenceContent` is never an authority fallback for `GameCatalog`. It is an opt-in evidence source governed by ADR 0042 and `NON_NATIVE_REFERENCE_CONTENT_CONTRACT.md`.

## Module ownership

### Owns

- calculator definitions and executions;
- saved calculator inputs where product value justifies persistence;
- character build plans and equipment sets;
- shareable bounded build representations;
- hunt search criteria and ranked result presentation;
- session-log parsing, normalized private session records and derived party/economy metrics;
- progression goals, checklists and completion projections;
- owner-private tracked-entity/routine/subscription preferences and derived change/progress signals;
- recommendation orchestration and explanation metadata;
- user preferences specific to companion workflows;
- calculator/recommendation/tracking freshness, evidence-authority and compatibility presentation.

### Must not own

- canonical item, creature, spell, vocation, quest, NPC, achievement or ruleset definitions;
- `ReferenceContent` source snapshots, extractor/provenance authority or raw source archives merely because a tool consumes them;
- arbitrary editorial articles or Wiki publication lifecycle;
- raw Canary schema access from UI/controllers;
- game-runtime mutations;
- authoritative market prices without a contracted source;
- payment-provider state or product fulfilment;
- game-balance decisions;
- private Game Analytics source events beyond explicitly approved projections;
- generic account authentication or authorization policy;
- notification transport/delivery state that belongs to `Notifications`;
- public game-data source facts merely because a user tracks them.

## Internal capability families

### `Calculators`

Candidate tools:

- experience between levels and time-to-goal;
- shared-experience range;
- stamina regeneration and usage planning;
- online/offline/exercise training;
- blessing and death-loss estimates;
- imbuement cost and cost per hour;
- health/mana leech and sustain;
- forge/upgrade probability and expected-cost analysis;
- weekly reward calculations;
- deterministic server-specific system calculations.

Rules:

- input schema is typed and bounded;
- formula version and source snapshot are stored with persisted results;
- output declares units, rounding and precision;
- incompatible ruleset data fails closed;
- currency inputs distinguish configured NPC price, observed market price and user override;
- a material gameplay parameter supported only by `NON_AUTHORITATIVE_REFERENCE` evidence cannot be presented as current authoritative deterministic Oteryn truth; obtain authoritative evidence or downgrade the workflow to an explicitly limited simulation/reference/recommendation.

### `BuildPlanner`

Candidate tools:

- equipment sets;
- charm/perk/proficiency planning;
- skill or specialization trees;
- spell/rune/ability loadouts;
- upgrade sequencing and cost projections;
- build comparison;
- shareable builds.

An authoritative build reference stores stable IDs and versions rather than copied display text. Rendering resolves current localized names through `GameCatalog` while preserving the originally pinned snapshot for reproducibility.

A reference-only planning workspace may additionally retain `ReferenceContent` source-local identities, but those identities cannot be serialized or displayed as native IDs unless an independently verified authority crosswalk supplies the native identity. Such a workspace remains visibly reference-scoped until reconciled.

### `HuntAdvisor`

Candidate tools:

- solo, duo and team hunts;
- level/vocation/build filters;
- required damage types and defensive profile;
- required quest/access state;
- expected experience, profit and difficulty ranges;
- party composition and shared-experience compatibility;
- route/map references when a later map programme is available;
- freshness and evidence confidence.

Hunt rankings may combine:

- deterministic eligibility;
- editorial recommendations;
- measured Game Analytics aggregates;
- user preferences;
- explicitly labelled non-authoritative reference evidence where the selected slice permits it.

The UI must explain which inputs affected ranking and must not present a sparse, biased or reference-only sample as universal/current truth.

### `SessionAnalysis`

Candidate tools:

- parser for approved session-log formats;
- loot split and payment settlement suggestions;
- extra expenses and manual adjustments;
- damage/healing contribution;
- experience, profit and supplies per hour;
- personal efficiency history;
- private comparisons across the player's own sessions;
- export/share of sanitized summaries.

Session analysis does not execute bank transfers or game economy mutations. A calculated split is advice until an independently contracted transaction feature exists.

### `ProgressTracker`

Candidate tools:

- quests and access chains;
- bestiary and bosstiary;
- achievements;
- weekly tasks and recurring goals;
- character development objectives;
- equipment acquisition goals;
- prerequisite graphs;
- owner-selected public/authorized entities or world signals to follow;
- reminders exposed through the separate Notifications boundary when approved.

Completion state may be:

- imported from an authoritative contracted projection;
- manually confirmed by the player;
- inferred with a visible confidence label.

Inferred state must never silently become authoritative game state. Reference-only quest/access/bestiary definitions may support planning candidates, but cannot establish that a character has access, completed a quest or that content is currently reachable.

#### Tracking, routines and change signals

The 2026-08-08 benchmark delta clarifies that tracking belongs inside the accepted `ProgressTracker` capability family rather than a new `Tracking` service.

Candidate tracked subjects include only explicitly supported identifiers/facts, for example:

- the owner's own character progression and goals;
- weekly or recurring task cycles;
- selected public characters where the source fact is already public and product policy permits following;
- selected worlds and public population/activity/status signals;
- selected public guild/house/event/system references where an accepted projection exists.

Rules:

- follow/subscription preference is Platform-owned owner-private state by default;
- the source fact remains owned by `PublicGameData`, `LiveOps`, `GameAnalytics`, `GameCatalog`, `ReferenceContent` or another accepted producer boundary;
- `Notifications` owns delivery attempts/channels only; it does not decide what is tracked, whether a source changed or whether a threshold was crossed;
- a signal records source identity/revision, observation/freshness and the comparison/threshold rule that produced it;
- missing/stale source evidence never becomes a false “no change”, “offline”, “completed” or reset signal;
- `ReferenceContent` cannot produce authoritative current-world change signals because it has no runtime observation authority;
- refresh/poll cadence is bounded and source-aware rather than one unbounded per-user loop;
- tracking cannot bypass privacy or expose fields that the underlying public/authorized projection would hide;
- public stalking graphs, “who follows whom” and social comparison are not implied;
- retention and deletion follow the user's private companion-history policy;
- high-cardinality tracked IDs and notification targets do not become unbounded metric labels.

A later `PublicPortal` Today/command-centre view may consume these owner-private signals, but presentation does not move tracking authority into `PublicPortal`.

#### Private representation handoff to `PublicPortal.Today`

When `PlayerCompanion` contributes owner-private routines, goals, tracking preferences, private completion state or derived signals to a `PublicPortal` composition, that contribution carries **owner-private representation semantics**, not merely the visibility of its underlying source fact.

Rules:

- tracking a public character/world/event does not make the owner's tracking choice, threshold, routine, goal or derived history public;
- any view model/card/summary carrying or influenced by owner-private PlayerCompanion state is private to the authenticated owner unless a separate explicit share contract says otherwise;
- the consuming `PublicPortal.Today` composition must propagate that privacy classification to the complete mixed response unless private/public fragments are proven isolated under ADR 0032;
- PlayerCompanion must expose enough security-context revision semantics for a future owner-scoped representation cache to fence stale reuse after relevant private-state changes; exact field/schema names remain deferred;
- those semantics include equivalent revision/fence information for owner identity, current authorization/privacy context, character/account ownership where applicable, and the private companion/tracking state that influenced the representation;
- deletion/tightening of a routine, goal, tracking preference, private signal, authorization or privacy rule must prevent an older materialized private representation from becoming valid again through cache replay;
- no PlayerCompanion output grants a public/shared-cache classification merely because all referenced source facts are public.

The cache store, TTL, middleware and response-header implementation remain `UNKNOWN` / owned by later Today delivery work. The security invariant does not: owner-private PlayerCompanion representation cannot be made cross-principal reusable by caching.

### `Recommendations`

Candidate outputs:

- next useful quest/access;
- equipment upgrade priority;
- build refinement order;
- suitable hunts;
- best next charm/perk/proficiency investment;
- training or currency-spend alternatives;
- goal suggestions based on the player's stated objective.

Each recommendation records:

```text
recommendation_type
basis: editorial | deterministic | analytics | personalized | reference
game_profile
ruleset_version
catalog_snapshot or null
reference_snapshot_id or null
source_evidence_class
input_summary
confidence
freshness
explanation_key
limitations
```

Generative AI may later help explain results, but it must not invent formulas, game entities, account state or unsupported recommendations. Deterministic and retrieved evidence remains authoritative within its own source class; `NON_AUTHORITATIVE_REFERENCE` evidence never upgrades itself into authoritative game truth.

## Result classification

Every calculation/advice output remains one of:

### `DETERMINISTIC`

A reproducible result computed from pinned **authoritative** rules and exact inputs when presented as current Oteryn gameplay truth.

Examples:

- experience required between levels;
- shared-experience eligibility;
- configured imbuement cost;
- expected training duration under an accepted formula.

A reproducible computation over reference-only gameplay parameters is not sufficient for this current-authoritative claim; it must be presented as a bounded reference/simulation workflow unless independently re-grounded in authoritative evidence.

### `SIMULATION`

A model with stated assumptions, distributions or uncertainty.

Examples:

- forge expected cost;
- survivability range;
- projected time to complete a bestiary set;
- a what-if computation using a pinned `ReferenceContent` snapshot whose Oteryn applicability is unknown.

### `RECOMMENDATION`

Advice that depends on priorities, editorial judgement, measured data, personalization or explicitly limited reference evidence.

Examples:

- best hunt for profit;
- preferred perk order;
- next equipment upgrade.

The UI must not visually collapse these classifications into one certainty level.

A tracked-source `SIGNAL` is not a fourth certainty class for calculations. It is a state-change/threshold observation over an already classified source. The UI must show its source and freshness and must not present an inferred signal as authoritative game state.

### Source-evidence authority is a separate dimension

Result certainty and source authority are orthogonal. A persisted/displayed result records an evidence class such as authoritative catalog/observed source, editorial/analytics evidence or `NON_AUTHORITATIVE_REFERENCE` where applicable.

When `NON_AUTHORITATIVE_REFERENCE` materially influences the result, the UI and serialized representation retain the exact `reference_snapshot_id`, profile/fact-family scope, assumptions and limitations. Hiding that dimension is a contract violation.

## Version and applicability contract

Every persisted plan, calculation or recommendation carries applicable dimensions:

```text
game_profile
ruleset_version
catalog_snapshot_id or null
reference_snapshot_id or null
source_evidence_class
world_id or null
season_id or null
effective_from
effective_until
formula_version
source_status
```

Tracked routines/signals additionally retain the relevant source revision/observation identity and the rule revision used to derive a change/threshold result.

Supported source statuses:

- `CURRENT` — compatible with the active ruleset and required authoritative sources;
- `STALE` — previously valid but superseded or beyond its freshness policy;
- `EXPERIMENTAL` — available with incomplete validation or intentionally limited evidence;
- `DEPRECATED` — retained for historical reproducibility but unavailable for new plans;
- `UNAVAILABLE` — required source missing or incompatible.

A reference snapshot whose applicability to current Oteryn is unproven cannot be labelled `CURRENT` merely because it is validated/reproducible. It normally remains `EXPERIMENTAL` or `UNAVAILABLE` for a consumer that requires authoritative current truth.

Ruleset activation must invalidate or reclassify affected cached outputs. Silent reuse of a prior-balance formula or reference snapshot as current is forbidden.

## Data ownership

### Platform-owned data

Potential Platform tables include:

- saved companion workspaces;
- build-plan headers and versioned selections;
- calculation records only where history is product-relevant;
- progression goals and manual completion state;
- owner-private tracking/subscription preferences and bounded derived signal history;
- normalized private session analyses;
- share grants or opaque share tokens;
- user companion preferences;
- recommendation snapshots and explanation metadata.

These records reference trusted Platform Identity and immutable ready account/character bindings where applicable.

### External/read data

- `GameCatalog` owns authoritative/compatibility game entities, relations, rules and source snapshots within its accepted contracts;
- `ReferenceContent` owns provenance-pinned `NON_AUTHORITATIVE_REFERENCE` snapshots, source-local identity, extraction/review state and bounded reference projections under ADR 0042;
- `Wiki` owns editorial explanation and guide publication;
- `PublicGameData` owns public and contract-approved character/world projections;
- `GameAnalytics` owns measured aggregate evidence and confidence;
- `LiveOps` owns current operational schedules/runtime-state projections.

PlayerCompanion consumes bounded application interfaces or versioned projections. It must not bypass them with ad hoc raw table queries or parse source archives directly. Persisting a track/subscription or plan does not copy source ownership into PlayerCompanion.

## Privacy and security

### Private by default

Private data includes:

- saved builds associated with an Identity;
- account-linked character goals;
- owner tracking/subscription preferences and private derived signal history;
- raw and normalized session logs;
- party membership, contribution and payment calculations;
- private notes and user-entered market prices;
- recommendation history tied to a character.

Default visibility is owner-only.

Materialized representations derived from any of these owner-private inputs remain private even when embedded in a page that also contains public data. Composition does not declassify private PlayerCompanion state.

### Shareable representations

A shared build or summary must:

- contain only allowlisted fields;
- use opaque, high-entropy server-generated identifiers when stored server-side;
- support revocation and expiry when linked to private persisted data;
- avoid Platform Identity IDs, Canary account IDs, sessions, emails and raw logs;
- be size-bounded and schema-validated when encoded in a URL;
- declare its ruleset/catalog version and any material `ReferenceContent` snapshot/evidence class;
- fail safely when referenced entities no longer exist or are incompatible.

Tracking lists and alert destinations are not shareable merely because their underlying source data is public.

### Session-log handling

- accept only documented text formats and bounded payload sizes;
- parse as untrusted input;
- escape all displayed names and values;
- never execute embedded markup, URLs or commands;
- retain raw logs only when explicitly required and under a documented retention period;
- permit deletion of the user's private history;
- exclude raw logs and player-identifying details from ordinary application logs and audit metadata;
- require explicit consent before sharing a party summary.

### Authorization

- authenticated owner access for private workspaces, tracking preferences and histories;
- server-side character ownership resolution;
- exact admin permission plus confirmed MFA for any privileged catalog/formula/recommendation management surface;
- no wildcard permission;
- rate limits for expensive simulations, parsers, tracking refresh/subscription changes and public share resolution;
- CSRF protection for browser state changes.

## Formula and rules engine direction

Formula implementations should be reusable domain services with:

- stable calculator identifier;
- semantic formula version;
- typed input and output contracts;
- supported game-profile/ruleset range or explicit reference-only applicability;
- deterministic rounding policy;
- source references and evidence class;
- fixtures and boundary cases;
- migration/reclassification rules for saved calculations;
- optional client-safe representation only when exact parity can be proven.

Do not duplicate business formulas independently in Blade templates, browser JavaScript, Platform API and the Rust client. Interactive frontends may preview results, but the server remains the authoritative calculator implementation for accepted formulas unless a separately versioned shared library and parity contract is adopted. A reference-only input never changes the source-authority label of the underlying gameplay fact.

## Cache and freshness

- immutable catalog snapshots may be cached by snapshot identity;
- immutable reference snapshots may be cached only by exact reference snapshot identity and never behind an authority-bearing active-profile alias;
- active ruleset aliases require deterministic invalidation;
- LiveOps/runtime state keeps its own short freshness boundary and must not inherit long page-cache TTLs;
- Game Analytics aggregates declare observation window and sample size;
- tracked-source evaluation must respect the producer's freshness/observation semantics and coalesce fan-out rather than multiplying raw polling per user;
- user-entered price overrides are scoped to the user/workspace and never promoted to global truth;
- stale recommendations remain visible only when useful and clearly labelled.

For owner-private PlayerCompanion representations, caching has an additional confidentiality invariant:

- private/mixed representations are not eligible for shared/public/CDN/anonymous-fragment caches;
- if a private server-side representation cache is adopted, its key/fence must bind the authenticated owner plus every authorization/privacy/ownership/private-state revision required to prove reuse remains valid;
- route/query/world/profile, a generic authenticated flag or public source revision alone is never sufficient cache identity for private output;
- logout, session/authentication generation change, ownership change, privacy/authorization tightening, and deletion/change of the private PlayerCompanion state represented in the output fence older cached representations;
- a stale private representation must fail closed rather than resurrect a removed goal/routine/track/signal;
- public sub-fragments can be cached independently only when private PlayerCompanion inputs cannot affect their bytes, inclusion, ordering, counts, cache identity or eligibility;
- combining a public cached fragment with owner-private content makes the combined materialized response private; public-fragment cacheability does not transfer upward to the mixed response.

## Multi-world and seasons

World and season are explicit dimensions, not globally assumed constants.

A tool declares whether it is:

- global to a game profile/ruleset;
- world-specific because prices, PvP type or availability differ;
- season-specific because progression or systems reset;
- character-specific;
- party-specific;
- reference-only with current Oteryn applicability explicitly unknown where the source cannot prove it.

The initial product may expose only one world, but schemas, cache keys, URLs and saved plans must not rely on an irreversible single-world assumption.

A tracked world/system routine must likewise carry its world/season/profile applicability; a reset in one season or world cannot silently mutate another track.

## Platform API and client integration

Future API/client exposure must use the same application services as the web UI.

Candidate API capabilities:

- list calculator metadata;
- execute selected deterministic calculators;
- read/save owner workspaces;
- read/update owner tracking/routine preferences and current bounded signal summaries where adopted;
- resolve shareable builds;
- fetch compatible recommendation summaries;
- receive version/freshness/evidence-authority metadata.

Requirements:

- versioned API contracts;
- appropriate user/service authentication;
- per-capability scopes and rate limits;
- no raw database model serialization;
- no formula duplication;
- explicit compatibility with client/game profile;
- explicit `NON_AUTHORITATIVE_REFERENCE` provenance where applicable;
- graceful unsupported-version behavior.

## Observability

Record bounded metrics for:

- calculator success/failure and latency by calculator ID/version;
- stale/incompatible source rejection;
- reference-source admission/rejection by bounded source/fact-family class, never raw paths as labels;
- parser failure class without raw input;
- recommendation source availability;
- tracked-signal evaluation/delivery handoff outcomes without tracked IDs as unbounded labels;
- private representation cache bypass/hit/invalidation outcomes by bounded class/reason, never by owner or tracked ID label;
- share resolution and revocation outcomes;
- expensive simulation limits;
- dependency failure and recovery.

Never log credentials, session identifiers, raw session logs, complete private build contents, private tracking lists/notification destinations, private cache keys containing raw principal identifiers, raw reference archive contents/host paths, or unredacted party data.

## Delivery priorities

### P0 — essential player utility

1. Loot Split and private Session Analyzer.
2. Hunt Finder with versioned eligibility and editorial evidence.
3. Equipment Explorer/Comparison using authoritative Game Catalog when available, with a separately labelled reference-only mode only under ADR 0042.
4. Character Build Planner.
5. Charm/Perk/Proficiency Planner.
6. Quest and Access Tracker.
7. EXP and Training calculators.
8. Validated shareable builds.

### P1 — deeper planning and personalization

1. Bestiary/Bosstiary Planner.
2. Forge/Upgrade Calculator.
3. Imbuement and sustain calculators.
4. Team Hunt Composer.
5. Weekly-task and personal-goal planner.
6. Owner-private tracked entities/routines/change signals with Notifications delivery integration.
7. Equipment-set comparison.
8. Damage, sustain and resistance simulation after formula proof.
9. Explainable next-action recommendations after Game Analytics contracts exist.

### P2 — separate later programmes

1. Interactive maps and route planning.
2. Raid/boss scheduling integrations.
3. Market-price trends and economy analytics.
4. Public build profiles and comparisons.
5. Advanced full-build simulation.
6. Community contribution workflows, including community-submitted hunt evidence.
7. Public/social tracking graphs or comparisons.

P2 capabilities require separate architecture, content provenance, moderation, privacy and operational decisions where applicable. Community-submitted hunt evidence must never silently mix with authoritative, reference-only or private-session facts; any aggregate requires explicit provenance, observation window, sample size/confidence and anti-manipulation policy.

## Vertical-slice implementation rule

Each capability must ship as a bounded complete slice:

1. accepted input/output and source contract, including evidence-authority class;
2. persistence only when required;
3. domain implementation;
4. authorization, validation and abuse limits;
5. real reachable UI;
6. current/stale/unavailable/invalid/empty/reference-only states as applicable;
7. localization;
8. responsive and accessible behavior;
9. focused and integration tests;
10. real E2E on the exact head;
11. version/freshness/provenance evidence;
12. documentation and module-catalog update.

A formula library, endpoint or dormant view alone is not a delivered player tool.

## Explicit non-goals

- copying TibiaPal, Tibia, RubinOT or another portal implementation;
- making Wiki or CMS a generic executable plugin system;
- automatic game actions or botting;
- client memory inspection or unauthorized telemetry;
- hidden scoring based on private players' data;
- public stalking/social graphs as an implication of private tracking;
- financial settlement or bank transfer execution;
- unversioned formulas;
- hiding `NON_AUTHORITATIVE_REFERENCE` provenance or treating reference data as an authority fallback;
- claiming an editorial or reference-backed recommendation is objectively current/optimal without the required evidence;
- microservice extraction without measured need.

## Completion criteria for the architecture programme

The PlayerCompanion architecture is defined by this document and ADR 0025, with ADR 0042 governing non-native reference inputs. Product delivery remains incomplete until the owner-approved capability inventory has an explicit implementation/defer/reject disposition and each implemented slice passes repository delivery gates.

A later architecture review may consider the boundary mature when:

- P0 scope is explicitly selected;
- required authoritative Game Catalog entities/formulas and accepted reference inventories exist for the exact selected modes;
- privacy/retention policy is accepted, including tracking/subscription and private representation/cache semantics when adopted;
- first vertical slices prove the boundary works without domain duplication or authority confusion;
- API/client reuse is tested where adopted;
- version transition behavior is exercised across at least one ruleset change;
- Game Analytics recommendations, if enabled, declare evidence and confidence.
