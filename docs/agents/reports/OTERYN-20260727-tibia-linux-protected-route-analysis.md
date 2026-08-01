# Tibia Linux 15.30 protected/unprotected route analysis

## Scope and evidence identity

This report documents static analysis of the official Linux client identified as:

```text
Tibia version: 15.30.358f69
ELF: x86-64
size: 51,952,736 bytes
sha256: 8b25d65ece158723dbb50a1b592c1ec8a3247a650fcd2d299bebdfd133cb5752
Build ID: a7256985ece88dc38f45b9248c6119c22359ae6a
persistent run ID: 30246435256-1
```

The binary remained in Docker volume `oteryn-tibia-linux-analysis`. GitHub artifacts contained text-only reports, disassembly, indices, and hashes. No CipSoft binary was committed or uploaded.

Evidence labels used below:

- **confirmed**: directly demonstrated by the analyzed binary or current OTClient source.
- **strong inference**: the control flow and data layout support the conclusion, but a semantic name or server-side behavior is not directly observable.
- **unknown**: static client analysis does not establish the fact.
- **disproven**: the analyzed evidence contradicts the hypothesis.

## Complete client-side flow

```text
loginservice JSON response
  -> required world fields are parsed
     - externaladdressprotected / externalportprotected
     - externaladdressunprotected / externalportunprotected
     - anticheatprotection
     - BattlEyeActivationTimestamp / BattlEyeInitiallyActive
  -> world record
  -> TGameserverGameSessionMetaInfo
     - first QHostAddress / port pair: protected endpoint
     - second QHostAddress / port pair: unprotected endpoint
     - protection mode and secondary flag
  -> BattlEye setup
     - QLibrary loads bin/BattlEye/BEClient.so
     - resolves Init
     - calls Init(2, initParameters)
     - protected address and port are supplied to the BE setup
     - three callbacks are supplied: 0x7418f0, 0x741b50, 0x741de0
  -> protection-state gate
     - when protectionMode == 1 and the relevant BE state byte is active,
       normal session construction is deferred until a later callback/state change
  -> TGameserverGameSession
  -> two QUrl records are created
     - protected record: isProtected = true
     - unprotected record: isProtected = false
  -> TGameserverDualConnection
     - protected becomes eligible at time T
     - unprotected becomes eligible at T + 1 ms
     - optimizeConnectionStability controls whether one or both may be in flight
  -> TGameserverTCPConnection
     - two QTcpSocket-capable providers exist
     - selected QUrl host and port are passed to QTcpSocket/QAbstractSocket connectToHost
  -> server sends GameserverMessageLoginChallenge
  -> TLoginProtocolMessageHandler builds GameclientMessageLogin protobuf
  -> packet is sent on the selected transport route
  -> server returns login error/wait/advice or GameserverMessageLoginSuccess
  -> on login success, session state is updated and
     gameReadyForSecondaryConnection is emitted
  -> GameclientMessageSecondaryLogin may establish/use the second connection
```

This is the complete client-side flow that can be established statically. Server-side acceptance rules are not observable in the client binary.

## World record and endpoint mapping

### Confirmed offsets

```text
world record +0x20  externaladdressunprotected
world record +0x38  externaladdressprotected
world record +0x50  externalportunprotected
world record +0x54  externalportprotected
world record +0x74  anticheatprotection
```

The parser rejects a world record if any of the four endpoint fields or `anticheatprotection` is absent.

The resulting semantic mapping is:

```text
protected route   = externaladdressprotected:externalportprotected
unprotected route = externaladdressunprotected:externalportunprotected
```

The presence of an unprotected endpoint is not by itself evidence of a security defect; the official client deliberately parses and retains both routes.

## Session metadata and BattlEye gate

Observed `TGameserverGameSessionMetaInfo` layout:

```text
meta +0x1a0  protected QHostAddress
meta +0x1b8  packed/adjacent port storage
meta +0x1bc  protected port used by BattlEye Init path
meta +0x1c0  unprotected QHostAddress
meta +0x1d8  protection mode
meta +0x1da  additional flag
```

After copying into `TGameserverGameSession`:

```text
session +0xbc0  protected address
session +0xbd8  ports
session +0xbe0  unprotected address
session +0xbf8  protection mode
session +0xbfa  additional flag
```

Confirmed gate:

```cpp
if (meta.protectionMode == 1 && battleyeState.active == 1) {
    // normal game-session construction is deferred
    // a later BattlEye callback/state transition resumes the flow
    return;
}

constructAndStartGameSession(meta);
```

This gate affects when normal connection/session flow starts. No direct branch was found in `TGameserverDualConnection` that says "BattlEye active means protected URL, inactive means unprotected URL".

## Exact protected/unprotected selector

The dual-connection URL record is reconstructed as:

```cpp
struct TDualConnectionUrlInfo {
    QUrl url;                 // node +0x20
    bool isProtected;         // node +0x28
    int64_t nextAttemptAtMs;  // node +0x30
    uint64_t failureCount;    // node +0x38
    bool activeOrRequested;   // node +0x40
};
```

Initialization at `TGameserverDualConnection` virtual slot 12 (`0xb55110`):

```cpp
void startRouteSelection() {
    const int64_t now = clock.nowMs();

    for (auto& route : routes) {
        route.nextAttemptAtMs = route.isProtected ? now : now + 1;
    }

    checkForStateChange();
}
```

Selection at `checkForStateChange()` (`0xb47f90`), normalized to semantic pseudocode:

```cpp
void checkForStateChange() {
    if (processingDisconnectEvents)
        return;

    if (!optimizeConnectionStability) {
        if (connectionsUsed() != EConnectionsUsed::None)
            return;

        if (connectionProvider.inFlightOrActiveCount() == 1)
            return;
    }

    const int64_t now = clock.nowMs();
    TDualConnectionUrlInfo* selected = nullptr;

    for (auto& route : routes) {
        if (!route.activeOrRequested && route.nextAttemptAtMs <= now) {
            selected = &route;
            break;
        }
    }

    if (!selected)
        return;

    connectionProvider.requestConnection(selected->url, connectionContext);
}
```

Operational consequence:

```cpp
if (!optimizeConnectionStability) {
    connect(protectedHost, protectedPort);       // eligible at T
    // after an error/disconnect and scheduler re-entry:
    connect(unprotectedHost, unprotectedPort);   // eligible at T + 1 ms
} else {
    connect(protectedHost, protectedPort);       // eligible at T
    connect(unprotectedHost, unprotectedPort);   // may also start at T + 1 ms
}
```

The selector is therefore schedule- and state-based, not a single BattlEye boolean branch.

## Primary, secondary, fallback, and connection state

`onConnectionConnected()` (`0xb482c0`) establishes primary/secondary semantics by connection order:

- the first provider that reaches connected emits `connected(QUrl)` and is the primary connection;
- when all configured providers are connected and the count is greater than one, the later connection emits `secondaryConnected(QUrl)`.

Because the protected route is scheduled first, it is normally primary and the unprotected route normally secondary. This is not a hard-coded identity: if protected fails before connecting, unprotected can become the only/primary connection.

`EConnectionsUsed` runtime values:

```text
0  none
1  unprotected only
2  protected only
3  both
```

On failure/disconnection:

```cpp
oldFailureCount = route.failureCount++;
route.activeOrRequested = false;
route.nextAttemptAtMs = now + 1000 * pow(2, min(oldFailureCount, 4));
```

Backoff is therefore 1, 2, 4, 8, then 16 seconds, capped at 16 seconds.

When `optimizeConnectionStability` is changed to false while both routes are active, the client disconnects the first active unprotected route, preserving protected where possible.

No independent application-level connect timeout was conclusively identified. Qt/socket errors and disconnect events feed the fallback scheduler.

## Actual socket opening

The game-network construction creates two `QTcpSocket` objects and wraps them in `TGameserverTCPConnection` / packet-connection layers.

The final connect path at `0xb403c0` performs the semantic equivalent of:

```cpp
const QString host = selectedUrl.host();
const uint16_t port = selectedUrl.port(-1);

socket.connectToHost(
    host,
    port,
    QIODevice::ReadWrite,
    QAbstractSocket::AnyIPProtocol
);
```

Therefore both route records are capable of reaching a real TCP socket; `TGameserverDualConnection` performs policy/scheduling while `TGameserverTCPConnection` performs the Qt socket operation.

## Packet routing inside the dual connection

A packet route selector at `TGameserverDualConnection` virtual slot 15 (`0xb552a0`) uses a packet field at `+0x34`:

```text
1  send through unprotected connection only
2  send through protected connection only
3  send through both connections
```

Receiving a packet of the observed transport/control type `2` resets backoff for the route that delivered it. The downstream `packetReceived` signal preserves whether the packet arrived through a protected or unprotected route.

## First game-server login exchange

### Challenge-driven protocol

The current client does not immediately send the historical OTClient RSA/XTEA login block after TCP connect. Its `TLoginProtocolMessageHandler` exposes:

```text
sendLoginMessage(GameclientMessageLogin)
sendSecondaryLoginMessage(GameclientMessageSecondaryLogin)
gameLoginChallengeMessage()
gameLoginErrorMessage(...)
gameLoginWaitMessage(...)
gameLoginAdviceMessage(...)
gameReadyForSecondaryConnection()
```

