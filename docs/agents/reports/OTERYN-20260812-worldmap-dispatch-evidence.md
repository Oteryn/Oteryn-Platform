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

The Qt jump-table case for method index 1 is `0xdf2b88`. It loads the method argument from `0x8(%rcx)`, restores the frame, and tail-jumps at `0xdf2b96` to:

```text
handleFullMapMessage body = 0xcec8d0
```

The Qt jump-table case for method index 8 is `0xdf2cd8`. It loads the method argument from `0x8(%rcx)`, restores the frame, and tail-jumps at `0xdf2ce6` to:

```text
handleFieldDataMessage body = 0xcd3190
```

This supersedes the earlier UNKNOWN handler-address state and must be used by continuation work.

## PROVEN — common map-data routine

`handleFieldDataMessage` directly calls `0x19a8a80` at `0xcd3224` and returns immediately afterwards:

```text
cd3190 ...
cd321a mov 0x28(%rax),%rsi
cd321e movq %xmm0,0x20(%rsp)
cd3224 call 0x19a8a80
cd3229 add $0x58,%rsp
cd322d ret
```

The directional Worldmap cases (`LeftColumn`, `RightColumn`, `TopRow`, `BottomRow`) construct region-like stack objects through helpers around `0xbc63e0`, `0xbc6470`, `0xbc6500`, and `0xbc6590`, then converge on:

```text
df2bd0 ...
df2beb mov 0x28(%rax),%rsi
df2bef call 0x19a8a80
```

Therefore it is directly verified that `0x19a8a80` is shared by `FieldData` and multiple directional map-update paths.

## INFERENCE — role of `0x19a8a80`

High confidence: `0x19a8a80` is the central routine that applies decoded map-field data over a supplied region/range. This is an inference from the verified callers and argument construction, not yet a recovered symbol or runtime capture.

The routine is non-trivial and extends at least through branches around `0x19a965f`. Its prologue stores incoming arguments, accesses nested message data, constructs temporary list/sequence state, and enters iteration-like control flow. Full semantic recovery is still required to prove content ordering and appearance/type extraction.

## PROVEN — useful FullMap observations

The beginning of `0xcec8d0` reads a nested object from the FullMap message and copies three adjacent 32-bit values from offsets corresponding to `+0x18`, `+0x1c`, `+0x20` in the generated object representation. It also shifts the first two values left by five bits before further processing and stores state at handler offsets `+0x98`/`+0xa0`.

Separately, the embedded standard protobuf descriptor for `Coordinate` was previously recovered and proves:

```text
Coordinate.x = field 1, uint32
Coordinate.y = field 2, uint32
Coordinate.z = field 3, uint32
```

The exact mapping between those generated-object offsets and the `Coordinate` fields is still an inference until the relevant runtime/generated representation is tied down directly.

## PROVEN — adjacent mutation handlers

Qt dispatch also establishes exact bodies for subsequent map mutations:

```text
handleCreateOnMapMessage  -> 0xcecc70
handleChangeOnMapMessage  -> 0xcecf40
handleDeleteOnMapMessage  -> 0xcd4e20
```

These functions expose coordinate/payload and storage-facing virtual calls and are likely useful for recovering ordered stack/content mutation semantics after the bulk-map path is understood.

## Rejected hypothesis

`0xde9ca0` is not a Worldmap static metacall. Exact Qt stringdata identified it as `tibia::sessiondump::TSessiondumpPlayer`. Do not reuse the earlier ordinal-based association.

## Current incomplete acceptance item

Not yet proven:

```text
(x, y, z) -> ordered field/tile contents -> appearance/type IDs
```

The next bounded objective is to fully trace `0x19a8a80`, identify the concrete iteration/content construction path and appearance-related calls, then select the lowest-risk decoded-message runtime interception point and capture at least one real map message without touching canonical staging services.

## Pending bounded trace

A follow-up read-only trace was triggered by reopening draft PR #1006:

- workflow run: `31581303533`
- job: `94064762140`
- workflow: `Tibia Client Analysis Trace`
- requested label: `oteryn-staging`

At the latest verified checkpoint this job was `queued` with `runner_id=0`; no result is claimed. The GitHub integration cannot list repository self-hosted runners (`403 Resource not accessible by integration`), so the reason for the queue is UNKNOWN.
