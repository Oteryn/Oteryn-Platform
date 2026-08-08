# Oteryn World Registry Contract

## Status

`GATEWAY PRODUCER IMPLEMENTED — NATIVE ADVERTISEMENT DISABLED BY DEFAULT`

## Native Oteryn-v2 authority note

This document remains authoritative implementation/compatibility evidence for the currently delivered Canary-oriented World Registry and Gateway producer.

For **native Oteryn-v2 world/channel identity and topology semantics**, the narrower accepted authorities are:

- ADR 0029 — Platform-owned UUIDv7 WorldId, ChannelId and first-class topology identity;
- `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`.

For **native Oteryn-v2 runtime status/readiness semantics**, the narrower accepted Platform consumer authority is:

- `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

Accordingly, statements below that equate `world_id` with the `game_worlds` database primary key, use `WorldRegistry::forAccount(canary_account_id)`, hard-code integer `channel_id = 1`, couple a channel to protocol-candidate/endpoint storage, or treat persisted `status` as sufficient live readiness describe **LEGACY / CURRENT IMPLEMENTATION COMPATIBILITY STATE**. They do not define the native target identity or runtime-status model.

No current runtime payload, database schema or Canary path is changed by this authority clarification.

This contract defines the authoritative world-routing and ordered gameplay-candidate policy consumed by the Oteryn Game Gateway.

The Platform-owned world route is consumed by the existing Gateway. The historical disabled native producer extension stores ordered per-world/channel candidates and policy revision, while creating no candidate rows and keeping every candidate disabled by default. It does not prove an Oteryn-v2 native consumer, a Rust native consumer, production activation or a production world route.

The first deployment may contain exactly one world, but no API/domain contract may permanently assume a singleton.

## Implemented state

Implemented in Oteryn Platform:

- `game_worlds` database table with non-singleton primary keys and unique slugs;
- `GameWorld` model;
- normalized `GameWorldStatus` vocabulary;
- account-aware `WorldRegistry` interface;
- fail-closed `DatabaseWorldRegistry` implementation;
- sanitized `GameWorldRoute` projection;
- no seeded world and no invented production hostname or port;
- `game_world_protocol_candidates` storage with disabled-by-default rows, deterministic order and exact endpoint/native protocol version/schema/capability projection;
- monotonic `gameplay_policy_revision` on each world;
- Gateway API v1 historical/disabled native gameplay offer, deterministic selection and Game Session v2 producer binding.

Sections below that retain the label “Phase 1” describe the original foundation. Where they conflict with this status section, this status/implemented-state section governs the declared compatibility implementation only. Accepted native-v2 identity/status/protocol authorities remain narrower authorities for native semantics.

Current world authorization policy:

```text
positive redeemed Canary account ID
    -> all worlds where status=online
    -> AND login_enabled=true
    -> AND route fields are syntactically valid
```

This is an MVP authorization policy behind an account-aware interface. It is not a claim that every future account may access every future world.

Still not implemented:

- character-to-world persistence for true multiworld;
- account/world entitlement policy beyond the single-world-ready MVP;
- accepted Oteryn-v2 native World Registry/Gateway consumer reconciliation for the historical Game Session v2 producer path;
- accepted Oteryn-v2 runtime-status/readiness producer integration and native World Registry/LiveOps projection;
- world administration UI;
- production route configuration or activation.

## Ownership

Oteryn Platform owns World Registry configuration and policy.

Game Gateway consumes World Registry through a narrow interface.

Canary remains the runtime owner of each current compatibility game world/process.

For the native target, Oteryn-v2 owns authoritative game-runtime status/readiness source facts while Platform consumes validated projections under `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

OTClient consumes only sanitized routing data returned by Gateway.

## Goals

- one authoritative source for world names, regions and login routes;
- support one world immediately without singleton coupling;
- future multiworld/multiregion expansion without changing login architecture;
- world-scoped Game Session semantics;
- explicit maintenance/login availability;
- no client-controlled game endpoint routing;
- future channel allocation can extend, not replace, the model.

## Non-goals

The first release does not implement:

- gameplay-state synchronization across Canary instances;
- channel allocation;
- dynamic autoscaling;
- queues/capacity balancing;
- tournament enrollment policy;
- production DNS/load-balancer management.

## World identity

Minimum logical record:

```text
world_id
slug
name
region
status
login_enabled
game_host
game_port
```

Compatibility semantics:

```text
world_id       stable database primary identity
slug           unique stable machine identifier
name           player-facing display name
region         normalized region metadata
status         operational/configured presentation state for this compatibility model
login_enabled  authoritative Platform gate for new login/session routing in this compatibility model
game_host      authoritative client routing hostname or IP
game_port      authoritative client routing TCP port
```

For native Oteryn-v2, ADR 0029 and `OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md` replace the local-ID interpretation, and `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md` prevents persisted configuration from being treated as sufficient runtime readiness.

Phase 1 persistence is the Platform database table:

```text
game_worlds
```

No production world row is seeded by the application. Deployment configuration must supply an exact verified route before a world can become login-available.

## Illustrative world only

The following remains an example and is **not** production evidence:

```yaml
world_id: 1
slug: oteryn-eu
name: Oteryn
region: EU
status: online
login_enabled: true
game_host: game-eu.oteryn.com
game_port: 7172
```

No production endpoint should be inferred from this document.

## Status vocabulary

Implemented compatibility values:

```text
online
maintenance
offline
unknown
```

Compatibility semantics:

- `online`: registry expects the world to be operational, subject to real runtime reachability;
- `maintenance`: intentionally unavailable/degraded for new player entry;
- `offline`: intentionally unavailable;
- `unknown`: authoritative runtime status cannot be determined inside the current compatibility model.

`status` is persisted operational/configuration/presentation state for the current compatibility implementation.

`login_enabled` is the explicit Platform authorization/routing gate for new Game Session creation in that implementation.

A world with `login_enabled=false` must not receive a new Game Session even if `status=online`.

Phase 1 `DatabaseWorldRegistry` returns only `online` + `login_enabled` worlds with syntactically routable host/port data.

For native Oteryn-v2, configured Platform policy and observed Oteryn-v2 runtime state are separate authorities. `status=online` plus `login_enabled=true` may be necessary Platform policy but are never sufficient proof that a native channel is currently ready for new admission. Fresh accepted runtime evidence is additionally required by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

## Fail-closed rules

For current compatibility login routing:

```text
invalid account identifier -> no worlds
missing world             -> deny
unknown authorization     -> deny
status != online          -> deny in Phase 1 registry projection
login_enabled = false     -> deny
invalid/missing route     -> deny
```

For native Oteryn-v2, the fail-closed gate is stricter: missing/stale/unavailable/invalid required runtime evidence, superseded runtime ownership, incompatible revisions or explicit non-readiness deny new routing/admission even when Platform configuration is enabled.

