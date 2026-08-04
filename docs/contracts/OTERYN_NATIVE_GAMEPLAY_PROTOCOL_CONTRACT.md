# Oteryn native gameplay protocol contract

Coordination ID: `OTS-20260804-native-protocol-selection`  
Contract revision: `1`  
Source of truth: `blakinio/Oteryn-Platform`  
Review IDL: [`oteryn_native_gameplay_v1.proto`](oteryn_native_gameplay_v1.proto)  
Architecture decision: [`ADR 0010`](../architecture/adr/0010-native-gameplay-protocol-selection.md)  
Security analysis: [`OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md`](../architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md)  
Rollout: [`OTERYN_NATIVE_PROTOCOL_ROLLOUT.md`](../architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md)

## 1. Status and authority

This document is the canonical producer/consumer contract for a future native Oteryn gameplay protocol. It is architecture and contract only. It does not prove that any runtime currently advertises, selects, issues, accepts, encodes or decodes `protocol-oteryn`.

The current delivered chain remains:

```text
Rust client
-> system browser
-> Oteryn Identity OAuth Authorization Code + PKCE
-> short-lived game:ticket bootstrap
-> one-time opaque Game Login Ticket
-> Oteryn Game Gateway
-> private atomic ticket redeem
-> authoritative login context and World Registry route
-> Game Session issuance
-> Otheryn game server
```

The native protocol reuses this chain. It does not introduce another login server, Identity authority, ticket type, password path, direct OAuth authentication to Otheryn, direct Game Login Ticket consumption by Otheryn or a Gateway bypass.

The following names are distinct and must never share one serialized version field:

| Concept | Initial value | Owner |
|---|---:|---|
| Gateway login API version | existing JSON `protocol_version: 1` | Game Gateway |
| Game Session contract version | target `2` | Gateway producer and Otheryn consumer |
| gameplay adapter family | `canary` or `oteryn` | World Registry vocabulary |
| native gameplay profile | `oteryn.native.v1` | this contract |
| native transport profile | `tcp.tls13.protobuf.be32.v1` | this contract |
| schema revision | `1` | this contract/IDL |
| client build metadata | bounded opaque build string | client, validated by Gateway |

## 2. Current versus target implementation truth

### Current

- Gateway `/v1/login` accepts Gateway API `protocol_version: 1` and one Game Login Ticket.
- Gateway redeems the ticket, obtains one authoritative world and account-authorized characters, creates a Game Session request containing account, world and login-attempt identifiers, and returns a session credential, worlds and characters.
- Otheryn provides profile-driven Canary-compatible protocol and transport behavior through its existing ASIO network stack.
- The Rust workspace contains `protocol-canary`, `protocol-core`, `game-domain`, `game-session` and transport crates. It does not contain `protocol-oteryn`.

### Target, disabled until later packages

- Gateway accepts a bounded gameplay support offer in the same login request and selects exactly one candidate from authoritative World Registry policy.
- Game Session contract version 2 binds that selection and all admission invariants.
- Otheryn exposes a separately configured native TLS endpoint and validates the bound selection before native gameplay messages.
- The Rust client implements an independent native adapter and binds one selected adapter to one session.

Any repository that describes target behavior must label it target/unimplemented until exact linked producer and consumer evidence exists.

## 3. Negotiation decision

### Selected mechanism

**The client sends a bounded supported-candidate offer in the existing Gateway login request; Gateway makes the final deterministic selection.**

The later Gateway producer package extends the JSON request without changing the meaning of the existing `protocol_version` field:

```json
{
  "protocol_version": 1,
  "game_login_ticket": "<opaque>",
  "gameplay_offer": {
    "offer_version": 1,
    "client_build": "<1..64 ASCII characters>",
    "client_platform": "windows-x86_64",
    "candidates": []
  }
}
```

Rules:

1. Gateway validates JSON shape, lengths, count limits, identifier syntax, duplicate keys and candidate canonical form before redeeming the ticket.
2. `candidates` contains `1..8` unique supported candidates. It is a set, not client preference order.
3. Gateway redeems the one-time ticket, obtains authoritative world/login context and reads the current ordered World Registry gameplay policy for that world/channel.
4. Gateway selects the first authoritative World Registry candidate that exactly matches a client-supported family/profile/transport/schema tuple and whose required capabilities are a subset of client-supported capabilities.
5. Gateway chooses the optional capability intersection. The client cannot add a capability that World Registry did not allow.
6. Gateway issues exactly one Game Session for the selected candidate or returns one terminal typed failure. It does not attempt a second candidate after session-issuer failure.
7. A no-match result after ticket redemption consumes the ticket. The client must acquire a fresh Game Login Ticket and repeat the complete login attempt. It must not retry a modified offer with the consumed ticket.
8. `Auto` preference is expressed by authoritative World Registry order, normally native first and Canary second during migration. The client does not author the production preference.
9. Development `ForceCanary`/`ForceOteryn` modes restrict the client offer to one diagnostic candidate; Gateway remains authoritative and may reject it.

### Rejected alternatives

- **Gateway returns candidates and client selects later:** rejected because it creates a second state transition after ticket consumption, expands ambiguous retry state and lets a compromised client influence selection later than necessary.
- **Dedicated wire negotiation at the game server:** rejected for initial rollout because it duplicates policy outside World Registry/Gateway and creates a downgrade point after credential handoff.
- **Infer adapter from first gameplay bytes:** forbidden because it is ambiguous, attacker-controlled and cannot securely bind Game Session authorization.

## 4. Authoritative ownership

| Responsibility | Sole owner | Consumer/validator |
|---|---|---|
| reusable credentials, MFA, OAuth, game-auth generation | Oteryn Identity/Platform | Gateway private client |
| ticket issue, expiry, atomic redeem | Platform | Gateway |
| configured world/channel adapter candidates and order | Platform World Registry | Gateway, operations tooling |
| candidate family/profile/transport/schema/capability metadata | Platform World Registry, constrained by this contract | Gateway |
| final selection decision | Game Gateway | client and Otheryn must verify |
| Game Session contract production | Gateway session issuer boundary | Otheryn |
| native endpoint and TLS identity | World Registry configuration backed by Otheryn deployment | Gateway/client |
| validation of selected profile at gameplay admission | Otheryn | client observes success/failure |
| gameplay legality, ordering, state and results | Otheryn | Rust client |
| locally implemented candidate set | Rust client build | Gateway receives bounded offer |
| adapter binding and wire decode/encode | Rust game-session/protocol layer | application/domain |
| rollout enable/disable flags and minimum versions | Platform World Registry/operations | Gateway/Otheryn/client telemetry |
| compatibility evidence matrix | this contract registry plus exact package evidence | all three repositories |

No client field establishes account ownership, character ownership, route ownership, rollout state or authoritative capabilities.

## 5. Candidate identifiers and canonical form

A candidate is uniquely identified by:

```text
family
profile
transport
schema_revision
schema_sha256
required_capabilities
optional_capabilities
```

Initial registered candidates:

| Family | Profile | Transport | Schema | Status |
|---|---|---|---|---|
| `canary` | exact source-proven profile such as `canary.current` | exact existing Canary transport profile | adapter-local | existing compatibility path; not defined by this IDL |
| `oteryn` | `oteryn.native.v1` | `tcp.tls13.protobuf.be32.v1` | revision `1` and exact IDL SHA-256 | target, disabled |

Identifier grammar is lowercase ASCII `[a-z0-9][a-z0-9._-]{0,63}`. Capability lists are sorted bytewise, unique and limited to 64 entries. Unknown required capabilities make a candidate unsupported. Unknown optional capabilities are not selected.

The base native v1 capability set is:

```text
actions.command-result.v1
ordering.server-sequence.v1
session.single-admission.v1
state.snapshot-delta.v1
state.revision.v1
reconciliation.movement.v1
inventory.server-authoritative.v1
combat.server-authoritative.v1
chat.semantic.v1
```

