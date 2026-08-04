# Oteryn native gameplay protocol contract

Coordination ID: `OTS-20260804-native-protocol-selection`  
Contract revision: `1`  
Source of truth: `blakinio/Oteryn-Platform`  
Review IDL: [`oteryn_native_gameplay_v1.proto`](oteryn_native_gameplay_v1.proto)  
Architecture decision: [`ADR 0010`](../architecture/adr/0010-native-gameplay-protocol-selection.md)  
Security analysis: [`OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md`](../architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md)  
Rollout: [`OTERYN_NATIVE_PROTOCOL_ROLLOUT.md`](../architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md)

## 1. Status and preserved authority chain

This is the canonical producer/consumer contract for a future native Oteryn gameplay protocol. It is contract and architecture only. It does not prove runtime support, enablement, deployment or production compatibility.

The existing entry chain remains mandatory:

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

No implementation may introduce another login server, Identity authority, ticket type, client password flow, direct OAuth authentication to Otheryn, direct Game Login Ticket consumption by Otheryn or a Gateway bypass.

The following concepts are distinct:

| Concept | Initial value | Owner |
|---|---|---|
| Gateway login API version | existing JSON `protocol_version: 1` | Game Gateway |
| gameplay-offer shape version | `1` | this contract/Gateway |
| Game Session contract version | `2` | Gateway producer and Otheryn consumer |
| gameplay adapter family | `canary` or `oteryn` | World Registry vocabulary |
| native gameplay profile | `oteryn.native.v1` | this contract |
| native transport profile | `tcp.tls13.protobuf.be32.v1` | this contract |
| schema revision | `1` | this contract/IDL |
| client build/platform metadata | bounded non-authoritative strings | Rust client, validated by Gateway |

One serialized version field must never represent more than one row.

## 2. Current and target truth

### Current

- `/v1/login` accepts Gateway API `protocol_version: 1` and one Game Login Ticket.
- Gateway redeems the ticket, obtains exactly one authoritative world plus account-authorized characters, creates a Game Session request containing account, world and login-attempt identifiers, and returns one session credential.
- Otheryn serves profile-driven Canary-compatible gameplay through its existing ASIO stack.
- Rust contains protocol-neutral contracts and `protocol-canary`; `protocol-oteryn` does not exist.

### Target, disabled until later packages

- Gateway accepts a bounded gameplay offer and makes one deterministic selection from current World Registry policy.
- Game Session contract version 2 stores the exact selection and admission bindings server-side behind an opaque credential.
- Otheryn exposes a separate native TLS listener and validates the stored binding before gameplay.
- Rust implements an independent native adapter and binds one immutable adapter identity to one Game Session.

Target behavior must remain labelled unimplemented until exact linked runtime evidence exists.

## 3. Negotiation decision

The client sends a bounded supported-candidate **set** in the existing Gateway login request. Gateway is the sole final selector.

Target request:

```json
{
  "protocol_version": 1,
  "game_login_ticket": "<opaque>",
  "gameplay_offer": {
    "offer_version": 1,
    "client_build": "oteryn-client-0.1.0+abcdef0",
    "client_platform": "windows-x86_64",
    "candidates": [
      {
        "family": "oteryn",
        "profile": "oteryn.native.v1",
        "transport": "tcp.tls13.protobuf.be32.v1",
        "schema_revision": 1,
        "schema_sha256": "<64 lowercase hex characters>",
        "capabilities": [
          "actions.command-result.v1",
          "chat.semantic.v1",
          "combat.server-authoritative.v1",
          "inventory.server-authoritative.v1",
          "ordering.server-sequence.v1",
          "reconciliation.movement.v1",
          "session.single-admission.v1",
          "state.revision.v1",
          "state.snapshot-delta.v1"
        ]
      }
    ]
  }
}
```

Request rules:

