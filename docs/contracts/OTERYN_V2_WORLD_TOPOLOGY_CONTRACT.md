# Oteryn v2 World Topology Boundary Contract

## Status

`ACCEPTED ARCHITECTURE CONTRACT — IMPLEMENTATION NOT YET AUTHORIZED`

This contract defines the native world/channel identity and topology boundary owned by Oteryn Platform World Registry and consumed by Game Gateway and Oteryn-v2.

It is intentionally separate from the currently implemented Canary-compatible World Registry representation. Current numeric IDs, `channel_id = 1`, Canary account filtering and direct world host/port fields remain current implementation/compatibility state until a separately authorized migration changes them.

## Canonical owner and consumers

Oteryn Platform World Registry / topology control owns and issues:

- canonical `WorldId`;
- canonical `ChannelId`;
- the durable relationship `ChannelRef = WorldId + ChannelId`;
- world/channel lifecycle configuration and routing eligibility policy;
- authorized topology projections consumed by Game Gateway;
- Platform-side mappings from local persistence/legacy compatibility state to canonical topology identity.

Game Gateway consumes the Registry through a narrow versioned boundary and returns only the routing/topology information appropriate to the client/admission flow.

Oteryn-v2 consumes Platform-issued WorldId/ChannelId. It does not mint a competing world or channel namespace.

## Canonical identities

```text
WorldId   = strongly typed UUIDv7, full 128 bits
ChannelId = strongly typed UUIDv7, full 128 bits
ChannelRef = WorldId + ChannelId
```

Required semantics:

- all 128 bits are preserved losslessly;
- nil/zero UUID is invalid;
- a WorldId is immutable for one logical world and never reused for another;
- a ChannelId is immutable for one logical channel and never reused for another;
- ChannelId is always semantically validated in WorldId scope;
- UUIDv7 ordering is not authorization, route freshness, writer authority, causality or fencing;
- possession of either identifier grants no access by itself.

## Local persistence versus canonical identity

Current Platform implementation uses integer `game_worlds.id` and integer protocol-candidate `channel_id` values.

For the native target:

```text
local world row id   != WorldId
local channel row id != ChannelId
legacy channel 1     != ChannelId
```

Platform may keep compact local primary keys for ORM/FK use. Canonical UUIDv7 values are the cross-boundary identities.

The exact physical schema, column names, indexes, backfill process and migration ordering are not frozen by this contract.

## First-class Channel boundary

A Channel is a durable topology entity, not a property invented by a route candidate.

The canonical conceptual separation is:

```text
World
  WorldId
  product/lifecycle metadata
  Channels[]

Channel
  WorldId
  ChannelId
  lifecycle/eligibility
  capacity/allocation policy
  current topology revision/fencing context
  Routes[]

RouteCandidate
  endpoint/ingress reference
  protocol contract/revision reference
  transport reference
  readiness/rollout metadata
```

This structure is semantic, not a frozen PHP class, SQL schema, JSON payload or protobuf message.

One Channel may have more than one eligible RouteCandidate during rollout, failover, drain or rollback. Route replacement does not change ChannelId when the logical channel remains the same.

## Identity stability rules

The following do not change `WorldId`:

- world rename or display-name change;
- slug/marketing metadata changes under their own compatibility policy;
- region metadata changes that do not create a different logical world;
- endpoint/ingress change;
- GameNode restart or replacement;
- deployment revision change;
- protocol revision change.

The following do not change `ChannelId` when the same semantic channel continues:

- GameNode relocation;
- process restart/recovery;
- endpoint/port/TLS route replacement;
- rolling deployment;
- protocol candidate replacement;
- current writer/ownership generation increase.

Creating a genuinely new logical world or channel requires a fresh identifier. Retired identifiers are not repurposed.

## GameNode, ownership and fencing

GameNode identity/process incarnation is separate from Channel identity.

Canonical placement is conceptually:

```text
ChannelRef
  -> current assigned runtime owner
  -> ownership generation / lease / fence
```

A new runtime owner after failure or relocation receives current authority through the accepted runtime assignment/fencing contract. It does not create a new ChannelId merely because the process changed.

Stale nodes/routes/generations must fail closed. The exact generation/lease representation belongs to the game-runtime/admission topology contract and is not frozen here.

## Native account-aware topology authorization

The native Registry/Gateway account boundary uses canonical Platform `AccountId` from ADR 0028.

Conceptually:

```text
AuthorizedTopologyForAccount(AccountId)
    -> permitted WorldId / ChannelRef choices
    -> sanitized route policy
```

The client may express a requested world/character/channel choice only as input to validate. It cannot grant itself access by supplying a valid-looking UUID, host or port.

The existing logical interface based on `canary_account_id` remains Canary compatibility only.

## Native topology projection

A future versioned native projection must preserve at least the following semantics when needed by its consumer:

```text
WorldTopologyRoute
    WorldId
    ChannelId
    sanitized world metadata
    login/admission eligibility
    topology/policy revision
    route candidates
```

A route candidate may contain a public endpoint reference and references to an accepted protocol/transport contract. This contract intentionally does not freeze:

- JSON or protobuf field names;
- HTTP/gRPC endpoint paths;
- exact transport framing;
- TLS layout;
- schema hashes;
- capability lists;
- final FND-02 protocol versioning tuple.

Those belong to their owning protocol/runtime contracts.

## Route and protocol separation

A RouteCandidate answers “how may this consumer reach this Channel under the current policy?”. It does not answer “what Channel is this?”.

Therefore:

```text
RouteCandidate change != Channel identity change
Protocol revision change != Channel identity change
Endpoint change != World identity change
```

World Registry may reject or advertise routes based on readiness, rollout and compatibility policy, but it must not redefine topology identity because a route changes.

## Canary anti-corruption boundary

Current Canary-oriented state is classified as:

```text
LEGACY / CURRENT IMPLEMENTATION COMPATIBILITY STATE
```

This includes:

- numeric `game_worlds.id` used by existing Gateway routes;
- `game_world_protocol_candidates.channel_id` and the current `1` convention;
- Canary account-scoped world lookup;
- Canary process/endpoint routing assumptions.

A compatibility adapter may resolve these values to a canonical Platform-owned `WorldId + ChannelId` during migration.

It must not:

- expose the numeric value as native WorldId/ChannelId;
- hash/truncate a legacy integer into a UUID outside the authoritative migration;
- let Canary mint canonical Platform topology identity;
- infer current GameNode writer authority from ChannelId.

## Fail-closed rules

A native route/admission projection fails closed when authoritative state cannot establish the required relationship, including:

```text
invalid/nil WorldId                     -> deny
invalid/nil ChannelId                   -> deny
ChannelId not valid for supplied WorldId -> deny
unknown account/world entitlement       -> deny
world/channel not eligible for admission -> deny
missing/invalid route policy            -> deny
stale or incompatible topology revision -> deny
unknown required readiness              -> deny where readiness is mandatory
mixed legacy/native identifier ambiguity -> deny
```

A route being syntactically valid never makes it authoritative.

## Versioning

Changing any of these is a breaking semantic contract change requiring an explicit accepted architecture/contract revision:

- owner or issuer of WorldId/ChannelId;
- UUIDv7 full-128-bit canonical representation;
- semantic ChannelRef scope;
- reuse/lifetime rules;
- separation of topology identity from route/GameNode/protocol candidate;
- native account authorization identity away from Platform AccountId.

Additive routing metadata may be backward compatible when consumers can safely ignore it and fail-closed behavior is preserved.

## Security and observability

WorldId/ChannelId are restricted operational identities, not secrets. Public disclosure is nevertheless minimized to the information required by the client/product surface.

Private topology may include details that must not be exposed publicly, such as management endpoints, private node identity, internal network routes, health-check secrets or fencing/lease internals.

Topology changes and privileged Registry mutations should be auditable with actor/service identity, affected WorldId/ChannelRef, prior/new revision and outcome once implementation exists.

No identifier value should be logged as bearer authorization evidence.

## Compatibility and migration requirements

A later implementation must use an explicit additive migration sequence, expected to include:

1. add canonical WorldId storage to Platform world records while retaining local integer PKs;
2. backfill existing logical worlds with one immutable Platform-issued WorldId each;
3. introduce first-class Channel records and Platform-issued ChannelId values;
4. map the current single-channel compatibility state to an explicit ChannelRef;
5. version Gateway/private APIs to emit canonical AccountId/WorldId/ChannelId for native consumers;
6. keep legacy Canary-compatible contracts during the bounded coexistence window;
7. coordinate Oteryn-v2 consumer adoption only under that repository's authority;
8. reject unsupported mixed old/new identifiers;
9. prove rollback without reusing or changing already issued WorldId/ChannelId values;
10. retire hard-coded integer-channel assumptions only after all required consumers have migrated.

Backfill algorithm, database DDL and rollout sequencing require their own migration review and tests.

## Relationship to gameplay admission

Platform topology authorization proves that a request may be routed toward a particular WorldId/ChannelId under current Platform policy.

It does not prove successful gameplay admission.

The authoritative game domain remains responsible for final checks such as current AccountId-to-CharacterId ownership, account concurrency, character/session leases, placement, bans/game state, topology revision/fencing validity and canonical GameSessionId creation.

## Cross-repository consumer contract

`CROSS-REPOSITORY IMPACT`

Oteryn-v2 must treat WorldId and ChannelId as externally owned Platform identities and preserve them losslessly. Its FND-ID foundation remains the consumer-side source for game-domain use of these identities.

No write to `blakinio/Oteryn-v2` is authorized by this Platform contract alone.

## Non-authorization

This contract authorizes no:

- Laravel migration or model change;
- first-class Channel runtime implementation;
- Gateway API/runtime change;
- route activation;
- GameNode allocator/fencing implementation;
- Canary data/runtime mutation;
- Oteryn-v2 repository write;
- protocol-oteryn implementation;
- staging or production deployment.

Those require separately authorized tasks and risk-proportional migration, compatibility, rollback and E2E evidence.