The exact compatibility implementation relationship between Canary runtime health and persisted `status` remains unchanged. The native semantic relationship is now defined by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`; exact Oteryn-v2 producer transport/cadence/health algorithms remain deferred to their owner.

## World authorization

Logical interface:

```text
WorldRegistry::forAccount(canary_account_id) -> list<GameWorldRoute>
```

The interface is account-aware from the first implementation.

Current MVP behavior accepts only a positive Canary account ID and then projects all eligible worlds. Future policy may narrow results for:

- test/preview access;
- tournament eligibility;
- region/product restrictions;
- maintenance allowlists;
- staff/internal worlds.

Client input cannot grant world access.

## Character association

Every Gateway-returned character must eventually resolve to a World Registry world.

Logical projection:

```text
character_name
world_id
```

For a single shared Canary database where existing `players` records do not encode world identity, a later single-world adapter may derive all current characters as belonging to the one configured world.

That would be explicit adapter behavior, not a claim that `players` intrinsically stores `world_id`.

Before true multiworld rollout, character-to-world persistence/ownership must be explicitly contracted and tested.

## Game Session world binding

Every target Game Session is logically bound to one `world_id`.

The Phase 1 Game Session value/interface contract carries `worldId`, but no Canary persistence adapter is implemented yet.

Gateway must eventually create/return routing only for the bound world.

OTClient cannot transform a session into authorization for another world by editing host/port.

Exact enforcement depends on the selected Game Session↔Canary adapter and remains an implementation gate.

## Sanitized Gateway projection

Target compatibility projection:

```json
{
  "id": 1,
  "slug": "oteryn-eu",
  "name": "Oteryn",
  "region": "EU",
  "status": "online",
  "host": "game-eu.oteryn.com",
  "port": 7172
}
```

The example values are illustrative and do not define the native Oteryn-v2 status projection envelope.

Do not expose:

- private/internal hostnames when separate from the public route;
- database connection data;
- management ports;
- health-check secrets;
- Canary service credentials;
- private channel/process identifiers not required by the client;
- native GameNode identity, ownership/fencing details or private runtime diagnostics not explicitly public-safe.

## Registry storage decision

Phase 1 selects **Platform database storage**.

Reasons:

- durable normalized model;
- natural multiworld expansion;
- Platform-owned write boundary;
- future audit/admin workflows can be added without changing world identity;
- no need to hard-code singleton routing in Gateway.

Requirements:

- migrations remain backward-compatible/reversible;
- writes remain Platform-owned;
- Gateway must consume through the `WorldRegistry` boundary rather than receive generic database authority;
- no route is treated as production-valid merely because it appears in documentation.

Trusted static configuration is no longer the selected Phase 1 persistence mechanism.

A future native runtime-status store/cache is a projection/read model and does not transfer authoritative runtime ownership from Oteryn-v2 to Platform.

## Route validation

Persisted schema enforces:

- unique world primary identity;
- unique `slug`;
- bounded database field types;
- boolean `login_enabled`;
- unsigned port storage.

Registry projection additionally fails closed unless:

- `slug`, `name`, `region`, and `game_host` are non-empty;
- `game_host` is a syntactically valid IP or hostname;
- `game_port` is in `1..65535`;
- `status=online`;
- `login_enabled=true`.

These are current compatibility checks. Native-v2 route/admission evaluation additionally requires the accepted canonical identity/topology and runtime-status evidence.

Do not allow arbitrary URL schemes/paths where only host+port are expected.

## Trust boundary

OTClient is not authoritative for:

- world ID;
- world availability;
- host;
- port;
- region;
- access policy.

Game Gateway must use Registry values, not client-provided routing.

For native Oteryn-v2, Gateway must also not manufacture runtime readiness from endpoint reachability or persisted Platform configuration when accepted runtime evidence is required.

A future admin UI editing worlds is a privileged operation requiring:

- explicit RBAC permission;
- confirmed MFA under existing admin policy;
- validation;
- audit event;
- safe handling of endpoint changes.

Admin editing is not required for the MVP.

## Availability semantics

Registry configuration eligibility and runtime reachability/readiness are distinct.

For the current compatibility model:

```text
status=online AND login_enabled=true
```

means the registry considers the world eligible for new routing under current policy.

It does not guarantee the Canary endpoint will still be reachable milliseconds later.

For native Oteryn-v2, the accepted model is explicitly two-source:

```text
Platform configured policy allows
AND fresh accepted Oteryn-v2 runtime readiness/compatibility evidence
```

before a route may be considered for new native admission. Stale, unavailable or invalid runtime evidence fails closed for new admission but is not automatically rendered as authoritative `offline` to public consumers.

Do not infer an authoritative world outage solely from one failed client/Gateway connection. World-level native status must aggregate current canonical channel evidence explicitly rather than infer the whole world from one channel.

## Multiworld evolution

Future example:

```text
World Registry
├── oteryn-eu
├── oteryn-na
├── tournament
├── test
└── preview
```

Required before enabling multiple worlds:

- character-to-world ownership/persistence contract;
- account/world authorization policy;
- world-scoped Game Session enforcement;
- separate routing endpoints;
- maintenance behavior;
- cross-world character name semantics if databases are separated;
- exact Canary database/runtime topology;
- E2E per world.

Native multiworld runtime-status projections additionally require canonical WorldId/ChannelId scope, current topology revision and explicit per-channel freshness/aggregation semantics.

## Multiregion evolution

`region` is metadata/policy, not automatic network routing.

Future multiregion may add:

```text
login_ingress_region
preferred_region
latency/capacity policy
region-specific Gateway endpoints
```

The first release must not make client geolocation or latency measurements authoritative for access control.

## Future channel extension

Future compatibility routing may add:

```text
world_id
channel_id
game_host
game_port
```

World remains the persistent player-facing logical world.

Channel is an allocated runtime subdivision/instance.

Channel allocation must not be implemented by overloading `world_id`.

For native Oteryn-v2, first-class WorldId/ChannelId topology is already accepted by ADR 0029 while runtime ownership/readiness remains a separate game-runtime authority consumed through `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

