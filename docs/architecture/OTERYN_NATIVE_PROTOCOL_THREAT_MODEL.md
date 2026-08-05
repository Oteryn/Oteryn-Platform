# Oteryn native gameplay protocol threat model

Coordination ID: `OTS-20260804-native-protocol-selection`  
Canonical contract: `../contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`

## Assets

- reusable Oteryn Identity credentials, MFA and `game_auth_generation`;
- one-time Game Login Ticket;
- opaque Game Session credential and server-side admission claims/state;
- account, character, world, channel and endpoint authorization;
- exact adapter/native-protocol-version/transport/schema/capability selection;
- authoritative commands, results and gameplay state;
- service identity, World Registry policy and rollout state;
- player communication and telemetry privacy.

## Trust boundaries

```text
system browser <-> Oteryn Identity public OAuth
Rust client <-> Platform public ticket issue
Rust client <-> Game Gateway public HTTPS
Gateway <-> Platform private redeem/context
Gateway <-> Game Session issuer private API
Rust client <-> Otheryn native TLS endpoint
Otheryn protocol projection <-> authoritative game/domain state
```

Private APIs require authenticated service identity, private ingress and normal TLS certificate/hostname validation. Client fields never establish account, character, world, endpoint, policy, capability or rollout authority.

## Threats and mandatory controls

| Threat | Control | Required negative proof |
|---|---|---|
| candidate stripping/reordering | offer is a bounded set; World Registry order is authoritative | stripping native cannot select an unadvertised native protocol version or password fallback |
| candidate injection | exact tuple, sorted list and deterministic digest must occur in offer and policy | injected family/native-protocol-version/schema/capability is rejected |
| downgrade after redeem | one Gateway selection bound to Game Session; no second candidate | every post-selection native failure is terminal for that ticket/session |
| stale or changed policy | current policy used at issuance; bound revision/tuple audited; Otheryn validates exact local readiness identity | tampered/mismatched revision fails; disabling advertisement stops new issuance without silently switching active sessions |
| Game Session replay | opaque secret, hashed server-side lookup, expiry, generation, exact audience and atomic single admission | second connection/character/world/channel/native-protocol-version/endpoint fails |
| cross-character use | authoritative ownership lookup plus atomic bind-on-first-admission | one credential cannot bind two characters |
| endpoint redirection | selected endpoint and TLS name come from World Registry; TLS identity is verified | route/TLS contradiction fails closed |
| credential leakage | structured redaction and synthetic fixtures | retained logs/artifacts contain no OAuth/ticket/session/identity/payload data |
| malformed/oversize frame | BE32 pre-allocation bound plus protobuf depth/count/string limits | zero, oversize, truncated, unknown-payload and nested inputs close deterministically |
| compression bomb | native v1 compression forbidden | compressed input is fatal with bounded resource use |
| stream manipulation | exact per-direction monotonic sequence | zero/gap/regression/wrap is fatal |
| command replay/conflict | 16-byte ID, command sequence and exact received-submessage byte hash in bounded result cache | exact cached duplicate is idempotent; conflict fatal; stale duplicate never reapplies |
| state desynchronization | complete digest-checked snapshot, strict revision deltas and one bounded resync | gaps never partially mutate committed state; duplicate/regressed deltas are fatal |
| client-authoritative mutation | semantic intent only | no fields permit claimed damage, inventory, loot, resource or cooldown result |
| information oracle | coarse stable reasons and bounded safe detail | unauthorized references disclose no hidden identity/state |
| rate exhaustion | independent finite login, TLS, bootstrap, frame, command, heartbeat and resync limits | every boundary fails with bounded CPU/memory/time |
| ambiguous ticket/session result | possibly consumed credentials are terminal | timeout/error requires fresh ticket and Game Session |
| private-service impersonation | authenticated private clients, private ingress and TLS verification | public/incorrect identity cannot redeem or issue sessions |

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

Before `CANDIDATE_SELECTED`, Gateway chooses at most one exact intersection candidate. Afterwards family/native-protocol-version/transport/schema/capabilities are immutable. Any failure transitions to `TERMINAL_FAILED`; a new attempt starts with new ticket, login attempt, session and command namespace.

## Policy-disable and emergency behavior

- Normal disablement removes native from new World Registry/Gateway selections.
- Already-issued unexpired credentials may bind and active sessions may drain while the exact listener/readiness identity remains enabled.
- An explicit admission-revocation generation rejects not-yet-bound credentials.
- Emergency listener shutdown closes native sessions; it does not migrate them to Canary.
- No Otheryn live query to World Registry is required during admission; the signed/stored session tuple and local readiness identity are the enforcement inputs.

## Abuse limits

- target Gateway request `<=16 KiB`; response `<=64 KiB`;
- 8 candidates; 64 capabilities/candidate; 64 bytes/token;
- native bootstrap deadline 5 seconds;
- frame/message 1 MiB; ordinary string 4 KiB; chat 1 KiB; nesting 32; repeated entries 4096;
- snapshot 256 chunks, 512 KiB/chunk, 16 MiB total;
- native v1 compression forbidden;
- exact command/heartbeat/resync rates are finite measured values selected by the implementation package before final tests.

## Telemetry

Allowed low-cardinality labels:

```text
family
native_protocol_version
transport
schema_revision
selection_result
bootstrap_error_code
protocol_error_code
action_reason
resync_reason
deployment_revision
```

Forbidden fields include OAuth token, ticket/session credential, session/command ID, account/character ID or name, chat, payload bytes and certificate private material. Raw `world_policy_revision` is not a general metric label; it belongs in bounded deployment evidence/audit records.

## Residual implementation evidence

- exact staging/production certificate and CA policy;
- atomic v2 hashed credential storage and admission transaction;
- measured command/frame/rate limits under realistic load;
- C++/Rust protobuf unknown-field and allocation behavior;
- representative snapshot size and mutation rate;
- drain duration and emergency rollback observability.

These are bounded implementation/measurement tasks and do not permit weakening this contract.
