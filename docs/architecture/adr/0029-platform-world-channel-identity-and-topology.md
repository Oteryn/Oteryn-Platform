# ADR 0029 — Platform-owned UUIDv7 WorldId, ChannelId and first-class topology identity

## Status

Accepted — 2026-08-07

## Context

Oteryn Platform already owns the World Registry and Game Gateway control-plane boundary. The current implementation predates the native Oteryn-v2 topology model and therefore exposes Platform-local integer persistence identities: `game_worlds.id` is used as route/world identity, `GameWorldRoute.id` is an integer, and protocol policy currently carries an integer `channel_id` that is hard-coded to `1` by `DatabaseWorldRegistry`.

Those values are valid implementation evidence for the existing Canary-compatible/single-channel path, but they are not a safe long-lived identity contract for the native Rust game domain. They couple external semantics to Laravel row layout, conflate channel identity with a protocol-candidate record, and make failover, relocation, rolling deployment and future multi-channel operation unnecessarily identity-breaking.

Oteryn-v2 foundation architecture already distinguishes durable logical world/channel identity from GameNode process incarnation, runtime ownership generations, endpoint routes and protocol details. Platform is the owner of the World Registry/topology-control boundary and therefore must be the canonical issuer of the externally consumed world/channel identities.

## Decision

### 1. Canonical WorldId

The native canonical logical world identity is:

```text
WorldId = strongly typed UUIDv7, full 128 bits
```

`WorldId` is:

- owned and issued by Oteryn Platform World Registry / topology control;
- globally unique for one logical world;
- immutable for that logical world's lifetime;
- never reused for a different logical world;
- independent from display name, slug, region, endpoint, database row, GameNode, deployment, protocol version and current operational state;
- preserved losslessly across authorized Platform <-> Gateway <-> Oteryn-v2 boundaries;
- identity only, not authorization, routing proof or writer authority.

Nil/zero UUID is invalid. UUIDv7 timestamp ordering is not topology authority, freshness, fencing or causality.

World rename, endpoint migration, GameNode restart/relocation and deployment replacement do not change `WorldId`.

### 2. Canonical ChannelId and ChannelRef

The native canonical channel identity is:

```text
ChannelId  = strongly typed UUIDv7, full 128 bits
ChannelRef = WorldId + ChannelId
```

`ChannelId` is owned and issued by Oteryn Platform World Registry / topology control.

Although its UUID representation is globally collision-resistant, its canonical semantic identity is always interpreted and validated in the explicit `WorldId` scope. A durable boundary must not silently discard the world scope and reinterpret a bare UUID as sufficient channel authority.

A channel is a logical topology subdivision of one world. Restart, recovery, relocation or transfer of execution to another GameNode preserves `WorldId + ChannelId` when the semantic channel remains the same.

A retired ChannelId is never reused for another semantic channel.

### 3. Platform-local row IDs remain persistence surrogates

Current and future database row primary keys may remain compact local persistence identities.

Conceptually:

```text
game_worlds.id       = Platform-local persistence surrogate
game_worlds.world_id = canonical WorldId

game_channels.id         = Platform-local persistence surrogate
game_channels.channel_id = canonical ChannelId
```

The exact table/column names are illustrative only. This ADR does not authorize a schema migration or freeze physical persistence layout.

Local row IDs may support Laravel ORM relations and foreign keys. They must not be exported as canonical native `WorldId` or `ChannelId` merely because they are current primary keys.

### 4. Channel becomes a first-class topology entity

Native architecture separates these concepts:

```text
World != Channel != Route != Endpoint != ProtocolCandidate != GameNode != Deployment
```

A Channel has its own durable identity and lifecycle. A protocol candidate or endpoint describes how an eligible client/runtime may reach a channel; it does not create or define channel identity.

One channel may have multiple route candidates during rollout, failover, draining or rollback. Replacing a route, hostname, port, TLS endpoint, protocol offer or deployment does not by itself change `ChannelId`.

Current `game_world_protocol_candidates.channel_id` is therefore compatibility/implementation state, not the final native Channel aggregate.

### 5. GameNode and writer authority are separate from Channel identity

GameNodes consume topology assignments. They do not mint canonical `WorldId` or `ChannelId` as an infrastructure side effect.

For example:

```text
WorldId W + ChannelId C + ownership_generation 41 -> GameNode A
GameNode A fails
WorldId W + ChannelId C + ownership_generation 42 -> GameNode B
```

The logical channel remains `W + C`. Current mutation authority changes through a separate generation/epoch/lease/fencing mechanism defined by the game-runtime/admission topology contracts.

`ChannelId` itself is never used as proof of current writer authority.

### 6. Routes and endpoints are replaceable topology data

`game_host`, `game_port`, TLS endpoint identity, transport metadata and protocol candidates are routing/configuration data, not WorldId or ChannelId identity.

The current `game_worlds.game_host` / `game_port` fields may remain valid for the existing single-route compatibility implementation until a separately authorized migration. Native topology must allow routes to evolve independently from world/channel identity.

Client-supplied endpoint or identifier values never become authoritative routing merely because they are syntactically valid.

### 7. Native account-aware Registry boundary uses AccountId

For native Oteryn-v2 routing, the account-facing World Registry boundary consumes the Platform-owned canonical `AccountId` defined by ADR 0028.