Authentication remains:

```text
Identity
-> Ticket
-> Gateway
-> world authorization
-> optional future channel allocation
-> Game Session
```

Gameplay-state synchronization between channels is outside this contract.

## Maintenance behavior

Recommended current compatibility transition:

```text
login_enabled=false
status=maintenance
```

Effects:

- no new Game Sessions;
- Gateway does not return the world as login-available;
- existing player connections are governed by separate operational/Canary policy;
- Registry alone does not imply forced disconnect.

For native Oteryn-v2, configured maintenance remains a Platform control-plane admission denial independent of current runtime health. Runtime health/readiness does not override Platform maintenance, and an editorial maintenance message does not itself establish runtime truth.

## Endpoint changes

Changing `game_host`/`game_port` affects new routing responses.

Recommended safe operational sequence for the current compatibility model:

1. disable new login for the world;
2. wait/revoke outstanding short-lived entry sessions as policy requires;
3. change route;
4. validate target readiness;
5. re-enable login.

Exact deployment automation remains outside Phase 1.

Native endpoint/route replacement must preserve canonical WorldId/ChannelId and obtain fresh accepted runtime readiness/revision evidence before new routing resumes.

## Phase 1 tests

Implemented focused tests prove:

- registry is empty by default;
- no production world route is invented/seeded;
- invalid account identifier returns no worlds;
- only online + login-enabled worlds are projected;
- malformed host is excluded;
- invalid port is excluded;
- sanitized route contains only approved route fields.

Additional persistence uniqueness is enforced by the database schema.

## Required future Gateway/MVP tests

Before public Gateway use, prove:

- one configured world loads through the Gateway registry boundary;
- missing world fails closed;
- `login_enabled=false` prevents Game Session creation;
- unauthorized account/world combination is omitted/denied once entitlement policy exists;
- Gateway response uses Registry route rather than client input;
- character references resolve to a known world;
- single-world character adapter does not claim intrinsic `players.world_id`;
- public projection does not expose private configuration fields.

Native-v2 implementation additionally requires the exact runtime-status validation suite listed by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.

## Required multiworld tests before Phase 9 completion

- account sees only authorized worlds;
- character lists preserve correct world association;
- session for world A cannot be used for world B according to the selected Canary adapter;
- maintenance world denies new sessions;
- one world outage does not corrupt other world routing;
- region/routing fields are returned correctly;
- duplicate/ambiguous character/world mapping fails closed;
- dynamic endpoint routing uses the exact selected world.

For native multiworld, a failed/unavailable channel must not fabricate whole-world offline when other current authoritative channel observations remain healthy.

## Versioning

Initial compatibility registry/API projection belongs to:

```text
protocol_version = 1
```

Additive optional fields may be backward compatible.

Changing the meaning of compatibility `world_id`, session world binding, or routing authority is a breaking contract change.

Native WorldId/ChannelId identity and runtime-status projection versioning are governed by their narrower accepted contracts.

## Remaining unknowns

1. Exact production Oteryn world public hostname/port.
2. Character-to-world persistence model for true multiworld.
3. Exact Game Session world-scope enforcement in Canary.
4. Exact Oteryn-v2 runtime-status producer schema/transport, reporting cadence/TTL, health/readiness algorithm and ownership-generation encoding; the Platform consumer semantics are defined by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`.
5. Future admin/world-management surface and its privileged write workflow.
6. Future per-account world entitlement persistence/policy beyond the current single-world-ready MVP.

These are not blockers for the implemented Phase 1 registry foundation, but they must be resolved before the corresponding functionality is implemented or claimed.