A later capability may add messages or fields only when its own schema, ownership, limits, supported pairs and downgrade behavior are registered. Capabilities cannot weaken base security or size limits.

## 6. Gateway response and selection proof

A successful later Gateway response retains Gateway API version 1 and adds a distinct selection object:

```json
{
  "protocol_version": 1,
  "game_session_contract_version": 2,
  "login_attempt_id": "<32 lowercase hex characters>",
  "session": {
    "credential": "<opaque>",
    "expires_at": "<RFC3339 UTC>"
  },
  "gameplay_selection": {
    "policy_revision": 42,
    "family": "oteryn",
    "profile": "oteryn.native.v1",
    "transport": "tcp.tls13.protobuf.be32.v1",
    "schema_revision": 1,
    "schema_sha256": "<64 lowercase hex characters>",
    "capabilities": [],
    "host": "game.example.invalid",
    "port": 7173,
    "tls_server_name": "game.example.invalid"
  },
  "worlds": [],
  "characters": []
}
```

Bounds:

- response body: `<= 64 KiB`;
- worlds: `1..16`;
- characters: `0..100`;
- selected capabilities: `<=64`;
- host/TLS name: `1..253` ASCII, no wildcard or URL syntax;
- port: `1..65535`;
- login attempt ID: exactly 16 random bytes rendered as 32 lowercase hex characters.

The client fails closed when the selected candidate was not in its exact offer, required fields are missing, identifiers are non-canonical, capabilities are contradictory, endpoint/TLS identity is invalid, schema hash is unknown or any bound field differs from the later server bootstrap.

## 7. Game Session contract version 2

The Game Session authorization is opaque to the client and integrity-protected by the existing issuer/consumer mechanism selected by the implementation package. Its authoritative stored/validated claims are:

```text
contract_version = 2
session_id = 16 random bytes
login_attempt_id = 16 random bytes
identity_id / bound Canary account ID
game_auth_generation
world_id
channel_id (explicit even when only one channel exists)
character_binding_mode = bind_on_first_admission
allowed_character_set_revision or authoritative account binding reference
selected family/profile/transport/schema revision/schema hash
selected sorted capability list and SHA-256 digest
World Registry policy revision
issued_at / not_before / expires_at
single_admission = true
audience = exact Otheryn world endpoint/service identity
```

### Character binding

Current Gateway login produces the authoritative character list and Game Session material in one response, so the initial native contract does not invent a pre-existing selected character at issuance.

The exact rule is `bind_on_first_admission`:

1. The client selects one character from the Gateway-provided authoritative list and sends its stable character identifier in native `ClientHello`.
2. Otheryn validates that the credential is unexpired, unconsumed, audience/world/profile/capability correct and belongs to the account that currently owns the requested character.
3. Otheryn atomically changes the session admission record from `ISSUED/UNBOUND` to `ACTIVE/BOUND(character_id)` exactly once.
4. The same credential cannot bind a second character, world, channel, endpoint, profile or connection.
5. Failure after an ambiguous consume/bind attempt is terminal for that credential. The client obtains a fresh ticket and session; it never guesses whether replay is safe.

This preserves the current Gateway character-presentation sequence while proving that a credential cannot be replayed for a different character.

### Revocation and replacement

- `game_auth_generation` mismatch rejects admission.
- expiry rejects admission.
- a replacement/relog uses a fresh ticket, login attempt, session ID and command namespace.
- queued commands from an old session are discarded locally and rejected server-side.
- active-session revocation/disconnect policy remains a separately versioned operational feature; absence of immediate disconnect must not permit new admission with stale generation.

## 8. Native transport and bootstrap

### Endpoint

Native v1 uses a separately advertised TCP endpoint. It may share a host with Canary but does not share framing detection. World Registry must not advertise native unless the exact native listener and TLS identity are ready.

### TLS

