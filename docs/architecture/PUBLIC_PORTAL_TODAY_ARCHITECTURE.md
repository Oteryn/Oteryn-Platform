# PublicPortal Today Architecture

## Status and authority

**CURRENT FOCUSED ARCHITECTURE — architecture only, not implementation or production proof.**

This document is the focused Platform architecture for the first-party `Today` / command-centre experience. It derives from accepted ADR 0032, `docs/architecture/LIVEOPS_ARCHITECTURE.md`, `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`, `docs/architecture/MODULE_CATALOG.md` and the source-module contracts they route to.

It does not authorize a runtime route, frontend implementation, source-module mutation, tracking producer, external/server-repository access, deployment, protected-environment operation or production activation.

No new ADR is required because this document does not change the durable ownership or privacy decisions accepted in ADR 0032. It turns those accepted decisions into one focused implementation boundary. Any later change that makes Today a source-of-truth domain, weakens private cache isolation or changes source ownership requires a new or superseding ADR.

## Product purpose

`Today` is the compact current-context entry point for a player. It answers “what matters now?” by composing already-authorized facts and actions from bounded modules. It is not a second homepage CMS, a dashboard database, a scheduler, a notification authority or a world-control plane.

The first implementation target is intentionally **public guest Today**. Authenticated/private composition is a later slice unless it can satisfy the complete ADR 0032 `PRIVATE_PERSONALIZED` cache/security acceptance in the same bounded delivery.

## Ownership and dependency direction

```text
Announcements / Events / CMS
  -> public editorial notices, events and news

LiveOps
  -> current world/service state, configured maintenance,
     freshness and later authoritative schedules/rotations

PublicGameData
  -> public world/player/community read projections

PlayerCompanion
  -> owner-private routines/goals/tracked signals only when authorized

PublicPortal.Today
  -> application composition, prioritization, view-model mapping,
     route/presentation/SEO/accessibility only
```

Rules:

- source modules retain source truth, publication/privacy eligibility, applicability and source-local business rules;
- Today consumes source-owned application/query boundaries, never raw cross-module persistence tables;
- source modules do not depend on Today view models or presentation types;
- Today may prioritize or omit cards for presentation, but it cannot rewrite a source fact into a more certain state;
- Today never becomes runtime readiness, admission, routing, game mutation, editorial publication, notification-decision or tracking authority.

## Representation classes

ADR 0032 defines the required security classes and this document adopts them without modification:

```text
PUBLIC_GUEST
PRIVATE_PERSONALIZED
UNAVAILABLE / fail-closed
```

### `PUBLIC_GUEST`

Contains only explicitly public inputs. No byte, card inclusion, omission, ordering, count, badge, recommendation or eligibility decision may be influenced by owner-private state.

### `PRIVATE_PERSONALIZED`

The complete materialized representation is private if any input or presentation decision is influenced by owner-private state. Public cards inside that response do not make the combined representation publicly cacheable.

### `UNAVAILABLE`

Used when the implementation cannot establish a safe privacy/authorization/composition context. Ambiguity must fail closed rather than falling back to a representation that could disclose or imply private state.

The initial public implementation should stay entirely inside `PUBLIC_GUEST` unless a separately complete private slice is delivered.

## Public Today source inventory

The first public Today slice may compose only sources that are already public and application-authorized. The expected source classes are:

1. **Announcements** — active published notices and severity/action metadata already approved for public display.
2. **Events/CMS** — public editorial event/news summaries appropriate to current context.
3. **LiveOps** — public-safe current world/service state and configured maintenance with evidence/freshness semantics.
4. **PublicGameData** — bounded public world/community context only where a concrete Today card has product value and existing privacy rules allow it.

A source being technically queryable is not sufficient reason to put it on Today. Each card needs a named user purpose, bounded query, explicit empty/unavailable behavior and source-owner approval through the existing public contract.

`PlayerCompanion` owner-private data is excluded from the first `PUBLIC_GUEST` slice.

## Card contract

Each Today card is a presentation projection over one accepted source boundary. A conceptual card envelope preserves:

```text
TodayCard
  kind
  source_owner
  canonical_source_identity_or_url
  applicability
  freshness_or_publication_state
  priority_class
  title / compact summary
  public action/link when allowed
  availability_state
  representation_schema_version
```

The exact PHP/interface shape is an implementation choice. The semantics are not.

A card must not strip a source's stale/unavailable/degraded state merely to fit a compact layout.

## Availability and partial failure

Today must distinguish at least:

```text
present
empty / no applicable published item
stale
unavailable
invalid / rejected upstream evidence where surfaced safely
not_applicable
```

