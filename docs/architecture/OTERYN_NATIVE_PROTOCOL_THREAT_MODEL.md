# Oteryn native gameplay protocol threat model

Coordination ID: `OTS-20260804-native-protocol-selection`  
Canonical contract: `../contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`

## Assets

- reusable Oteryn Identity credentials and MFA state;
- one-time Game Login Ticket and game-auth generation;
- Game Session credential and admission state;
- account, character, world and channel authorization;
- selected adapter/profile/transport/schema/capabilities;
- authoritative gameplay state, commands and results;
- service identity, World Registry policy and rollout state;
- privacy of player communications and telemetry.

## Trust boundaries

```text
system browser <-> Oteryn Identity public OAuth
Rust client <-> Platform public ticket issue
Rust client <-> Game Gateway public HTTPS
Gateway <-> Platform private redeem/context
Gateway <-> Game Session issuer private API
Rust client <-> Otheryn native TLS endpoint
Otheryn protocol layer <-> authoritative game/domain state
```

Private APIs require authenticated service identity, private ingress and normal certificate/hostname validation. A public client field is never trusted as account, character, world, endpoint, policy or capability authority.

## Threats and mandatory controls

| Threat | Control | Required negative proof |
|---|---|---|
| candidate stripping/reordering | client offer is a bounded set; authoritative World Registry order controls selection | stripping native cannot cause an unadvertised profile or password fallback |
| candidate injection | exact canonical tuple and capability intersection; selected result must be in offer and policy | injected family/profile/schema/capability is rejected |
| protocol downgrade after auth | one selection bound to Game Session; no second candidate after issuance/admission failure | native failure never reconnects with Canary using same ticket/session |
| stale World Registry policy | policy revision bound to session; Otheryn validates admissibility | disabled/stale revision fails before gameplay |
| Game Session replay | opaque credential, expiry, generation, audience and atomic single admission | second connection, character, world, channel or profile fails |
| cross-character use | bind-on-first-admission plus authoritative ownership check | one credential cannot bind two characters |
| endpoint redirection | Gateway returns validated host/port/TLS name from World Registry; TLS identity checked | host/TLS contradiction fails closed |
| credential theft in logs/artifacts | structured redaction; no raw payload/ticket/session/command IDs | repository and staging log scan contains no secret material |
| malformed/oversize frames | 4-byte bounded length, pre-allocation checks, protobuf depth/count limits | zero, oversize, truncated and nested inputs close deterministically |
| compression bomb | native v1 has no compression; future profile has explicit output/ratio/time bounds | compressed v1 frame is fatal; synthetic bomb stays bounded |
| sequence manipulation | exact stream and command monotonicity plus duplicate payload hash | gap/regression/wrap/different-payload duplicate is rejected |
| command replay | session-scoped ID, bounded dedup result cache, no reconnect replay | exact duplicate is idempotent; stale duplicate never reapplies |
| state desynchronization | revisioned snapshots/deltas, digest, bounded resync | gap cannot partially mutate client state |
| client-authoritative mutation | semantic intent only; Otheryn owns all legality/effects | claimed damage/inventory/resource fields do not exist |
| information oracle | stable coarse reason codes and bounded safe text | unauthorized references do not disclose hidden identity/state |
| rate exhaustion | independent public login, TLS handshake, bootstrap, frame, command and resync limits | each boundary fails with bounded work/memory |
| ambiguous ticket/session outcome | consumed or possibly consumed credentials are never retried | timeout/error requires fresh ticket and session |
| service impersonation | mTLS or equivalent authenticated private clients, private ingress, TLS validation | public/incorrect service identity cannot redeem or issue sessions |

## Downgrade state machine

```text
UNAUTHENTICATED
  -> TICKET_ISSUED
  -> GATEWAY_REQUEST_VALIDATED
  -> TICKET_REDEEMED
  -> CANDIDATE_SELECTED
  -> SESSION_ISSUED
  -> TLS_ESTABLISHED
  -> SESSION_BOUND
  -> GAMEPLAY_ACTIVE
```

Before `CANDIDATE_SELECTED`, Gateway may choose any single allowed intersection candidate. After that state, no transition can change family/profile/transport/schema/capabilities. Failure transitions only to `TERMINAL_FAILED`; a fresh login starts from `UNAUTHENTICATED` with new identifiers.

## Abuse limits

- Gateway request body: existing 4 KiB until the producer extension measures and explicitly increases it; target request remains `<=16 KiB`.
- offer candidates: 8; capabilities per candidate: 64; capability token: 64 bytes.
- Gateway response: 64 KiB.
- native TLS bootstrap deadline: 5 seconds.
- native frame: 1 MiB; ordinary string: 4 KiB; chat: 1 KiB; nesting: 32; repeated entries: 4096.
- snapshot: 256 chunks, 512 KiB per chunk, 16 MiB complete.
- compressed native v1 frames: forbidden.
- command/resync/heartbeat rates: exact values are deployment measurements in the Otheryn implementation package; they must be finite and independent.

## Telemetry

Allowed low-cardinality labels:

```text
family
profile
transport
schema_revision
selection_result
bootstrap_error_code
protocol_error_code
action_reason
resync_reason
world_policy_revision bucket or deployment revision
```

Forbidden telemetry fields include raw ticket/session credential, OAuth token, session ID, command ID, account ID, character ID/name, chat text, payload bytes and certificate private material.

## Residual risks requiring implementation evidence

- exact TLS certificate/CA strategy for production and staging;
- concrete Game Session credential format and atomic storage transaction;
- measured command/frame/rate limits under realistic load;
- protobuf library unknown-field and allocation behavior in C++ and Rust;
- state snapshot size for representative worlds;
- operational drain time and rollback observability.

These are bounded implementation/measurement tasks, not permission to weaken the contract.