- TLS `1.3` is mandatory.
- ALPN is exactly `oteryn-game/1`.
- the client validates the certificate chain and exact `tls_server_name`; production verification policy is deployment-owned and cannot be disabled by gameplay data.
- no plaintext native profile exists.
- TLS AEAD provides integrity and confidentiality; native v1 adds no application checksum or MAC.
- authentication/session credentials, raw command IDs and full account/character identifiers are redacted from logs.

### Bootstrap

After TLS succeeds:

1. Client sends `ClientHello` within 5 seconds.
2. `ClientHello` includes schema revision, exact profile/transport identifiers, selected capability digest, opaque Game Session credential, login attempt ID and selected character ID.
3. Otheryn validates every field against the stored Game Session authorization before any gameplay command is accepted.
4. Otheryn returns exactly one `ServerHello` success or one typed `ProtocolError`, then closes on failure.
5. `ServerHello` echoes session ID, profile, schema hash, capability digest, policy revision and initial server sequence.
6. Any disagreement is `SESSION_BINDING_MISMATCH` and is session-fatal. No Canary retry occurs on that connection or credential.

## 9. Framing and serialization

### Frame boundary

Every post-TLS application frame is:

```text
uint32_be payload_length
payload_length bytes of serialized WireEnvelope
```

Rules:

- minimum payload length: `1`;
- maximum payload length: `1,048,576` bytes;
- the prefix is not included in `payload_length`;
- zero, oversize, truncated, trailing or multiple-envelope payloads are session-fatal;
- parsers validate the length before allocation and use bounded reusable buffers;
- one frame contains exactly one `WireEnvelope`.

### Serialization

- protobuf `proto3` is used with the checked-in review IDL.
- schema revision `1` is explicit in every envelope and bootstrap.
- field numbers are never reused, and removed fields remain `reserved`.
- scalar default values are not used to represent required semantic presence; required semantic fields use validated non-zero/non-empty values or explicit `optional` fields.
- unknown protobuf fields are ignored and preserved only where the implementation library naturally supports preservation; they cannot activate behavior.
- an unknown `oneof` message kind, unknown required enum value or missing required semantic field is a protocol violation.
- enum zero is always `UNSPECIFIED` and invalid where a concrete value is required.
- strings are UTF-8, normalized only for display, and bounded before allocation.

### Limits

| Item | Limit |
|---|---:|
| frame payload | 1 MiB |
| ordinary decoded message | 1 MiB |
| UTF-8 string | 4 KiB unless a smaller field limit is stated |
| chat text | 1 KiB UTF-8 |
| capability token | 64 bytes |
| capability entries | 64 |
| repeated entries in one ordinary message | 4096 |
| protobuf nesting depth | 32 |
| snapshot chunks | 256 |
| one snapshot chunk payload | 512 KiB encoded |
| complete initial snapshot | 16 MiB encoded across chunks |
| command queue accepted by protocol session | implementation-bounded; minimum contract does not permit unbounded growth |

### Compression

Native v1 base profile uses `compression = none`. A compressed-frame flag or compressed payload is a fatal protocol violation.

A later compression capability must define a new transport/profile revision. It must enforce before allocation:

- compressed frame `<=1 MiB`;
- decompressed output `<=4 MiB` per frame;
- ratio `<=32:1`;
- finite CPU/time budget;
- no nested compression.

## 10. Ordering and duplicate rules

Each direction has a `stream_sequence` (`uint64`) in every `WireEnvelope`:

- first value is `1`;
- values increase by exactly `1`;
- `0`, wrap, gap or regression is session-fatal;
- TLS/TCP ordering means a gap is not repaired by application replay.

Every client gameplay command additionally has:

- `command_id`: exactly 16 client-generated CSPRNG bytes, unique within the Game Session;
- `client_sequence`: `uint64`, first `1`, increasing by exactly `1` for commands only;
- `created_monotonic_ms`: optional local diagnostic duration value, never authoritative time.