These are presentation/application states, not one global source enum.

Rules:

- a failed source does not turn into an empty collection that implies “nothing is happening”;
- stale LiveOps state remains stale, never normal/offline/zero by inference;
- an unavailable editorial provider does not imply no news/events;
- one unavailable provider does not require the entire public Today page to fail when remaining cards can be rendered truthfully;
- required shell/navigation failures may still fail the page according to PublicPortal/operations policy;
- partial failure is visible in a bounded, user-understandable way without exposing internal exception details;
- recovery replaces the degraded card only after the source query again returns accepted applicable data.

## Prioritization

PublicPortal owns presentation ordering but not source truth.

The first implementation should use a deterministic, versioned priority policy based on card class and urgency rather than opaque scoring. Example priority classes may include critical service/maintenance notice, time-sensitive operational context, active editorial event and ordinary editorial update. Exact names/order are implementation/product constants and should be tested.

Prioritization must not:

- compare heterogeneous source relevance scores as if globally equivalent;
- suppress stale/unavailable indicators in favor of visually “clean” content;
- use owner-private state in `PUBLIC_GUEST` ordering;
- turn an editorial notice into authoritative runtime state;
- reorder based on unbounded per-request remote work.

## World/profile/ruleset/season applicability

Today must preserve every applicability dimension required by the source. When a world context is selected, a card for another world/profile/ruleset/season must not leak into the representation because it shares a slug, title, category or cache entry.

The route/query design may initially have a global/default context, but internal composition APIs must not erase source applicability in ways that make later multi-world rollout unsafe.

Cache keys, canonical links and card identity include the dimensions necessary to prevent cross-world/profile/season reuse once those dimensions participate in the selected representation.

## Freshness

Freshness remains source-owned.

- LiveOps cards preserve LiveOps evidence/freshness and observation age.
- Editorial cards preserve publication/effective-window semantics from their source.
- PublicGameData preserves its own data/runtime freshness rules.
- PublicPortal may present age/updated labels but cannot extend source TTL/authority.

A Today response may have a response generation time, but that timestamp is not evidence that every card is fresh.

## Cache architecture

### Public guest

A `PUBLIC_GUEST` response or public-only card fragment may be shared-cached only when:

- every input is public-only;
- the cache key binds every required applicability/locale/representation dimension;
- the cache lifetime does not extend any input source's accepted publication/freshness boundary;
- a newer restrictive/publication revision fences older derived output where relevant;
- dependency unavailability is not accidentally cached as authoritative empty/normal state beyond the intended degraded-response policy.

The safe effective TTL is bounded by the most restrictive relevant input, not the longest one.

### Private personalized

ADR 0032 governs without relaxation:

- the complete response is private/non-shareable if any private input influences it;
- shared/public/CDN/anonymous-fragment caches may never store `PRIVATE_PERSONALIZED` bytes;
- public and authenticated/private variants never share cache identity;
- `Vary` metadata alone is insufficient proof of cross-principal safety;
- a future owner-scoped private representation cache must bind the owner plus all required auth/privacy/ownership/tracking/applicability/schema revisions;
- logout, session replacement, authorization/privacy tightening, ownership change and private preference/signal changes fence prior personalized representations.

Implementation of private Today is incomplete until two-user, auth-transition and shared-cache negative paths from ADR 0032 pass against the real cache path.

## URL, navigation and SEO

The public Today route must use the existing PublicPortal shared shell and discoverability policy.

Requirements for an implemented public route:

- stable internal canonical URL;
- EN/PL localization following existing locale conventions;
- one canonical URL per locale/applicability context as supported by the current portal model;
- `hreflang`/canonical metadata consistent with the existing public shell;
- sitemap/navigation activation only when the route is truly implemented and enabled;
- no indexing of private/authenticated variants;
- source cards link to their canonical public source URL rather than duplicate full source detail inside Today.

The exact route path is an implementation decision and is not frozen by this architecture document.

## Accessibility and responsive behavior

Today is a high-density composition surface and must remain usable without relying on color, hover or desktop width.

An implementation must provide:

- semantic heading/card structure;
- keyboard-reachable links/actions;
- visible focus states;
- status/severity text or accessible names in addition to visual treatment;
- sensible reading/order semantics matching visual priority;
- no horizontal dependency for ordinary mobile use;
- desktop, tablet and mobile layouts without hiding freshness/unavailable meaning;
- reduced-motion compatibility for any optional motion;
- EN/PL strings that do not encode runtime state only through iconography.

## Performance and fan-out

Today must not become an unbounded synchronous fan-out endpoint.