The first authentication message is built after `GameserverMessageLoginChallenge` supplies two 32-bit values at challenge-object offsets `+0x18` and `+0x1c`. Their use and compatibility with the historical protocol strongly identify them as challenge timestamp and random value, but the exact generated protobuf field names were not recovered.

### Confirmed `GameclientMessageLogin` protobuf wire layout

The generated serializer at `0x176c6d0` proves this top-level layout. Protobuf byte offsets are variable because varints and length-delimited values are variable-length; the stable identifiers are field numbers and wire types.

| Field | Wire tag | Wire type | Object storage | Client-side source / status |
|---:|---:|---|---|---|
| 1 | `0x08` | varint | `+0x30` | option/platform enum lookup; exact semantic name **unknown** |
| 2 | `0x10` | varint | `+0x34` | hard-coded `1530`; client version **confirmed** |
| 3 | `0x18` | varint | `+0x38` | parsed numeric build/asset value; **strong inference**: asset version |
| 4 | `0x22` | length-delimited | `+0x18` | configuration/session string; **strong inference**: session key |
| 5 | `0x2a` | length-delimited | `+0x20` | selected character string; **strong inference**: character name |
| 6 | `0x30` | varint | `+0x3c` | runtime client/OS argument; exact semantic name **unknown** |
| 7 | `0x3a` | embedded message | `+0x28` | login/challenge/client-integrity submessage |

The builder sets top-level presence bits for all seven fields before emitting `sendLoginMessage`.

### Nested field 7

The nested object contains:

```text
five length-delimited/string-or-bytes values at +0x18, +0x20, +0x28, +0x30, +0x38
two uint32 values at +0x40 and +0x44
```

The two integers are copied directly from `GameserverMessageLoginChallenge` offsets `+0x18` and `+0x1c` and are therefore **strongly inferred** to be the challenge timestamp and random byte/value.

At least one nested value is obtained as a byte array from a session/controller virtual method. Other values come from session metadata/configuration accessors. Static evidence available in the recovered report set does not safely assign all five generated protobuf field names or prove which one, if any, is a BattlEye-produced token.

### Outer framing and opcode

The binary contains `EGameclientMessageType`, `GameclientMessageLogin`, and a generic `fireEmitSignalForNewClientProtocolMessage` path. The available report set proves protobuf serialization but does not conclusively recover the numeric enum value/outer frame assigned to `GameclientMessageLogin`.

Therefore the exact numeric outer opcode is **unknown**. Substituting the historical OTClient `ClientPendingGame` byte would be unsupported and incorrect for this client generation.

### Login success and session creation

`TLoginProtocolMessageHandler::onLoginSuccess` around `0xe10df0`:

1. marks the owning session/controller state active;
2. extracts a string from `GameserverMessageLoginSuccess`;
3. stores it in session state;
4. emits `gameReadyForSecondaryConnection()`.

This proves that server login success creates/updates client session state and unlocks the secondary-login flow.

## BattlEye integration

Confirmed Linux package and loader behavior:

```text
bin/BattlEye/BEClient.so
bin/BattlEye/BEClient.cfg
  GameID tibia
  MasterPort 7171
ABI: BECLIENT_1.0
exports: Init, GetVer, _0 ... _7
```

The main client:

1. loads the module using `QLibrary`;
2. resolves `Init`;
3. calls an `Init(2, initParameters)`-shaped function;
4. supplies the protected game-server address/port;
5. supplies callbacks at `0x7418f0`, `0x741b50`, and `0x741de0`;
6. changes internal state and resumes the deferred session flow through callback/state transitions.

What is **not proven** by the recovered static report set:

- the exact semantic role of each of the three callbacks;
- the exact bytes, if any, passed between `BEClient.so` and the game-server transport;
- whether a specific nested login field is produced by `BEClient.so`;
- the server-side validation rule for a missing or invalid BattlEye token/state.

Consequently, the claim that `BEClient.so` participates only in local activation is **disproven** as an architectural description because the module receives server endpoint data and callback hooks that gate/resume session flow. The stronger claim that it definitely contributes bytes to the first network handshake remains **unknown** until callback disassembly or a bounded dynamic trace proves it.

## Comparison with current `blakinio/otclient`

Current OTClient `ProtocolGame::onConnect()` sends its login packet immediately unless the historical challenge feature is enabled. `ProtocolGame::sendLoginPacket()` builds the legacy layout:

```text
ClientPendingGame opcode
OS / protocol version
optional client/content/asset version
optional RSA block containing XTEA key
session key and character name, or account/password/authenticator
challenge timestamp and random byte
RSA padding/encryption
then XTEA/sequenced packet mode
```

The official 15.30 client instead uses generated protobuf messages, dual-route transport, login challenge/success messages, and optional secondary login. The current OTClient implementation is therefore not wire-compatible with the analyzed official 15.30 login path without substantial new protocol work.