Otheryn keeps a bounded deduplication/result cache for the active session. An exact duplicate `(session_id, command_id, client_sequence, canonical payload hash)` returns the known latest result without applying the command twice. Reuse of an ID or sequence with different payload is session-fatal. A duplicate outside the bounded cache is rejected as `STALE_COMMAND`; it is never re-applied speculatively.

Server gameplay/state ordering uses:

- `server_sequence`: the envelope stream sequence;
- `server_tick`: monotonic game tick, non-decreasing, not wall-clock time;
- `state_revision`: strictly increasing authoritative state baseline/delta revision.

## 11. Command vocabulary and authority

The client sends semantic intent only. Otheryn validates and owns all legality, mutation, random outcomes and persistence.

| Command | Required semantic input | Authoritative server result |
|---|---|---|
| `Step` | direction | collision, speed, destination and movement events |
| `StopMovement` | none | final movement state |
| `SetAttackTarget` / `ClearAttackTarget` | session-scoped entity handle | target validity, combat timing/effects |
| `SetFollowTarget` / `ClearFollowTarget` | session-scoped entity handle | pathing/follow state |
| `CastSpell` | stable spell ID plus optional target/position | knowledge, resources, cooldown, range, effects |
| `Use` | source item reference | script, ownership, range and mutations |
| `UseWith` | source plus target item/entity/position | script and all effects |
| `MoveItem` | source, destination, quantity | identity, ownership, capacity and committed move |
| `QuickLoot` / `LootCorpse` | corpse handle and mode | ownership/range/rules/capacity/transfers |
| `Say` | mode, channel/recipient, text | permission, delivery and chat event |
| `Logout` | none | safe logout decision and session end |

Session-scoped entity/container/item handles are opaque and cannot be interpreted as database IDs. Position bounds are `x,y <=65535`, `z <=15`. Quantity is `1..65535`.

The client never sends claimed damage, healing, mana use, cooldown completion, item ownership, loot acquisition, completed inventory mutation or server tick.

## 12. Action result lifecycle

One command may produce zero or more `ActionResult` messages. Allowed transitions are:

```text
PENDING
  -> ACCEPTED | REJECTED | DELAYED | EXPIRED | CANCELLED
DELAYED
  -> DELAYED | ACCEPTED | REJECTED | EXPIRED | CANCELLED
ACCEPTED
  -> EFFECT_OBSERVED | COMPLETED | CANCELLED
EFFECT_OBSERVED
  -> EFFECT_OBSERVED | COMPLETED
terminal = REJECTED | COMPLETED | EXPIRED | CANCELLED
```

Semantics:

- `ACCEPTED`: server accepted the intent for processing; it does not claim the visible effect occurred.
- `REJECTED`: no effect from this command is committed; stable reason is required.
- `DELAYED`: command remains pending; an optional earliest server tick may be provided.
- `EFFECT_OBSERVED`: at least one authoritative effect associated with the command was emitted; related state revision is included.
- `COMPLETED`: command lifecycle is terminal. Long-lived target selection completes when the target state changes; later combat effects are separate authoritative events.
- `EXPIRED`: bounded server/client validity window elapsed before execution.
- `CANCELLED`: session shutdown or explicit server cancellation prevented remaining work.

Stable rejection/error reasons are drawn from the IDL and include at minimum:

```text
INVALID_COMMAND
UNSUPPORTED_COMMAND
NOT_AUTHENTICATED
SESSION_STALE
STALE_REFERENCE
NOT_FOUND
NOT_OWNED
NOT_VISIBLE
OUT_OF_RANGE
PATH_BLOCKED
COOLDOWN_ACTIVE
RESOURCE_INSUFFICIENT
CAPACITY_EXCEEDED
PERMISSION_DENIED
STATE_CONFLICT
RATE_LIMITED
SERVER_UNAVAILABLE
```

Human-readable text is optional, localized by the client when a stable code exists, bounded to 256 UTF-8 bytes and never contains secrets or internal identifiers.