1. Target request body is at most `16 KiB`. JSON shape, duplicate object keys, trailing JSON, identifier grammar, lengths, counts and candidate uniqueness are validated before ticket redeem.
2. `client_build` is `1..64` printable ASCII bytes and is compatibility metadata, not authority.
3. `client_platform` uses lowercase ASCII grammar `[a-z0-9][a-z0-9._-]{0,63}` and is telemetry/compatibility metadata, not authority.
4. `candidates` contains `1..8` unique tuples. Array order has no preference meaning.
5. Each candidate contains exactly family, profile, transport, schema revision/hash and a sorted unique capability list of at most 64 tokens.
6. Gateway redeems the ticket, obtains the one current authoritative world and reads the current ordered World Registry candidate policy for initial `channel_id = 1`.
7. Gateway selects the first enabled World Registry candidate whose exact family/profile/transport/schema tuple was offered and whose required capabilities are present. Selected optional capabilities are the authoritative allowed intersection.
8. Gateway issues exactly one Game Session for that selection or returns one terminal failure. It never tries another candidate after session-issuer invocation.
9. A syntactically valid no-match result occurs after successful redeem and consumes the ticket. A fresh attempt requires a fresh ticket.
10. Production `Auto` preference comes only from World Registry order. Development force modes may restrict the offered set but cannot force acceptance.

Rejected mechanisms:

- returning a candidate set for a second client-side decision after redeem;
- game-server negotiation after credential handoff;
- inferring/falling back from gameplay bytes;
- translating native messages through Canary packets.

## 4. Gateway public failures and ticket state

| HTTP | Public code | Ticket state | Meaning |
|---:|---|---|---|
| `400` | `invalid_request` | unconsumed because validation precedes redeem | malformed, oversize, non-canonical or contradictory request |
| `401` | `invalid_login` | invalid or consumed according to authoritative redeem result | ticket cannot be redeemed |
| `409` | `unsupported_gameplay_pair` | consumed | valid login context but no exact allowed candidate intersection |
| `503` | `login_unavailable` | treat as consumed whenever redeem may have occurred | Platform, World Registry, readiness or session issuer unavailable/ambiguous |

Responses do not disclose candidate policy, account ownership, route internals or which private dependency failed. A client never retries the same ticket after `401`, `409`, `503` or an ambiguous network failure.

## 5. Authoritative ownership

| Responsibility | Sole owner | Validator/consumer |
|---|---|---|
| reusable credentials, OAuth, MFA, game-auth generation | Oteryn Identity/Platform | Gateway private client |
| ticket issue, expiry and atomic redeem | Platform | Gateway |
| ordered candidates, endpoints, policy revision and rollout flags per world/channel | Platform World Registry | Gateway/operations |
| final candidate selection | Game Gateway | client and Otheryn |
| Game Session v2 server-side claim production | Gateway session issuer boundary | Otheryn |
| native listener readiness and TLS service identity | Otheryn deployment/readiness | Gateway/World Registry |
| first character admission and exact session consume | Otheryn | client observes result |
| gameplay legality, state, ordering and results | Otheryn | Rust client |
| locally compiled candidate support | Rust build | Gateway receives bounded offer |
| immutable adapter binding and wire mapping | Rust game-session/protocol layer | domain/application |
| exact compatibility matrix/evidence | this registry plus linked package manifests | all repositories |

No client field establishes account, character, route, endpoint, policy, rollout or capability authority.

## 6. Candidate canonicalization

Identifier grammar is lowercase ASCII `[a-z0-9][a-z0-9._-]{0,63}`. Capability tokens use the same grammar, are sorted by unsigned UTF-8 byte order and are unique.

A candidate identity is the tuple:

```text
family
profile
transport
schema_revision
schema_sha256
selected_capability_digest_sha256
```

Schema hash is SHA-256 of the exact UTF-8 bytes of the checked-in IDL with LF line endings and no BOM.

Capability digest is SHA-256 of the concatenation, in sorted order, of each UTF-8 capability token followed by one byte `0x0A`. The base native list is non-empty. Producers and consumers compare both the sorted list and its digest.

Base native v1 capabilities are exactly:

```text
actions.command-result.v1
chat.semantic.v1
combat.server-authoritative.v1
inventory.server-authoritative.v1
ordering.server-sequence.v1
reconciliation.movement.v1
session.single-admission.v1
state.revision.v1
state.snapshot-delta.v1
```

Unknown required capabilities make a pair unsupported. Unknown optional capabilities are excluded from selection. Capabilities cannot weaken base TLS, framing, authority or size limits.

## 7. Gateway response

Target success response retains Gateway API version 1:

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
    "capabilities": ["<sorted selected capability>"],
    "capability_digest_sha256": "<64 lowercase hex characters>",
    "host": "game.example.invalid",
    "port": 7173,
    "tls_server_name": "game.example.invalid"
  },
  "worlds": ["<exactly one existing world object>"],
  "characters": []
}
```

Bounds:

- response body `<=64 KiB`;
- exactly one world for initial contract revision 1;
- `0..100` characters, all belonging to that world;
- selected capabilities `1..64` for native v1;
- host/TLS name `1..253` ASCII bytes, no wildcard/URL syntax;
- port `1..65535`;
- login attempt ID exactly 16 random bytes rendered as 32 lowercase hex characters.

`gameplay_selection.host/port/tls_server_name` is the authoritative selected gameplay endpoint. The legacy `worlds[0].host/port` remains presentation/Canary compatibility data and must not override a native selection when the endpoints differ.

The client fails closed when the selection was not exactly offered, identifiers are non-canonical, schema/list/digest disagree, endpoint/TLS identity is invalid or any field differs from server bootstrap.

## 8. Game Session contract version 2

### Credential representation

The initial v2 credential is an opaque high-entropy CSPRNG bearer secret/reference. Otheryn issuer/consumer storage retains only the repository-approved hash and server-side claims. It is not a client-readable JWT and contains no self-asserted client claims. A future storage implementation may change only through a compatible contract that preserves opacity, one-time admission and all bindings.

### Exact server-side claims

```text
contract_version = 2
session_id = 16 random bytes
login_attempt_id = 16 random bytes
game_account_id = authoritative Otheryn account ID returned by Platform ticket redeem
identity_security_generation = authoritative game_auth_generation
world_id = exact Platform World Registry world
channel_id = 1 for initial revision
world_policy_revision
endpoint_id = stable World Registry/Otheryn endpoint identifier
audience = "otheryn-world:<world_id>:channel:<channel_id>:endpoint:<endpoint_id>"
character_binding_mode = bind_on_first_admission
selected family/profile/transport/schema_revision/schema_sha256
selected sorted capabilities/capability_digest_sha256
issued_at/not_before/expires_at
single_admission = true
admission_state = ISSUED_UNBOUND | ACTIVE_BOUND | CONSUMED_FAILED | CLOSED
```

The raw Oteryn Identity subject is not sent to Otheryn. Identity binding is transitive and authoritative through ticket redeem, `game_account_id` and `identity_security_generation`.

### Character binding

1. The client selects one stable character ID from the Gateway-provided authoritative list and sends it in `ClientHello`.
2. Otheryn loads current ownership for `game_account_id` and validates that exact character.
3. In one atomic transaction/state transition it validates unexpired/unconsumed claims and changes `ISSUED_UNBOUND` to `ACTIVE_BOUND(character_id, connection_id)`.
4. No map, entity or character state is exposed before success.
5. The same credential cannot bind another character, world, channel, endpoint, profile, schema, capability set or connection.
6. Wrong binding or ambiguous consume/bind failure makes the credential terminal; a fresh ticket/session is required.

### Revocation, policy and replacement

- generation mismatch rejects new admission;
- expiry rejects admission;
- Gateway binds the current policy revision at issuance;
- Otheryn validates that the bound tuple matches its exact enabled listener/readiness identity, but does not query live World Registry during admission;
- disabling advertisement stops new issuance; already-issued unexpired sessions may bind/drain unless an explicit admission-revocation generation is raised or the native listener is emergency-disabled;
- tampered policy revision or tuple mismatch fails closed;
- relog/replacement creates new ticket, login attempt, session, command namespace, sequences and snapshot;
- old queued commands are discarded locally and rejected server-side.

## 9. Native transport and bootstrap

Native v1 uses a separately advertised TCP endpoint. It never shares a sniffing/fallback listener with Canary.

TLS rules:

- TLS 1.3 only;
- ALPN exactly `oteryn-game/1`;
- normal certificate-chain and exact `tls_server_name` validation;
- no plaintext profile;
- TLS AEAD supplies confidentiality/integrity; no application checksum/MAC;
- credentials, session/command IDs, account/character identifiers and payloads are redacted.

Bootstrap:

1. `ClientHello` must arrive within 5 seconds of TLS completion.
2. It carries the opaque credential, login attempt, selected character, world/channel/policy, exact profile/transport/schema/list/digest and bounded build metadata.
3. Otheryn validates all fields against stored v2 claims and atomically binds admission.
4. Otheryn emits one `ServerHello` or one safe typed `ProtocolError`, then closes on failure.
5. `ServerHello` echoes the immutable selection and admission identity.
6. Any mismatch is session-fatal `SESSION_BINDING_MISMATCH`; no candidate switch occurs.

## 10. Framing and serialization

Every post-TLS frame is:

```text
uint32_be payload_length
payload_length bytes containing exactly one protobuf WireEnvelope
```

- payload length `1..1,048,576` bytes, excluding the prefix;
- zero, oversize, truncated, trailing or multiple-envelope input is session-fatal;
- length is validated before allocation and reusable buffers are bounded;
- protobuf `proto3`, schema revision 1;
- field numbers are never reused; removed fields are `reserved`;
- unknown fields are ignored and cannot activate behavior;
- after decode exactly one known `payload` oneof must be set;
- unknown numeric enum values in required semantic fields, zero `UNSPECIFIED` where concrete value is required, and missing semantic presence are violations;
- strings are valid UTF-8 and bounded before allocation.

Limits:

| Item | Limit |
|---|---:|
| frame/ordinary decoded message | 1 MiB |
| ordinary UTF-8 string | 4 KiB unless smaller below |
| chat text | 1 KiB |
| capability token/list | 64 bytes / 64 entries |
| repeated entries in an ordinary message | 4096 |
| protobuf nesting depth | 32 |
| snapshot chunks | 256 |
| one encoded snapshot chunk frame | 512 KiB |
| complete initial snapshot | 16 MiB encoded |

Native v1 compression is `none`; compressed data/flags are fatal. Any later compression requires a new transport/profile revision and explicit compressed/decompressed/ratio/CPU limits.

## 11. Ordering and duplicate policy

Each direction uses `WireEnvelope.stream_sequence`:

- first value `1`;
- increase by exactly `1`;
- zero, gap, regression or wrap is session-fatal;
- no application replay repairs a TCP/TLS sequence gap.

Every gameplay command additionally uses:

- `command_id`: exactly 16 CSPRNG bytes, unique within the Game Session;
- `client_sequence`: first `1`, increasing by exactly `1` for commands;
- optional local monotonic diagnostic duration, never authoritative time.

Otheryn keeps a bounded active-session result cache. The command payload hash is SHA-256 of the exact serialized `CommandEnvelope` submessage bytes as received, excluding the outer `WireEnvelope`; unknown fields in commands are rejected so the byte identity is stable.

- exact duplicate ID, sequence and payload hash inside the cache returns the known latest result and never reapplies the command;
- reuse of an ID or sequence with a different payload is session-fatal;
- an exact duplicate no longer in cache is rejected as `STALE_COMMAND` and never reapplied.

Server ordering uses envelope `stream_sequence`, non-decreasing `server_tick` and strictly increasing committed `state_revision`.

## 12. Commands and server authority

The client sends intent only. Otheryn owns legality, timing, random outcomes, mutation and persistence.

| Command | Intent input | Server authority |
|---|---|---|
| `Step`, `StopMovement` | direction/stop | collision, speed, path and final position |
| attack/follow set/clear | session entity handle | visibility, target rules, path and combat state |
| `CastSpell` | stable spell ID and optional target | knowledge, resources, cooldown, range/LOS and effects |
| `Use`, `UseWith` | source and optional target | identity, ownership, range and scripts |
| `MoveItem` | source, destination, quantity | ownership, capacity and committed move |
| `QuickLoot`, `LootCorpse` | corpse handle/mode | ownership, rules, range, capacity and transfers |
| `Say` | mode/channel/recipient/text | permission, moderation and delivery |
| `Logout` | none | fight/state/save/session lifecycle |

Entity/item/container handles are opaque and session-scoped, not database IDs. Position bounds are `x,y <=65535`, `z <=15`; quantity `1..65535`.

The client never sends claimed damage, healing, mana/resource use, cooldown completion, ownership, loot acquisition, completed inventory mutation or server tick.

## 13. Action lifecycle

Every admitted command eventually receives one terminal `ActionResult` unless transport/session termination prevents delivery. On termination, the client locally marks all nonterminal commands cancelled and never retries them automatically.

Allowed transitions:

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

- `ACCEPTED`: admitted for processing, not proof of visible effect;
- `REJECTED`: no effect from this command committed; stable reason required;
- `DELAYED`: pending, optionally with earliest server tick;
- `EFFECT_OBSERVED`: at least one authoritative correlated effect/revision emitted;
- `COMPLETED`: terminal lifecycle result;
- `EXPIRED`: validity window elapsed before execution;
- `CANCELLED`: shutdown/server cancellation prevents remaining work.

Stable reasons are the IDL `ActionReason` enum, including `STALE_COMMAND`. Safe detail is optional, at most 256 UTF-8 bytes and contains no secrets/internal identifiers.

## 14. Snapshot, deltas and reconciliation

Initial state is:

```text
SnapshotBegin(snapshot_id, state_revision, chunk_count, total_encoded_bytes)
SnapshotChunk(snapshot_id, chunk_index, records...)
SnapshotEnd(snapshot_id, state_revision, chunks_sha256)
```

Rules:

- snapshot ID exactly 16 random bytes;
- `1..256` contiguous chunks indexed from zero;
- canonical record ordering by record type then stable key;
- complete encoded size `<=16 MiB`;
- `chunks_sha256` is SHA-256 of the concatenation, in chunk-index order, of the exact on-wire serialized `WireEnvelope` payload bytes for each `SnapshotChunk` frame, excluding each 4-byte frame-length prefix;
- no gameplay-ready state is committed until count/order/bounds/digest validate;
- an incomplete snapshot is discarded, never merged.

Core snapshot records cover visible tiles/regions, visible entities, local player state, inventory, open containers, cooldowns, combat/target state and negotiated capability state.

Each `StateDelta` uses envelope `server_tick` and contains:

```text
base_revision
revision = base_revision + 1
ordered mutations
```

A delta applies only when `base_revision` equals the committed revision. Any duplicate, regression, conflict or malformed mutation is protocol-fatal. A gap is not partially applied: the client sends one bounded `ResyncRequest`, freezes authoritative mutations and accepts a complete replacement snapshot or typed close. Resync is rate-limited and cannot busy-loop.

Movement may be reversibly predicted and tagged with `command_id`; authoritative movement reconciles/rolls it back. Inventory, containers, loot, combat outcomes, resources, cooldowns and persistence are never client-authoritative.

Native v1 has no session resume and no automatic command replay. Every reconnect/relog performs fresh ticket, Gateway selection, Game Session, admission and full snapshot.

## 15. Failure classes

| Class | Example | Behavior |
|---|---|---|
| command-local | range/cooldown/permission | typed rejection; session remains |
| recoverable synchronization | revision gap | one bounded resync; no partial apply |
| admission-fatal | expired/replayed/cross-bound credential | safe bootstrap error where possible, close, no fallback |
| protocol-fatal | length/sequence/unknown payload/conflicting duplicate | close immediately, redacted metric |
| service unavailable | Platform/Registry/issuer/Otheryn unavailable | terminal login failure; fresh attempt |
| implementation fault | impossible invariant/serialization fault | fail closed and alert without sensitive dump |

## 16. Downgrade and security invariants

- no password fallback;
- one Gateway selection only;
- no adapter change after ticket redeem, Gateway response, session issuance, credential handoff, TLS bootstrap, partial admission or protocol failure;
- Otheryn accepts only the stored exact selection and audience;
- disabled/unready candidates are not issued;
- disabling advertisement stops new sessions while explicit revocation/emergency listener shutdown controls already-issued sessions;
- ambiguous outcomes are never retried with the same credential;
- public Gateway, private Platform/issuer and native listener apply independent finite size/rate/deadline controls;
- private APIs remain service-authenticated and private; TLS/hostname validation remains normal;
- logs and artifacts exclude raw tickets, credentials, OAuth tokens, session/command IDs, account/character identifiers, chat and payloads.

## 17. Compatibility matrix

| Client | Gateway | Otheryn | Result |
|---|---|---|---|
| current no-offer client | current Gateway | current Canary path | current behavior only |
| offer client | current strict Gateway | current Otheryn | unsupported; Gateway producer must ship first |
| offer client | extended Gateway, native disabled | current Otheryn | explicitly offered/allowed Canary only |
| no-native client | extended Gateway, native ready | native Otheryn | Canary only if offered and allowed; otherwise `409` |
| native client | Gateway advertises contradictory readiness | disabled/mismatched Otheryn | hard failure; candidate must not be issued |
| exact native client | exact v2 Gateway | exact native v1 Otheryn | supported only after exact integrated staging evidence |
| any schema/list/digest/profile mismatch | any | any | fail closed |

Every supported pair records exact Git SHAs or immutable image/artifact digests, schema SHA-256, policy revision, capability digest and fixture manifest. Branch names, broad ranges and “latest” are insufficient.

## 18. Deprecation and rollout

Canary remains a first-class compatibility family unless a later authorized ADR changes it. Native deprecation/removal requires replacement profile, measured population, minimum-supported date, warning period, exact pair evidence and rollback.

Dependency order:

```text
canonical contract/correspondence
-> Platform/Gateway producer disabled by default
-> Otheryn v2 consumer/native producer disabled by default
-> Rust protocol-oteryn not offered in production
-> automatic selection and exact staging E2E
-> bounded enablement
```

Platform and Otheryn packages are server-first-safe only while native advertisement/listener stays disabled. Rust is client-first-safe only while production offers exclude native. Activation is atomic-required across exact deployed revisions. Rollback disables advertisement first, stops new issuance, drains/closes native sessions and then disables the listener; active native sessions never switch to Canary.

## 19. Contract tests and fixture ownership

| Artifact | Owner |
|---|---|
| Gateway JSON offer/selection/policy/session-producer fixtures | `blakinio/Oteryn-Platform` |
| Game Session v2 admission and native producer golden frames | `blakinio/Otheryn` |
| Rust encode/decode, malformed input and normalized replay | `blakinio/otclient` |
| exact-pair manifest and full journey | integration/E2E package |

Fixtures are synthetic and contain no real credentials, identities, endpoints, chat or proprietary captures.

Required negative coverage includes malformed/duplicate/oversize offer, selected-not-offered, disabled/contradictory readiness, ticket consume/no-match/issuer ambiguity, replay and cross-character/world/channel/profile/endpoint use, generation/expiry, TLS/ALPN/certificate mismatch, malformed/oversize frame, unknown payload/enum, sequence gap/regression/wrap, command duplicate/conflict/stale cache, malformed snapshot/digest, delta gap/duplicate/regression, no post-selection fallback and redaction scans.

## 20. Non-goals

This contract does not authorize Tokio or Rust dependency changes, creation of `protocol-oteryn`, Otheryn listener/handler changes, Gateway runtime/API changes, Game Session storage changes, migrations, deployment, endpoint/secrets, production enablement, Canary removal or production compatibility claims.
