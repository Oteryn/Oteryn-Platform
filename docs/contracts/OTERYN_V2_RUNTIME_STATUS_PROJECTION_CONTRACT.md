# Oteryn-v2 Runtime Status Projection Contract

## Status

`ACCEPTED PLATFORM CONSUMER ARCHITECTURE CONTRACT — PRODUCER TRANSPORT / IMPLEMENTATION DEFERRED`

This document defines the Oteryn Platform semantic boundary for consuming native Oteryn-v2 world/channel runtime observations in World Registry, Game Gateway, LiveOps and public read models.

It does **not** define or authorize the Oteryn-v2 producer transport, reporting cadence, health algorithm, orchestration implementation, admission credential, Game Session lease/fencing representation, database schema, deployment or production activation.

## Authority and purpose

This contract is a focused consequence of already accepted architecture:

- ADR 0029: Platform owns canonical `WorldId`, `ChannelId`, topology identity, configured routing policy and authorized topology projection;
- ADR 0031: Oteryn-v2 owns authoritative game-runtime/gameplay source facts while Platform consumes them through explicit contracts;
- `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`: topology identity is separate from GameNode, route, endpoint, deployment and current writer authority;
- `MODULE_CATALOG.md`: LiveOps owns authoritative time-sensitive world/service presentation and freshness semantics once a real producer exists;
- read-only Oteryn-v2 ADR-0009: GameNode health/readiness/capacity, ChannelRuntime lifecycle and ownership generation are runtime concepts; a channel is routable only after readiness/revision checks, and unhealthy/suspected runtime ownership stops new routing.

The exact Oteryn-v2 `OPS-CHANNEL-01`, `FND-03` and `FND-04` producer contracts remain external authority. Missing producer details stay `UNKNOWN`.

## Core separation

Native status has at least two different authorities and they must never be collapsed into one field:

```text
PLATFORM CONFIGURED CONTROL PLANE             OTERYN-V2 OBSERVED RUNTIME

World Registry                                Game runtime / orchestration authority
- WorldId / ChannelId identity                - GameNode health/readiness
- lifecycle / maintenance intent              - ChannelRuntime lifecycle
- login/admission policy                      - current runtime ownership generation
- route eligibility policy                    - capacity / overload facts
- rollout / topology revision                 - runtime revision compatibility
- administrative disable/drain intent         - recovery / suspected / failed facts
                 \                           /
                  \                         /
                   v                       v
                    admission/status evaluation
                              |
             +----------------+----------------+
             |                                 |
             v                                 v
        Game Gateway                      LiveOps / public views
        fail-closed route                 truthful fresh/stale/
        and admission gate               unavailable presentation
```

A Platform configuration that says a channel is enabled is not proof that the game runtime is ready. A runtime observation that says a channel is healthy is not permission to bypass Platform maintenance, entitlement, rollout or login policy.

## Canonical scope identity

Every native channel-level runtime observation that can affect routing or public status is scoped to:

```text
WorldId + ChannelId
```

using the full canonical identities accepted by ADR 0029.

When an observation is about a GameNode or runtime owner, its runtime identity/generation is additional evidence. It never replaces `WorldId + ChannelId`.

Rules:

- nil/invalid WorldId or ChannelId => reject;
- ChannelId not valid in the supplied WorldId scope => reject;
- a route, endpoint, process restart, deployment or GameNode replacement does not create a new WorldId/ChannelId;
- an observation for another world/channel must never be reused by similarity of hostname, port, slug or local database ID;
- legacy Canary integer world/channel identifiers are compatibility-only and cannot be interpreted as native identifiers.

## Runtime producer authority

For native Oteryn-v2, authoritative runtime observations originate from the accepted Oteryn-v2 runtime/orchestration authority, not from:

- Platform CMS or Announcements;
- persisted Platform `game_worlds.status` alone;
- a browser/client connection result;
- a Gateway probe invented as a substitute for game-runtime authority;
- a public monitor with no accepted runtime identity;
- direct reads from undocumented game persistence;
- historical Otheryn/Canary producer semantics.

Platform may validate, cache, aggregate and project accepted observations. That consumer projection does not transfer source-of-truth ownership from Oteryn-v2 to Platform.

## Required semantic observation envelope

The exact JSON/protobuf/gRPC/event shape is deliberately not frozen. Any accepted producer contract must provide enough information for the Platform consumer to establish these semantics when they are applicable:

```text
RuntimeObservation
  WorldId
  ChannelId
  source runtime identity
  current ownership / fencing generation or equivalent freshness authority
  runtime lifecycle / readiness facts
  revision / compatibility facts required for routing
  capacity / overload facts required for admission policy
  observation identity or monotonic source revision
  observed_at
  freshness / expiry contract
```

The representation of source identity, generation, revision and time bounds belongs to the accepted cross-repository producer contract. The semantic requirements do not.

### Minimum evidence properties

An observation used for native admission must be:

- authentic and from the accepted service/runtime authority;
- scoped to exactly one canonical WorldId/ChannelId;
- applicable to the current topology/policy revision required by the consumer;
- associated with current runtime ownership/fencing evidence when stale-owner acceptance is possible;
- ordered or comparable strongly enough to reject superseded observations;
- bounded by an explicit freshness policy;
- compatible with the currently selected protocol/runtime revisions required by the route;
- free of ambiguity between old/new owners after failover or recovery.

Wall-clock `observed_at` alone is not sufficient fencing or causality. A delayed message from a stale GameNode cannot regain authority because its timestamp appears newer on a skewed clock.

## Consumer evidence state

Platform classifies the **evidence**, separately from the game-runtime lifecycle value:

```text
fresh        accepted observation is within its applicability/freshness contract
stale        previously accepted observation exists but is no longer fresh
unavailable  authoritative observation cannot currently be obtained
invalid      malformed, unauthenticated, contradictory or inapplicable evidence
```

These are Platform consumer states. They do not redefine Oteryn-v2 ChannelRuntime lifecycle enums.

Rules:

- `stale` is not `offline`;
- `unavailable` is not `offline`;
- `invalid` is not `offline`;
- no observation is not equivalent to zero players, no channels or a stopped world;
- one failed client/Gateway connection is not authoritative world health evidence;
- consumer caches may preserve last-known facts for presentation only when they also preserve that those facts are stale.

## Admission/readiness evaluation

Native routing/admission is the intersection of independent conditions:

```text
PlatformPolicyAllows(WorldId, ChannelId)
AND FreshRuntimeObservation(WorldId, ChannelId)
AND RuntimeExplicitlyReadyForNewAdmission
AND RequiredRevisionCompatibilityMatches
AND CurrentOwnership/FencingEvidenceMatches
AND Capacity/OverloadPolicyAllows (where required)
= candidate may be considered for new admission
```

This is still pre-admission. The authoritative Oteryn-v2 game domain performs final gameplay admission under ADR 0031/FND-04 authority.

### Fail-closed behavior for new admission

New native admission/routing is denied when any required runtime evidence is:

- missing;
- stale;
- unavailable;
- invalid/contradictory;
- associated with a superseded runtime owner/generation;
- incompatible with the selected topology/protocol/runtime revision;
- explicitly not ready, draining, suspected, recovering, fenced, failed or otherwise non-admissible under the accepted producer lifecycle contract;
- over the accepted capacity/admission boundary when capacity evidence is required.

`status=online` and `login_enabled=true` in current compatibility storage are never sufficient native runtime-readiness proof.

Failing closed for **new admissions** does not itself authorize disconnecting existing gameplay sessions. Existing-session recovery/drain/disconnect behavior belongs to Oteryn-v2 session/runtime operational contracts.

## Platform configured maintenance and runtime state

Maintenance intent is Platform/control-plane policy when configured through the accepted World Registry/operations boundary.

Therefore:

```text
Platform maintenance/disabled policy
    -> can deny new admission even if runtime is healthy

Runtime unhealthy/not-ready observation
    -> can deny new admission even if Platform policy says enabled
```

Neither authority overwrites the other to make a single mutable status truth.

A maintenance announcement may explain planned work, but editorial content is not the admission gate and cannot fabricate runtime readiness.

## World-level aggregation

World-level status is an explicit projection over the world's current canonical channels and policy. It is not inferred from one arbitrary channel.

At minimum:

- one failed/unavailable channel does not automatically mean the entire world is offline;
- a world with mixed channel states must preserve partial/degraded evidence rather than collapse it to a false binary state;
- a world with no fresh observations is `unavailable/unknown` from the consumer's perspective unless authoritative configured policy independently establishes maintenance/disabled intent;
- aggregation must use the current channel membership/topology revision, not a stale channel inventory;
- a retired channel cannot keep a world unhealthy merely because an old observation remains cached.

The exact public world-status vocabulary and aggregation thresholds may be versioned by LiveOps, but they must preserve these truthfulness constraints.

## LiveOps and public presentation

LiveOps/public consumers may transform authoritative runtime facts into player-facing state, but must preserve provenance and freshness semantics.

Required behavior:

- display configured maintenance when authoritative Platform policy says maintenance, while still treating runtime observations separately;
- display an authoritative runtime outage/offline statement only when the accepted runtime/status contract actually establishes that fact at the required scope;
- display `stale`, `unknown` or `unavailable` semantics when runtime evidence is stale/unavailable rather than fabricating `offline`;
- never fabricate `0 online` from an unavailable player-count source;
- distinguish service/world/channel scope;
- retain observation/freshness metadata internally and expose enough user-facing age/degraded context to avoid presenting stale data as live;
- redact private GameNode identity, management endpoints, lease/fencing internals and sensitive topology from public output.

Public presentation may be less detailed than internal status, but it cannot be more certain than the evidence.

## Capacity and player-count facts

Capacity/player-count observations are runtime facts when sourced from Oteryn-v2.

Platform may consume them for routing policy, dashboards or public views only under an explicit contract that defines:

- scope (`WorldId`, `ChannelId`, GameNode or aggregate);
- observation time and freshness;
- revision/applicability;
- whether the value is authoritative, estimated or delayed;
- overload/admission semantics where used as a gate;
- privacy/public allowlist.

A missing/stale count is not zero. A public count does not itself prove admission capacity.

## Ordering, failover and stale-owner rejection

After GameNode failure/replacement or ownership-generation change:

- observations from the superseded owner/generation are rejected for admission authority;
- newly recovered runtime does not become routable until the accepted runtime contract establishes readiness and revision compatibility;
- current canonical WorldId/ChannelId remain stable when the logical channel is recovered;
- a delayed healthy message from the old owner cannot override a newer suspected/recovering/fenced generation;
- Platform projection storage must be able to distinguish current from superseded evidence without trusting message arrival order alone.

Exact generation/fencing encoding remains Oteryn-v2 authority.

## Storage and caching boundary

Platform may persist/cache a native runtime-status projection for resiliency and presentation. Such storage is a **read model**, not the runtime source of truth.

Any implementation must preserve:

- source observation identity/revision;
- canonical scope identity;
- observation/freshness metadata;
- distinction between configured policy and observed runtime state;
- current-versus-superseded ownership evidence where relevant;
- rebuild/reconciliation path from authoritative source evidence;
- bounded retention appropriate to operational/security policy.

A cache TTL may not silently extend producer authority beyond the accepted freshness contract.

## Security and observability

Private runtime status can reveal topology and failure information. Internal projections may contain data that public endpoints must not expose.

Implementation must provide:

- authenticated service-to-service producer identity;
- authorization for status ingestion/consumption where required;
- replay/stale-owner rejection;
- correlation across observation, topology revision and admission decision;
- metrics for observation age, stale/unavailable transitions, rejected superseded observations and fail-closed admission decisions;
- logs that omit credentials, bearer tokens and private fencing secrets;
- public redaction/allowlisting.

## Compatibility boundary

The existing Canary-compatible World Registry may continue using its implemented persisted vocabulary:

```text
online
maintenance
offline
unknown
```

and `login_enabled` under its declared compatibility semantics.

For native Oteryn-v2:

- that persisted field is configuration/compatibility evidence, not sufficient live runtime truth;
- Canary Redis/runtime readers remain compatibility adapters;
- historical Otheryn readiness checks are reconciliation evidence only;
- native runtime observations use canonical WorldId/ChannelId and the accepted cross-repository producer contract.

Do not silently map a legacy Canary status row into fresh Oteryn-v2 runtime readiness.

## Versioning and change control

A breaking semantic change requires explicit contract revision when changing any of:

- runtime source-of-truth owner;
- canonical scope identity away from WorldId/ChannelId;
- separation between configured policy and observed runtime facts;
- stale/unavailable truthfulness rules;
- fail-closed admission requirement for missing required runtime evidence;
- stale-owner/generation rejection requirement;
- public rule that missing/stale data cannot be represented as authoritative offline/zero state.

Transport fields may evolve compatibly when semantic meaning and fail-closed behavior are preserved.

## Deferred producer/implementation details

The following remain `UNKNOWN` until accepted by their owning contracts/evidence:

- exact Oteryn-v2 status event/API schema and transport;
- exact GameNode/ChannelRuntime status enum encoding;
- exact health/readiness algorithm;
- exact reporting/heartbeat cadence and TTL durations;
- exact ownership-generation/fencing representation;
- exact capacity thresholds and overload policy;
- exact LiveOps persistence schema and public API vocabulary;
- exact rollout/deployment topology and production environment values.

These unknowns do not permit a consumer to fail open or substitute configuration state as runtime truth.

## Validation requirements before implementation/activation claims

A future implementation must prove at least:

1. accepted producer identity and exact contract version;
2. WorldId/ChannelId scope validation;
3. fresh observation allows evaluation only when Platform policy also allows;
4. stale/unavailable/invalid observations deny new native admission;
5. superseded owner/generation observations are rejected;
6. recovery becomes routable only after fresh ready/compatible evidence;
7. Platform maintenance disables admission independently of runtime health;
8. one channel failure does not fabricate whole-world offline when other authoritative channels remain healthy;
9. stale/unavailable public facts render stale/unknown/unavailable, never fabricated offline/zero;
10. private runtime/topology/fencing data is absent from public output;
11. projection rebuild/reconciliation preserves observation ordering and freshness;
12. exact-revision cross-repository E2E covers runtime loss, recovery and Gateway route withdrawal before production activation.

## Non-authorization

This contract authorizes no:

- Laravel runtime/model/migration implementation;
- LiveOps module implementation;
- Gateway behavior change;
- Oteryn-v2 repository write;
- Oteryn-v2 OPS-CHANNEL-01/FND contract implementation;
- health probe, heartbeat or status endpoint deployment;
- public status-route activation;
- staging/production deployment or production mutation.

Those require separately authorized implementation tasks and exact producer/consumer evidence.