## 13. Initial snapshot and state deltas

### Initial snapshot

After `ServerHello`, Otheryn emits:

```text
SnapshotBegin(snapshot_id, state_revision, chunk_count, total_encoded_bytes)
SnapshotChunk(snapshot_id, chunk_index, records...)
SnapshotEnd(snapshot_id, state_revision, sha256)
```

Rules:

- snapshot ID is 16 random bytes;
- chunk count is `1..256`;
- chunk indexes are contiguous from `0`;
- records within and across chunks are in canonical order defined by record type then stable key;
- complete encoded snapshot is `<=16 MiB`;
- SHA-256 covers the concatenation of canonical serialized `SnapshotChunk` payloads in index order;
- the client exposes no gameplay-ready state until all chunks, digest and bounds validate;
- a second snapshot replaces, never merges with, an incomplete snapshot.

Core v1 snapshot records cover visible map regions/tiles, visible entities, local player stats/resources, inventory, open containers, cooldowns, combat/target state and negotiated feature state. Features outside the core profile require explicit capabilities and extension schemas.

### Deltas

Each `StateDelta` contains:

```text
base_revision
revision = base_revision + 1
server_tick
ordered mutations
```

The client applies a delta only when `base_revision` equals its current committed revision. An exact duplicate revision with the same canonical hash is ignored. A conflicting duplicate, regression or malformed mutation is session-fatal.

A revision gap does not permit partial application. The client sends one bounded `ResyncRequest(current_revision, reason)` and freezes authoritative mutation presentation. Otheryn responds with a fresh complete snapshot or closes with a typed error. Resync requests are rate-limited; no busy loop is allowed.

### Authority and reconciliation

- movement may use reversible visual prediction tagged with the originating command ID;
- authoritative entity movement/position mutations reconcile or roll back prediction;
- inventory, containers, loot, combat outcomes, resources, cooldowns and persistence are never client-authoritative;
- UI pending states are cleared only by typed result/state evidence or session termination;
- a new Game Session always begins with a full snapshot.

Native v1 has no session resume and no automatic command replay after disconnect. Reconnect/relog performs fresh OAuth/ticket/Gateway selection/session admission and a full snapshot.

## 14. Failure classes

| Class | Examples | Required behavior |
|---|---|---|
| request-local rejection | unsupported command, range/cooldown failure | typed `ActionResult.REJECTED`; session remains |
| recoverable synchronization | valid session but lost state baseline | one bounded resync; no partial apply |
| admission-fatal | expired/replayed credential, character/profile mismatch | typed bootstrap error where safe, close, no fallback |
| protocol-fatal | bad length, sequence gap, unknown message kind, conflicting duplicate | close immediately; bounded redacted metric |
| service-unavailable | issuer, World Registry or Otheryn unavailable | terminal login failure; fresh attempt required |
| implementation fault | impossible invariant/internal serialization error | fail closed, alert, no sensitive dump |

## 15. Downgrade and security invariants

- No password fallback exists in native flow.
- Gateway selects once. It never retries Canary after native session issuance fails.
- The client never changes adapter after ticket consumption, Gateway response, credential handoff, TLS bootstrap, partial admission or protocol failure.
- Otheryn accepts only the exact family/profile/transport/schema/capability digest bound to Game Session.
- A candidate or endpoint not present in current authoritative World Registry policy is never selected.
- A stale policy revision may finish only if Gateway and Otheryn both validate the exact bound revision as still admissible; otherwise admission fails and a fresh attempt is required.
- Ticket, session credential, session ID, command ID, account/character identifiers and raw payloads are not logged. Metrics use bounded reason codes and profile identifiers.
- Public Gateway requests remain size/rate limited; private Platform and session-issuer APIs remain service-authenticated and private.
- Native listener rate limits TLS handshakes, bootstrap attempts, frame rate, commands and resync requests independently.
- Ambiguous ticket/session/character-bind outcomes are never retried with the same credential.

