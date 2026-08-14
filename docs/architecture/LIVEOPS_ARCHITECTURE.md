# LiveOps Architecture

## Status and authority

**CURRENT FOCUSED ARCHITECTURE — architecture only, not implementation or production proof.**

This document is the focused Platform architecture for time-sensitive world/service state consumed by `LiveOps` and composed by `PublicPortal`. It derives from accepted ADR 0029, ADR 0031, ADR 0032 and `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

It does not define or authorize Oteryn-v2 producer transport, wire/IDL bytes, heartbeat cadence, runtime health algorithms, game-runtime mutation, server-repository access, deployment, protected-environment operations or production activation.

No new ADR is required for this focused architecture because it does not change an accepted durable decision. It consolidates and narrows already accepted ownership and truthfulness rules into an implementation-ready Platform boundary. A later change to those durable rules requires a new or superseding ADR.

## Purpose

`LiveOps` provides truthful, bounded, time-sensitive projections for player-facing and operational Platform consumers without turning Platform configuration, editorial content, cached data or failed probes into fabricated runtime truth.

The first intended implementation sequence is deliberately narrow:

1. world/channel status projection over the accepted runtime-status consumer contract;
2. Platform-owned maintenance intent/presentation;
3. server-save schedule only after an authoritative source and applicability contract are proven;
4. status/maintenance history derived from accepted authoritative changes;
5. raids, bosses, boosts, rotations or runtime events only when their own authoritative producer exists.

Repository search on the trusted base found no implemented `server save` / `server_save` source. Therefore server-save timing remains `UNKNOWN` and must not be guessed from historical Tibia conventions, CMS content, local configuration or a periodic timer invented by Platform.

## Ownership model

Live state is composed from separate authorities. They remain separate even when a consumer presents one card.

```text
Platform World Registry / control plane
  -> canonical WorldId / ChannelId topology
  -> configured maintenance / disabled / admission policy intent

Oteryn-v2 runtime authority
  -> observed runtime lifecycle/readiness/capacity facts
  -> current runtime ownership/fencing evidence
  -> observation revision and freshness authority

GameCatalog
  -> stable deterministic server-system definitions

Announcements / Events / CMS / Wiki
  -> editorial explanation and scheduled editorial content only

LiveOps
  -> validates, normalizes, stores bounded derived projections/history
  -> preserves source authority, applicability and freshness

PublicPortal Today / World Hub / public status
  -> composes presentation only
```

Rules:

- Platform maintenance intent may deny or explain availability without claiming runtime health.
- Runtime health/readiness never overrides Platform maintenance, disablement, entitlement or routing policy.
- Editorial content may explain planned maintenance but is never runtime authority.
- Stable deterministic definitions belong to `GameCatalog`; current schedule/rotation/runtime state belongs to `LiveOps` only when a current-state producer exists.
- `PublicPortal` never becomes world-routing, admission or runtime-readiness authority by consuming LiveOps.

## Canonical scope and applicability

Every native channel-level projection uses canonical `WorldId + ChannelId` from ADR 0029. World-level projections aggregate only the current canonical channel membership for the applicable topology revision.

Every delivered LiveOps fact must carry or be able to establish the dimensions needed to prevent cross-context reuse. Depending on the source, these include:

- `WorldId`;
- `ChannelId` when channel-scoped;
- game profile/ruleset version;
- season or effective interval when applicable;
- topology/policy revision when required for interpretation;
- source observation/revision identity;
- source observation time and freshness/expiry semantics.

A hostname, port, slug, deployment identity, process identity, Canary integer identifier or cached route must never substitute for canonical native scope identity.

## Evidence state is separate from business/runtime state

`LiveOps` preserves the accepted consumer evidence states from the runtime-status contract:

```text
fresh | stale | unavailable | invalid
```

These describe consumer evidence, not the game runtime lifecycle itself.

Consequences:

- `stale != offline`;
- `unavailable != offline`;
- `invalid != offline`;
- missing player-count evidence is not `0`;
- missing schedule evidence is not `none`;
- a failed browser/Gateway request is not authoritative world failure evidence;
- last-known facts may be presented only with their stale/age state preserved.

A public projection may simplify internal detail, but it cannot be more certain than its evidence.

## Projection envelope

The implementation may choose persistence and transport details, but a LiveOps projection must preserve enough semantic information to support this conceptual envelope:

```text
LiveProjection
  projection_kind
  canonical_scope
  applicability
  source_kind
  source_revision_or_observation_identity
  observed_at
  freshness_state
  fresh_until_or_equivalent_expiry
  value_or_typed_runtime_state
  projection_schema_version