Conceptually:

```text
WorldRegistry::forAccount(AccountId) -> authorized topology projection
```

The existing `forAccount(canary_account_id)` implementation remains valid only inside the declared Canary compatibility path. `canary_account_id` must not become the native topology authorization identity.

### 8. Canary numeric world/channel state is compatibility-only

Current numeric world identity, hard-coded `channel_id = 1`, Canary process mapping and Canary route configuration are classified as:

```text
LEGACY / CURRENT IMPLEMENTATION COMPATIBILITY STATE
```

They may remain behind explicit adapters while required, but must not be promoted to canonical native `WorldId` or `ChannelId`.

A compatibility adapter may map legacy state to a Platform-owned canonical world/channel pair. It must not derive UUID identity by truncation, hashing or undocumented deterministic reinterpretation outside the authoritative Platform migration.

### 9. Protocol contract remains separately owned

World Registry may associate a route candidate with an accepted protocol family/revision/capability policy, but World Registry does not define the final Oteryn-v2 gameplay wire contract.

The currently implemented exact transport/schema/hash/capability tuple remains implementation/history evidence and must not be treated as an implicit freeze of the future `FND-02` / `protocol-oteryn` contract.

Topology identity survives protocol revision changes that do not create a different semantic world or channel.

### 10. Platform control-plane authority stops before gameplay authority

Platform World Registry owns topology identity, topology policy and authorized routing projection.

It does not become authoritative for:

- gameplay state;
- character mutation ownership;
- canonical gameplay `GameSessionId` issuance;
- current game-runtime mutation lease/fence;
- server-authoritative admission success.

The game domain revalidates the routed WorldId/ChannelId together with account, character, session, lease, revision and fencing state before gameplay authority exists.

## Cross-repository impact

`CROSS-REPOSITORY IMPACT`

Oteryn-v2 consumes Platform-issued `WorldId` and `ChannelId` literally and losslessly. It must not mint, hash, truncate, synthesize, re-key or reinterpret them.

The Oteryn-v2 foundation model remains the consumer-side semantic reference for how world/channel IDs participate in admission, runtime placement and fencing. Platform remains the canonical issuer/owner for these two identifiers.

This ADR does not itself authorize an Oteryn-v2 repository write.

## Relationship to existing Platform contracts

`WORLD_REGISTRY_CONTRACT.md` remains authoritative evidence for the currently implemented Canary-compatible registry, including its current numeric IDs and single-channel behavior.

For native Oteryn-v2 identity/topology semantics, this ADR and `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md` are narrower and authoritative where the older contract equates `world_id` with a database primary key, uses `canary_account_id` as account identity or treats integer channel `1` as the target model.

No current runtime payload or database schema is changed by this decision alone.

## Consequences

### Positive

- native identity is decoupled from Laravel persistence layout;
- multi-channel topology can evolve without redefining world identity;
- failover and rolling deployment can replace GameNodes/routes without re-keying channels;
- split-brain prevention can use explicit fencing generations instead of abusing identity values;
- protocol evolution is decoupled from topology identity;
- Canary-specific numeric IDs remain contained in an anti-corruption boundary;
- Platform and Rust domains get a type-safe, durable vocabulary for routing/admission contracts.

### Costs

- Platform needs an additive WorldId migration and a first-class Channel persistence/domain model before native runtime use;
- legacy routes require explicit mapping during coexistence;
- Gateway/private API contracts require versioned migration to canonical AccountId/WorldId/ChannelId values;
- topology revisions, channel lifecycle, readiness, allocation and fencing still require focused downstream contracts and tests.

## Rejected alternatives

### Use `game_worlds.id` as permanent native WorldId

Rejected because it exposes local persistence identity and couples native contracts to Platform database layout.

### Keep `channel_id = 1` as native ChannelId

Rejected because it is a compatibility shortcut, not durable topology identity, and cannot safely support multi-channel recovery or relocation semantics.

### Let protocol candidates define channels

Rejected because protocol/route candidates are replaceable connectivity policy while Channel is durable logical topology identity.

### Let each GameNode mint ChannelId

Rejected because process lifecycle is not topology identity ownership and would make restart/recovery create accidental new channels.

### Encode endpoint or deployment into WorldId/ChannelId

Rejected because operational placement changes must not force logical identity changes.

## Required follow-up

Separately authorized implementation/contract tasks must later define and prove:

1. additive canonical WorldId storage/backfill while retaining local `game_worlds.id`;
2. first-class Channel domain/persistence model and canonical ChannelId issuance;
3. one-to-one migration/mapping for current numeric world/channel compatibility state;
4. versioned native World Registry/Gateway projection using AccountId, WorldId and ChannelId;
5. topology revision and stale-route rejection semantics;
6. channel lifecycle, capacity/allocation, readiness and maintenance policy;
7. GameNode assignment plus ownership generation/lease/fencing contract;
8. route-set rollout/drain/failover/rollback semantics;
9. cross-language UUID fixtures and mixed-version fail-closed behavior;
10. coordinated Oteryn-v2 consumer reconciliation under its own repository authority;
11. explicit retirement criteria for hard-coded `channel_id=1` and legacy Canary mappings.

No Laravel/PHP runtime change, database migration, Oteryn-v2 repository write, protocol activation, channel allocator, deployment or production operation is authorized by this ADR alone.