See the threat model for negative tests.

## 16. Compatibility matrix

The matrix is exact-pair evidence, not semantic version optimism.

| Client | Gateway producer | Otheryn | Result before enablement |
|---|---|---|---|
| current client without gameplay offer | current Gateway | current Otheryn Canary | current Canary-compatible behavior only |
| client with offer support | current Gateway rejecting unknown field | current Otheryn | unsupported; producer must ship first |
| client with offer support | extended Gateway, native disabled | current Otheryn | Gateway selects explicitly allowed Canary candidate only |
| client without native adapter | extended Gateway, native allowed | native-capable Otheryn | Gateway may select Canary only when client offered it; otherwise typed incompatibility |
| native-capable client | extended Gateway, native allowed | Otheryn native disabled/not ready | native candidate must not be advertised; contradiction is hard failure |
| exact native-capable client | exact Gateway contract v2 | exact Otheryn native v1 | supported only after linked integrated staging evidence |
| any mismatched schema hash/profile/capability digest | any | any | unsupported/fail closed |

Every supported row must record exact Git SHAs, schema SHA-256, fixture version and integrated test evidence. “Latest”, branch names and broad version ranges are insufficient.

## 17. Deprecation policy

- `canary` remains a first-class compatibility family during and after native rollout unless a later separately authorized migration changes that decision.
- Native profile deprecation requires a new contract revision, replacement profile, measured client population, minimum-supported date, staged warning window, rollback plan and exact supported-pair evidence.
- World Registry may stop advertising a profile only after all required producer/consumer versions are deployed and rollback remains available.
- Unknown or below-minimum profiles fail before gameplay with a typed update/incompatibility result.

## 18. Contract tests and fixtures

### Ownership

| Artifact | Owner |
|---|---|
| Gateway JSON offer/selection fixtures and policy selection tests | `blakinio/Oteryn-Platform` |
| Game Session claim/binding fixtures and native producer golden frames | `blakinio/Otheryn` producer package, mirrored hash in Platform tests |
| Rust decoder/encoder golden fixtures and normalized replay | `blakinio/otclient` |
| cross-repository exact-pair manifest and integrated journey | integration/E2E package under coordination ID |

Fixtures committed to repositories are synthetic and contain no real tickets, credentials, account IDs, character names, endpoints or captures.

Required deterministic negative cases include:

- zero/9 candidates, duplicates and non-canonical identifiers;
- selected candidate absent from client offer;
- native disabled but advertised;
- stale/contradictory World Registry revision;
- ticket consumed then no match/issuer failure;
- session replay, cross-character, cross-world, cross-channel, cross-profile and cross-endpoint use;
- expired/revoked generation;
- TLS/ALPN/certificate mismatch;
- zero/oversize/truncated frames;
- unknown message kind/required enum;
- stream or command sequence gap/regression/wrap;
- duplicate command with same and different payload;
- malformed snapshot, digest mismatch, delta gap/conflict;
- no native-to-Canary switch after any post-selection failure;
- no password/OAuth/ticket leakage in logs or artifacts.

## 19. Producer/consumer dependency graph

```text
this canonical contract + linked correspondence
  -> Platform/Game Gateway producer extension (disabled)
  -> Otheryn Game Session v2 consumer + native producer (disabled)
  -> Rust protocol-oteryn adapter + native codec support
  -> automatic selection and exact integrated staging E2E
  -> bounded native enablement
```

The Platform producer may merge before consumers only while native advertisement is disabled. Otheryn may merge before the Rust consumer only while the native listener/candidate is disabled. The Rust adapter may merge without production selection only behind internal capability registration. Activation is atomic-required across exact deployed revisions.

## 20. Non-goals

This revision does not authorize runtime code, Tokio, Rust dependencies, a `protocol-oteryn` crate, Otheryn network handlers, Gateway request/response changes, Game Session storage changes, database migrations, production endpoints/secrets, deployment, native enablement, Canary removal or a production compatibility claim.
