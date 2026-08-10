# ADR 0032: Portal composition, player tracking and server-system ownership refinement

- Status: Accepted
- Date: 2026-08-09
- Decision record: task `OTERYN-20260808-portal-architecture-delta` / PR #933
- Extends: ADR 0025; does not supersede it

## Context

ADR 0025 established the durable `PlayerCompanion` boundary, retained the Laravel modular monolith and separated `LiveOps`, `GameCatalog`, `Wiki`, `PublicGameData` and future `GameAnalytics` ownership. A refreshed 2026-08-08 portal/player-tool benchmark exposed four sub-boundary decisions that are intended to govern later implementation tasks and therefore must survive the current task as explicit ADR authority.

The decisions concern:

1. whether a first-party `Today` / command-centre experience is a new domain or a composition of existing owners;
2. who owns owner-private tracking subscriptions, routines and derived change/progress signals;
3. who owns stable typed definitions for server-specific systems versus editorial explanation and current operational state;
4. whether a future World Hub may become world-routing/admission authority.

These are durable responsibility allocations, not implementation proof. They do not authorize server-repository access, runtime implementation, production activation, payment work, telemetry collection or public disclosure of private tracking data.

## Decision

### 1. `Today` / command-centre is `PublicPortal` composition

`PublicPortal` may compose one compact current-context experience from already-authorized sources:

```text
PublicPortal Today view model
  <- Announcements / Events / CMS editorial presentation
  <- LiveOps current schedules, rotations, maintenance and runtime freshness
  <- PublicGameData public world/player/community projections
  <- PlayerCompanion owner-private routines, goals and tracked signals when authenticated
```

`PublicPortal` owns presentation, prioritization and view-model composition only. It does not acquire authority over the underlying editorial, runtime, game-data or private-player facts.

Every composed value preserves its source owner's applicability, freshness, confidence, privacy and unavailable semantics. Missing or stale evidence must never be converted into fabricated `0`, `offline`, `none`, completed or unchanged state.

#### 1a. `Today` private-representation and cache isolation

Privacy classification propagates through composition. If any `Today` / command-centre representation contains owner-private `PlayerCompanion` routines, goals, tracking preferences, private derived signals or another owner-private input, the **complete materialized representation is `PRIVATE_PERSONALIZED`** even when every other card in that response is public.

Required representation classes are at least:

- `PUBLIC_GUEST` — contains only explicitly public inputs and no value, omission, ordering, badge, count, recommendation or eligibility decision influenced by owner-private state;
- `PRIVATE_PERSONALIZED` — contains or is influenced by any authenticated owner-private input;
- `UNAVAILABLE` / fail-closed — required privacy/authorization context is ambiguous or cannot be proven safe.

Cache rules:

- `PRIVATE_PERSONALIZED` output must never enter a shared/public page cache, anonymous fragment cache, CDN cache, reverse-proxy shared cache or another cache identity reusable across principals;
- the default delivery posture for a mixed public/private Today response is shared-cache bypass plus explicit private/non-shareable response semantics; `no-store` is valid where no owner-scoped representation cache is intentionally implemented;
- `PUBLIC_GUEST` and authenticated/private variants never share cache identity merely because route, query, world, profile or the visible public cards happen to match;
- `Vary`-style metadata alone is not sufficient if an intermediary could still collapse identities or retain owner-private bytes; the implementation must prove the actual cache path is principal-safe or bypass it;
- an anonymous request must never inherit an authenticated user's private fragment, ordering, count, recommendation, suppression decision or other personalized derivative.

A future **private server-side representation cache** is permitted only when its identity binds, at minimum, the authenticated owner/principal plus every security-context dimension whose change can invalidate reuse. Depending on the adopted identity model this includes equivalent semantics for:

```text
owner_identity
session/authentication_generation or equivalent session fence
authorization_revision
privacy_revision
account/character ownership revision when owner-private character context participates
tracking/routine preference revision or private-companion revision
world/profile/season/applicability dimensions used by the representation
representation/schema version
```

The exact encoding and cache technology are deferred. A key based only on route/query/world/profile, a generic `authenticated=true` bit, role name, or another non-owner-specific discriminator is forbidden for private representations.

Private-cache reuse must re-establish all required security-context equivalence. The following transitions fence or invalidate prior personalized representations before they may be reused:

- logout or authentication/session invalidation;
- session replacement or authentication-generation change;
- account/character ownership change affecting the represented private context;
- authorization removal/tightening;
- privacy setting tightening or visibility revision;
- deletion/change of tracked routines, goals, subscriptions or private signals where the older representation is no longer valid;
- representation/schema incompatibility or applicability change.

Public sub-fragments may still be cached independently, but only behind a proof of **composition isolation**:

- the fragment input set is public-only;
- its cache key/eligibility does not depend on owner-private state;
- private data cannot alter fragment bytes, inclusion, ordering, counts or semantic meaning;
- the fragment remains safe when rendered for guest, User A and User B;
- combining it with a private fragment never causes the combined/private response to inherit the public fragment's shared-cache eligibility.

This decision defines the security contract only. It does not select Laravel cache middleware, response headers, CDN vendor, reverse proxy, cache store, TTL or invalidation implementation.

Minimum later validation must include:

| Scenario | Required result |
| --- | --- |
| User A materializes personalized Today; User B requests equivalent route/world/profile | B never receives A private bytes or private-influenced presentation |
| Authenticated User A then anonymous/guest request hits same public dimensions | Guest response contains no A private fragment or derivative |
| Guest response exists, then User A authenticates | Authenticated composition cannot reuse guest identity as proof that private state is absent/current |
| User A logs out or session is replaced | prior personalized representation is fenced from the new/anonymous session |
| account/character ownership changes | older owner-context representation cannot be reused under the new ownership revision |
| tracking preference/goal/private signal is deleted or revised | stale private representation cannot resurrect removed private content/state |
| privacy/authorization tightens while an older private representation exists | older representation fails closed instead of bypassing the new restriction |
| CDN/proxy/shared-cache simulation | no `PRIVATE_PERSONALIZED` bytes are stored or served through a cross-principal cache identity |
| independently cacheable public fragment is combined with private cards | public fragment remains public-only, while the combined representation remains private/non-shareable |

### 2. Owner-private tracking belongs to `PlayerCompanion.ProgressTracker`

`PlayerCompanion.ProgressTracker` owns the Platform-side user intent and derived workflow state for adopted tracking features:

- owner-private tracked-entity and routine preferences;
- subscription/watch preferences for supported public or owner-authorized facts;
- threshold/comparison rules;
- bounded derived change/progress signals;
- private signal history where product value justifies retention.

The underlying fact remains owned by its accepted producer such as `PublicGameData`, `LiveOps`, `GameAnalytics` or `GameCatalog`. Persisting a tracking preference does not copy source authority into `PlayerCompanion`.

`Notifications` owns formatting, transport/channel delivery attempts and delivery status for an already-authorized notification. It does **not** own what is tracked, whether the source fact changed, whether a threshold crossed, or the domain rule that decides a notification should exist.

Tracking is owner-private by default. Public follower graphs, stalking surfaces or social comparison do not follow from this decision. Refresh cadence, retention, abuse controls, privacy and high-cardinality observability must be bounded before implementation.

### 3. Stable server-specific system definitions belong to `GameCatalog`

When an authoritative structured source exists, `GameCatalog` owns the stable typed definition of a server-specific system, including applicable version/profile/ruleset/season dimensions, deterministic parameters and canonical entity relations.

The adjacent ownership remains deliberately split:

```text
GameCatalog
  -> stable typed definition and deterministic parameters/relations

Wiki
  -> explanatory guide, editorial strategy and player-facing prose

LiveOps
  -> current schedule, active rotation, current season/runtime state and freshness

PlayerCompanion
  -> calculators, planners and trackers consuming the owned facts above
```

This does not create a generic plugin framework or a new `GameSystems` deployable module. If no authoritative structured source exists, a topic may remain Wiki/editorial only and must not be promoted to deterministic or live truth.

### 4. World Hub is a view, never routing/admission authority

A future World Hub may compose public world identity/presentation, `PublicGameData` population/community projections, `LiveOps` current status/history and optional evidence-backed `GameAnalytics` trends.

It must preserve world/profile/ruleset/season applicability and observation age. It does not issue `WorldId`/`ChannelId`, select authoritative game placement, establish runtime readiness or control admission/routing. Cached portal state is never routing or gameplay authority.

### 5. Community-contributed hunt evidence stays deferred

Community-submitted hunt evidence may later enrich `PlayerCompanion.HuntAdvisor`, but remains P2/discovery until a separate provenance/privacy/moderation/anti-manipulation contract exists. Private session logs are never silently promoted into public evidence. Any adopted aggregate must expose source/provenance, observation window and sample/confidence semantics.

### 6. No new top-level service is introduced

These decisions refine existing boundaries. They do not justify a new tracking service, dashboard service, World Hub service or generic server-system service. Service extraction still requires measured independent scaling, isolation, lifecycle or ownership need.

## Consequences

### Positive

- later implementation tasks can discover one canonical owner for tracking intent, delivery, server-system definitions and portal composition;
- `Notifications` remains transport-focused instead of becoming a second domain-rules engine;
- deterministic definitions, editorial content and live state cannot silently overwrite each other's authority;
- the homepage/Today experience can become useful without creating another data authority;
- mixed public/private Today composition has an explicit confidentiality boundary independent from composition-time authorization;
- shared-cache eligibility cannot propagate from public cards to an owner-private combined representation;
- future multi-world presentation cannot accidentally become admission or routing authority;
- the existing modular-monolith deployment remains simple.

### Costs and constraints

- tracking requires bounded source-aware evaluation and privacy/retention policy before implementation;
- personalized Today requires deliberate shared-cache bypass or owner-scoped private-cache identity/invalidation rather than a generic portal page-cache key;
- public/private fragment decomposition requires proof that private inputs cannot influence the public fragment;
- server-specific systems require authoritative structured provenance before deterministic catalogue representation;
- Today and World Hub must expose stale/unavailable states rather than hide dependency failures;
- future community evidence requires separate moderation and anti-manipulation work.

## Implementation and activation boundary

This ADR is architecture authority only. It does not prove or authorize implementation of:

- `Today` / command-centre routes or UI;
- response/fragment cache middleware, CDN/reverse-proxy configuration or cache-store selection;
- tracking persistence, pollers, queues or notification fan-out;
- typed server-specific system schemas/importers;
- World Hub routes or analytics;
- community-contributed hunt evidence;
- new Platform APIs;
- production activation or deployment;
- Oteryn-v2/Canary/runtime changes;
- payment or protected-environment operations.

Each adopted user-facing slice still requires a bounded task, complete backend/frontend/integration states, privacy/abuse controls, the two-user/auth-transition/cache-boundary negative paths above, applicable real E2E and exact-head validation.

## References

- ADR 0025 — Player Companion and portal-tools boundary
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/agents/reports/OTERYN-20260808-portal-product-delta.md`
- Issue #302
- Issue #301
- Issue #489
- Issue #941 — owner-private Today cache isolation contract
- PR #933