### OTClient capability matrix

| Capability without `BEClient.so` | Result | Basis |
|---|---|---|
| Obtain a character list from loginservice | **strong inference** | Character-list retrieval precedes game-server BE setup, but current OTClient must implement the current loginservice contract. |
| Select the unprotected endpoint | **confirmed as implementable; not present as the official dual policy** | The endpoint is explicitly provided and retained by the official client; current OTClient accepts a host/port but does not implement this dual-route scheduler. |
| Establish TCP to that endpoint | **confirmed** | OTClient and the official client can open a normal TCP connection. |
| Complete the first 15.30 game-server handshake as-is | **disproven** | Current OTClient sends the legacy raw RSA/XTEA login packet, while the analyzed client sends challenge-driven protobuf messages. |
| Enter an optional/protected world without BE | **unknown** | Server-side enforcement is not present in the client binary; a safe dynamic acceptance test is required. |

A realistic interoperability effort would require, at minimum:

- current loginservice JSON support;
- protected/unprotected endpoint and protection-mode handling;
- current protobuf schemas or wire-compatible encoders/decoders;
- current transport framing/message-type mapping;
- challenge, login-success, and secondary-login state machine;
- an explicit decision not to claim protected-world compatibility until BattlEye/server enforcement is understood.

## Server-side controls

### Confirmed from client expectations

- the server sends a login challenge before the primary login message;
- the client handles explicit login error, wait, advice, and success responses;
- login success returns session data used for secondary connection flow;
- route identity is preserved through the client transport layer.

### Strong inference

- client version, asset/build version, session key, character identity, client/OS identity, challenge values, and nested integrity/session values are intended inputs to server authentication;
- protected-world policy likely validates more than mere TCP reachability.

### Unknown

- which protobuf fields the server marks required versus optional;
- whether the server independently checks route identity;
- whether the server requires a valid BattlEye-derived value on protected or optional worlds;
- whether the unprotected endpoint intentionally accepts only specific world/protection states;
- whether malformed/missing nested values fail closed.

These cannot be answered by static analysis of the client alone.

## Minimal safe validation test

A defensible test must be narrowly scoped and stop at the first decisive response:

1. use only the researcher's own account and a disposable character;
2. do not patch, disable, hook, or impersonate `BEClient.so`;
3. capture only the official client's TCP challenge and first login request/response on one selected world;
4. implement or replay only a structurally valid, non-automating OTClient login attempt using the loginservice-issued session key and the unprotected endpoint;
5. stop immediately on explicit rejection or `GameserverMessageLoginSuccess`/equivalent session acceptance;
6. do not enter gameplay, move, interact, or affect other players;
7. compare protected-world behavior with the official client and document the first server-side divergence;
8. redact account/session secrets and do not publish reusable bypass material.

The decisive security question is not whether TCP connects. It is whether the server accepts a protected/optional-world session without the expected BattlEye state or data.

## Responsible disclosure decision

**Static evidence alone does not justify reporting a vulnerability to CipSoft.** The dual endpoint design and unprotected fallback are intentional client behavior.

A responsible report becomes justified only if the minimal test demonstrates at least one of the following:

- a protected/optional-world login succeeds without required BattlEye state/data;
- the server accepts the unprotected route when policy says protected is mandatory;
- required challenge/session/integrity values are not validated;
- route or secondary-session state can be confused in a way that weakens server enforcement.

Without such evidence, the correct conclusion is an interoperability gap, not a confirmed security issue.

## Rejected hypotheses

- `0xc49630` is a protected/unprotected selector: **disproven**; it belongs to partial session-dump/QBuffer writing.
- the branch on world-record `+0x69` is the BattlEye route branch: **disproven**; it selects normal versus session-dump metadata types.
- functions around `0x722200` and `0x7226e0` are route selectors: **disproven**; they are destructors.
- arbitrary `dlopen`/`dlsym` references prove BE loading: **disproven**; the valid BE loader uses `QLibrary`.
- the current official first login packet is the legacy OTClient RSA/XTEA block: **disproven**; the analyzed client uses challenge-driven protobuf messages.
- primary and secondary are permanently synonymous with protected and unprotected: **disproven**; they are connection-order states, although protected is normally scheduled first.

## Remaining technical limitation

A final bounded workflow intended to extract the three BattlEye callback bodies and raw generated protobuf descriptors remained queued on the `oteryn-staging` self-hosted runner while all GitHub-hosted checks completed. No callback role or protobuf field name has been guessed to compensate for that infrastructure limitation.

The unresolved items are explicitly marked `unknown` above; they are not required to establish the route scheduler, actual socket flow, protobuf-versus-legacy incompatibility, or the responsible-disclosure threshold.