The first slice should use a fixed allowlisted provider set with bounded per-provider queries. Implementations should avoid N+1 cross-module calls and preserve per-provider failure classification.

A future aggregation/cache layer remains derived state and does not acquire source authority. Service extraction requires measured scaling/isolation need; ADR 0032 explicitly rejects creating a new top-level dashboard service merely for composition.

## Security and privacy

- browser-supplied account/character identifiers never authorize private card access;
- public cards use public-safe source DTOs/queries, not broad model serialization;
- internal errors, SQL/Redis details, topology/fencing internals and private source metadata are not public output;
- external links follow the existing allowlist/safe-link policy of their source;
- Today never turns private tracking lists, goals, notification destinations or session logs into public card metadata, logs or metrics labels;
- authenticated/private composition must re-establish authorization through source/application boundaries rather than trust public-route context.

## Observability

Operational telemetry should distinguish:

- page composition success versus partial success/unavailable;
- provider-level availability class;
- bounded composition latency per provider class;
- stale-card presentation counts by low-cardinality card/source kind;
- cache hit/miss where safe;
- private/shared-cache policy violations as security-significant events if detected.

Do not place account IDs, character IDs, private tracking identity, free-form content or source payloads in metric labels.

## First implementation handoff

Recommended first complete vertical slice:

```text
PublicPortal PUBLIC_GUEST Today
  -> fixed public provider registry
  -> Announcements/Events/CMS + available LiveOps public query
  -> deterministic typed card mapping/prioritization
  -> partial-failure/freshness states
  -> shared-shell EN/PL responsive UI
  -> bounded public cache policy or explicit no-cache initially
  -> sitemap/navigation only after route exists
  -> focused integration + real browser E2E
```

If the available LiveOps implementation is not yet sufficient for a useful card, the first Today slice may still deliver editorial/public cards, but it must render LiveOps as explicitly unavailable/not-yet-provided rather than fabricate normal state. The implementation task must state which providers are real at its exact base.

Minimum implementation acceptance is **provider-capability aware at the exact implementation base**. Observation-age, stale-state and recovery assertions apply only to a provider that actually exists and can emit those states. A provider that is not implemented or not yet authoritative must instead receive explicit unavailable/not-yet-provided coverage, and tests must prove that Today does not fabricate current, stale or recovered source evidence.

1. source modules are accessed only through bounded public application/query interfaces;
2. no raw cross-module persistence query is introduced in PublicPortal;
3. `PUBLIC_GUEST` representation is provably uninfluenced by owner-private state;
4. one implemented provider outage yields truthful partial presentation rather than authoritative empty state;
5. when a real LiveOps provider exists, stale LiveOps evidence remains stale and observation age is preserved; otherwise LiveOps renders explicit unavailable/not-yet-provided and no current/stale observation is fabricated;
6. deterministic priority/card order is covered by tests;
7. locale/applicability cache identity prevents cross-context reuse;
8. any public cache cannot outlive source publication/freshness authority;
9. private/authenticated bytes never enter the first public slice;
10. canonical/source links and SEO metadata are correct for delivered routes;
11. EN/PL desktop/tablet/mobile accessibility and responsive behavior pass;
12. browser E2E covers success, empty and partial dependency failure; with a real LiveOps provider it additionally covers stale/unavailable/recovery, while without one it covers explicit unavailable/not-yet-provided and proves that no runtime state or recovery is fabricated;
13. dependency responses contain no internal exception/topology/private detail;
14. exact final-head self-review, CI, review hygiene and merge gates pass;
15. production activation remains separately gated.

## Authenticated/private follow-up gate

Do not add private PlayerCompanion cards as an incremental “small enhancement” to the public cache path. The private slice is a separate security-sensitive delivery that must prove all ADR 0032 two-principal/auth-transition/cache-isolation cases against the real implementation.

Until that gate passes, the public route remains `PUBLIC_GUEST` even for authenticated users or the product must explicitly route authenticated users through the same public-only representation without consulting private state.

## Deferred Today capabilities

Until separate bounded delivery exists:

- owner-private routines/goals/tracked signals;
- personalized ordering/suppression/recommendations;
- social/follower activity;
- notification inbox behavior;
- public GameAnalytics trends without an accepted evidence source;
- World Hub functionality;
- arbitrary user-configurable dashboard widgets;
- plugin-driven cards;
- production-specific CDN/private cache tuning.

Absence of any deferred capability is not an authoritative “none” source fact.

## References

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/LIVEOPS_ARCHITECTURE.md`
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`
- `docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md`
- `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md`
- `docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md`
- Issue #1049