```

When stale-owner/replay rejection is material, the projection also preserves the current authority/fencing generation or an equivalent source-owned ordering discriminator. Wall-clock time alone is not sufficient causality.

The browser-facing view model may omit sensitive internals, but storage and reconciliation cannot discard the evidence needed to detect stale or superseded observations.

## World and service status

World status is a projection, not a mutable binary flag.

The implementation must keep at least these independent dimensions logically separable:

1. configured Platform policy/maintenance intent;
2. observed runtime fact from the accepted runtime authority;
3. evidence freshness/validity;
4. topology applicability.

World-level aggregation must preserve degraded/partial state when channels disagree. One failed or unavailable channel does not automatically make the world offline. A retired channel cannot keep the current world projection degraded because an old cached observation remains.

A public `offline` claim is allowed only when authoritative evidence at the applicable scope actually establishes that state. Otherwise presentation uses truthful stale/unknown/unavailable/degraded semantics.

## Maintenance

Configured maintenance is Platform control-plane policy, not an Oteryn-v2 runtime observation.

A maintenance projection may contain:

- canonical scope;
- planned start/end when explicitly configured;
- current configured intent/state;
- policy/topology revision;
- optional editorial explanation reference that remains owned by Announcements/CMS/Events.

LiveOps may compose the configured maintenance projection with observed runtime evidence, but it must not overwrite one authority with the other. A runtime that remains healthy during a maintenance window is still maintenance-blocked by Platform policy; a runtime that is unavailable outside maintenance remains unavailable rather than implicitly scheduled maintenance.

## Server save

`ServerSave` is a separate typed schedule/current-state capability. It must not be derived from maintenance, inferred from a conventional clock time, parsed from free-form news or copied from a historical OTS/Tibia assumption.

Before implementation, the slice must prove:

- authoritative producer/owner;
- canonical world/profile/ruleset applicability;
- timezone/time-base semantics;
- recurrence/effective revision semantics;
- observation/configuration freshness rules;
- behavior for delayed, skipped, rescheduled or unavailable evidence.

Until these are proven, public/server-side output is explicitly unavailable/not implemented rather than guessed.

## History and reconciliation

LiveOps history is derived evidence, not source authority.

The initial history model should record meaningful accepted state transitions/revisions rather than every poll heartbeat. A future persistence design must preserve enough information to explain:

- source and canonical scope;
- accepted source revision/observation identity;
- previous/new projected state where safe;
- observation/effective timestamps;
- evidence/freshness transition;
- projection schema version.

Repeated identical observations should not create unbounded history. Reconciliation must reject superseded runtime owners/revisions and must be able to recover from delayed/out-of-order delivery without trusting arrival order alone.

Retention is bounded by operational/product need and must not retain private topology or high-cardinality runtime detail without justification.

## Caching and freshness

Caching is allowed only as a bounded projection/read optimization.

- cache lifetime never extends producer authority beyond its accepted freshness contract;
- cache keys include every world/profile/ruleset/season/topology dimension required for applicability;
- a stale cached value remains stale;
- dependency failure must not be converted into an empty successful cache entry that looks authoritative;
- restrictive/newer source revisions fence older derived state;
- public LiveOps fragments may be shared-cached only when their input set is public-only and freshness semantics are preserved;
- when a LiveOps fragment enters a `PRIVATE_PERSONALIZED` Today representation, ADR 0032 private-response/cache-isolation rules govern the complete representation.

## PublicPortal Today and World Hub consumption

`PublicPortal` may consume LiveOps through an application/query boundary that returns typed projection state. It must not read LiveOps persistence tables directly or reconstruct freshness from timestamps independently.

Public composition must preserve:

- source applicability;
- evidence state;
- observation age/freshness;
- partial dependency failure;
- public-safe redaction;
- canonical world/profile context.

A missing LiveOps dependency is different from “nothing is happening”. Today/World Hub must render partial/unavailable state rather than silently dropping facts in a way that implies current normal operation.

## Security, privacy and observability

Internal runtime observations may reveal GameNode identity, private endpoints, fencing generations, capacity detail and failure topology. Public projections are allowlisted and redact those internals.

Implementation observability should include bounded metrics/events for:

- observation age/freshness;
- stale/unavailable/invalid transitions;
- rejected superseded/replayed observations;
- projection lag;
- world/channel degraded aggregation;
- dependency recovery;
- cache age relative to producer authority.

Do not put credentials, bearer tokens, private endpoints, fencing secrets, raw private payloads or high-cardinality user/source identifiers into ordinary logs or metric labels.

## Persistence and migration policy

The first Platform implementation, if persistence is required, must use additive/reversible Platform-owned storage. Projection tables are read models, not game-runtime truth.

Requirements:

- immutable canonical scope references or validated canonical identifiers;
- monotonic/source revision information sufficient for stale rejection;
- explicit schema/version fields where projection meaning can evolve;
- indexes matching bounded current-state and history queries;
- deterministic rollback that can disable/rebuild the projection without mutating Oteryn-v2;
- no direct shared/native game-table writes.

Exact schema, queue/event transport and cache store remain implementation decisions and must not be frozen by this architecture document.

## Failure semantics

Fail closed for authority claims and fail truthful for presentation:

- malformed/unauthenticated/inapplicable observation -> `invalid`, never accepted;
- producer unreachable -> `unavailable` while any last-known value keeps its age/stale label;
- freshness expiry -> `stale`;
- superseded authority generation -> reject for current authority;
- topology mismatch -> reject/inapplicable;
- partial channel evidence -> preserve degraded/partial aggregate;
- projection storage unavailable -> public/application dependency unavailable, not fabricated offline/normal;
- recovery -> only a newly accepted applicable authoritative observation may restore `fresh` state.

## First implementation handoff

The first bounded Platform implementation task may start only after it proves the exact source for the capability it will expose. It must not inspect an external server repository from a Platform invocation unless the owner separately authorizes that repository access.

Recommended first vertical slice:

```text
LiveOps WorldStatus + configured Maintenance
  -> source contract adapters already accepted on Platform side
  -> typed current projection/query
  -> public-safe status fragment / Today-ready query
  -> stale/unavailable/degraded presentation
  -> focused observability
  -> exact-head integration and browser evidence
