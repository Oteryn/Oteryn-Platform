# Oteryn v2 World Topology Boundary Contract

## Status

`ACCEPTED ARCHITECTURE CONTRACT — LIMITED DISPOSABLE ISSUANCE AUTHORIZED (#1416)`

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

## Bounded disposable preproduction issuance (#1416)

The separately owner-authorized [Platform #1416](https://github.com/Oteryn/Oteryn-Platform/issues/1416) implements only a private Registry-owned issuance/readback slice for disposable preproduction qualification. Required review and exact-candidate qualification are recorded by the live task/PR; this section makes no deployment or readiness claim.

### Storage and identity lifetime

- `game_worlds.id` remains the local integer PK. Nullable unique `game_worlds.world_id` is canonical WorldId.
- `game_channels` is independent first-class topology storage: local surrogate, restrictive World FK, globally unique canonical ChannelId and explicit `channel_key` unique within the owning World.
- No default, seed, ordinary model creation or backfill issues a WorldId. An explicitly provisioned local World row is a selector for the private owner, never the exported native identity.
- Channel keys use the existing bounded lower-case slug shape (1–64 ASCII characters) as an owner-authored replay selector. They are not canonical identity or UUID derivation.
- The private Registry acquires the World owner row lock before the scoped Channel lookup, issues UUIDv7 through the pinned upstream primitive, commits and independently reads back the persisted joined pair. That World lock serializes every issuer for the same World, including first creation of an absent Channel. A second absent-Channel range lock is unnecessary and can deadlock independent Worlds.
- Outer transactions are refused and the first data read in the issuer transaction is World `FOR UPDATE`. The first consistent Channel read therefore follows acquisition of World ownership and observes the preceding same-World issuer's commit. Pinned Laravel13.30.1 uses a normal PDO transaction and the same write connection; exact MariaDB11.8.9 opens the consistent read view only for a nonlocking read. This guarantee depends on retaining that order and sole ordinary Registry issuance. Existing identity model guards and unique/FK constraints remain. No isolation setting or retry loop is introduced.
- Exact replay and owner/process restart retain the same pair. Different logical Channels require distinct fresh IDs. Metadata or endpoint changes retain existing IDs.
- Ordinary model instance writes cannot mint, replace, clear or reassign canonical identity. Deletion of issued records is refused, including a stale World object whose current DB row has since received identity. Privileged raw SQL is outside that ordinary ownership boundary.

### Private operator command and receipt

First explicitly provision a disposable World through the existing `game-auth:world:ensure` owner with login disabled. Then invoke:

```text
php artisan game-auth:native-topology:issue --world-row-id=<local-row> --channel-key=<authored-key>
```

Neither the command nor its service accepts caller WorldId/ChannelId, routes, requested readiness or current writer claims. It does not change World status, login eligibility, protocol candidates, integer Gateway payloads or current `channel_id = 1` compatibility state.

Successful JSON is a committed identity-only receipt with exactly:

```text
version = 1
purpose = disposable-preproduction-native-topology
issuer = oteryn-platform-world-registry
world_id = canonical UUIDv7 WorldId
channel_id = canonical UUIDv7 ChannelId belonging to world_id
```

The pair retains distinct semantic domains. The DTO alone is not authenticated custody evidence. A qualifying runner must bind the actual trusted Registry process, exact producer revision, isolated database custody and committed readback. Copied JSON, APP_ENV, database names, operator flags and UUID timestamps cannot establish custody or freshness.

No topology revision is derived from `gameplay_policy_revision`, UUID time, native-evidence source ordinal or GameNode generation. This immutable identity receipt proves no route eligibility, health, readiness, account entitlement, Character ownership, placement, runtime fence, admission or current actor/controller authority. Game consumption and current assignment remain separately authorized and qualified.

### Closed disposable execution profile

Service and command fail closed outside APP_ENV `testing` or `preproduction`, and refuse an existing outer transaction so no uncommitted receipt can escape. Supported database profiles are:

- testing SQLite `:memory:` for ordinary feature qualification;
- testing MariaDB `oteryn_concurrency` on loopback TCP for the existing independent-process CI route;
- retained SQLite regular file `<real sys_get_temp_dir()>/oteryn-native-topology-<unique lower-case hex>/oteryn-native-topology.sqlite`, with exact canonical containment and no directory/file symlink.

The file profile is checked before opening the issuer connection or performing mutation. These checks prevent accidental execution against other configured databases; actual custody is established by the external qualifying runner. They do not turn a fixture name into product identity.

### Supported rollback and retention

Issued identities remain retained even when login is disabled or the World is inactive. The additive migration refuses down before any DDL when any nonnull WorldId or Channel record exists; malformed retained state also blocks erasure. With no issued native state, down removes only the additions and preserves existing integer World rows.

Schema rollback is supported only under externally exclusive, quiescent disposable schema custody. Check-then-DDL is not claimed safe against concurrent privileged migration/issuer operations. Ordinary model guards are not protection from privileged raw SQL. An entire database snapshot cannot prove its own issuance history survived restoration; externally anchored restore, production rollout and backfill are outside #1416.

Tests dispose their isolated fixture separately before framework migration teardown. That explicit fixture destruction is not a product identity-deletion API, permission to weaken down, or proof of snapshot restore safety.

### Compatibility and next consumer

The current World Registry, Ensure command, GameWorldRoute, account filtering and integer protocol-candidate namespace retain their existing contracts. No native routing is activated. A separately allocated Oteryn/Oteryn-Game qualification must pin the integrated Platform producer and invoke this actual disposable command before consuming the canonical pair. CharacterBootstrapIntent caller UUIDs are not substituted for Registry issuance.

## Relationship to gameplay admission

Platform topology authorization proves that a request may be routed toward a particular WorldId/ChannelId under current Platform policy.

It does not prove successful gameplay admission.

The authoritative game domain remains responsible for final checks such as current AccountId-to-CharacterId ownership, account concurrency, character/session leases, placement, bans/game state, topology revision/fencing validity and canonical GameSessionId creation.

## Cross-repository consumer contract

`CROSS-REPOSITORY IMPACT`

Oteryn-v2 must treat WorldId and ChannelId as externally owned Platform identities and preserve them losslessly. Its FND-ID foundation remains the consumer-side source for game-domain use of these identities.

No write to `blakinio/Oteryn-v2` is authorized by this Platform contract alone.

## Non-authorization

This architecture contract alone grants no mutation authority. The separately authorized #1416 slice above is bounded; the following operations require their own authority beyond that slice:

- additional Laravel migrations, backfill or model/lifecycle expansion;
- production Channel routing/allocation runtime implementation;
- Gateway API/runtime change;
- route activation;
- GameNode allocator/fencing implementation;
- Canary data/runtime mutation;
- Oteryn-v2 repository write;
- protocol-oteryn implementation;
- staging or production deployment.

Those require separately authorized tasks and risk-proportional migration, compatibility, rollback and E2E evidence.
