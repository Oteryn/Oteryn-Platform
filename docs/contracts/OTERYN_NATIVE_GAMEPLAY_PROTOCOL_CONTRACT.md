# Oteryn Native Gameplay Protocol Contract

Status: `NORMATIVE`  
Contract revision: `2`  
Schema revision: `2`  
Canonical schema SHA-256: `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`  
Coordination ID: `OTS-20260804-native-protocol-selection`

## 1. Purpose and authority

This document is the canonical contract for the first native Oteryn gameplay protocol. It governs the Platform, Game Gateway, Otheryn and the Rust client.

The initial native implementation has exactly one identity:

| Dimension | Value | Authority |
|---|---|---|
| family | `oteryn` | this contract |
| native protocol version | `1` | this contract |
| native transport identifier | `tcp.tls13.protobuf.be32.v1` | this contract |
| schema revision | `2` | this contract/IDL |
| schema SHA-256 | `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9` | exact canonical IDL bytes |

There is no native profile field, value, placeholder, catalogue, enum, registry, factory, ordering or user/admin selection. No dotted native profile identifier is part of the native identity. Future native variants require a separate ADR, contract/schema revision and coordinated migration.

Canary compatibility profiles remain a separate compatibility mechanism. They are not changed by this contract and must not be imported into native Oteryn structures.

## 2. Component boundaries

- Oteryn Identity authenticates the user and never sends the user's password to Gateway or Otheryn.
- Platform World Registry is the policy authority for enabled protocol families, endpoint identity, exact native tuple and readiness.
- Game Gateway consumes a one-time Game Login Ticket, selects one offered protocol family and issues Game Session v2.
- Otheryn is authoritative for character admission and gameplay state/results.
- The Rust client implements independent `protocol-canary` and `protocol-oteryn` adapters over protocol-neutral transport/session infrastructure.
- Otheryn serves Canary-compatible gameplay through its existing ASIO stack and isolated Canary compatibility-profile mechanism.
- Native Oteryn uses a separate TLS/ASIO listener and parser. No byte sniffing, parser fallback or adapter translation is permitted.

Production native advertisement and the native listener remain disabled unless a separate explicit authorization names the concrete environment and activation.

## 3. Vocabulary

- **family** — independently implemented gameplay protocol family, currently `canary` or `oteryn`.
- **native_protocol_version** — native Oteryn gameplay version. The only allowed current value is integer `1`.
- **transport** — exact connection/framing/security identifier.
- **schema_revision/schema_sha256** — exact protobuf contract revision and digest.
- **capabilities** — exact sorted unique semantic feature tokens; they do not create alternative profiles.
- **policy_revision** — monotonically increasing World Registry policy revision bound to selection/session.
- **login_attempt_id** — opaque identifier binding the Identity→Gateway→Otheryn attempt.
- **command_id** — client-generated UUIDv4 scoped to one admitted Game Session.
- **server_sequence** — strictly increasing server event sequence scoped to one admitted Game Session.
- **state_revision** — monotonic revision of authoritative state domains used for delta/gap handling.

## 4. Canonical capability set

Native v1 requires exactly this sorted list:

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

The canonical capability digest is SHA-256 over each UTF-8 token followed by `\n`, in the listed byte order:

```text
f762b55d5108c135079cf0427424d9e9973e76b102321bcb5cacd1fe35a0f018
```

Unknown, duplicate, unsorted, omitted or additional native capability tokens fail closed. Capability order is canonical, not a preference order.

## 5. Gateway offer and selection

### 5.1 Authoritative offer

The Gateway response contains the one-time opaque Game Login Ticket and an authoritative gameplay offer. The client supplies build/platform metadata but does not choose account, endpoint, policy, schema, capability digest or session claims.

Example native candidate:

```json
{
  "family": "oteryn",
  "native_protocol_version": 1,
  "transport": "tcp.tls13.protobuf.be32.v1",
  "schema_revision": 2,
  "schema_sha256": "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9",
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
```

Rules:

1. Gateway builds candidates only from enabled World Registry policy intersected with exact healthy readiness.
2. A native candidate is emitted only when every native tuple value and capability digest matches exactly.
3. Disabled, missing, stale, contradictory or duplicate readiness emits no native candidate.
4. `candidates` contains `1..8` unique tuples. Array order has no preference meaning. At most one candidate may have `family = oteryn`; when present, its `native_protocol_version` is exactly `1`.
5. Each native candidate contains exactly family, native protocol version, transport, schema revision/hash and a sorted unique capability list of at most 64 tokens. It contains no `profile` key, alias or placeholder.
6. Gateway rejects selected-not-offered, duplicate, unknown, malformed or unsupported tuples.
7. Gateway never invents a second native candidate and never falls back after selection/session issuance.

