# OTERYN Tibia native client action proof — 2026-08-12

## Objective

Prove that movement/rotation can be initiated through the official Linux Tibia client's internal protocol action layer without OCR, X11 key injection, mouse coordinates, or pixel-based semantic inference.

## Runtime

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006`
- Runner: `oteryn-synology-staging` (`oteryn-staging` label)
- Owned container: `oteryn-tibia-client-analysis`
- Live client PID during proofs: `21545`
- Client binary: `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`
- Client PIE base during proofs: `0x564276be9000`
- Network: client -> local SOCKS5 `127.0.0.1:25344` -> wireproxy/WARP; direct client TCP remained zero.

## PROVEN — live handler instance

Read-only memory scan located exactly one live `TPlayerProtocolMessageHandler` instance by its runtime vptr.

- static object vptr basis: `0x308a008`
- runtime vptr: `0x564279c73008`
- unique live object: `0x56427ebeebe0` in `[heap]`
- candidate count: `1`
- run: `31638654454`
- job: `94255291393`
- commit: `e20325b3b82a0421fa50e3fcc5414990938a01df`

## PROVEN — movement/rotation signal mapping

The official client metadata names the player protocol methods and corresponding protobuf message classes in this order:

1. `sendGoNorth` -> `GameclientMessageGoNorth`
2. `sendGoEast` -> `GameclientMessageGoEast`
3. `sendGoSouth` -> `GameclientMessageGoSouth`
4. `sendGoWest` -> `GameclientMessageGoWest`
5. `sendStop` -> `GameclientMessageStop`
6. `sendCancel` -> `GameclientMessageCancel`
7. `sendGoNorthEast` -> `GameclientMessageGoNorthEast`
8. `sendGoSouthEast` -> `GameclientMessageGoSouthEast`
9. `sendGoSouthWest` -> `GameclientMessageGoSouthWest`
10. `sendGoNorthWest` -> `GameclientMessageGoNorthWest`
11. `sendGoPath` -> `GameclientMessageGoPath`
12. `sendRotateNorth` -> `GameclientMessageRotateNorth`
13. `sendRotateEast` -> `GameclientMessageRotateEast`
14. `sendRotateSouth` -> `GameclientMessageRotateSouth`
15. `sendRotateWest` -> `GameclientMessageRotateWest`

Zero-argument Qt signal wrappers were disassembled and mapped to signal IDs:

- `0xee2cd0` -> ID 1 (`GoNorth`)
- `0xee2d50` -> ID 2 (`GoEast`)
- `0xee2dd0` -> ID 3 (`GoSouth`)
- `0xee2e50` -> ID 4 (`GoWest`)
- `0xee30d0` -> ID 5 (`Stop`)
- `0xee3150` -> ID 6 (`Cancel`)
- `0xee2ed0` -> ID 7 (`GoNorthEast`)
- `0xee2f50` -> ID 8 (`GoSouthEast`)
- `0xee2fd0` -> ID 9 (`GoSouthWest`)
- `0xee3050` -> ID 10 (`GoNorthWest`)
- `0xee31d0` -> ID 12 (`RotateNorth`)
- `0xee3250` -> ID 13 (`RotateEast`)
- `0xee32d0` -> ID 14 (`RotateSouth`)
- `0xee3350` -> ID 15 (`RotateWest`)

`GoPath` is argument-bearing and is not part of the zero-argument wrapper list above.

Metadata validation:

- run: `31638928121`
- job: `94256223308`
- commit: `0274c4ff570285c28b2690779141f24531afc595`
- result: PASS
- network after read-only mapping: `ACTIVE_LOCAL_SOCKS_COUNT=2`, `ACTIVE_DIRECT_TCP_COUNT=0`.

## PROVEN — native RotateEast invocation without GUI

The live handler object was rediscovered dynamically, not hard-coded. The runtime address of `sendRotateEast` was calculated from the current PIE base:

- handler object: `0x56427ebeebe0`
- static wrapper: `0xee3250`
- runtime wrapper: `0x564277acc250`

The function was called directly on the live object through GDB with no xdotool/OCR/UI action:

`((void (*)(void *))runtime_sendRotateEast)(handler_object)`

Results:

- `GDB_CALL_RC=0`
- `NATIVE_SEND_ROTATE_EAST_CALL_RETURNED=true`
- client PID survived
- local SOCKS before: `2`
- direct TCP before: `0`
- local SOCKS after: `2`
- direct TCP after: `0`
- `CLIENT_SURVIVED_NATIVE_ROTATE=true`

Evidence:

- run: `31639062297`
- job: `94256667804`
- commit: `48fd4b73a8f2caf260333681d27d89876a8367e2`
- result: PASS

## PROVEN — native East/West movement invocation without GUI

The same live handler instance was used. Runtime wrappers:

- `sendGoEast`: static `0xee2d50`, runtime `0x564277acbd50`
- `sendGoWest`: static `0xee2e50`, runtime `0x564277acbe50`

Both were called directly, with no keyboard, mouse, OCR or screenshot-based targeting.

Results:

- `GDB_GO_EAST_RC=0`
- `NATIVE_SEND_GO_EAST_CALL_RETURNED=true`
- `GDB_GO_WEST_RC=0`
- `NATIVE_SEND_GO_WEST_CALL_RETURNED=true`
- client survived both calls
- local SOCKS remained `2`
- direct TCP remained `0`

The active game SOCKS connection also showed traffic over the bounded action interval:

- `bytes_acked: 8097 -> 8231` (+134)
- `bytes_received: 11166 -> 11324` (+158)
- `segs_out: 456 -> 466`
- `segs_in: 477 -> 487`

This network delta is supporting evidence that the live internal action calls progressed through the active session. It is not by itself used to identify the exact encrypted/framed message bytes.

Evidence:

- run: `31639224501`
- job: `94257213013`
- commit: `04ce5fc9e934d4ea6bb54bf8141d54eeb900318a`
- result: PASS

## FACT — other native actions exposed by the binary

The binary exposes structured outbound methods/protobuf classes including:

- `sendAttack` / `GameclientMessageAttack`
- `sendFollow` / `GameclientMessageFollow`
- `sendUseObject` / `GameclientMessageUseObject`
- `sendUseTwoObjects` / `GameclientMessageUseTwoObjects`
- `sendUseOnCreature` / `GameclientMessageUseOnCreature`
- `sendMoveObject` / `GameclientMessageMoveObject`
- `sendTalkMessage`
- container operations such as close/up/seek/action messages.

Their argument ABI/runtime object ownership still requires exact mapping before direct invocation. Do not guess their arguments.

## Semantic status

### PROVEN

- OCR is not required to issue the internal movement/rotation action signal.
- X11 keyboard/mouse input is not required to invoke `RotateEast`, `GoEast`, or `GoWest` on the live authenticated client.
- The official client has a structured internal action/protobuf path suitable as the basis of a non-GUI control layer.
- The authenticated client/session survived the tested native calls and remained WARP-confined.

### INFERENCE with strong supporting evidence

- The movement calls progressed into the active network session: socket counters increased during the native East/West pair. Exact downstream protobuf/framing capture was not instrumented in that run, so the specific serialized message instance was not independently logged.

### UNKNOWN / next proof

1. Capture the downstream message at `TProtocolClientMessageProcessor` / `prepareAndEnqueueGameclientMessage` / `TProtocolWriter` while performing one native action, to link the direct wrapper invocation to the exact outbound message instance.
2. Map exact ABI/signatures and live handler objects for `Attack`, `Follow`, `UseObject`, `UseTwoObjects`, `UseOnCreature`, `MoveObject`, `Talk` and container operations.
3. Replace GDB as the execution mechanism with a small task-owned control bridge/API that dispatches actions on the correct client/Qt thread.
4. Pair native writes with decoded `GameState` reads so actions can use semantic targets (creature IDs, object IDs, tile positions) rather than screen coordinates.

## Target architecture

The resulting control surface should be semantic and GUI-independent, approximately:

```text
read_game_state()
turn(direction)
move(direction)
stop()
go_path(path)
attack(creature_id)
follow(creature_id)
use(object_ref)
use_with(object_ref, target_ref)
move_object(object_ref, destination, count)
say/talk(...)
container_action(...)
```

No OCR/pixel targeting is part of the intended steady-state architecture. X11/CV remains recovery/diagnostic only.