```

`ServerSave` joins that slice only if its authoritative source is proven before implementation; otherwise it remains a separately blocked capability with no guessed fallback.

Minimum implementation acceptance:

1. canonical WorldId/ChannelId and applicability validation;
2. configured maintenance and observed runtime remain independent;
3. fresh accepted evidence can produce a fresh projection;
4. stale/unavailable/invalid evidence never becomes authoritative offline/zero/none;
5. superseded runtime owner/revision is rejected;
6. mixed channel states preserve partial/degraded world semantics;
7. topology membership changes fence retired-channel observations;
8. public output redacts private runtime topology/fencing detail;
9. cache cannot extend producer freshness or cross applicability dimensions;
10. dependency loss/recovery is observable and deterministic;
11. PublicPortal consumes the application/query boundary rather than LiveOps tables;
12. EN/PL desktop/tablet/mobile zero-retry browser evidence covers fresh, stale/unavailable and recovery states for every delivered public route;
13. exact final-head self-review, required CI and real integration evidence pass before merge;
14. production activation remains separately gated.

## Deferred capabilities

The following remain unavailable until separately proven and implemented:

- authoritative server-save schedule;
- raid/boss schedules;
- boosted creature/boss projections;
- runtime events/rotations;
- season-current operational state not already sourced by an accepted producer;
- notification fan-out;
- public historical analytics;
- production environment activation.

No absence above may be represented as an authoritative empty value.

## References

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md`
- `docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md`
- `docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md`
- `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`
- `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md`
- Issue #1046