### 5.2 Client offer

The Rust client sends one bounded offer containing its exact supported candidate tuples. The native candidate is present only when the compiled `protocol-oteryn` implementation and schema digest match this contract.

```json
{
  "offer_version": 1,
  "client_build": "<bounded canonical build id>",
  "client_platform": "<bounded canonical platform id>",
  "candidates": [
    {
      "family": "oteryn",
      "native_protocol_version": 1,
      "transport": "tcp.tls13.protobuf.be32.v1",
      "schema_revision": 2,
      "schema_sha256": "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9",
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
```

The client may also offer independently supported Canary tuples through the existing Canary compatibility mechanism. It must not synthesize a native candidate from a Canary profile.

### 5.3 Deterministic family selection

Production `Auto` mode accepts the Gateway-selected candidate only when it exactly matches one client-offered candidate. Family priority is server policy, not array order or a user/admin native choice.

After ticket redeem, candidate selection or credential issuance, adapter binding is immutable. Any transport, TLS, schema, capability, admission or session failure is terminal and requires a fresh Identity→ticket→Gateway flow. There is no same-session or same-ticket fallback to Canary or another native version.

Debug/test force-family modes may exist only behind non-production controls and still must select an exact offered tuple. No force-profile mode exists.

## 6. Game Login Ticket

The ticket is opaque to the client, one-time, short-lived and audience-bound to Gateway. It contains or resolves authoritative values including:

```text
ticket_id
login_attempt_id
identity_subject_binding
game_account_id
identity_security_generation
issued_at/not_before/expires_at
single_use = true
```

Gateway atomically consumes the ticket before issuing a session. Replay, expiry, audience mismatch, generation mismatch, malformed claims or ambiguous consume state fail closed. A ticket is never sent to Otheryn and an OAuth access/refresh token is never sent to Gateway/Otheryn.

## 7. Game Session v2

Game Session v2 is issued only after exact candidate selection. The response is authoritative and contains:

```json
{
  "protocol_version": 1,
  "game_session_contract_version": 2,
  "gameplay_selection": {
    "family": "oteryn",
    "native_protocol_version": 1,
    "transport": "tcp.tls13.protobuf.be32.v1",
    "schema_revision": 2,
    "schema_sha256": "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9",
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
    ],
    "capability_digest_sha256": "f762b55d5108c135079cf0427424d9e9973e76b102321bcb5cacd1fe35a0f018",
    "policy_revision": 1,
    "host": "native.invalid",
    "port": 7173,
    "tls_server_name": "native.invalid"
  },
  "credential": "<opaque-one-time-session-credential>"
}
```

The v2 claim set contains or resolves:

```text
session_id
login_attempt_id
game_account_id
identity_security_generation
world_id
channel_id
world_policy_revision
endpoint_id = stable World Registry/Otheryn endpoint identifier
audience = "otheryn-world:<world_id>:channel:<channel_id>:endpoint:<endpoint_id>"
character_binding_mode = bind_on_first_admission
selected family/native_protocol_version/transport/schema_revision/schema_sha256
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
5. The same credential cannot bind another character, world, channel, endpoint, native protocol version, schema, capability set or connection.
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

## 8. Native transport and bootstrap

Native v1 uses a separately advertised TCP endpoint. It never shares a sniffing/fallback listener with Canary.

TLS rules:

- TLS 1.3 only;
- ALPN exactly `oteryn-game/1`;
- normal certificate-chain and exact `tls_server_name` validation;
- no plaintext or cross-family fallback;
- TLS AEAD supplies confidentiality/integrity; no application checksum/MAC;
- credentials, session/command IDs, account/character identifiers and payloads are redacted.

Bootstrap:

1. `ClientHello` must arrive within 5 seconds of TLS completion.
2. It carries the opaque credential, login attempt, selected character, world/channel/policy, exact native protocol version/transport/schema/list/digest and bounded build metadata.
3. Otheryn validates all fields against stored v2 claims and atomically binds admission.
4. Otheryn emits one `ServerHello` or one safe typed `ProtocolError`, then closes on failure.
5. `ServerHello` echoes the immutable selection and admission identity.
6. Any mismatch is session-fatal `SESSION_BINDING_MISMATCH`; no candidate switch occurs.

## 9. Framing and serialization

Every post-TLS frame is:

```text
uint32_be payload_length
payload_length bytes containing exactly one protobuf WireEnvelope
```

`uint32_be` is an unsigned 32-bit big-endian integer. The prefix is not included in `payload_length`.

- payload length `1..1,048,576` bytes, excluding the prefix;
- zero, oversize, truncated, trailing or multiple-envelope input is session-fatal;
- length is validated before allocation and reusable buffers are bounded;
- protobuf `proto3`, schema revision 2;
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

Native v1 compression is `none`; compressed data/flags are fatal. Any later compression requires a new transport or schema revision and explicit compressed/decompressed/ratio/CPU limits.

## 10. Ordering and duplicate policy

Each admitted session starts with `server_sequence = 1`. Every server envelope increments by exactly one, including command results, state deltas, corrections, chat and logout. The client accepts only the next expected sequence; a gap suspends application and triggers one bounded resync request. Duplicate/old sequence is ignored only when it is byte-identical and already applied; conflicting duplicate is fatal.

Each command carries:

```text
command_id UUIDv4, non-zero
client_sequence uint64, starts at 1 and increments by exactly one
expected_state_revision for the affected domain when required
```

Otheryn keeps a bounded per-session idempotency cache keyed by `command_id` and the canonical command hash. Exact duplicate returns the original result/effects without re-execution. Same ID with different bytes is fatal `COMMAND_ID_CONFLICT`. Out-of-order/gapped client sequence is rejected without state mutation. Cache exhaustion fails closed rather than evicting an entry whose retry window is still valid.

## 11. Snapshot, delta and reconciliation

After `ServerHello`, Otheryn sends one complete initial snapshot before accepting gameplay commands. It contains all state needed to render/play the admitted character: self, visible entities, map/tiles, inventory/equipment/containers, stats/resources/cooldowns, combat/target state, chat-channel state and authoritative clocks/revisions.

Snapshot chunks carry one snapshot ID, chunk index/count, base server sequence and per-domain revisions. They are applied atomically only after every chunk validates. Mixed IDs, duplicate conflicting chunks, missing chunks, excess size or timeout discard the assembly and require one bounded replacement snapshot.

Deltas carry `base_state_revision` and `new_state_revision`. The client applies only when base equals current. A revision gap or impossible transition suspends affected-domain application, sends one bounded `ResyncRequest` and accepts either contiguous replay or a complete replacement snapshot. It never guesses, skips or applies future deltas.

Movement may be predicted locally only for presentation. `MoveCommand` carries command ID, direction/path intent and expected movement revision. Otheryn validates collision, speed, conditions and position, then emits accepted/rejected result plus authoritative movement delta/correction. Corrections replace predicted state; they do not create a second command.

## 12. Authoritative action lifecycle

The client sends semantic commands only. It never sends raw Canary opcodes, raw map mutations, damage, loot results or authoritative coordinates beyond bounded intent.

Every command yields exactly one terminal `CommandResult` (`accepted`, `rejected`, `delayed`, `completed`, `cancelled` as applicable) and zero or more authoritative effect/state events correlated by command ID.

Required v1 commands/events cover:

- movement and correction;
- attack/follow target set/clear;
- spell cast accepted/rejected/delayed/effect/completed;
- item use, use-with and move;
- loot success/failure;
- inventory/equipment/container changes;
- chat send/receive/channel lifecycle;
- logout and terminal disconnect reason;
- resync request/snapshot replacement.

Otheryn validates ownership, visibility, range, cooldown, resources, state revision and permissions. Rejected/delayed commands do not mutate unauthorized state. Effects are server-derived and ordered by server sequence.

## 13. Error taxonomy

Errors are typed, safe and terminal according to scope. They never include credential material, internal stack traces, database identifiers or sensitive payload excerpts.

| Code | Scope | Retry |
|---|---|---|
| `MALFORMED_FRAME` | session | fresh session |
| `FRAME_TOO_LARGE` | session | fresh session |
| `DECODE_FAILED` | session | fresh session |
| `UNSUPPORTED_WIRE_VERSION` | session | update client/server |
| `UNSUPPORTED_NATIVE_PROTOCOL_VERSION` | selection/session | update/policy correction |
| `SCHEMA_MISMATCH` | selection/session | exact deployment correction |
| `CAPABILITY_MISMATCH` | selection/session | exact deployment correction |
| `TICKET_REPLAY` | Gateway | fresh Identity flow |
| `SESSION_REPLAY` | Otheryn | fresh Identity flow |
| `SESSION_EXPIRED` | Otheryn | fresh Identity flow |
| `SESSION_BINDING_MISMATCH` | Otheryn | fresh Identity flow |
| `COMMAND_ID_CONFLICT` | session | fresh session |
| `SEQUENCE_GAP` | affected state/session | bounded resync then fresh session |
| `STATE_REVISION_MISMATCH` | command/domain | correction/resync |
| `COMMAND_REJECTED` | command | client-visible reason policy |
| `INTERNAL_UNAVAILABLE` | session | fresh flow after service recovery |

Unknown error codes are fatal and displayed generically.

## 14. Readiness and World Registry

Otheryn readiness is authenticated and contains only exact current listener identity and health:

```json
{
  "enabled": false,
  "family": "oteryn",
  "native_protocol_version": 1,
  "transport": "tcp.tls13.protobuf.be32.v1",
  "schema_revision": 2,
  "schema_sha256": "9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9",
  "capability_digest_sha256": "f762b55d5108c135079cf0427424d9e9973e76b102321bcb5cacd1fe35a0f018",
  "endpoint_id": "<stable-id>",
  "alpn": "oteryn-game/1",
  "tls_min_version": "1.3"
}
```

Gateway advertises native only when:

- policy row exists and is explicitly enabled;
- endpoint/readiness is healthy and fresh;
- every tuple/digest/TLS/ALPN value matches exactly;
- no contradictory or duplicate native row/readiness exists;
- production environment has separate explicit activation authorization.

Missing/false/stale/mismatch means no native offer and no native Game Session.

## 15. Security, privacy and observability

Required controls:

- TLS 1.3 and exact ALPN/SNI/certificate validation;
- one-time ticket, one-time single-admission session credential and atomic bind;
- exact account/world/channel/endpoint/policy/native-version/schema/capability binding;
- anti-replay for tickets, sessions, command IDs and sequences;
- bounded frame, decode, string, repeated-field, nesting, snapshot, queue and timeout limits;
- no plaintext fallback, parser sniffing, candidate switch or post-selection family fallback;
- no Oteryn password in the Rust client;
- no OAuth token to Gateway/Otheryn;
- no raw identity subject to Otheryn;
- credentials, account/character/session/command identifiers, chat, payloads and sensitive reasons redacted from logs/traces/artifacts;
- metrics use bounded categorical labels only; no high-cardinality IDs;
- malformed/ambiguous state fails closed.

Required bounded telemetry includes offer/selection outcome, exact non-sensitive tuple revision, readiness mismatch category, TLS/ALPN outcome, admission outcome, parser rejection category, sequence/revision gaps, resync count, command-result category, queue depth, snapshot size/chunks, command latency, CPU/memory/allocation and disconnect reason category.

## 16. Rollout and rollback

Required merge order:

1. Platform canonical contract/IDL correction;
2. Otheryn correspondence pinned to the exact merged Platform commit and schema digest;
3. Rust correspondence pinned to exact merged Platform and Otheryn commits;
4. corrected Platform/Gateway producer, disabled;
5. Otheryn native listener/session/gameplay producer, disabled;
6. Rust adapter, not offered in production;
7. bounded real staging E2E and rollback;
8. production canary only after separate environment-specific owner authorization.

Rollback order:

1. disable native advertisement for fresh sessions;
2. increment policy revision if required;
3. verify Gateway selects no native session;
4. drain/close native sessions according to policy;
5. disable Otheryn native listener;
6. preserve Canary for fresh explicitly allowed sessions;
7. never switch an active/failed native session to Canary.

Canary remains a first-class compatibility family unless a later authorized ADR changes it. Any additional native version or native deprecation/removal requires a separate later ADR, a new contract/schema revision, coordinated Platform/Gateway/Otheryn/Rust migration, measured population evidence, warning period, exact-pair evidence and rollback. The current contract has no placeholder for that future work.

## 17. Required validation

Before each merge and again on integrated staging:

- canonical schema SHA and fixtures;
- JSON/IDL no-native-profile assertions;
- parser malformed/truncated/oversize/trailing/unknown-enum/oneof/UTF-8/nesting/collection tests;
- TLS/ALPN/SNI/certificate negatives;
- ticket/session replay and cross-binding negatives;
- selected-not-offered/native-version/schema/capability/readiness/downgrade negatives;
- command duplicate/conflict/sequence/idempotency tests;
- snapshot chunk/gap/timeout/size/replacement tests;
- delta revision gap/replay/replacement tests;
- movement correction and full authoritative action lifecycle;
- log/artifact redaction checks;
- explicit `protocol-canary` regression;
- bounded real Identity→ticket→Gateway→Game Session v2→TLS→Otheryn→Rust journey;
- rollback rehearsal proving no fresh native selection/listener and no cross-family switch.

Critical, high or material-medium audit findings block merge. Each final repository head and the integrated manifest require fresh independent architecture, security, parser, CI/regression and rollout/rollback perspectives.
