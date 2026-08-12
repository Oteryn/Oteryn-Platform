# OTERYN-20260812 Worldmap dispatch evidence

## Scope

Durable bounded reverse-engineering checkpoint for `OTERYN-20260811-tibia-client-analysis` on draft PR #1006. No proprietary Tibia binary or extracted asset is stored here; only addresses, hashes, control-flow observations, and GitHub Actions evidence.

## Verified client identity

- Client: `/data/client-15.32.df7b29/bin/client`
- Size: `51965216`
- SHA-256: `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`
- Successful dispatch run: `31580692975`
- Successful dispatch job: `94062813472`
- Runner: `oteryn-synology-staging`
- Owned container: `oteryn-tibia-client-analysis`
- Canonical `oteryn-staging` container inventory diff before/after: unchanged in the successful job.

## PROVEN — exact Worldmap Qt metaobject

`TWorldmapProtocolMessageHandler` resolves to:

```text
QMetaObject VA       0x3087800
Qt stringdata VA     0x1cd8a54
Qt metadata VA       0x1cd8820
static_metacall      0xdf2a60
jump table VA        0x1d8bd10
metadata header      13 0 0 0 14 14 0 0 0 0 0 0 0 1
class                tibia::worldmap::TWorldmapProtocolMessageHandler
method_count         14
signal_count         1
```

Method indices decoded from the Qt6 metadata table:

```text
0  publishGameAction
1  handleFullMapMessage
2  handleLeftColumnMessage
3  handleRightColumnMessage
4  handleTopRowMessage
5  handleBottomRowMessage
6  handleTopFloorMessage
7  handleBottomFloorMessage
8  handleFieldDataMessage
9  handleCreateOnMapMessage
10 handleChangeOnMapMessage
11 handleDeleteOnMapMessage
12 handleAmbientLightMessage
13 handleTibiaTimeMessage
```

## PROVEN — exact handler bodies

```text
handleFullMapMessage       -> 0xcec8d0
handleFieldDataMessage     -> 0xcd3190
handleCreateOnMapMessage   -> 0xcecc70
handleChangeOnMapMessage   -> 0xcecf40
handleDeleteOnMapMessage   -> 0xcd4e20
```

The Qt jump-table case for method index 1 is `0xdf2b88` and tail-jumps to `0xcec8d0`. The case for method index 8 is `0xdf2cd8` and tail-jumps to `0xcd3190`.

## PROVEN — common map-data routine

`handleFieldDataMessage` directly calls `0x19a8a80` at `0xcd3224` and returns immediately afterwards. Directional Worldmap cases (`LeftColumn`, `RightColumn`, `TopRow`, `BottomRow`) also converge on `0x19a8a80` after constructing region-like arguments.

Therefore `0x19a8a80` is directly verified as a shared routine used by FieldData and multiple directional map-update paths.

## INFERENCE — role of `0x19a8a80`

High confidence: `0x19a8a80` is the central routine that applies decoded map-field data over a supplied region/range. Its full content-order and appearance semantics remain to be recovered/proven.

## PROVEN — Coordinate schema

An embedded standard protobuf `FileDescriptorProto` for `shared.proto` proves:

```text
Coordinate.x = field 1, uint32
Coordinate.y = field 2, uint32
Coordinate.z = field 3, uint32
```

The beginning of `handleFullMapMessage` reads a nested object and copies three adjacent 32-bit values from generated-object offsets `+0x18`, `+0x1c`, `+0x20`. Exact identity of those generated offsets with Coordinate fields remains an inference until tied directly to the generated runtime type.

## INFERENCE — generated protobuf defaults

Current static evidence gives these high-confidence candidates, but they are not yet promoted to PROVEN type identities:

```text
0x313a820 -> Coordinate default instance candidate
0x313a860 -> MapFieldData default instance candidate
0x314b480 -> AppearanceInstance default instance candidate
```

`0x313a820` recurs in FullMap/FieldData/Create/Change/Delete at coordinate dereference sites. `0x313a860` occurs at map-field payload selection before accesses around `+0x28`. `0x314b480` is used by Create/Change when selecting an appearance-like payload.

## Rejected hypotheses

- `0xde9ca0` is not a Worldmap static metacall. Exact Qt stringdata identifies it as `tibia::sessiondump::TSessiondumpPlayer`. Do not reuse the earlier ordinal-based association.
- Full `.text` traversal with Python/Capstone is not an appropriate bounded method on this runner; native streaming `objdump` succeeded and should remain preferred.
- Do not start from encrypted TCP unless the decoded protobuf path is disproved by new evidence.

## Runtime capture / authentication boundary

The final acceptance proof is still missing:

```text
(x, y, z) -> ordered field/tile contents -> appearance/type IDs
```

Static reverse can continue without account credentials. However, a real decoded FullMap/FieldData message is normally produced only after the client has an active game-world session (initial world entry or a subsequent map update). Therefore an authenticated game session is likely required for the final bounded runtime capture.

This is currently an **INFERENCE**, not proof that credentials must be supplied manually. Before requesting any credential from the user, the continuation agent must inspect the owned runtime for an already usable safe session/test-account mechanism without exposing secrets. Never print, persist, commit, or copy passwords, tokens, session keys, account data, or protected character data into GitHub Actions logs or repository files.

If no safe authenticated session is available, stop at that authority boundary and ask the user only to perform the minimum interactive login needed to establish the session. Do not ask the user to paste credentials into chat, workflow inputs, repository secrets created ad hoc, scripts, or logs.

## Pending trace state

Several bounded trace runs were triggered while the self-hosted runner queue was congested. At the last reliable checkpoint, a trace run/job had acquired the runner but its final result was not retained as verified evidence in the durable record. Do not assume success or failure from chat history. Inspect live GitHub workflow state and use only completed run/job evidence.

## Exact continuation objective

1. Verify live branch HEAD, PR #1006 state, CI, path ownership, runner/runtime state and ownership labels.
2. Read this report plus the active task/handover; do not reconstruct from chat.
3. Inspect completed trace runs and persist any newly verified result.
4. Fully trace `0x19a8a80` enough to prove field iteration/content ordering and identify appearance/type extraction.
5. Tie the default-instance candidates above to concrete generated protobuf types where possible.
6. Select the lowest-risk interception point after `TProtobufServerMessageTranslator` and before mutation of `TWorldMapStorage`.
7. Determine whether the owned runtime already has a safe authenticated-session mechanism. Do not expose credentials.
8. If a safe session exists, perform one bounded decoded-message capture and normalize it to deterministic `(x,y,z) -> ordered contents -> appearance/type IDs` records.
9. If interactive login is genuinely required, checkpoint all static findings and stop only at that explicit user-authority boundary with precise login instructions.
10. After proof, clean temporary analysis workflows before any terminal merge decision; preserve proprietary client material outside Git.
