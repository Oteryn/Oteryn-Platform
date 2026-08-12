# OTERYN-20260811 Tibia client analysis handover

## Purpose

Durable continuation record for the Tibia Linux client materialization and map-protocol reverse-engineering work performed on the Oteryn Synology staging runner. A continuation agent should be able to resume from this document plus the active task record without reading chat history.

## Repository and runtime boundary

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (`ops: bootstrap Tibia client analysis on Oteryn runner`), draft at time of this handover
- Runner label: `oteryn-staging`
- Verified runner name: `oteryn-synology-staging`
- Owned analysis container: `oteryn-tibia-client-analysis`
- Owned persistent bind path: `/volume1/docker/oteryn/tibia-analysis`
- Ownership labels: `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`
- Canonical `oteryn-staging` Compose services are out of scope and were verified unchanged by the successful inspection run.

## Current official Tibia client identity

PROVEN from the official CipSoft manifest and deterministic runtime verification:

- Client version: `15.32.df7b29`
- Materialized executable: `/data/client-15.32.df7b29/bin/client`
- Executable size: `51,965,216` bytes
- Executable SHA-256: `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`
- ELF: 64-bit LSB PIE, x86-64, stripped, GNU/Linux
- Build ID: `427ad268e6d482f3ff96c72406a64c432040fecf`
- ELF entry point: `0x6afb50`

Official packed payload used to produce the executable:

- Manifest file: `bin/client.lzma`
- Packed size: `10,150,849` bytes
- Packed SHA-256: `496c5b3517c0996a1bbd0e76a7738d450f79d0bf4fef140a807044776042dc9b`

## CipSoft package-envelope reverse result

The current `client.lzma` is not a standard standalone `.lzma` stream from byte zero. Earlier failed attempts with GNU `xz`, a reconstructed classic header, and legal raw-LZMA parameter sweeps disproved that model.

The working decode path established that the CipSoft package contains an outer envelope and an inner LZMA stream. The useful inner stream begins at offset `45` and decodes with:

- LZMA1 `lc=3`
- `lp=0`
- `pb=2`
- dictionary size `32 MiB`

Using that envelope interpretation produced the exact manifest-declared executable size and SHA-256 above. Do not redo the earlier blind header/parameter guessing unless the official package format changes.

## Successful validation evidence

### Final executable inspection

- GitHub Actions run: `31534432923`
- Job: `93922033384`
- Job name: `Inspect verified Tibia executable`
- Result: `SUCCESS`
- Runner: `oteryn-synology-staging`

All steps passed:

1. `Verify runtime and executable identity`
2. `Install binary inspection tools`
3. `Inspect imports strings and protocol clues`
4. `Verify staging unchanged`

The executable identity check required:

- path `/data/client-15.32.df7b29/bin/client`
- size exactly `51965216`
- SHA-256 exactly `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`

The staging-preservation step diffed the canonical Compose-project container inventory before and after the job and passed.

## Key binary findings relevant to map extraction

The client is stripped, but embedded RTTI/type names, Qt metadata, protobuf type names, handler names, and signal/slot strings expose a high-value protocol pipeline.

### Server-message / map protobuf types found

PROVEN strings/types include:

```text
`tibia::protobuf::protocol::GameserverMessageFullMap`
`tibia::protobuf::protocol::GameserverMessageLeftColumn`
`tibia::protobuf::protocol::GameserverMessageRightColumn`
`tibia::protobuf::protocol::GameserverMessageTopRow`
`tibia::protobuf::protocol::GameserverMessageBottomRow`
`tibia::protobuf::protocol::GameserverMessageTopFloor`
`tibia::protobuf::protocol::GameserverMessageBottomFloor`
`tibia::protobuf::protocol::GameserverMessageFieldData`
`tibia::protobuf::protocol::GameserverMessageCreateOnMap`
`tibia::protobuf::protocol::GameserverMessageChangeOnMap`
`tibia::protobuf::protocol::GameserverMessageDeleteOnMap`
`tibia::protobuf::protocol::MapFieldData`
`tibia::protobuf::protocol::MapArea`
`tibia::protobuf::protocol::Coordinate`
`tibia::protobuf::protocol::AppearanceInstance`
```

### World-map message handler found

PROVEN type:

```text
`tibia::worldmap::TWorldmapProtocolMessageHandler`
```

PROVEN handler names:

```text
handleFullMapMessage
handleLeftColumnMessage
handleRightColumnMessage
handleTopRowMessage
handleBottomRowMessage
handleTopFloorMessage
handleBottomFloorMessage
handleFieldDataMessage
handleCreateOnMapMessage
handleChangeOnMapMessage
handleDeleteOnMapMessage
```

Corresponding receive-side signals/slots are also present:

```text
receivedFullMapMessage
receivedLeftColumnMessage
receivedRightColumnMessage
receivedTopRowMessage
receivedBottomRowMessage
receivedTopFloorMessage
receivedBottomFloorMessage
receivedFieldDataMessage
receivedCreateOnMapMessage
receivedChangeOnMapMessage
receivedDeleteOnMapMessage
```

### Network/protocol pipeline types found

PROVEN type names include:

```text
`tibia::network::TGameserverTCPConnection`
`tibia::network::TGameserverNetworkPacketConnection`
`tibia::network::TGameserverNetworkPacketRawDataProcessor`
`tibia::network::TUnencryptedRawMessageStream`
`tibia::network::TGameserverNetworkPacketSequenceFlowProcessor`
`tibia::protocol::TProtocolReader`
`tibia::protocol::TProtobufServerMessageTranslator`
`tibia::protocol::TProtocolServerMessageProcessor`
`tibia::protocol::TProtocolMessageQueue`
```

Additional relevant strings include:

```text
packetReceived
onGameserverNetworkPacketReceived
receivedMessage
gameserverMessageAvailable
onGameserverMessageAvailable
clientMessageReadyToProcess
fireEmitSignalForNewProtocolMessage
Error while processing network packet
```

### Storage / appearance / minimap types found

PROVEN type names include:

```text
`tibia::worldmap::TWorldMapStorage`
`tibia::worldmap::TWorldMapCoordinate`
`tibia::worldmap::TWorldMapSubfieldCoordinate`
`tibia::worldmap::TWorldMapViewport`
`tibia::appearances::TAppearancesManager`
`tibia::appearances::TAppearanceInstanceTracker`
`tibia::appearances::TAppearanceInstanceFactory`
`tibia::appearances::TObjectAppearanceInstance`
`tibia::minimap::TMinimapProtocolMessageHandler`
`tibia::minimap::TMinimapTileStorage`
`tibia::minimap::TMinimapTileManager`
`tibia::minimap::TMinimapDiskIO`
`tibia::minimap::TMinimapDiskIOAsync`
```

This strongly supports a collector that records decoded protobuf map messages before they are applied to world-map storage, while resolving appearance/type IDs through the client appearance system.

## Derived architecture conclusion

DERIVED from the proven type/message inventory:

```text
Gameserver network packet
    -> packet/raw-data processing
    -> TProtocolReader
    -> TProtobufServerMessageTranslator
    -> decoded GameserverMessage*
    -> protocol message queue / processor
    -> TWorldmapProtocolMessageHandler
    -> TWorldMapStorage
```

The preferred extraction point is after protobuf translation and before or at the `TWorldmapProtocolMessageHandler` receive/handle boundary. At that point the client should already have semantic map messages such as `FullMap`, directional map deltas, floor changes, `FieldData`, and create/change/delete mutations, avoiding the need to reconstruct encrypted/raw network framing first.

Target collector output should ultimately normalize to something equivalent to:

```text
(x, y, z) -> ordered field/tile contents -> appearance/type identifiers -> optional resolved appearance metadata
```

This is a research conclusion, not yet a completed runtime hook.

## Rejected hypotheses / dead ends

Do not repeat these unless new evidence invalidates them:

1. **The original uploaded 1.4 MB `Tibia/Tibia` is the game client.** Rejected: it is the launcher/updater.
2. **Direct unauthenticated/default `curl` to the obvious CipSoft static URLs is sufficient.** Rejected by HTTP 403. A request shape matching the established Linux package flow was required.
3. **`client.lzma` is a normal classic LZMA stream from offset 0.** Rejected by decoder failure and the zeroed/outer envelope.
4. **Only the 13-byte header is unusual and the remaining bytes are ordinary raw LZMA1.** Rejected by exhaustive legal `lc/lp/pb` tests against ELF magic.
5. **Blindly restoring a standard `lzma.NewWriter` header is sufficient.** Rejected by decompression failure.

## Existing workflow files on PR #1006

Current changed workflow paths include:

```text
.github/workflows/tibia-client-analysis-one-shot.yml
.github/workflows/tibia-client-analysis-continue.yml
.github/workflows/tibia-client-analysis-relay.yml
```

These are operational/research workflows created during the bootstrap and analysis. The PR body already states that temporary one-shot workflow material must not be merged blindly; inspect and reduce/clean these before any terminal merge decision.

## Exact next research objective

Continue with static/dynamic reverse of the decoded map-message boundary, starting with:

1. locate code/xrefs for `TWorldmapProtocolMessageHandler` and specifically `handleFullMapMessage` / `handleFieldDataMessage`;
2. recover the protobuf field layout for `MapFieldData`, `MapArea`, `Coordinate`, `AppearanceInstance`, and the relevant `GameserverMessage*` wrappers;
3. identify the lowest-risk runtime interception point after `TProtobufServerMessageTranslator` and before mutation of `TWorldMapStorage`;
4. prove with a bounded runtime capture that one received map message can be converted to deterministic `(x,y,z, contents/appearance IDs)` records;
5. only after that design the exporter/collector and any OTBM conversion path.

Do not start by sniffing encrypted TCP unless the decoded-message route is disproved.

## Safety boundary for continuation

- Do not modify, restart, stop, recreate, or clean up canonical `oteryn-staging` services.
- Do not use blanket Docker cleanup.
- Reuse the owned `oteryn-tibia-client-analysis` container only after verifying its ownership labels.
- Preserve `/volume1/docker/oteryn/tibia-analysis` while reverse work still depends on the downloaded/materialized client.
- Do not commit proprietary Tibia binaries or extracted client assets to the repository. Store only hashes, paths, metadata, derived research notes, scripts that do not embed the binary, and bounded evidence references.
- No credentials, login tokens, account secrets, or protected data belong in Git logs/task records.

## Start-here command for a continuation agent

Read in this order:

```text
docs/agents/PROMPTING_STANDARD.md
docs/agents/PROMPTING_HANDOVER.md
docs/agents/CONTEXT_HANDOFF.md
docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md
docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
```

Then verify live branch/head/PR/CI/runtime state. Do not rediscover already PROVEN facts unless current evidence conflicts with this checkpoint.

<!-- BEGIN GENERATED RUNTIME EVIDENCE -->

## Generated bounded runtime evidence

Source: GitHub Actions run `31570468029` on exact runner `oteryn-synology-staging`. Evidence is bounded text/assembly metadata from the already-owned analysis runtime; no Tibia binary or extracted proprietary asset is committed.

```text
CLIENT_IDENTITY
version=15.32.df7b29
client_size=51965216
client_sha256=e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe
protodesc_cold_size=40808
protodesc_cold_sha256=1106af5ecbecd85a701b800ec4f697edab4288a4aa64b3fa5781222e6f65c324

PROTO_TARGET_SUMMARY
TARGET GameserverMessageFullMap count=0 offsets=
TARGET GameserverMessageFieldData count=0 offsets=
TARGET MapFieldData count=0 offsets=
TARGET MapArea count=0 offsets=
TARGET Coordinate count=6 offsets=0x89,0x2795,0x27de,0x288a,0x2904,0x299a
TARGET AppearanceInstance count=0 offsets=

PROTO_TARGET_CONTEXTS_BOUNDED
TARGET GameserverMessageFullMap count=0 offsets=
TARGET GameserverMessageFieldData count=0 offsets=
TARGET MapFieldData count=0 offsets=
TARGET MapArea count=0 offsets=
TARGET Coordinate count=6 offsets=0x89,0x2795,0x27de,0x288a,0x2904,0x299a
BEGIN Coordinate off=0x89 base=0x9
00000009  ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff  ................
00000019  ff ff ff ff ff ff ff 18 00 00 00 1c 00 00 00 20  ............... 
00000029  00 00 00 00 00 00 00 01 00 00 00 02 00 00 00 00  ................
00000039  00 00 00 00 00 00 00 00 00 00 00 0b 00 00 00 ff  ................
00000049  ff ff ff 28 00 00 00 00 00 00 00 00 00 00 00 00  ...(............
00000059  00 00 00 00 00 00 00 0a 0c 73 68 61 72 65 64 2e  .........shared.
00000069  70 72 6f 74 6f 12 15 74 69 62 69 61 2e 70 72 6f  proto..tibia.pro
00000079  74 6f 62 75 66 2e 73 68 61 72 65 64 22 2d 0a 0a  tobuf.shared"-..
00000089  43 6f 6f 72 64 69 6e 61 74 65 12 09 0a 01 78 18  Coordinate....x.
00000099  01 20 01 28 0d 12 09 0a 01 79 18 02 20 01 28 0d  . .(.....y.. .(.
000000a9  12 09 0a 01 7a 18 03 20 01 28 0d 2a 94 01 0a 0d  ....z.. .(.*....
000000b9  50 4c 41 59 45 52 5f 41 43 54 49 4f 4e 12 16 0a  PLAYER_ACTION...
000000c9  12 50 4c 41 59 45 52 5f 41 43 54 49 4f 4e 5f 4e  .PLAYER_ACTION_N
000000d9  4f 4e 45 10 00 12 16 0a 12 50 4c 41 59 45 52 5f  ONE......PLAYER_
000000e9  41 43 54 49 4f 4e 5f 4c 4f 4f 4b 10 01 12 15 0a  ACTION_LOOK.....
000000f9  11 50 4c 41 59 45 52 5f 41 43 54 49 4f 4e 5f 55  .PLAYER_ACTION_U
00000109  53 45 10 02 12 16 0a 12 50 4c 41 59 45 52 5f 41  SE......PLAYER_A
00000119  43 54 49 4f 4e 5f 4f 50 45 4e 10 03 12 24 0a 20  CTION_OPEN...$. 
END Coordinate
BEGIN Coordinate off=0x2795 base=0x2715
00002715  70 72 6f 74 6f 62 75 66 2e 6d 61 70 2e 4e 70 63  protobuf.map.Npc
00002725  12 33 0a 0e 72 65 73 6f 75 72 63 65 5f 66 69 6c  .3..resource_fil
00002735  65 73 18 03 20 03 28 0b 32 1b 2e 74 69 62 69 61  es.. .(.2..tibia
00002745  2e 70 72 6f 74 6f 62 75 66 2e 6d 61 70 2e 4d 61  .protobuf.map.Ma
00002755  70 46 69 6c 65 12 43 0a 18 74 6f 70 5f 6c 65 66  pFile.C..top_lef
00002765  74 5f 74 69 6c 65 5f 63 6f 6f 72 64 69 6e 61 74  t_tile_coordinat
00002775  65 18 04 20 01 28 0b 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
00002785  70 72 6f 74 6f 62 75 66 2e 73 68 61 72 65 64 2e  protobuf.shared.
00002795  43 6f 6f 72 64 69 6e 61 74 65 12 47 0a 1c 62 6f  Coordinate.G..bo
000027a5  74 74 6f 6d 5f 72 69 67 68 74 5f 74 69 6c 65 5f  ttom_right_tile_
000027b5  63 6f 6f 72 64 69 6e 61 74 65 18 05 20 01 28 0b  coordinate.. .(.
000027c5  32 21 2e 74 69 62 69 61 2e 70 72 6f 74 6f 62 75  2!.tibia.protobu
000027d5  66 2e 73 68 61 72 65 64 2e 43 6f 6f 72 64 69 6e  f.shared.Coordin
000027e5  61 74 65 22 d2 01 0a 04 41 72 65 61 12 0f 0a 07  ate"....Area....
000027f5  61 72 65 61 5f 69 64 18 01 20 01 28 0d 12 0c 0a  area_id.. .(....
00002805  04 6e 61 6d 65 18 02 20 01 28 09 12 30 0a 09 61  .name.. .(..0..a
00002815  72 65 61 5f 74 79 70 65 18 03 20 01 28 0e 32 1d  rea_type.. .(.2.
00002825  2e 74 69 62 69 61 2e 70 72 6f 74 6f 62 75 66 2e  .tibia.protobuf.
END Coordinate
BEGIN Coordinate off=0x27de base=0x275e
0000275e  74 6f 70 5f 6c 65 66 74 5f 74 69 6c 65 5f 63 6f  top_left_tile_co
0000276e  6f 72 64 69 6e 61 74 65 18 04 20 01 28 0b 32 21  ordinate.. .(.2!
0000277e  2e 74 69 62 69 61 2e 70 72 6f 74 6f 62 75 66 2e  .tibia.protobuf.
0000278e  73 68 61 72 65 64 2e 43 6f 6f 72 64 69 6e 61 74  shared.Coordinat
0000279e  65 12 47 0a 1c 62 6f 74 74 6f 6d 5f 72 69 67 68  e.G..bottom_righ
000027ae  74 5f 74 69 6c 65 5f 63 6f 6f 72 64 69 6e 61 74  t_tile_coordinat
000027be  65 18 05 20 01 28 0b 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
000027ce  70 72 6f 74 6f 62 75 66 2e 73 68 61 72 65 64 2e  protobuf.shared.
000027de  43 6f 6f 72 64 69 6e 61 74 65 22 d2 01 0a 04 41  Coordinate"....A
000027ee  72 65 61 12 0f 0a 07 61 72 65 61 5f 69 64 18 01  rea....area_id..
000027fe  20 01 28 0d 12 0c 0a 04 6e 61 6d 65 18 02 20 01   .(.....name.. .
0000280e  28 09 12 30 0a 09 61 72 65 61 5f 74 79 70 65 18  (..0..area_type.
0000281e  03 20 01 28 0e 32 1d 2e 74 69 62 69 61 2e 70 72  . .(.2..tibia.pr
0000282e  6f 74 6f 62 75 66 2e 6d 61 70 2e 41 52 45 41 5f  otobuf.map.AREA_
0000283e  54 59 50 45 12 13 0a 0b 73 75 62 61 72 65 61 5f  TYPE....subarea_
0000284e  69 64 73 18 04 20 03 28 0d 12 3b 0a 10 6c 61 62  ids.. .(..;..lab
0000285e  65 6c 5f 63 6f 6f 72 64 69 6e 61 74 65 18 05 20  el_coordinate.. 
0000286e  01 28 0b 32 21 2e 74 69 62 69 61 2e 70 72 6f 74  .(.2!.tibia.prot
END Coordinate
BEGIN Coordinate off=0x288a base=0x280a
0000280a  18 02 20 01 28 09 12 30 0a 09 61 72 65 61 5f 74  .. .(..0..area_t
0000281a  79 70 65 18 03 20 01 28 0e 32 1d 2e 74 69 62 69  ype.. .(.2..tibi
0000282a  61 2e 70 72 6f 74 6f 62 75 66 2e 6d 61 70 2e 41  a.protobuf.map.A
0000283a  52 45 41 5f 54 59 50 45 12 13 0a 0b 73 75 62 61  REA_TYPE....suba
0000284a  72 65 61 5f 69 64 73 18 04 20 03 28 0d 12 3b 0a  rea_ids.. .(..;.
0000285a  10 6c 61 62 65 6c 5f 63 6f 6f 72 64 69 6e 61 74  .label_coordinat
0000286a  65 18 05 20 01 28 0b 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
0000287a  70 72 6f 74 6f 62 75 66 2e 73 68 61 72 65 64 2e  protobuf.shared.
0000288a  43 6f 6f 72 64 69 6e 61 74 65 12 18 0a 10 72 65  Coordinate....re
0000289a  6a 65 63 74 5f 64 6f 6e 61 74 69 6f 6e 73 18 06  ject_donations..
000028aa  20 01 28 08 12 0d 0a 05 61 6c 69 61 73 18 07 20   .(.....alias.. 
000028ba  01 28 09 22 63 0a 03 4e 70 63 12 0c 0a 04 6e 61  .(."c..Npc....na
000028ca  6d 65 18 01 20 01 28 09 12 3a 0a 0f 74 69 6c 65  me.. .(..:..tile
000028da  5f 63 6f 6f 72 64 69 6e 61 74 65 18 02 20 01 28  _coordinate.. .(
000028ea  0b 32 21 2e 74 69 62 69 61 2e 70 72 6f 74 6f 62  .2!.tibia.protob
000028fa  75 66 2e 73 68 61 72 65 64 2e 43 6f 6f 72 64 69  uf.shared.Coordi
0000290a  6e 61 74 65 12 12 0a 0a 73 75 62 61 72 65 61 5f  nate....subarea_
0000291a  69 64 18 03 20 01 28 0d 22 e6 01 0a 07 4d 61 70  id.. .(."....Map
END Coordinate
BEGIN Coordinate off=0x2904 base=0x2884
00002884  68 61 72 65 64 2e 43 6f 6f 72 64 69 6e 61 74 65  hared.Coordinate
00002894  12 18 0a 10 72 65 6a 65 63 74 5f 64 6f 6e 61 74  ....reject_donat
000028a4  69 6f 6e 73 18 06 20 01 28 08 12 0d 0a 05 61 6c  ions.. .(.....al
000028b4  69 61 73 18 07 20 01 28 09 22 63 0a 03 4e 70 63  ias.. .(."c..Npc
000028c4  12 0c 0a 04 6e 61 6d 65 18 01 20 01 28 09 12 3a  ....name.. .(..:
000028d4  0a 0f 74 69 6c 65 5f 63 6f 6f 72 64 69 6e 61 74  ..tile_coordinat
000028e4  65 18 02 20 01 28 0b 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
000028f4  70 72 6f 74 6f 62 75 66 2e 73 68 61 72 65 64 2e  protobuf.shared.
00002904  43 6f 6f 72 64 69 6e 61 74 65 12 12 0a 0a 73 75  Coordinate....su
00002914  62 61 72 65 61 5f 69 64 18 03 20 01 28 0d 22 e6  barea_id.. .(.".
00002924  01 0a 07 4d 61 70 46 69 6c 65 12 34 0a 09 66 69  ...MapFile.4..fi
00002934  6c 65 5f 74 79 70 65 18 01 20 01 28 0e 32 21 2e  le_type.. .(.2!.
00002944  74 69 62 69 61 2e 70 72 6f 74 6f 62 75 66 2e 6d  tibia.protobuf.m
00002954  61 70 2e 4d 41 50 5f 46 49 4c 45 5f 54 59 50 45  ap.MAP_FILE_TYPE
00002964  12 3e 0a 13 74 6f 70 5f 6c 65 66 74 5f 63 6f 6f  .>..top_left_coo
00002974  72 64 69 6e 61 74 65 18 02 20 01 28 0b 32 21 2e  rdinate.. .(.2!.
00002984  74 69 62 69 61 2e 70 72 6f 74 6f 62 75 66 2e 73  tibia.protobuf.s
00002994  68 61 72 65 64 2e 43 6f 6f 72 64 69 6e 61 74 65  hared.Coordinate
END Coordinate
BEGIN Coordinate off=0x299a base=0x291a
0000291a  69 64 18 03 20 01 28 0d 22 e6 01 0a 07 4d 61 70  id.. .(."....Map
0000292a  46 69 6c 65 12 34 0a 09 66 69 6c 65 5f 74 79 70  File.4..file_typ
0000293a  65 18 01 20 01 28 0e 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
0000294a  70 72 6f 74 6f 62 75 66 2e 6d 61 70 2e 4d 41 50  protobuf.map.MAP
0000295a  5f 46 49 4c 45 5f 54 59 50 45 12 3e 0a 13 74 6f  _FILE_TYPE.>..to
0000296a  70 5f 6c 65 66 74 5f 63 6f 6f 72 64 69 6e 61 74  p_left_coordinat
0000297a  65 18 02 20 01 28 0b 32 21 2e 74 69 62 69 61 2e  e.. .(.2!.tibia.
0000298a  70 72 6f 74 6f 62 75 66 2e 73 68 61 72 65 64 2e  protobuf.shared.
0000299a  43 6f 6f 72 64 69 6e 61 74 65 12 11 0a 09 66 69  Coordinate....fi
000029aa  6c 65 5f 6e 61 6d 65 18 03 20 01 28 09 12 14 0a  le_name.. .(....
000029ba  0c 66 69 65 6c 64 73 5f 77 69 64 74 68 18 04 20  .fields_width.. 
000029ca  01 28 0d 12 15 0a 0d 66 69 65 6c 64 73 5f 68 65  .(.....fields_he
000029da  69 67 68 74 18 05 20 01 28 0d 12 0f 0a 07 61 72  ight.. .(.....ar
000029ea  65 61 5f 69 64 18 06 20 01 28 0d 12 14 0a 0c 73  ea_id.. .(.....s
000029fa  63 61 6c 65 5f 66 61 63 74 6f 72 18 07 20 01 28  cale_factor.. .(
00002a0a  01 2a 62 0a 0d 4d 41 50 5f 46 49 4c 45 5f 54 59  .*b..MAP_FILE_TY
00002a1a  50 45 12 19 0a 15 4d 41 50 5f 46 49 4c 45 5f 54  PE....MAP_FILE_T
00002a2a  59 50 45 5f 53 55 42 41 52 45 41 10 00 12 1b 0a  YPE_SUBAREA.....
END Coordinate
TARGET AppearanceInstance count=0 offsets=

WHOLE_PROTO_SECTION_DECODE_RAW_HEAD
Failed to parse input.

WORLDMAP_META_CONTROL_FLOW
  dff70c:	e9 5f be 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff72d:	48 8d 35 c0 15 ee 00 	lea    0xee15c0(%rip),%rsi        # 1ce0cf4 <std::_Sp_make_shared_tag::_S_ti()::__tag@@Base+0x4b004>
  dff73b:	e8 80 d3 6d ff       	call   4dcac0 <strcmp@plt>
  dff75c:	e9 0f be 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff77f:	48 8d 35 ee b6 f6 00 	lea    0xf6b6ee(%rip),%rsi        # 1d6ae74 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x818b4>
  dff78c:	e8 2f d3 6d ff       	call   4dcac0 <strcmp@plt>
  dff7a0:	48 8d 35 f1 47 e7 00 	lea    0xe747f1(%rip),%rsi        # 1c73f98 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x44e708>
  dff7aa:	e8 11 d3 6d ff       	call   4dcac0 <strcmp@plt>
  dff7bd:	e9 ae bd 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff7cb:	eb c8                	jmp    dff795 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x260f65>
  dff7dd:	48 8d 35 e0 74 ed 00 	lea    0xed74e0(%rip),%rsi        # 1cd6cc4 <std::_Sp_make_shared_tag::_S_ti()::__tag@@Base+0x40fd4>
  dff7eb:	e8 d0 d2 6d ff       	call   4dcac0 <strcmp@plt>
  dff80c:	e9 5f bd 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff82d:	48 8d 35 00 16 ee 00 	lea    0xee1600(%rip),%rsi        # 1ce0e34 <std::_Sp_make_shared_tag::_S_ti()::__tag@@Base+0x4b144>
  dff83b:	e8 80 d2 6d ff       	call   4dcac0 <strcmp@plt>
  dff85c:	e9 0f bd 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff87d:	48 8d 35 70 39 ef 00 	lea    0xef3970(%rip),%rsi        # 1cf31f4 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x9c34>
  dff88b:	e8 30 d2 6d ff       	call   4dcac0 <strcmp@plt>
  dff8ac:	e9 bf bc 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff8cd:	48 8d 35 30 e4 f8 00 	lea    0xf8e430(%rip),%rsi        # 1d8dd04 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa4744>
  dff8db:	e8 e0 d1 6d ff       	call   4dcac0 <strcmp@plt>
  dff8e4:	48 8d 35 29 07 f9 00 	lea    0xf90729(%rip),%rsi        # 1d90014 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa6a54>
  dff8ee:	e8 cd d1 6d ff       	call   4dcac0 <strcmp@plt>
  dff8fb:	48 8d 45 10          	lea    0x10(%rbp),%rax
  dff914:	e9 57 bc 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff94d:	48 8d 35 d0 df f8 00 	lea    0xf8dfd0(%rip),%rsi        # 1d8d924 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa4364>
  dff95b:	e8 60 d1 6d ff       	call   4dcac0 <strcmp@plt>
  dff97c:	e9 ef bb 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff99d:	48 8d 35 20 d9 f8 00 	lea    0xf8d920(%rip),%rsi        # 1d8d2c4 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa3d04>
  dff9ab:	e8 10 d1 6d ff       	call   4dcac0 <strcmp@plt>
  dff9cc:	e9 9f bb 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dff9ed:	48 8d 35 40 d8 f8 00 	lea    0xf8d840(%rip),%rsi        # 1d8d234 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa3c74>
  dff9fb:	e8 c0 d0 6d ff       	call   4dcac0 <strcmp@plt>
  dffa1c:	e9 4f bb 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffa3d:	48 8d 35 20 d7 f8 00 	lea    0xf8d720(%rip),%rsi        # 1d8d164 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa3ba4>
  dffa4b:	e8 70 d0 6d ff       	call   4dcac0 <strcmp@plt>
  dffa6c:	e9 ff ba 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffa8d:	48 8d 35 f4 b0 f6 00 	lea    0xf6b0f4(%rip),%rsi        # 1d6ab88 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x815c8>
  dffa9b:	e8 20 d0 6d ff       	call   4dcac0 <strcmp@plt>
  dffabc:	e9 af ba 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffadd:	48 8d 35 dc 27 eb 00 	lea    0xeb27dc(%rip),%rsi        # 1cb22c0 <std::_Sp_make_shared_tag::_S_ti()::__tag@@Base+0x1c5d0>
  dffaeb:	e8 d0 cf 6d ff       	call   4dcac0 <strcmp@plt>
  dffb0c:	e9 5f ba 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffb2d:	48 8d 35 80 90 ed 00 	lea    0xed9080(%rip),%rsi        # 1cd8bb4 <std::_Sp_make_shared_tag::_S_ti()::__tag@@Base+0x42ec4>
  dffb3b:	e8 80 cf 6d ff       	call   4dcac0 <strcmp@plt>
  dffb5c:	e9 0f ba 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffb7d:	48 8d 35 40 b1 f6 00 	lea    0xf6b140(%rip),%rsi        # 1d6acc4 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x81704>
  dffb8b:	e8 30 cf 6d ff       	call   4dcac0 <strcmp@plt>
  dffbac:	e9 bf b9 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffbcd:	48 8d 35 b8 d4 f8 00 	lea    0xf8d4b8(%rip),%rsi        # 1d8d08c <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa3acc>
  dffbdb:	e8 e0 ce 6d ff       	call   4dcac0 <strcmp@plt>
  dffbfc:	e9 6f b9 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffc1d:	48 8d 35 0c 0f f3 00 	lea    0xf30f0c(%rip),%rsi        # 1d30b30 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x47570>
  dffc2b:	e8 90 ce 6d ff       	call   4dcac0 <strcmp@plt>
  dffc4c:	e9 1f b9 6d ff       	jmp    4db570 <QObject::qt_metacast(char const*)@plt>
  dffc70:	e8 3b c7 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  dffc7d:	3d 62 01 00 00       	cmp    $0x162,%eax
  dffc98:	83 fb 07             	cmp    $0x7,%ebx
  dffc9d:	3d 62 01 00 00       	cmp    $0x162,%eax
  dffcaf:	eb d3                	jmp    dffc84 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261454>
  dffcc6:	e8 15 63 ff ff       	call   df5fe0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2577b0>
  dffccf:	eb b3                	jmp    dffc84 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261454>
  dffcf0:	e8 bb c6 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  dffcfd:	83 f8 03             	cmp    $0x3,%eax
  dffd10:	83 fb 07             	cmp    $0x7,%ebx
  dffd15:	83 f8 03             	cmp    $0x3,%eax
  dffd25:	eb db                	jmp    dffd02 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2614d2>
  dffd38:	83 f8 02             	cmp    $0x2,%eax
  dffd3d:	83 f8 03             	cmp    $0x3,%eax
  dffd42:	83 f8 01             	cmp    $0x1,%eax
  dffd4c:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
  dffd5f:	48 8d 35 ba 56 28 02 	lea    0x22856ba(%rip),%rsi        # 3085420 <QObject::staticMetaObject@Qt_6>
  dffd66:	e8 55 f0 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dffd6f:	eb 91                	jmp    dffd02 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2614d2>
  dffd7b:	e8 40 ae e3 ff       	call   c3abc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x9c390>
  dffd84:	e9 79 ff ff ff       	jmp    dffd02 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2614d2>
  dffd93:	e8 f8 b1 e3 ff       	call   c3af90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x9c760>
  dffd9c:	e9 61 ff ff ff       	jmp    dffd02 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2614d2>
  dffdab:	e8 20 b1 e3 ff       	call   c3aed0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x9c6a0>
  dffdb4:	e9 49 ff ff ff       	jmp    dffd02 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2614d2>
  dffdd0:	e8 db c5 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  dffddd:	83 f8 04             	cmp    $0x4,%eax
  dffdf0:	83 fb 07             	cmp    $0x7,%ebx
  dffdf5:	83 f8 04             	cmp    $0x4,%eax
  dffe05:	eb db                	jmp    dffde2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2615b2>
  dffe1e:	e8 4d 94 fe ff       	call   de9270 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24aa40>
  dffe27:	eb b9                	jmp    dffde2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2615b2>
  dffe40:	e8 6b c5 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  dffe51:	8d 50 ff             	lea    -0x1(%rax),%edx
  dffe54:	83 fa 05             	cmp    $0x5,%edx
  dffe68:	83 fb 07             	cmp    $0x7,%ebx
  dffe71:	83 f8 06             	cmp    $0x6,%eax
  dffe81:	eb d6                	jmp    dffe59 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261629>
  dffeb4:	e8 d7 95 fe ff       	call   de9490 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24ac60>
  dffebd:	eb 9a                	jmp    dffe59 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261629>
  dffec4:	48 8d 35 15 f7 27 02 	lea    0x227f715(%rip),%rsi        # 307f5e0 <QObject::staticMetaObject@Qt_6>
  dffece:	e8 ed ee 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dffed3:	eb be                	jmp    dffe93 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261663>
  dffef2:	e8 b9 c4 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  dfff01:	83 f8 02             	cmp    $0x2,%eax
  dfff20:	83 fd 07             	cmp    $0x7,%ebp
  dfff25:	83 f8 02             	cmp    $0x2,%eax
  dfff35:	eb cf                	jmp    dfff06 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2616d6>
  dfff40:	83 f8 01             	cmp    $0x1,%eax
  dfff45:	83 f8 02             	cmp    $0x2,%eax
  dfff56:	e8 f5 3a e3 ff       	call   c33a50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x95220>
  dfff5b:	eb a9                	jmp    dfff06 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2616d6>
  dfff72:	e8 69 3b 8b ff       	call   6b3ae0 <std::runtime_error::~runtime_error()@plt+0x1d3810>
  dfff88:	48 8d 75 08          	lea    0x8(%rbp),%rsi
  dfff93:	e8 68 0c 16 00       	call   f60c00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3c23d0>
  dfffb0:	48 8d 75 08          	lea    0x8(%rbp),%rsi
  dfffbb:	e8 40 0c 16 00       	call   f60c00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3c23d0>
  dfffc5:	e9 3c ff ff ff       	jmp    dfff06 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2616d6>
  dfffdb:	e9 26 ff ff ff       	jmp    dfff06 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2616d6>
  dffff0:	e8 bb c3 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00010:	83 fb 07             	cmp    $0x7,%ebx
  e00024:	eb db                	jmp    e00001 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2617d1>
  e0003b:	e8 10 e5 e3 ff       	call   c3e550 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x9fd20>
  e00044:	eb bb                	jmp    e00001 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2617d1>
  e00060:	e8 4b c3 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0006d:	83 f8 01             	cmp    $0x1,%eax
  e00080:	83 fb 07             	cmp    $0x7,%ebx
  e00085:	83 f8 01             	cmp    $0x1,%eax
  e00095:	eb db                	jmp    e00072 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261842>
  e000b0:	48 8d 35 e9 6a 2b 02 	lea    0x22b6ae9(%rip),%rsi        # 30b6ba0 <QObject::staticMetaObject@Qt_6>
  e000ba:	e8 01 ed 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e000c3:	eb ad                	jmp    e00072 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261842>
  e000ca:	48 8d 35 cf 6a 2b 02 	lea    0x22b6acf(%rip),%rsi        # 30b6ba0 <QObject::staticMetaObject@Qt_6>
  e000d4:	e8 e7 ec 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e000dd:	eb 93                	jmp    e00072 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261842>
  e000f0:	e8 bb c2 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00110:	83 fb 07             	cmp    $0x7,%ebx
  e00124:	eb db                	jmp    e00101 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2618d1>
  e00134:	48 8d 35 05 be 2a 02 	lea    0x22abe05(%rip),%rsi        # 30abf40 <QObject::staticMetaObject@Qt_6>
  e00142:	e8 79 ec 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0014b:	eb b4                	jmp    e00101 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2618d1>
  e00160:	e8 4b c2 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0016d:	83 f8 01             	cmp    $0x1,%eax
  e00180:	83 fb 07             	cmp    $0x7,%ebx
  e00185:	83 f8 01             	cmp    $0x1,%eax
  e00195:	eb db                	jmp    e00172 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261942>
  e001be:	4c 8d 0d 93 59 39 02 	lea    0x2395993(%rip),%r9        # 3195b58 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x36f0>
  e001d8:	3b 7a 20             	cmp    0x20(%rdx),%edi
  e001ed:	3b 7a 20             	cmp    0x20(%rdx),%edi
  e001fd:	eb d1                	jmp    e001d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2619a0>
  e00205:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
  e0020f:	48 8d 35 ca 52 28 02 	lea    0x22852ca(%rip),%rsi        # 30854e0 <QObject::staticMetaObject@Qt_6>
  e00223:	e8 98 eb 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0022c:	e9 41 ff ff ff       	jmp    e00172 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261942>
  e0023b:	4c 39 ca             	cmp    %r9,%rdx
  e00244:	3b 7a 20             	cmp    0x20(%rdx),%edi
  e00253:	e9 1a ff ff ff       	jmp    e00172 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261942>
  e00270:	e8 3b c1 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0027d:	83 f8 05             	cmp    $0x5,%eax
  e00290:	83 fb 07             	cmp    $0x7,%ebx
  e00295:	83 f8 05             	cmp    $0x5,%eax
  e002a5:	eb db                	jmp    e00282 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261a52>
  e002be:	e8 ed f4 fe ff       	call   def7b0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x250f80>
  e002c7:	eb b9                	jmp    e00282 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261a52>
  e002e0:	e8 cb c0 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00300:	83 fb 07             	cmp    $0x7,%ebx
  e00314:	eb db                	jmp    e002f1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261ac1>
  e00324:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
  e00330:	48 8d 35 e9 af 2c 02 	lea    0x22cafe9(%rip),%rsi        # 30cb320 <QObject::staticMetaObject@Qt_6>
  e00346:	48 8d 54 24 1c       	lea    0x1c(%rsp),%rdx
  e00352:	e8 69 ea 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0035b:	eb 94                	jmp    e002f1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261ac1>
  e00370:	e8 3b c0 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0037d:	83 f8 05             	cmp    $0x5,%eax
  e00390:	83 fb 07             	cmp    $0x7,%ebx
  e00395:	83 f8 05             	cmp    $0x5,%eax
  e003a5:	eb db                	jmp    e00382 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261b52>
  e003be:	e8 5d f5 fe ff       	call   def920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2510f0>
  e003c7:	eb b9                	jmp    e00382 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261b52>
  e003e0:	e8 cb bf 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00400:	83 fb 07             	cmp    $0x7,%ebx
  e00414:	eb db                	jmp    e003f1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261bc1>
  e00430:	ff 52 68             	call   *0x68(%rdx)
  e00437:	eb b8                	jmp    e003f1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261bc1>
  e00450:	e8 5b bf 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0045d:	83 f8 0b             	cmp    $0xb,%eax
  e00470:	83 fb 07             	cmp    $0x7,%ebx
  e00475:	83 f8 0b             	cmp    $0xb,%eax
  e0047e:	83 f8 09             	cmp    $0x9,%eax
  e0048a:	eb d6                	jmp    e00462 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c32>
  e0049e:	e8 9d f5 fe ff       	call   defa40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x251210>
  e004a7:	eb b9                	jmp    e00462 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c32>
  e004ba:	48 8d 1d 1f 4b 33 02 	lea    0x2334b1f(%rip),%rbx        # 3134fe0 <typeinfo for QSGRectangleNode@@Base+0xcb58>
  e004c4:	eb 9c                	jmp    e00462 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c32>
  e004e0:	e8 cb be 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e004ed:	83 f8 13             	cmp    $0x13,%eax
  e00500:	83 fb 07             	cmp    $0x7,%ebx
  e00505:	83 f8 13             	cmp    $0x13,%eax
  e00515:	eb db                	jmp    e004f2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc2>
  e0052e:	e8 6d 97 fe ff       	call   de9ca0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b470>
  e00537:	eb b9                	jmp    e004f2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc2>
  e00550:	e8 5b be 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00559:	8d 53 ff             	lea    -0x1(%rbx),%edx
  e0055c:	83 fa 02             	cmp    $0x2,%edx
  e00561:	83 fb 08             	cmp    $0x8,%ebx
  e00566:	83 fb 06             	cmp    $0x6,%ebx
  e00578:	83 fb 01             	cmp    $0x1,%ebx
  e00594:	83 f8 01             	cmp    $0x1,%eax
  e00599:	83 f8 02             	cmp    $0x2,%eax
  e005a2:	41 80 7c 24 38 00    	cmpb   $0x0,0x38(%r12)
  e005b2:	eb c9                	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e005b8:	48 8d 6c 24 18       	lea    0x18(%rsp),%rbp
  e005c7:	e8 f4 c6 e4 ff       	call   c4ccc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xae490>
  e005df:	e8 ac fc 6d ff       	call   4e0290 <QUrl::~QUrl()@plt>
  e005e8:	eb 93                	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e005f0:	48 8d 6c 24 18       	lea    0x18(%rsp),%rbp
  e005ff:	e8 6c c5 e4 ff       	call   c4cb70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xae340>
  e00604:	eb c6                	jmp    e005cc <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d9c>
  e0060b:	e9 6d ff ff ff       	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e00620:	e8 8b bd 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0062d:	83 f8 05             	cmp    $0x5,%eax
  e00640:	83 fb 07             	cmp    $0x7,%ebx
  e00645:	83 f8 05             	cmp    $0x5,%eax
  e00655:	eb db                	jmp    e00632 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e02>
  e0066e:	e8 6d c5 ff ff       	call   dfcbe0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x25e3b0>
  e00677:	eb b9                	jmp    e00632 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e02>
  e00690:	e8 1b bd 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0069d:	83 f8 07             	cmp    $0x7,%eax
  e006b0:	83 fb 07             	cmp    $0x7,%ebx
  e006b5:	83 f8 07             	cmp    $0x7,%eax
  e006c5:	eb db                	jmp    e006a2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e72>
  e006de:	e8 dd 9b fe ff       	call   dea2c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24ba90>
  e006e7:	eb b9                	jmp    e006a2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e72>
  e00700:	e8 ab bc 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00711:	8d 50 ff             	lea    -0x1(%rax),%edx
  e00714:	83 fa 08             	cmp    $0x8,%edx
  e00728:	83 fb 07             	cmp    $0x7,%ebx
  e00731:	83 f8 09             	cmp    $0x9,%eax
  e00741:	eb d6                	jmp    e00719 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261ee9>
  e00774:	e8 67 9e fe ff       	call   dea5e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bdb0>
  e0077d:	eb 9a                	jmp    e00719 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261ee9>
  e00784:	48 8d 35 55 ee 27 02 	lea    0x227ee55(%rip),%rsi        # 307f5e0 <QObject::staticMetaObject@Qt_6>
  e0078e:	e8 2d e6 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00793:	eb be                	jmp    e00753 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261f23>
  e007b0:	e8 fb bb 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e007bd:	83 f8 06             	cmp    $0x6,%eax
  e007d0:	83 fb 07             	cmp    $0x7,%ebx
  e007d5:	83 f8 06             	cmp    $0x6,%eax
  e007e5:	eb db                	jmp    e007c2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261f92>
  e007fe:	e8 1d a1 fe ff       	call   dea920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24c0f0>
  e00807:	eb b9                	jmp    e007c2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261f92>
  e00820:	e8 8b bb 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0082d:	83 f8 01             	cmp    $0x1,%eax
  e00840:	83 fb 07             	cmp    $0x7,%ebx
  e00845:	83 f8 01             	cmp    $0x1,%eax
  e00855:	eb db                	jmp    e00832 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262002>
  e00864:	48 8d 35 d5 af 2c 02 	lea    0x22cafd5(%rip),%rsi        # 30cb840 <QObject::staticMetaObject@Qt_6>
  e00872:	e8 49 e5 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0087b:	eb b5                	jmp    e00832 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262002>
  e00890:	e8 1b bb 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e008b0:	83 fb 07             	cmp    $0x7,%ebx
  e008c4:	eb db                	jmp    e008a1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262071>
  e008ec:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
  e008ff:	48 8d 54 24 18       	lea    0x18(%rsp),%rdx
  e00909:	48 8d 54 24 14       	lea    0x14(%rsp),%rdx
  e00913:	48 8d 35 26 70 28 02 	lea    0x2287026(%rip),%rsi        # 3087940 <QObject::staticMetaObject@Qt_6>
  e00921:	e8 9a e4 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0092a:	e9 72 ff ff ff       	jmp    e008a1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262071>
  e00940:	e8 6b ba 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e0094d:	83 f8 01             	cmp    $0x1,%eax
  e00960:	83 fb 07             	cmp    $0x7,%ebx
  e00965:	83 f8 01             	cmp    $0x1,%eax
  e00975:	eb db                	jmp    e00952 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262122>
  e00984:	48 8d 35 d5 41 27 02 	lea    0x22741d5(%rip),%rsi        # 3074b60 <QObject::staticMetaObject@Qt_6>
  e00992:	e8 29 e4 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e0099b:	eb b5                	jmp    e00952 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262122>
  e009b2:	e8 f9 b9 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e009c1:	83 f8 07             	cmp    $0x7,%eax
  e009e0:	83 fd 07             	cmp    $0x7,%ebp
  e009e5:	83 f8 07             	cmp    $0x7,%eax
  e009f5:	eb cf                	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00a06:	48 8d 15 df b9 f8 00 	lea    0xf8b9df(%rip),%rdx        # 1d8c3ec <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa2e2c>
  e00a16:	ff e0                	jmp    *%rax
  e00a23:	e8 d8 37 e5 ff       	call   c54200 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xb59d0>
  e00a28:	eb 9c                	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00a49:	e8 32 cb e7 ff       	call   c7d580 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xded50>
  e00a4e:	e9 73 ff ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00a5f:	e8 cc 3f e5 ff       	call   c54a30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xb6200>
  e00a64:	e9 5d ff ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00a73:	e8 b8 bb e5 ff       	call   c5c630 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbde00>
  e00a78:	e9 49 ff ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00a80:	49 83 bc 24 c0 00 00 	cmpq   $0x0,0xc0(%r12)
  e00aaf:	e9 12 ff ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00abb:	e8 b0 3b e5 ff       	call   c54670 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xb5e40>
  e00ac0:	e9 01 ff ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00acb:	e8 c0 a9 e5 ff       	call   c5b490 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbcc60>
  e00ad0:	e9 f1 fe ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00adf:	e8 ac 9c 6d ff       	call   4da790 <QTimer::isActive() const@plt>
  e00aef:	e8 c4 f7 6d ff       	call   4e02b8 <QTimer::start()@plt>
  e00af4:	e9 cd fe ff ff       	jmp    e009c6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262196>
  e00b10:	e8 9b b8 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00b1d:	83 f8 01             	cmp    $0x1,%eax
  e00b30:	83 fb 07             	cmp    $0x7,%ebx
  e00b35:	83 f8 01             	cmp    $0x1,%eax
  e00b45:	eb db                	jmp    e00b22 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2622f2>
  e00b66:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
  e00b79:	48 8d 54 24 1e       	lea    0x1e(%rsp),%rdx
  e00b8a:	48 8d 35 cf 41 27 02 	lea    0x22741cf(%rip),%rsi        # 3074d60 <QObject::staticMetaObject@Qt_6>
  e00b94:	e8 27 e2 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00b9d:	eb 83                	jmp    e00b22 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2622f2>
  e00ba2:	48 8d 35 b7 41 27 02 	lea    0x22741b7(%rip),%rsi        # 3074d60 <QObject::staticMetaObject@Qt_6>
  e00bac:	e8 0f e2 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00bb5:	e9 68 ff ff ff       	jmp    e00b22 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2622f2>
  e00bd0:	e8 db b7 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00bf0:	83 fb 07             	cmp    $0x7,%ebx
  e00c04:	eb db                	jmp    e00be1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2623b1>
  e00c14:	48 8d 35 c5 6d 27 02 	lea    0x2276dc5(%rip),%rsi        # 30779e0 <QObject::staticMetaObject@Qt_6>
  e00c22:	e8 99 e1 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00c2b:	eb b4                	jmp    e00be1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2623b1>
  e00c40:	e8 6b b7 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00c4d:	83 f8 05             	cmp    $0x5,%eax
  e00c60:	83 fb 07             	cmp    $0x7,%ebx
  e00c65:	83 f8 05             	cmp    $0x5,%eax
  e00c75:	eb db                	jmp    e00c52 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262422>
  e00c8e:	e8 fd 05 ff ff       	call   df1290 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x252a60>
  e00c97:	eb b9                	jmp    e00c52 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262422>
  e00cb0:	e8 fb b6 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00cbd:	83 f8 01             	cmp    $0x1,%eax
  e00cd0:	83 fb 07             	cmp    $0x7,%ebx
  e00cd5:	83 f8 01             	cmp    $0x1,%eax
  e00ce5:	eb db                	jmp    e00cc2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262492>
  e00cfd:	48 8d 35 1c 4a 27 02 	lea    0x2274a1c(%rip),%rsi        # 3075720 <QObject::staticMetaObject@Qt_6>
  e00d07:	e8 b4 e0 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00d10:	eb b0                	jmp    e00cc2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262492>
  e00d1d:	48 8d 35 fc 49 27 02 	lea    0x22749fc(%rip),%rsi        # 3075720 <QObject::staticMetaObject@Qt_6>
  e00d3d:	48 8d 54 24 1f       	lea    0x1f(%rsp),%rdx
  e00d46:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
  e00d52:	e8 69 e0 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00d5b:	e9 62 ff ff ff       	jmp    e00cc2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262492>
  e00d70:	e8 3b b6 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00d7d:	83 f8 01             	cmp    $0x1,%eax
  e00d90:	83 fb 07             	cmp    $0x7,%ebx
  e00d95:	83 f8 01             	cmp    $0x1,%eax
  e00da5:	eb db                	jmp    e00d82 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262552>
  e00dbc:	48 39 eb             	cmp    %rbp,%rbx
  e00dc1:	eb bf                	jmp    e00d82 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262552>
  e00dcc:	48 39 dd             	cmp    %rbx,%rbp
  e00dd1:	80 7b 08 00          	cmpb   $0x0,0x8(%rbx)
  e00ddb:	80 bf 88 00 00 00 00 	cmpb   $0x0,0x88(%rdi)
  e00de8:	e8 b3 30 16 00       	call   f63ea0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3c5670>
  e00df1:	eb d5                	jmp    e00dc8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262598>
  e00dff:	e8 2c 37 e5 ff       	call   c54530 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xb5d00>
  e00e08:	e9 75 ff ff ff       	jmp    e00d82 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262552>
  e00e22:	e8 89 b5 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00e48:	83 fd 07             	cmp    $0x7,%ebp
  e00e5c:	eb d7                	jmp    e00e35 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262605>
  e00e6b:	e8 80 ae 97 00       	call   177bcf0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd4c0>
  e00e76:	e8 45 f0 e7 ff       	call   c7fec0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xe1690>
  e00e7b:	48 8d 05 ae 65 28 02 	lea    0x22865ae(%rip),%rax        # 3087430 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xfe5cd0>
  e00e8d:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
  e00e92:	e8 b9 b2 97 00       	call   177c150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd920>
  e00e97:	eb 9c                	jmp    e00e35 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262605>
  e00e9c:	e9 2f 40 75 ff       	jmp    554ed0 <std::runtime_error::~runtime_error()@plt+0x74c00>
  e00ec0:	e8 eb b4 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00ee0:	83 fb 07             	cmp    $0x7,%ebx
  e00ef4:	eb db                	jmp    e00ed1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2626a1>
  e00f04:	48 8d 35 d5 75 28 02 	lea    0x22875d5(%rip),%rsi        # 30884e0 <QObject::staticMetaObject@Qt_6>
  e00f12:	e8 a9 de 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00f1b:	eb b4                	jmp    e00ed1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2626a1>
  e00f30:	e8 7b b4 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00f50:	83 fb 07             	cmp    $0x7,%ebx
  e00f64:	eb db                	jmp    e00f41 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262711>
  e00f74:	48 8d 35 65 e6 27 02 	lea    0x227e665(%rip),%rsi        # 307f5e0 <QObject::staticMetaObject@Qt_6>
  e00f82:	e8 39 de 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  e00f8b:	eb b4                	jmp    e00f41 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262711>
  e00fa0:	e8 0b b4 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00fad:	83 f8 01             	cmp    $0x1,%eax
  e00fc0:	83 fb 07             	cmp    $0x7,%ebx
  e00fc5:	83 f8 01             	cmp    $0x1,%eax
  e00fd5:	eb db                	jmp    e00fb2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x262782>
  e00ff0:	48 8d 35 69 e2 29 02 	lea    0x229e269(%rip),%rsi        # 309f260 <QObject::staticMetaObject@Qt_6>
  e00ffa:	e8 c1 dd 6d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
```

<!-- END GENERATED RUNTIME EVIDENCE -->

<!-- BEGIN GENERATED SCHEMA EVIDENCE -->

## Generated protobuf schema evidence

Source: GitHub Actions run `31578344093` on `oteryn-synology-staging`. Sanitized descriptor metadata only.

```text
candidate_descriptor_starts=7
DESCRIPTOR start=0x60 end=0x6b2 name=shared.proto package=tibia.protobuf.shared messages=1 enums=6
DESCRIPTOR start=0x1040 end=0x2507 name=appearances.proto package=tibia.protobuf.appearances messages=28 enums=1
DESCRIPTOR start=0x26a0 end=0x2abb name=map.proto package=tibia.protobuf.map messages=4 enums=2
DESCRIPTOR start=0x2c00 end=0x69dc name=sounds-common.proto package=tibia.protobuf.sound messages=5 enums=3
DESCRIPTOR start=0x6bc0 end=0x70ad name=sounds.proto package=tibia.protobuf.sound messages=6 enums=0
DESCRIPTOR start=0x70c0 end=0x93b1 name=google/protobuf/descriptor.proto package=google.protobuf messages=22 enums=0
DESCRIPTOR start=0x9e60 end=0x9f2c name=google/protobuf/cpp_features.proto package=pb messages=1 enums=0

TARGET_SCHEMAS
MESSAGE tibia.protobuf.shared.Coordinate source=shared.proto descriptor=0x60-0x6b2
  field=1 name=x label=optional type=uint32 oneof=-
  field=2 name=y label=optional type=uint32 oneof=-
  field=3 name=z label=optional type=uint32 oneof=-

TARGET_STATUS
AppearanceInstance=NOT_FOUND
Coordinate=FOUND
GameserverMessageBottomFloor=NOT_FOUND
GameserverMessageBottomRow=NOT_FOUND
GameserverMessageFieldData=NOT_FOUND
GameserverMessageFullMap=NOT_FOUND
GameserverMessageLeftColumn=NOT_FOUND
GameserverMessageRightColumn=NOT_FOUND
GameserverMessageTopFloor=NOT_FOUND
GameserverMessageTopRow=NOT_FOUND
MapArea=NOT_FOUND
MapFieldData=NOT_FOUND

DESCRIPTOR_INVENTORY
shared.proto package=tibia.protobuf.shared messages=Coordinate
appearances.proto package=tibia.protobuf.appearances messages=Appearances,SpritePhase,SpriteAnimation,Box,SpriteInfo,FrameGroup,Appearance,AppearanceFlags,AppearanceFlagImbueable,AppearanceFlagBank,AppearanceFlagWrite,AppearanceFlagWriteOnce,AppearanceFlagLight,AppearanceFlagHeight,AppearanceFlagShift,AppearanceFlagClothes,AppearanceFlagDefaultAction,AppearanceFlagMarket,AppearanceFlagNPC,AppearanceFlagAutomap,AppearanceFlagHook,AppearanceFlagLenshelp,AppearanceFlagChangedToExpire,AppearanceFlagCyclopedia,AppearanceFlagUpgradeClassification,AppearanceFlagSkillWheelGem,AppearanceFlagProficiency,SpecialMeaningAppearanceIds
map.proto package=tibia.protobuf.map messages=Map,Area,Npc,MapFile
sounds-common.proto package=tibia.protobuf.sound messages=SimpleSoundEffect,RandomSoundEffect,DelayedSoundEffect,AppearanceTypesCountSoundEffect,MinMaxFloat
sounds.proto package=tibia.protobuf.sound messages=Sounds,Sound,NumericSoundEffect,AmbienceStream,AmbienceObjectStream,MusicTemplate
google/protobuf/descriptor.proto package=google.protobuf messages=FileDescriptorSet,FileDescriptorProto,DescriptorProto,ExtensionRangeOptions,FieldDescriptorProto,OneofDescriptorProto,EnumDescriptorProto,EnumValueDescriptorProto,ServiceDescriptorProto,MethodDescriptorProto,FileOptions,MessageOptions,FieldOptions,OneofOptions,EnumOptions,EnumValueOptions,ServiceOptions,MethodOptions,UninterpretedOption,FeatureSet,SourceCodeInfo,GeneratedCodeInfo
google/protobuf/cpp_features.proto package=pb messages=CppFeatures
```

<!-- END GENERATED SCHEMA EVIDENCE -->

<!-- BEGIN GENERATED DECODED MAP REVERSE -->

## Generated decoded-map reverse evidence

Source: GitHub Actions run `31578497058` on `oteryn-synology-staging`. Sanitized descriptors/disassembly only.

```text
candidate_descriptor_starts=7
DESCRIPTOR file_off=0x2959c00-0x295a252 name=shared.proto package=tibia.protobuf.shared messages=1
DESCRIPTOR file_off=0x295abe0-0x295c0a7 name=appearances.proto package=tibia.protobuf.appearances messages=28
DESCRIPTOR file_off=0x295c240-0x295c65b name=map.proto package=tibia.protobuf.map messages=4
DESCRIPTOR file_off=0x295c7a0-0x296057c name=sounds-common.proto package=tibia.protobuf.sound messages=5
DESCRIPTOR file_off=0x2960760-0x2960c4d name=sounds.proto package=tibia.protobuf.sound messages=6
DESCRIPTOR file_off=0x2960c60-0x2962f51 name=google/protobuf/descriptor.proto package=google.protobuf messages=22
DESCRIPTOR file_off=0x2963a00-0x2963acc name=google/protobuf/cpp_features.proto package=pb messages=1

TARGET_SCHEMAS
MESSAGE tibia.protobuf.shared.Coordinate source=shared.proto file_off=0x2959c00-0x295a252
  field=1 name=x label=optional type=uint32 oneof=-
  field=2 name=y label=optional type=uint32 oneof=-
  field=3 name=z label=optional type=uint32 oneof=-

TARGET_STATUS
AppearanceInstance=NOT_FOUND
Coordinate=FOUND
GameserverMessageBottomFloor=NOT_FOUND
GameserverMessageBottomRow=NOT_FOUND
GameserverMessageFieldData=NOT_FOUND
GameserverMessageFullMap=NOT_FOUND
GameserverMessageLeftColumn=NOT_FOUND
GameserverMessageRightColumn=NOT_FOUND
GameserverMessageTopFloor=NOT_FOUND
GameserverMessageTopRow=NOT_FOUND
MapArea=NOT_FOUND
MapFieldData=NOT_FOUND

PROTOCOL_RELATED_DESCRIPTORS
shared.proto package=tibia.protobuf.shared file_off=0x2959c00 messages=Coordinate
appearances.proto package=tibia.protobuf.appearances file_off=0x295abe0 messages=Appearances,SpritePhase,SpriteAnimation,Box,SpriteInfo,FrameGroup,Appearance,AppearanceFlags,AppearanceFlagImbueable,AppearanceFlagBank,AppearanceFlagWrite,AppearanceFlagWriteOnce,AppearanceFlagLight,AppearanceFlagHeight,AppearanceFlagShift,AppearanceFlagClothes,AppearanceFlagDefaultAction,AppearanceFlagMarket,AppearanceFlagNPC,AppearanceFlagAutomap,AppearanceFlagHook,AppearanceFlagLenshelp,AppearanceFlagChangedToExpire,AppearanceFlagCyclopedia,AppearanceFlagUpgradeClassification,AppearanceFlagSkillWheelGem,AppearanceFlagProficiency,SpecialMeaningAppearanceIds
map.proto package=tibia.protobuf.map file_off=0x295c240 messages=Map,Area,Npc,MapFile
sounds-common.proto package=tibia.protobuf.sound file_off=0x295c7a0 messages=SimpleSoundEffect,RandomSoundEffect,DelayedSoundEffect,AppearanceTypesCountSoundEffect,MinMaxFloat

worldmap_class_string_va=0x1cd8bb4
qt_metacast_reference_count=18
worldmap_qt_metacast_ordinal=15
qt_metacall_call_count=44
worldmap_candidate_qt_metacall_base_call=0xe004e0
CANDIDATE_QT_METACALL_NEIGHBORHOOD
  e0049e:	e8 9d f5 fe ff       	call   defa40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x251210>
  e004a3:	8b 44 24 0c          	mov    0xc(%rsp),%eax
  e004a7:	eb b9                	jmp    e00462 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c32>
  e004a9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  e004b0:	48 8b 4d 08          	mov    0x8(%rbp),%rcx
  e004b4:	8b 09                	mov    (%rcx),%ecx
  e004b6:	85 c9                	test   %ecx,%ecx
  e004b8:	75 c9                	jne    e00483 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c53>
  e004ba:	48 8d 1d 1f 4b 33 02 	lea    0x2334b1f(%rip),%rbx        # 3134fe0 <typeinfo for QSGRectangleNode@@Base+0xcb58>
  e004c1:	48 89 1a             	mov    %rbx,(%rdx)
  e004c4:	eb 9c                	jmp    e00462 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261c32>
  e004c6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  e004cd:	00 00 00 
  e004d0:	41 54                	push   %r12
  e004d2:	49 89 fc             	mov    %rdi,%r12
  e004d5:	55                   	push   %rbp
  e004d6:	48 89 cd             	mov    %rcx,%rbp
  e004d9:	53                   	push   %rbx
  e004da:	89 f3                	mov    %esi,%ebx
  e004dc:	48 83 ec 10          	sub    $0x10,%rsp
  e004e0:	e8 cb be 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e004e5:	85 c0                	test   %eax,%eax
  e004e7:	78 0c                	js     e004f5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc5>
  e004e9:	85 db                	test   %ebx,%ebx
  e004eb:	75 13                	jne    e00500 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cd0>
  e004ed:	83 f8 13             	cmp    $0x13,%eax
  e004f0:	7e 2e                	jle    e00520 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cf0>
  e004f2:	83 e8 14             	sub    $0x14,%eax
  e004f5:	48 83 c4 10          	add    $0x10,%rsp
  e004f9:	5b                   	pop    %rbx
  e004fa:	5d                   	pop    %rbp
  e004fb:	41 5c                	pop    %r12
  e004fd:	c3                   	ret
  e004fe:	66 90                	xchg   %ax,%ax
  e00500:	83 fb 07             	cmp    $0x7,%ebx
  e00503:	75 f0                	jne    e004f5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc5>
  e00505:	83 f8 13             	cmp    $0x13,%eax
  e00508:	7f e8                	jg     e004f2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc2>
  e0050a:	48 8b 55 00          	mov    0x0(%rbp),%rdx
  e0050e:	48 c7 02 00 00 00 00 	movq   $0x0,(%rdx)
  e00515:	eb db                	jmp    e004f2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc2>
  e00517:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  e0051e:	00 00 
  e00520:	89 c2                	mov    %eax,%edx
  e00522:	48 89 e9             	mov    %rbp,%rcx
  e00525:	31 f6                	xor    %esi,%esi
  e00527:	4c 89 e7             	mov    %r12,%rdi
  e0052a:	89 44 24 0c          	mov    %eax,0xc(%rsp)
  e0052e:	e8 6d 97 fe ff       	call   de9ca0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b470>
  e00533:	8b 44 24 0c          	mov    0xc(%rsp),%eax
  e00537:	eb b9                	jmp    e004f2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261cc2>
  e00539:	90                   	nop
  e0053a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  e00540:	41 54                	push   %r12
  e00542:	49 89 fc             	mov    %rdi,%r12
  e00545:	55                   	push   %rbp
  e00546:	48 89 cd             	mov    %rcx,%rbp
  e00549:	53                   	push   %rbx
  e0054a:	89 f3                	mov    %esi,%ebx
  e0054c:	48 83 ec 20          	sub    $0x20,%rsp
  e00550:	e8 5b be 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00555:	85 c0                	test   %eax,%eax
  e00557:	78 12                	js     e0056b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d3b>
  e00559:	8d 53 ff             	lea    -0x1(%rbx),%edx
  e0055c:	83 fa 02             	cmp    $0x2,%edx
  e0055f:	76 17                	jbe    e00578 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d48>
  e00561:	83 fb 08             	cmp    $0x8,%ebx
  e00564:	74 12                	je     e00578 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d48>
  e00566:	83 fb 06             	cmp    $0x6,%ebx
  e00569:	74 12                	je     e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e0056b:	48 83 c4 20          	add    $0x20,%rsp
  e0056f:	5b                   	pop    %rbx
  e00570:	5d                   	pop    %rbp
  e00571:	41 5c                	pop    %r12
  e00573:	c3                   	ret
  e00574:	0f 1f 40 00          	nopl   0x0(%rax)
  e00578:	83 fb 01             	cmp    $0x1,%ebx
  e0057b:	74 13                	je     e00590 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d60>
  e0057d:	48 83 c4 20          	add    $0x20,%rsp
  e00581:	83 e8 03             	sub    $0x3,%eax
  e00584:	5b                   	pop    %rbx
  e00585:	5d                   	pop    %rbp
  e00586:	41 5c                	pop    %r12
  e00588:	c3                   	ret
  e00589:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  e00590:	48 8b 5d 00          	mov    0x0(%rbp),%rbx
  e00594:	83 f8 01             	cmp    $0x1,%eax
  e00597:	74 57                	je     e005f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261dc0>
  e00599:	83 f8 02             	cmp    $0x2,%eax
  e0059c:	74 1a                	je     e005b8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d88>
  e0059e:	85 c0                	test   %eax,%eax
  e005a0:	75 db                	jne    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e005a2:	41 80 7c 24 38 00    	cmpb   $0x0,0x38(%r12)
  e005a8:	74 5c                	je     e00606 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261dd6>
  e005aa:	41 0f b7 54 24 1c    	movzwl 0x1c(%r12),%edx
  e005b0:	89 13                	mov    %edx,(%rbx)
  e005b2:	eb c9                	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e005b4:	0f 1f 40 00          	nopl   0x0(%rax)
  e005b8:	48 8d 6c 24 18       	lea    0x18(%rsp),%rbp
  e005bd:	4c 89 e6             	mov    %r12,%rsi
  e005c0:	89 44 24 0c          	mov    %eax,0xc(%rsp)
  e005c4:	48 89 ef             	mov    %rbp,%rdi
  e005c7:	e8 f4 c6 e4 ff       	call   c4ccc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xae490>
  e005cc:	48 8b 4c 24 18       	mov    0x18(%rsp),%rcx
  e005d1:	48 8b 13             	mov    (%rbx),%rdx
  e005d4:	48 89 ef             	mov    %rbp,%rdi
  e005d7:	48 89 0b             	mov    %rcx,(%rbx)
  e005da:	48 89 54 24 18       	mov    %rdx,0x18(%rsp)
  e005df:	e8 ac fc 6d ff       	call   4e0290 <QUrl::~QUrl()@plt>
  e005e4:	8b 44 24 0c          	mov    0xc(%rsp),%eax
  e005e8:	eb 93                	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e005ea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  e005f0:	48 8d 6c 24 18       	lea    0x18(%rsp),%rbp
  e005f5:	4c 89 e6             	mov    %r12,%rsi
  e005f8:	89 44 24 0c          	mov    %eax,0xc(%rsp)
  e005fc:	48 89 ef             	mov    %rbp,%rdi
  e005ff:	e8 6c c5 e4 ff       	call   c4cb70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xae340>
  e00604:	eb c6                	jmp    e005cc <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d9c>
  e00606:	83 ca ff             	or     $0xffffffff,%edx
  e00609:	89 13                	mov    %edx,(%rbx)
  e0060b:	e9 6d ff ff ff       	jmp    e0057d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261d4d>
  e00610:	41 54                	push   %r12
  e00612:	49 89 fc             	mov    %rdi,%r12
  e00615:	55                   	push   %rbp
  e00616:	48 89 cd             	mov    %rcx,%rbp
  e00619:	53                   	push   %rbx
  e0061a:	89 f3                	mov    %esi,%ebx
  e0061c:	48 83 ec 10          	sub    $0x10,%rsp
  e00620:	e8 8b bd 6d ff       	call   4dc3b0 <QObject::qt_metacall(QMetaObject::Call, int, void**)@plt>
  e00625:	85 c0                	test   %eax,%eax
  e00627:	78 0c                	js     e00635 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e05>
  e00629:	85 db                	test   %ebx,%ebx
  e0062b:	75 13                	jne    e00640 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e10>
  e0062d:	83 f8 05             	cmp    $0x5,%eax
  e00630:	7e 2e                	jle    e00660 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x261e30>
  e00632:	83 e8 06             	sub    $0x6,%eax
  e00635:	48 83 c4 10          	add    $0x10,%rsp
  e00639:	5b                   	pop    %rbx
  e0063a:	5d                   	pop    %rbp
  e0063b:	41 5c                	pop    %r12
```

<!-- END GENERATED DECODED MAP REVERSE -->

<!-- BEGIN GENERATED WORLDMAP HANDLER EVIDENCE -->

## Generated Worldmap handler evidence

Source: GitHub Actions run `31578626668` on `oteryn-synology-staging`. Sanitized disassembly only.

```text
candidate_worldmap_static_metacall=0xde9ca0
direct_call_count=16
CALL site=0xde9ee0 target=0x4dedc0 text=de9ee0:	e8 db 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xde9f10 target=0x4dedc0 text=de9f10:	e8 ab 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xde9f80 target=0x4dedc0 text=de9f80:	e8 3b 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea0e2 target=0x4dedc0 text=dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea10d target=0x4dedc0 text=dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea1bd target=0x4dedc0 text=dea1bd:	e8 fe 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea1ed target=0x4dedc0 text=dea1ed:	e8 ce 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea220 target=0x4dedc0 text=dea220:	e8 9b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea250 target=0x4dedc0 text=dea250:	e8 6b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea280 target=0x4dedc0 text=dea280:	e8 3b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea2b0 target=0x4dedc0 text=dea2b0:	e8 0b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea3b0 target=0x4dedc0 text=dea3b0:	e8 0b 4a 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea3e0 target=0x4dedc0 text=dea3e0:	e8 db 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea410 target=0x4dedc0 text=dea410:	e8 ab 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea440 target=0x4dedc0 text=dea440:	e8 7b 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CALL site=0xdea46d target=0x4dedc0 text=dea46d:	e8 4e 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>

FULL_DISASSEMBLY

/data/client-15.32.df7b29/bin/client:     file format elf64-x86-64


Disassembly of section .text:

0000000000de9ca0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b470>:
  de9ca0:	85 f6                	test   %esi,%esi
  de9ca2:	75 24                	jne    de9cc8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b498>
  de9ca4:	83 fa 13             	cmp    $0x13,%edx
  de9ca7:	0f 87 1a 01 00 00    	ja     de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9cad:	48 8d 35 78 1d fa 00 	lea    0xfa1d78(%rip),%rsi        # 1d8ba2c <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa246c>
  de9cb4:	89 d2                	mov    %edx,%edx
  de9cb6:	48 83 ec 28          	sub    $0x28,%rsp
  de9cba:	48 63 04 96          	movslq (%rsi,%rdx,4),%rax
  de9cbe:	48 01 f0             	add    %rsi,%rax
  de9cc1:	ff e0                	jmp    *%rax
  de9cc3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
  de9cc8:	83 fe 05             	cmp    $0x5,%esi
  de9ccb:	0f 85 f6 00 00 00    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9cd1:	48 8b 51 08          	mov    0x8(%rcx),%rdx
  de9cd5:	48 8b 31             	mov    (%rcx),%rsi
  de9cd8:	48 8d 0d a1 fd ff ff 	lea    -0x25f(%rip),%rcx        # de9a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b250>
  de9cdf:	48 8b 02             	mov    (%rdx),%rax
  de9ce2:	48 39 c8             	cmp    %rcx,%rax
  de9ce5:	0f 84 d5 00 00 00    	je     de9dc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b590>
  de9ceb:	48 8d 0d be fd ff ff 	lea    -0x242(%rip),%rcx        # de9ab0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b280>
  de9cf2:	48 39 c8             	cmp    %rcx,%rax
  de9cf5:	0f 84 d5 00 00 00    	je     de9dd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5a0>
  de9cfb:	48 8d 0d ee fd ff ff 	lea    -0x212(%rip),%rcx        # de9af0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b2c0>
  de9d02:	48 39 c8             	cmp    %rcx,%rax
  de9d05:	0f 84 d5 00 00 00    	je     de9de0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5b0>
  de9d0b:	48 8d 0d 0e fe ff ff 	lea    -0x1f2(%rip),%rcx        # de9b20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b2f0>
  de9d12:	48 39 c8             	cmp    %rcx,%rax
  de9d15:	0f 84 d5 00 00 00    	je     de9df0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5c0>
  de9d1b:	48 8d 0d 2e fe ff ff 	lea    -0x1d2(%rip),%rcx        # de9b50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b320>
  de9d22:	48 39 c8             	cmp    %rcx,%rax
  de9d25:	0f 84 e5 00 00 00    	je     de9e10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5e0>
  de9d2b:	48 8d 0d 3e fe ff ff 	lea    -0x1c2(%rip),%rcx        # de9b70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b340>
  de9d32:	48 39 c8             	cmp    %rcx,%rax
  de9d35:	0f 84 e5 00 00 00    	je     de9e20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5f0>
  de9d3b:	48 8d 0d 4e fe ff ff 	lea    -0x1b2(%rip),%rcx        # de9b90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b360>
  de9d42:	48 39 c8             	cmp    %rcx,%rax
  de9d45:	0f 84 e5 00 00 00    	je     de9e30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b600>
  de9d4b:	48 8d 0d 5e fe ff ff 	lea    -0x1a2(%rip),%rcx        # de9bb0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b380>
  de9d52:	48 39 c8             	cmp    %rcx,%rax
  de9d55:	0f 84 dd 03 00 00    	je     dea138 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b908>
  de9d5b:	48 8d 0d 6e fe ff ff 	lea    -0x192(%rip),%rcx        # de9bd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b3a0>
  de9d62:	48 39 c8             	cmp    %rcx,%rax
  de9d65:	0f 84 d5 00 00 00    	je     de9e40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b610>
  de9d6b:	48 8d 0d 7e fe ff ff 	lea    -0x182(%rip),%rcx        # de9bf0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b3c0>
  de9d72:	48 39 c8             	cmp    %rcx,%rax
  de9d75:	0f 84 d5 00 00 00    	je     de9e50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b620>
  de9d7b:	48 8d 0d 8e fe ff ff 	lea    -0x172(%rip),%rcx        # de9c10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b3e0>
  de9d82:	48 39 c8             	cmp    %rcx,%rax
  de9d85:	0f 84 dd 00 00 00    	je     de9e68 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b638>
  de9d8b:	48 8d 0d 9e fe ff ff 	lea    -0x162(%rip),%rcx        # de9c30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b400>
  de9d92:	48 39 c8             	cmp    %rcx,%rax
  de9d95:	0f 84 85 03 00 00    	je     dea120 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b8f0>
  de9d9b:	48 8d 0d be fe ff ff 	lea    -0x142(%rip),%rcx        # de9c60 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b430>
  de9da2:	48 39 c8             	cmp    %rcx,%rax
  de9da5:	0f 85 a5 03 00 00    	jne    dea150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b920>
  de9dab:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9db0:	75 15                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9db2:	c7 06 0c 00 00 00    	movl   $0xc,(%rsi)
  de9db8:	c3                   	ret
  de9db9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  de9dc0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9dc5:	74 39                	je     de9e00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b5d0>
  de9dc7:	c3                   	ret
  de9dc8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  de9dcf:	00 
  de9dd0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9dd5:	75 f0                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9dd7:	c7 06 01 00 00 00    	movl   $0x1,(%rsi)
  de9ddd:	c3                   	ret
  de9dde:	66 90                	xchg   %ax,%ax
  de9de0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9de5:	75 e0                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9de7:	c7 06 02 00 00 00    	movl   $0x2,(%rsi)
  de9ded:	c3                   	ret
  de9dee:	66 90                	xchg   %ax,%ax
  de9df0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9df5:	75 d0                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9df7:	c7 06 03 00 00 00    	movl   $0x3,(%rsi)
  de9dfd:	c3                   	ret
  de9dfe:	66 90                	xchg   %ax,%ax
  de9e00:	c7 06 00 00 00 00    	movl   $0x0,(%rsi)
  de9e06:	c3                   	ret
  de9e07:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9e0e:	00 00 
  de9e10:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e15:	75 b0                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e17:	c7 06 04 00 00 00    	movl   $0x4,(%rsi)
  de9e1d:	c3                   	ret
  de9e1e:	66 90                	xchg   %ax,%ax
  de9e20:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e25:	75 a0                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e27:	c7 06 05 00 00 00    	movl   $0x5,(%rsi)
  de9e2d:	c3                   	ret
  de9e2e:	66 90                	xchg   %ax,%ax
  de9e30:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e35:	75 90                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e37:	c7 06 06 00 00 00    	movl   $0x6,(%rsi)
  de9e3d:	c3                   	ret
  de9e3e:	66 90                	xchg   %ax,%ax
  de9e40:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e45:	75 80                	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e47:	c7 06 08 00 00 00    	movl   $0x8,(%rsi)
  de9e4d:	c3                   	ret
  de9e4e:	66 90                	xchg   %ax,%ax
  de9e50:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e55:	0f 85 6c ff ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e5b:	c7 06 09 00 00 00    	movl   $0x9,(%rsi)
  de9e61:	c3                   	ret
  de9e62:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9e68:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  de9e6d:	0f 85 54 ff ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  de9e73:	c7 06 0a 00 00 00    	movl   $0xa,(%rsi)
  de9e79:	c3                   	ret
  de9e7a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9e80:	31 c9                	xor    %ecx,%ecx
  de9e82:	ba 0c 00 00 00       	mov    $0xc,%edx
  de9e87:	48 8d 35 32 25 2a 02 	lea    0x22a2532(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9e8e:	48 83 c4 28          	add    $0x28,%rsp
  de9e92:	e9 29 4f 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9e97:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9e9e:	00 00 
  de9ea0:	31 c9                	xor    %ecx,%ecx
  de9ea2:	ba 0a 00 00 00       	mov    $0xa,%edx
  de9ea7:	48 8d 35 12 25 2a 02 	lea    0x22a2512(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9eae:	48 83 c4 28          	add    $0x28,%rsp
  de9eb2:	e9 09 4f 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9eb7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9ebe:	00 00 
  de9ec0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ec4:	ba 0b 00 00 00       	mov    $0xb,%edx
  de9ec9:	48 89 e1             	mov    %rsp,%rcx
  de9ecc:	48 8d 35 ed 24 2a 02 	lea    0x22a24ed(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9ed3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9eda:	00 
  de9edb:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9ee0:	e8 db 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9ee5:	48 83 c4 28          	add    $0x28,%rsp
  de9ee9:	c3                   	ret
  de9eea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9ef0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ef4:	ba 02 00 00 00       	mov    $0x2,%edx
  de9ef9:	48 89 e1             	mov    %rsp,%rcx
  de9efc:	48 8d 35 bd 24 2a 02 	lea    0x22a24bd(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f03:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f0a:	00 
  de9f0b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f10:	e8 ab 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f15:	eb ce                	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f17:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f1e:	00 00 
  de9f20:	31 c9                	xor    %ecx,%ecx
  de9f22:	ba 05 00 00 00       	mov    $0x5,%edx
  de9f27:	48 8d 35 92 24 2a 02 	lea    0x22a2492(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f2e:	48 83 c4 28          	add    $0x28,%rsp
  de9f32:	e9 89 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f3e:	00 00 
  de9f40:	31 c9                	xor    %ecx,%ecx
  de9f42:	ba 04 00 00 00       	mov    $0x4,%edx
  de9f47:	48 8d 35 72 24 2a 02 	lea    0x22a2472(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f4e:	48 83 c4 28          	add    $0x28,%rsp
  de9f52:	e9 69 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f57:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f5e:	00 00 
  de9f60:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9f64:	ba 03 00 00 00       	mov    $0x3,%edx
  de9f69:	48 89 e1             	mov    %rsp,%rcx
  de9f6c:	48 8d 35 4d 24 2a 02 	lea    0x22a244d(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f73:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f7a:	00 
  de9f7b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f80:	e8 3b 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f85:	e9 5b ff ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f8a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9f90:	31 c9                	xor    %ecx,%ecx
  de9f92:	ba 0d 00 00 00       	mov    $0xd,%edx
  de9f97:	48 8d 35 22 24 2a 02 	lea    0x22a2422(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f9e:	48 83 c4 28          	add    $0x28,%rsp
  de9fa2:	e9 19 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9fa7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9fae:	00 00 
  de9fb0:	48 8b 07             	mov    (%rdi),%rax
  de9fb3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  de9fb7:	48 8b 80 48 01 00 00 	mov    0x148(%rax),%rax
  de9fbe:	48 83 c4 28          	add    $0x28,%rsp
  de9fc2:	ff e0                	jmp    *%rax
  de9fc4:	0f 1f 40 00          	nopl   0x0(%rax)
  de9fc8:	48 8b 07             	mov    (%rdi),%rax
  de9fcb:	48 8d 15 4e c9 e3 ff 	lea    -0x1c36b2(%rip),%rdx        # c26920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x880f0>
  de9fd2:	48 8b 80 40 01 00 00 	mov    0x140(%rax),%rax
  de9fd9:	48 39 d0             	cmp    %rdx,%rax
  de9fdc:	0f 84 9e fe ff ff    	je     de9e80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b650>
  de9fe2:	48 83 c4 28          	add    $0x28,%rsp
  de9fe6:	ff e0                	jmp    *%rax
  de9fe8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  de9fef:	00 
  de9ff0:	48 8b 07             	mov    (%rdi),%rax
  de9ff3:	48 8b 80 38 01 00 00 	mov    0x138(%rax),%rax
  de9ffa:	48 83 c4 28          	add    $0x28,%rsp
  de9ffe:	ff e0                	jmp    *%rax
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
  dea012:	ff e0                	jmp    *%rax
  dea014:	0f 1f 40 00          	nopl   0x0(%rax)
  dea018:	31 c9                	xor    %ecx,%ecx
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea02f:	90                   	nop
  dea030:	31 c9                	xor    %ecx,%ecx
  dea032:	ba 08 00 00 00       	mov    $0x8,%edx
  dea037:	48 8d 35 82 23 2a 02 	lea    0x22a2382(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea047:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea04e:	00 00 
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
  dea082:	e9 39 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea087:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea08e:	00 00 
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
  dea09e:	ff e0                	jmp    *%rax
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
  dea0d5:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
  dea0da:	48 89 e1             	mov    %rsp,%rcx
  dea0dd:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea0e7:	e9 f9 fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea0ec:	0f 1f 40 00          	nopl   0x0(%rax)
  dea0f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea0f4:	31 d2                	xor    %edx,%edx
  dea0f6:	48 89 e1             	mov    %rsp,%rcx
  dea0f9:	48 8d 35 c0 22 2a 02 	lea    0x22a22c0(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea100:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea107:	00 
  dea108:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea112:	e9 ce fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea117:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea11e:	00 00 
  dea120:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea125:	0f 85 9c fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea12b:	c7 06 0b 00 00 00    	movl   $0xb,(%rsi)
  dea131:	c3                   	ret
  dea132:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea138:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea13d:	0f 85 84 fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea143:	c7 06 07 00 00 00    	movl   $0x7,(%rsi)
  dea149:	c3                   	ret
  dea14a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea150:	48 8d 0d 29 fb ff ff 	lea    -0x4d7(%rip),%rcx        # de9c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b450>
  dea157:	48 39 c8             	cmp    %rcx,%rax
  dea15a:	0f 85 67 fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea160:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea165:	0f 85 5c fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea16b:	c7 06 0d 00 00 00    	movl   $0xd,(%rsi)
  dea171:	c3                   	ret
  dea172:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  dea179:	00 00 00 00 
  dea17d:	0f 1f 00             	nopl   (%rax)
  dea180:	31 c9                	xor    %ecx,%ecx
  dea182:	31 d2                	xor    %edx,%edx
  dea184:	48 8d 35 b5 64 2c 02 	lea    0x22c64b5(%rip),%rsi        # 30b0640 <QSortFilterProxyModel::staticMetaObject@Qt_6>
  dea18b:	e9 30 4c 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea190:	31 c9                	xor    %ecx,%ecx
  dea192:	31 d2                	xor    %edx,%edx
  dea194:	48 8d 35 65 5e 2c 02 	lea    0x22c5e65(%rip),%rsi        # 30b0000 <QSortFilterProxyModel::staticMetaObject@Qt_6>
  dea19b:	e9 20 4c 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea1a0:	48 83 ec 18          	sub    $0x18,%rsp
  dea1a4:	31 d2                	xor    %edx,%edx
  dea1a6:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea1ab:	48 89 e1             	mov    %rsp,%rcx
  dea1ae:	48 8d 35 0b d7 29 02 	lea    0x229d70b(%rip),%rsi        # 30878c0 <QObject::staticMetaObject@Qt_6>
  dea1b5:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea1bc:	00 
  dea1bd:	e8 fe 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea1c2:	48 83 c4 18          	add    $0x18,%rsp
  dea1c6:	c3                   	ret
  dea1c7:	90                   	nop
  dea1c8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  dea1cf:	00 
  dea1d0:	48 83 ec 18          	sub    $0x18,%rsp
  dea1d4:	31 d2                	xor    %edx,%edx
  dea1d6:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea1db:	48 89 e1             	mov    %rsp,%rcx
  dea1de:	48 8d 35 bb b7 29 02 	lea    0x229b7bb(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea1e5:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea1ec:	00 
  dea1ed:	e8 ce 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea1f2:	48 83 c4 18          	add    $0x18,%rsp
  dea1f6:	c3                   	ret
  dea1f7:	90                   	nop
  dea1f8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  dea1ff:	00 
  dea200:	48 83 ec 18          	sub    $0x18,%rsp
  dea204:	ba 01 00 00 00       	mov    $0x1,%edx
  dea209:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea20e:	48 89 e1             	mov    %rsp,%rcx
  dea211:	48 8d 35 88 b7 29 02 	lea    0x229b788(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea218:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea21f:	00 
  dea220:	e8 9b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea225:	48 83 c4 18          	add    $0x18,%rsp
  dea229:	c3                   	ret
  dea22a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea230:	48 83 ec 18          	sub    $0x18,%rsp
  dea234:	ba 02 00 00 00       	mov    $0x2,%edx
  dea239:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea23e:	48 89 e1             	mov    %rsp,%rcx
  dea241:	48 8d 35 58 b7 29 02 	lea    0x229b758(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea248:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea24f:	00 
  dea250:	e8 6b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea255:	48 83 c4 18          	add    $0x18,%rsp
  dea259:	c3                   	ret
  dea25a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea260:	48 83 ec 18          	sub    $0x18,%rsp
  dea264:	ba 03 00 00 00       	mov    $0x3,%edx
  dea269:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea26e:	48 89 e1             	mov    %rsp,%rcx
  dea271:	48 8d 35 28 b7 29 02 	lea    0x229b728(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea278:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea27f:	00 
  dea280:	e8 3b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea285:	48 83 c4 18          	add    $0x18,%rsp
  dea289:	c3                   	ret
  dea28a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea290:	48 83 ec 18          	sub    $0x18,%rsp
  dea294:	ba 04 00 00 00       	mov    $0x4,%edx
  dea299:	48 89 74 24 08       	mov    %rsi,0x8(%rsp)
  dea29e:	48 89 e1             	mov    %rsp,%rcx
  dea2a1:	48 8d 35 f8 b6 29 02 	lea    0x229b6f8(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea2a8:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea2af:	00 
  dea2b0:	e8 0b 4b 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea2b5:	48 83 c4 18          	add    $0x18,%rsp
  dea2b9:	c3                   	ret
  dea2ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea2c0:	85 f6                	test   %esi,%esi
  dea2c2:	75 1c                	jne    dea2e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bab0>
  dea2c4:	83 fa 07             	cmp    $0x7,%edx
  dea2c7:	77 65                	ja     dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea2c9:	48 8d 35 ac 17 fa 00 	lea    0xfa17ac(%rip),%rsi        # 1d8ba7c <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa24bc>
  dea2d0:	89 d2                	mov    %edx,%edx
  dea2d2:	48 83 ec 18          	sub    $0x18,%rsp
  dea2d6:	48 63 04 96          	movslq (%rsi,%rdx,4),%rax
  dea2da:	48 01 f0             	add    %rsi,%rax
  dea2dd:	ff e0                	jmp    *%rax
  dea2df:	90                   	nop
  dea2e0:	83 fe 05             	cmp    $0x5,%esi
  dea2e3:	75 49                	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea2e5:	48 8b 51 08          	mov    0x8(%rcx),%rdx
  dea2e9:	48 8b 31             	mov    (%rcx),%rsi
  dea2ec:	48 8d 0d dd fe ff ff 	lea    -0x123(%rip),%rcx        # dea1d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b9a0>
  dea2f3:	48 8b 02             	mov    (%rdx),%rax
  dea2f6:	48 39 c8             	cmp    %rcx,%rax
  dea2f9:	74 35                	je     dea330 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb00>
  dea2fb:	48 8d 0d fe fe ff ff 	lea    -0x102(%rip),%rcx        # dea200 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b9d0>
  dea302:	48 39 c8             	cmp    %rcx,%rax
  dea305:	74 39                	je     dea340 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb10>
  dea307:	48 8d 0d 22 ff ff ff 	lea    -0xde(%rip),%rcx        # dea230 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24ba00>
  dea30e:	48 39 c8             	cmp    %rcx,%rax
  dea311:	74 3d                	je     dea350 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb20>
  dea313:	48 8d 0d 46 ff ff ff 	lea    -0xba(%rip),%rcx        # dea260 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24ba30>
  dea31a:	48 39 c8             	cmp    %rcx,%rax
  dea31d:	0f 85 75 01 00 00    	jne    dea498 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bc68>
  dea323:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea328:	0f 84 92 01 00 00    	je     dea4c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bc90>
  dea32e:	c3                   	ret
  dea32f:	90                   	nop
  dea330:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea335:	75 f7                	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea337:	c7 06 00 00 00 00    	movl   $0x0,(%rsi)
  dea33d:	c3                   	ret
  dea33e:	66 90                	xchg   %ax,%ax
  dea340:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea345:	75 e7                	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea347:	c7 06 01 00 00 00    	movl   $0x1,(%rsi)
  dea34d:	c3                   	ret
  dea34e:	66 90                	xchg   %ax,%ax
  dea350:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea355:	75 d7                	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea357:	c7 06 02 00 00 00    	movl   $0x2,(%rsi)
  dea35d:	c3                   	ret
  dea35e:	66 90                	xchg   %ax,%ax
  dea360:	48 8b 07             	mov    (%rdi),%rax
  dea363:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea367:	48 8b 80 a8 00 00 00 	mov    0xa8(%rax),%rax
  dea36e:	48 83 c4 18          	add    $0x18,%rsp
  dea372:	ff e0                	jmp    *%rax
  dea374:	0f 1f 40 00          	nopl   0x0(%rax)
  dea378:	48 8b 07             	mov    (%rdi),%rax
  dea37b:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea37f:	48 8b 80 a0 00 00 00 	mov    0xa0(%rax),%rax
  dea386:	48 83 c4 18          	add    $0x18,%rsp
  dea38a:	ff e0                	jmp    *%rax
  dea38c:	0f 1f 40 00          	nopl   0x0(%rax)
  dea390:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea394:	ba 04 00 00 00       	mov    $0x4,%edx
  dea399:	48 89 e1             	mov    %rsp,%rcx
  dea39c:	48 8d 35 fd b5 29 02 	lea    0x229b5fd(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea3a3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea3aa:	00 
  dea3ab:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea3b0:	e8 0b 4a 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea3b5:	48 83 c4 18          	add    $0x18,%rsp
  dea3b9:	c3                   	ret
  dea3ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea3c0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea3c4:	ba 03 00 00 00       	mov    $0x3,%edx
  dea3c9:	48 89 e1             	mov    %rsp,%rcx
  dea3cc:	48 8d 35 cd b5 29 02 	lea    0x229b5cd(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea3d3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea3da:	00 
  dea3db:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea3e0:	e8 db 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea3e5:	eb ce                	jmp    dea3b5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb85>
  dea3e7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea3ee:	00 00 
  dea3f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea3f4:	ba 02 00 00 00       	mov    $0x2,%edx
  dea3f9:	48 89 e1             	mov    %rsp,%rcx
  dea3fc:	48 8d 35 9d b5 29 02 	lea    0x229b59d(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea403:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea40a:	00 
  dea40b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea410:	e8 ab 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea415:	eb 9e                	jmp    dea3b5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb85>
  dea417:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea41e:	00 00 
  dea420:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea424:	ba 01 00 00 00       	mov    $0x1,%edx
  dea429:	48 89 e1             	mov    %rsp,%rcx
  dea42c:	48 8d 35 6d b5 29 02 	lea    0x229b56d(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea433:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea43a:	00 
  dea43b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea440:	e8 7b 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea445:	e9 6b ff ff ff       	jmp    dea3b5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb85>
  dea44a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea450:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea454:	31 d2                	xor    %edx,%edx
  dea456:	48 89 e1             	mov    %rsp,%rcx
  dea459:	48 8d 35 40 b5 29 02 	lea    0x229b540(%rip),%rsi        # 30859a0 <QObject::staticMetaObject@Qt_6>
  dea460:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea467:	00 
  dea468:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea46d:	e8 4e 49 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea472:	e9 3e ff ff ff       	jmp    dea3b5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bb85>
  dea477:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea47e:	00 00 
  dea480:	48 8b 07             	mov    (%rdi),%rax
  dea483:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea487:	48 8b 80 b0 00 00 00 	mov    0xb0(%rax),%rax
  dea48e:	48 83 c4 18          	add    $0x18,%rsp
  dea492:	ff e0                	jmp    *%rax
  dea494:	0f 1f 40 00          	nopl   0x0(%rax)
  dea498:	48 8d 0d f1 fd ff ff 	lea    -0x20f(%rip),%rcx        # dea290 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24ba60>
  dea49f:	48 39 c8             	cmp    %rcx,%rax
  dea4a2:	0f 85 86 fe ff ff    	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea4a8:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea4ad:	0f 85 7b fe ff ff    	jne    dea32e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24bafe>
  dea4b3:	c7 06 04 00 00 00    	movl   $0x4,(%rsi)
  dea4b9:	c3                   	ret
  dea4ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea4c0:	c7 06 03 00 00 00    	movl   $0x3,(%rsi)
  dea4c6:	c3                   	ret
  dea4c7:	90                   	nop
  dea4c8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  dea4cf:	00 
  dea4d0:	31 c9                	xor    %ecx,%ecx
  dea4d2:	31 d2                	xor    %edx,%edx
  dea4d4:	48 8d 35 a5 60 18 02 	lea    0x21860a5(%rip),%rsi        # 2f70580 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xecee20>
  dea4db:	e9 e0 48 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea4e0:	31 c9                	xor    %ecx,%ecx
  dea4e2:	ba 01 00 00 00       	mov    $0x1,%edx
  dea4e7:	48 8d 35 92 60 18 02 	lea    0x2186092(%rip),%rsi        # 2f70580 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xecee20>
  dea4ee:	e9 cd 48 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea4f3:	90                   	nop
  dea4f4:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  dea4fb:	00 00 00 00 
  dea4ff:	90                   	nop

target_count=0
```

<!-- END GENERATED WORLDMAP HANDLER EVIDENCE -->

<!-- BEGIN GENERATED WORLDMAP JUMP TABLE -->

## Generated Worldmap jump-table evidence

Source: GitHub Actions run `31578873658` on `oteryn-synology-staging`. Sanitized address/disassembly metadata only.

```text
candidate_worldmap_static_metacall=0xde9ca0
jump_table_va=0x1d8ba2c
jump_table_file_offset=0x1d8ba2c
jump_table_section=.rodata
entry_count=20

METHOD_CASE_TARGETS
method_index=0 rel=-16390460 rel_hex=0xff05e6c4 target=0xdea0f0
method_index=1 rel=-16390524 rel_hex=0xff05e684 target=0xdea0b0
method_index=2 rel=-16390972 rel_hex=0xff05e4c4 target=0xde9ef0
method_index=3 rel=-16390860 rel_hex=0xff05e534 target=0xde9f60
method_index=4 rel=-16390892 rel_hex=0xff05e514 target=0xde9f40
method_index=5 rel=-16390924 rel_hex=0xff05e4f4 target=0xde9f20
method_index=6 rel=-16390588 rel_hex=0xff05e644 target=0xdea070
method_index=7 rel=-16390620 rel_hex=0xff05e624 target=0xdea050
method_index=8 rel=-16390652 rel_hex=0xff05e604 target=0xdea030
method_index=9 rel=-16390676 rel_hex=0xff05e5ec target=0xdea018
method_index=10 rel=-16391052 rel_hex=0xff05e474 target=0xde9ea0
method_index=11 rel=-16391020 rel_hex=0xff05e494 target=0xde9ec0
method_index=12 rel=-16391084 rel_hex=0xff05e454 target=0xde9e80
method_index=13 rel=-16390812 rel_hex=0xff05e564 target=0xde9f90
method_index=14 rel=-16390700 rel_hex=0xff05e5d4 target=0xdea000
method_index=15 rel=-16390716 rel_hex=0xff05e5c4 target=0xde9ff0
method_index=16 rel=-16390756 rel_hex=0xff05e59c target=0xde9fc8
method_index=17 rel=-16390780 rel_hex=0xff05e584 target=0xde9fb0
method_index=18 rel=-16390556 rel_hex=0xff05e664 target=0xdea090
method_index=19 rel=-16390540 rel_hex=0xff05e674 target=0xdea0a0

CASE_DISASSEMBLY
CASE method_index=0 target=0xdea0f0
  dea0f2:	41 08 31             	or     %sil,(%r9)
  dea0f5:	d2 48 89             	rorb   %cl,-0x77(%rax)
  dea0f8:	e1 48                	loope  dea142 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b912>
  dea0fa:	8d 35 c0 22 2a 02    	lea    0x22a22c0(%rip),%esi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea100:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea107:	00 
  dea108:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea112:	e9 ce fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea117:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea11e:	00 00 
  dea120:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea125:	0f 85 9c fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea12b:	c7 06 0b 00 00 00    	movl   $0xb,(%rsi)
  dea131:	c3                   	ret
  dea132:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea138:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea13d:	0f 85 84 fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea143:	c7 06 07 00 00 00    	movl   $0x7,(%rsi)
  dea149:	c3                   	ret
  dea14a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  dea150:	48 8d 0d 29 fb ff ff 	lea    -0x4d7(%rip),%rcx        # de9c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b450>
  dea157:	48 39 c8             	cmp    %rcx,%rax
  dea15a:	0f 85 67 fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea160:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea165:	0f 85 5c fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea16b:	c7 06 0d 00 00 00    	movl   $0xd,(%rsi)
  dea171:	c3                   	ret
  dea172:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  dea179:	00 00 00 00 
CASE method_index=1 target=0xdea0b0
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
  dea0d5:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
  dea0da:	48 89 e1             	mov    %rsp,%rcx
  dea0dd:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea0e7:	e9 f9 fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea0ec:	0f 1f 40 00          	nopl   0x0(%rax)
  dea0f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea0f4:	31 d2                	xor    %edx,%edx
  dea0f6:	48 89 e1             	mov    %rsp,%rcx
  dea0f9:	48 8d 35 c0 22 2a 02 	lea    0x22a22c0(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea100:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea107:	00 
  dea108:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea112:	e9 ce fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea117:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea11e:	00 00 
  dea120:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
  dea125:	0f 85 9c fc ff ff    	jne    de9dc7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b597>
  dea12b:	c7 06 0b 00 00 00    	movl   $0xb,(%rsi)
  dea131:	c3                   	ret
  dea132:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
CASE method_index=2 target=0xde9ef0
  de9ef0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ef4:	ba 02 00 00 00       	mov    $0x2,%edx
  de9ef9:	48 89 e1             	mov    %rsp,%rcx
  de9efc:	48 8d 35 bd 24 2a 02 	lea    0x22a24bd(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f03:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f0a:	00 
  de9f0b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f10:	e8 ab 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f15:	eb ce                	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f17:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f1e:	00 00 
  de9f20:	31 c9                	xor    %ecx,%ecx
  de9f22:	ba 05 00 00 00       	mov    $0x5,%edx
  de9f27:	48 8d 35 92 24 2a 02 	lea    0x22a2492(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f2e:	48 83 c4 28          	add    $0x28,%rsp
  de9f32:	e9 89 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f3e:	00 00 
  de9f40:	31 c9                	xor    %ecx,%ecx
  de9f42:	ba 04 00 00 00       	mov    $0x4,%edx
  de9f47:	48 8d 35 72 24 2a 02 	lea    0x22a2472(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f4e:	48 83 c4 28          	add    $0x28,%rsp
  de9f52:	e9 69 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f57:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f5e:	00 00 
  de9f60:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9f64:	ba 03 00 00 00       	mov    $0x3,%edx
  de9f69:	48 89 e1             	mov    %rsp,%rcx
  de9f6c:	48 8d 35 4d 24 2a 02 	lea    0x22a244d(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f73:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
CASE method_index=3 target=0xde9f60
  de9f60:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9f64:	ba 03 00 00 00       	mov    $0x3,%edx
  de9f69:	48 89 e1             	mov    %rsp,%rcx
  de9f6c:	48 8d 35 4d 24 2a 02 	lea    0x22a244d(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f73:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f7a:	00 
  de9f7b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f80:	e8 3b 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f85:	e9 5b ff ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f8a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9f90:	31 c9                	xor    %ecx,%ecx
  de9f92:	ba 0d 00 00 00       	mov    $0xd,%edx
  de9f97:	48 8d 35 22 24 2a 02 	lea    0x22a2422(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f9e:	48 83 c4 28          	add    $0x28,%rsp
  de9fa2:	e9 19 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9fa7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9fae:	00 00 
  de9fb0:	48 8b 07             	mov    (%rdi),%rax
  de9fb3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  de9fb7:	48 8b 80 48 01 00 00 	mov    0x148(%rax),%rax
  de9fbe:	48 83 c4 28          	add    $0x28,%rsp
  de9fc2:	ff e0                	jmp    *%rax
  de9fc4:	0f 1f 40 00          	nopl   0x0(%rax)
  de9fc8:	48 8b 07             	mov    (%rdi),%rax
  de9fcb:	48 8d 15 4e c9 e3 ff 	lea    -0x1c36b2(%rip),%rdx        # c26920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x880f0>
  de9fd2:	48 8b 80 40 01 00 00 	mov    0x140(%rax),%rax
  de9fd9:	48 39 d0             	cmp    %rdx,%rax
  de9fdc:	0f 84 9e fe ff ff    	je     de9e80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b650>
  de9fe2:	48 83 c4 28          	add    $0x28,%rsp
  de9fe6:	ff e0                	jmp    *%rax
CASE method_index=4 target=0xde9f40
  de9f40:	31 c9                	xor    %ecx,%ecx
  de9f42:	ba 04 00 00 00       	mov    $0x4,%edx
  de9f47:	48 8d 35 72 24 2a 02 	lea    0x22a2472(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f4e:	48 83 c4 28          	add    $0x28,%rsp
  de9f52:	e9 69 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f57:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f5e:	00 00 
  de9f60:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9f64:	ba 03 00 00 00       	mov    $0x3,%edx
  de9f69:	48 89 e1             	mov    %rsp,%rcx
  de9f6c:	48 8d 35 4d 24 2a 02 	lea    0x22a244d(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f73:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f7a:	00 
  de9f7b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f80:	e8 3b 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f85:	e9 5b ff ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f8a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9f90:	31 c9                	xor    %ecx,%ecx
  de9f92:	ba 0d 00 00 00       	mov    $0xd,%edx
  de9f97:	48 8d 35 22 24 2a 02 	lea    0x22a2422(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f9e:	48 83 c4 28          	add    $0x28,%rsp
  de9fa2:	e9 19 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9fa7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9fae:	00 00 
  de9fb0:	48 8b 07             	mov    (%rdi),%rax
  de9fb3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  de9fb7:	48 8b 80 48 01 00 00 	mov    0x148(%rax),%rax
  de9fbe:	48 83 c4 28          	add    $0x28,%rsp
  de9fc2:	ff e0                	jmp    *%rax
  de9fc4:	0f 1f 40 00          	nopl   0x0(%rax)
CASE method_index=5 target=0xde9f20
  de9f20:	31 c9                	xor    %ecx,%ecx
  de9f22:	ba 05 00 00 00       	mov    $0x5,%edx
  de9f27:	48 8d 35 92 24 2a 02 	lea    0x22a2492(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f2e:	48 83 c4 28          	add    $0x28,%rsp
  de9f32:	e9 89 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f3e:	00 00 
  de9f40:	31 c9                	xor    %ecx,%ecx
  de9f42:	ba 04 00 00 00       	mov    $0x4,%edx
  de9f47:	48 8d 35 72 24 2a 02 	lea    0x22a2472(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f4e:	48 83 c4 28          	add    $0x28,%rsp
  de9f52:	e9 69 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f57:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f5e:	00 00 
  de9f60:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9f64:	ba 03 00 00 00       	mov    $0x3,%edx
  de9f69:	48 89 e1             	mov    %rsp,%rcx
  de9f6c:	48 8d 35 4d 24 2a 02 	lea    0x22a244d(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f73:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f7a:	00 
  de9f7b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f80:	e8 3b 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f85:	e9 5b ff ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f8a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9f90:	31 c9                	xor    %ecx,%ecx
  de9f92:	ba 0d 00 00 00       	mov    $0xd,%edx
  de9f97:	48 8d 35 22 24 2a 02 	lea    0x22a2422(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f9e:	48 83 c4 28          	add    $0x28,%rsp
  de9fa2:	e9 19 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9fa7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
CASE method_index=6 target=0xdea070
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
  dea082:	e9 39 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea087:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea08e:	00 00 
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
  dea09e:	ff e0                	jmp    *%rax
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
  dea0d5:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
  dea0da:	48 89 e1             	mov    %rsp,%rcx
  dea0dd:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea0e7:	e9 f9 fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea0ec:	0f 1f 40 00          	nopl   0x0(%rax)
  dea0f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
CASE method_index=7 target=0xdea050
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
  dea082:	e9 39 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea087:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea08e:	00 00 
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
  dea09e:	ff e0                	jmp    *%rax
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
CASE method_index=8 target=0xdea030
  dea034:	00 00                	add    %al,(%rax)
  dea036:	00 48 8d             	add    %cl,-0x73(%rax)
  dea039:	35 82 23 2a 02       	xor    $0x22a2382,%eax
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea047:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea04e:	00 00 
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
  dea082:	e9 39 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea087:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea08e:	00 00 
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
  dea09e:	ff e0                	jmp    *%rax
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
CASE method_index=9 target=0xdea018
  dea019:	c9                   	leave
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea02f:	90                   	nop
  dea030:	31 c9                	xor    %ecx,%ecx
  dea032:	ba 08 00 00 00       	mov    $0x8,%edx
  dea037:	48 8d 35 82 23 2a 02 	lea    0x22a2382(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea047:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea04e:	00 00 
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
  dea082:	e9 39 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea087:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea08e:	00 00 
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
CASE method_index=10 target=0xde9ea0
  de9ea0:	31 c9                	xor    %ecx,%ecx
  de9ea2:	ba 0a 00 00 00       	mov    $0xa,%edx
  de9ea7:	48 8d 35 12 25 2a 02 	lea    0x22a2512(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9eae:	48 83 c4 28          	add    $0x28,%rsp
  de9eb2:	e9 09 4f 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9eb7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9ebe:	00 00 
  de9ec0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ec4:	ba 0b 00 00 00       	mov    $0xb,%edx
  de9ec9:	48 89 e1             	mov    %rsp,%rcx
  de9ecc:	48 8d 35 ed 24 2a 02 	lea    0x22a24ed(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9ed3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9eda:	00 
  de9edb:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9ee0:	e8 db 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9ee5:	48 83 c4 28          	add    $0x28,%rsp
  de9ee9:	c3                   	ret
  de9eea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9ef0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ef4:	ba 02 00 00 00       	mov    $0x2,%edx
  de9ef9:	48 89 e1             	mov    %rsp,%rcx
  de9efc:	48 8d 35 bd 24 2a 02 	lea    0x22a24bd(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f03:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f0a:	00 
  de9f0b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f10:	e8 ab 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f15:	eb ce                	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f17:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f1e:	00 00 
  de9f20:	31 c9                	xor    %ecx,%ecx
CASE method_index=11 target=0xde9ec0
  de9ec0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ec4:	ba 0b 00 00 00       	mov    $0xb,%edx
  de9ec9:	48 89 e1             	mov    %rsp,%rcx
  de9ecc:	48 8d 35 ed 24 2a 02 	lea    0x22a24ed(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9ed3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9eda:	00 
  de9edb:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9ee0:	e8 db 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9ee5:	48 83 c4 28          	add    $0x28,%rsp
  de9ee9:	c3                   	ret
  de9eea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9ef0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ef4:	ba 02 00 00 00       	mov    $0x2,%edx
  de9ef9:	48 89 e1             	mov    %rsp,%rcx
  de9efc:	48 8d 35 bd 24 2a 02 	lea    0x22a24bd(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f03:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9f0a:	00 
  de9f0b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9f10:	e8 ab 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f15:	eb ce                	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  de9f17:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f1e:	00 00 
  de9f20:	31 c9                	xor    %ecx,%ecx
  de9f22:	ba 05 00 00 00       	mov    $0x5,%edx
  de9f27:	48 8d 35 92 24 2a 02 	lea    0x22a2492(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f2e:	48 83 c4 28          	add    $0x28,%rsp
  de9f32:	e9 89 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9f37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9f3e:	00 00 
  de9f40:	31 c9                	xor    %ecx,%ecx
CASE method_index=12 target=0xde9e80
  de9e80:	31 c9                	xor    %ecx,%ecx
  de9e82:	ba 0c 00 00 00       	mov    $0xc,%edx
  de9e87:	48 8d 35 32 25 2a 02 	lea    0x22a2532(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9e8e:	48 83 c4 28          	add    $0x28,%rsp
  de9e92:	e9 29 4f 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9e97:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9e9e:	00 00 
  de9ea0:	31 c9                	xor    %ecx,%ecx
  de9ea2:	ba 0a 00 00 00       	mov    $0xa,%edx
  de9ea7:	48 8d 35 12 25 2a 02 	lea    0x22a2512(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9eae:	48 83 c4 28          	add    $0x28,%rsp
  de9eb2:	e9 09 4f 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9eb7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9ebe:	00 00 
  de9ec0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ec4:	ba 0b 00 00 00       	mov    $0xb,%edx
  de9ec9:	48 89 e1             	mov    %rsp,%rcx
  de9ecc:	48 8d 35 ed 24 2a 02 	lea    0x22a24ed(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9ed3:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  de9eda:	00 
  de9edb:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  de9ee0:	e8 db 4e 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9ee5:	48 83 c4 28          	add    $0x28,%rsp
  de9ee9:	c3                   	ret
  de9eea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  de9ef0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  de9ef4:	ba 02 00 00 00       	mov    $0x2,%edx
  de9ef9:	48 89 e1             	mov    %rsp,%rcx
  de9efc:	48 8d 35 bd 24 2a 02 	lea    0x22a24bd(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f03:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
CASE method_index=13 target=0xde9f90
  de9f90:	31 c9                	xor    %ecx,%ecx
  de9f92:	ba 0d 00 00 00       	mov    $0xd,%edx
  de9f97:	48 8d 35 22 24 2a 02 	lea    0x22a2422(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  de9f9e:	48 83 c4 28          	add    $0x28,%rsp
  de9fa2:	e9 19 4e 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  de9fa7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  de9fae:	00 00 
  de9fb0:	48 8b 07             	mov    (%rdi),%rax
  de9fb3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  de9fb7:	48 8b 80 48 01 00 00 	mov    0x148(%rax),%rax
  de9fbe:	48 83 c4 28          	add    $0x28,%rsp
  de9fc2:	ff e0                	jmp    *%rax
  de9fc4:	0f 1f 40 00          	nopl   0x0(%rax)
  de9fc8:	48 8b 07             	mov    (%rdi),%rax
  de9fcb:	48 8d 15 4e c9 e3 ff 	lea    -0x1c36b2(%rip),%rdx        # c26920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x880f0>
  de9fd2:	48 8b 80 40 01 00 00 	mov    0x140(%rax),%rax
  de9fd9:	48 39 d0             	cmp    %rdx,%rax
  de9fdc:	0f 84 9e fe ff ff    	je     de9e80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b650>
  de9fe2:	48 83 c4 28          	add    $0x28,%rsp
  de9fe6:	ff e0                	jmp    *%rax
  de9fe8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  de9fef:	00 
  de9ff0:	48 8b 07             	mov    (%rdi),%rax
  de9ff3:	48 8b 80 38 01 00 00 	mov    0x138(%rax),%rax
  de9ffa:	48 83 c4 28          	add    $0x28,%rsp
  de9ffe:	ff e0                	jmp    *%rax
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
CASE method_index=14 target=0xdea000
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
  dea012:	ff e0                	jmp    *%rax
  dea014:	0f 1f 40 00          	nopl   0x0(%rax)
  dea018:	31 c9                	xor    %ecx,%ecx
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea02f:	90                   	nop
  dea030:	31 c9                	xor    %ecx,%ecx
  dea032:	ba 08 00 00 00       	mov    $0x8,%edx
  dea037:	48 8d 35 82 23 2a 02 	lea    0x22a2382(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea047:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea04e:	00 00 
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
  dea070:	31 c9                	xor    %ecx,%ecx
  dea072:	ba 06 00 00 00       	mov    $0x6,%edx
  dea077:	48 8d 35 42 23 2a 02 	lea    0x22a2342(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea07e:	48 83 c4 28          	add    $0x28,%rsp
CASE method_index=15 target=0xde9ff0
  de9ff0:	48 8b 07             	mov    (%rdi),%rax
  de9ff3:	48 8b 80 38 01 00 00 	mov    0x138(%rax),%rax
  de9ffa:	48 83 c4 28          	add    $0x28,%rsp
  de9ffe:	ff e0                	jmp    *%rax
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
  dea012:	ff e0                	jmp    *%rax
  dea014:	0f 1f 40 00          	nopl   0x0(%rax)
  dea018:	31 c9                	xor    %ecx,%ecx
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea02f:	90                   	nop
  dea030:	31 c9                	xor    %ecx,%ecx
  dea032:	ba 08 00 00 00       	mov    $0x8,%edx
  dea037:	48 8d 35 82 23 2a 02 	lea    0x22a2382(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea047:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea04e:	00 00 
  dea050:	31 c9                	xor    %ecx,%ecx
  dea052:	ba 07 00 00 00       	mov    $0x7,%edx
  dea057:	48 8d 35 62 23 2a 02 	lea    0x22a2362(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea05e:	48 83 c4 28          	add    $0x28,%rsp
  dea062:	e9 59 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea06e:	00 00 
CASE method_index=16 target=0xde9fc8
  de9fca:	07                   	(bad)
  de9fcb:	48 8d 15 4e c9 e3 ff 	lea    -0x1c36b2(%rip),%rdx        # c26920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x880f0>
  de9fd2:	48 8b 80 40 01 00 00 	mov    0x140(%rax),%rax
  de9fd9:	48 39 d0             	cmp    %rdx,%rax
  de9fdc:	0f 84 9e fe ff ff    	je     de9e80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b650>
  de9fe2:	48 83 c4 28          	add    $0x28,%rsp
  de9fe6:	ff e0                	jmp    *%rax
  de9fe8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  de9fef:	00 
  de9ff0:	48 8b 07             	mov    (%rdi),%rax
  de9ff3:	48 8b 80 38 01 00 00 	mov    0x138(%rax),%rax
  de9ffa:	48 83 c4 28          	add    $0x28,%rsp
  de9ffe:	ff e0                	jmp    *%rax
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
  dea012:	ff e0                	jmp    *%rax
  dea014:	0f 1f 40 00          	nopl   0x0(%rax)
  dea018:	31 c9                	xor    %ecx,%ecx
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea02f:	90                   	nop
  dea030:	31 c9                	xor    %ecx,%ecx
  dea032:	ba 08 00 00 00       	mov    $0x8,%edx
  dea037:	48 8d 35 82 23 2a 02 	lea    0x22a2382(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea03e:	48 83 c4 28          	add    $0x28,%rsp
  dea042:	e9 79 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CASE method_index=17 target=0xde9fb0
  de9fb0:	48 8b 07             	mov    (%rdi),%rax
  de9fb3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  de9fb7:	48 8b 80 48 01 00 00 	mov    0x148(%rax),%rax
  de9fbe:	48 83 c4 28          	add    $0x28,%rsp
  de9fc2:	ff e0                	jmp    *%rax
  de9fc4:	0f 1f 40 00          	nopl   0x0(%rax)
  de9fc8:	48 8b 07             	mov    (%rdi),%rax
  de9fcb:	48 8d 15 4e c9 e3 ff 	lea    -0x1c36b2(%rip),%rdx        # c26920 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x880f0>
  de9fd2:	48 8b 80 40 01 00 00 	mov    0x140(%rax),%rax
  de9fd9:	48 39 d0             	cmp    %rdx,%rax
  de9fdc:	0f 84 9e fe ff ff    	je     de9e80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b650>
  de9fe2:	48 83 c4 28          	add    $0x28,%rsp
  de9fe6:	ff e0                	jmp    *%rax
  de9fe8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  de9fef:	00 
  de9ff0:	48 8b 07             	mov    (%rdi),%rax
  de9ff3:	48 8b 80 38 01 00 00 	mov    0x138(%rax),%rax
  de9ffa:	48 83 c4 28          	add    $0x28,%rsp
  de9ffe:	ff e0                	jmp    *%rax
  dea000:	48 8b 07             	mov    (%rdi),%rax
  dea003:	48 8b 71 08          	mov    0x8(%rcx),%rsi
  dea007:	48 8b 80 30 01 00 00 	mov    0x130(%rax),%rax
  dea00e:	48 83 c4 28          	add    $0x28,%rsp
  dea012:	ff e0                	jmp    *%rax
  dea014:	0f 1f 40 00          	nopl   0x0(%rax)
  dea018:	31 c9                	xor    %ecx,%ecx
  dea01a:	ba 09 00 00 00       	mov    $0x9,%edx
  dea01f:	48 8d 35 9a 23 2a 02 	lea    0x22a239a(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea026:	48 83 c4 28          	add    $0x28,%rsp
  dea02a:	e9 91 4d 6f ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CASE method_index=18 target=0xdea090
  dea090:	48 8b 07             	mov    (%rdi),%rax
  dea093:	48 8b 80 50 01 00 00 	mov    0x150(%rax),%rax
  dea09a:	48 83 c4 28          	add    $0x28,%rsp
  dea09e:	ff e0                	jmp    *%rax
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
  dea0d5:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
  dea0da:	48 89 e1             	mov    %rsp,%rcx
  dea0dd:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea0e7:	e9 f9 fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea0ec:	0f 1f 40 00          	nopl   0x0(%rax)
  dea0f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea0f4:	31 d2                	xor    %edx,%edx
  dea0f6:	48 89 e1             	mov    %rsp,%rcx
  dea0f9:	48 8d 35 c0 22 2a 02 	lea    0x22a22c0(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea100:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea107:	00 
  dea108:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
CASE method_index=19 target=0xdea0a0
  dea0a0:	48 8b 07             	mov    (%rdi),%rax
  dea0a3:	48 8b 80 58 01 00 00 	mov    0x158(%rax),%rax
  dea0aa:	e9 33 ff ff ff       	jmp    de9fe2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b7b2>
  dea0af:	90                   	nop
  dea0b0:	48 8b 41 18          	mov    0x18(%rcx),%rax
  dea0b4:	48 8b 51 10          	mov    0x10(%rcx),%rdx
  dea0b8:	48 8d 35 01 23 2a 02 	lea    0x22a2301(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea0bf:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea0c6:	00 
  dea0c7:	48 8b 49 08          	mov    0x8(%rcx),%rcx
  dea0cb:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
  dea0d0:	ba 01 00 00 00       	mov    $0x1,%edx
  dea0d5:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
  dea0da:	48 89 e1             	mov    %rsp,%rcx
  dea0dd:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  dea0e2:	e8 d9 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea0e7:	e9 f9 fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea0ec:	0f 1f 40 00          	nopl   0x0(%rax)
  dea0f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
  dea0f4:	31 d2                	xor    %edx,%edx
  dea0f6:	48 89 e1             	mov    %rsp,%rcx
  dea0f9:	48 8d 35 c0 22 2a 02 	lea    0x22a22c0(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  dea100:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  dea107:	00 
  dea108:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
  dea10d:	e8 ae 4c 6f ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  dea112:	e9 ce fd ff ff       	jmp    de9ee5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24b6b5>
  dea117:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  dea11e:	00 00 
  dea120:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)

WORLDMAP_STRING_NEIGHBORHOOD
va_or_fileoff=0x1cd8bb4 text=tibia::worldmap::TWorldmapProtocolMessageHandler
va_or_fileoff=0x1cd8c2f text=handleFullMapMessage
va_or_fileoff=0x1cd8c44 text=tibia::protobuf::protocol::GameserverMessageFullMap
va_or_fileoff=0x1cd8c87 text=handleLeftColumnMessage
va_or_fileoff=0x1cd8c9f text=tibia::protobuf::protocol::GameserverMessageLeftColumn
va_or_fileoff=0x1cd8ce8 text=handleRightColumnMessage
va_or_fileoff=0x1cd8d01 text=tibia::protobuf::protocol::GameserverMessageRightColumn
va_or_fileoff=0x1cd8d4c text=handleTopRowMessage
va_or_fileoff=0x1cd8d60 text=tibia::protobuf::protocol::GameserverMessageTopRow
va_or_fileoff=0x1cd8da1 text=handleBottomRowMessage
va_or_fileoff=0x1cd8db8 text=tibia::protobuf::protocol::GameserverMessageBottomRow
va_or_fileoff=0x1cd8dff text=handleTopFloorMessage
va_or_fileoff=0x1cd8e15 text=tibia::protobuf::protocol::GameserverMessageTopFloor
va_or_fileoff=0x1cd8e5a text=handleBottomFloorMessage
va_or_fileoff=0x1cd8e73 text=tibia::protobuf::protocol::GameserverMessageBottomFloor
va_or_fileoff=0x1cd8ebe text=handleFieldDataMessage
va_or_fileoff=0x1cd8ed5 text=tibia::protobuf::protocol::GameserverMessageFieldData
va_or_fileoff=0x1cd8f1c text=handleCreateOnMapMessage
va_or_fileoff=0x1cd8f35 text=tibia::protobuf::protocol::GameserverMessageCreateOnMap
va_or_fileoff=0x1cd8f80 text=handleChangeOnMapMessage
va_or_fileoff=0x1cd8f99 text=tibia::protobuf::protocol::GameserverMessageChangeOnMap
va_or_fileoff=0x1cd8fe4 text=handleDeleteOnMapMessage
va_or_fileoff=0x1cd8ffd text=tibia::protobuf::protocol::GameserverMessageDeleteOnMap
va_or_fileoff=0x1cd9048 text=handleAmbientLightMessage
va_or_fileoff=0x1cd9062 text=tibia::protobuf::protocol::GameserverMessageAmbientLight
va_or_fileoff=0x1cd90af text=handleTibiaTimeMessage
va_or_fileoff=0x1cd90c6 text=tibia::protobuf::protocol::GameserverMessageTibiaTime
```

<!-- END GENERATED WORLDMAP JUMP TABLE -->

<!-- BEGIN GENERATED WORLDMAP VTABLE -->

## Generated Worldmap vtable evidence

Source: GitHub Actions run `31579018335` on `oteryn-synology-staging`. Sanitized address/disassembly metadata only.

```text
qt_metacall_candidate=0xe004d0
qword_hits=2
HIT index=0 file_off=0x28d740 va=0x28d740 section=.rela.dyn
HIT index=1 file_off=0x308c258 va=0x308c258 section=.data.rel.ro

candidate_address_points=7
CANDIDATE index=0 address_point_va=0x308c248 file_off=0x308c248 qt_metacall_slot_index=2
  vtable_offset=0x130 target=0xc31990
  vtable_offset=0x138 target=0xc2a420
  vtable_offset=0x140 target=0xc26920
  vtable_offset=0x148 target=0xc2e3c0
  vtable_offset=0x150 target=0xc269f0
  vtable_offset=0x158 target=0xc26b70
  slot=0x0 ptr=0xdedea0
  slot=0x8 ptr=0xdfe4e0
  slot=0x10 ptr=0xe004d0
  slot=0x18 ptr=0xc3efc0
  slot=0x20 ptr=0xc3f180
  slot=0x60 ptr=0xc3a190
  slot=0x68 ptr=0xc21510
  slot=0x70 ptr=0xc21520
  slot=0x78 ptr=0xc21530
  slot=0x80 ptr=0xc21660
  slot=0x88 ptr=0xc293d0
  slot=0x90 ptr=0xc26700
  slot=0x98 ptr=0xc26cf0
  slot=0xa0 ptr=0xc21700
  slot=0xa8 ptr=0xc27280
  slot=0xb0 ptr=0xc21740
  slot=0xb8 ptr=0xc21690
  slot=0xc0 ptr=0xc26df0
  slot=0xc8 ptr=0xc27020
  slot=0xd0 ptr=0xc26940
  slot=0xd8 ptr=0xc216d0
  slot=0xe0 ptr=0xc268b0
  slot=0xe8 ptr=0xc27350
  slot=0xf0 ptr=0xc21810
  slot=0xf8 ptr=0xc21820
  slot=0x100 ptr=0xc26350
  slot=0x108 ptr=0xc27390
  slot=0x110 ptr=0xc273c0
  slot=0x118 ptr=0xc21830
  slot=0x120 ptr=0xc21860
  slot=0x128 ptr=0xc2a960
  slot=0x130 ptr=0xc31990
  slot=0x138 ptr=0xc2a420
  slot=0x140 ptr=0xc26920
  slot=0x148 ptr=0xc2e3c0
  slot=0x150 ptr=0xc269f0
  slot=0x158 ptr=0xc26b70
CANDIDATE index=1 address_point_va=0x308c240 file_off=0x308c240 qt_metacall_slot_index=3
  vtable_offset=0x130 target=0xc2a960
  vtable_offset=0x138 target=0xc31990
  vtable_offset=0x140 target=0xc2a420
  vtable_offset=0x148 target=0xc26920
  vtable_offset=0x150 target=0xc2e3c0
  vtable_offset=0x158 target=0xc269f0
  slot=0x8 ptr=0xdedea0
  slot=0x10 ptr=0xdfe4e0
  slot=0x18 ptr=0xe004d0
  slot=0x20 ptr=0xc3efc0
  slot=0x28 ptr=0xc3f180
  slot=0x68 ptr=0xc3a190
  slot=0x70 ptr=0xc21510
  slot=0x78 ptr=0xc21520
  slot=0x80 ptr=0xc21530
  slot=0x88 ptr=0xc21660
  slot=0x90 ptr=0xc293d0
  slot=0x98 ptr=0xc26700
  slot=0xa0 ptr=0xc26cf0
  slot=0xa8 ptr=0xc21700
  slot=0xb0 ptr=0xc27280
  slot=0xb8 ptr=0xc21740
  slot=0xc0 ptr=0xc21690
  slot=0xc8 ptr=0xc26df0
  slot=0xd0 ptr=0xc27020
  slot=0xd8 ptr=0xc26940
  slot=0xe0 ptr=0xc216d0
  slot=0xe8 ptr=0xc268b0
  slot=0xf0 ptr=0xc27350
  slot=0xf8 ptr=0xc21810
  slot=0x100 ptr=0xc21820
  slot=0x108 ptr=0xc26350
  slot=0x110 ptr=0xc27390
  slot=0x118 ptr=0xc273c0
  slot=0x120 ptr=0xc21830
  slot=0x128 ptr=0xc21860
  slot=0x130 ptr=0xc2a960
  slot=0x138 ptr=0xc31990
  slot=0x140 ptr=0xc2a420
  slot=0x148 ptr=0xc26920
  slot=0x150 ptr=0xc2e3c0
  slot=0x158 ptr=0xc269f0
CANDIDATE index=2 address_point_va=0x308c238 file_off=0x308c238 qt_metacall_slot_index=4
  vtable_offset=0x130 target=0xc21860
  vtable_offset=0x138 target=0xc2a960
  vtable_offset=0x140 target=0xc31990
  vtable_offset=0x148 target=0xc2a420
  vtable_offset=0x150 target=0xc26920
  vtable_offset=0x158 target=0xc2e3c0
  slot=0x10 ptr=0xdedea0
  slot=0x18 ptr=0xdfe4e0
  slot=0x20 ptr=0xe004d0
  slot=0x28 ptr=0xc3efc0
  slot=0x30 ptr=0xc3f180
  slot=0x70 ptr=0xc3a190
  slot=0x78 ptr=0xc21510
  slot=0x80 ptr=0xc21520
  slot=0x88 ptr=0xc21530
  slot=0x90 ptr=0xc21660
  slot=0x98 ptr=0xc293d0
  slot=0xa0 ptr=0xc26700
  slot=0xa8 ptr=0xc26cf0
  slot=0xb0 ptr=0xc21700
  slot=0xb8 ptr=0xc27280
  slot=0xc0 ptr=0xc21740
  slot=0xc8 ptr=0xc21690
  slot=0xd0 ptr=0xc26df0
  slot=0xd8 ptr=0xc27020
  slot=0xe0 ptr=0xc26940
  slot=0xe8 ptr=0xc216d0
  slot=0xf0 ptr=0xc268b0
  slot=0xf8 ptr=0xc27350
  slot=0x100 ptr=0xc21810
  slot=0x108 ptr=0xc21820
  slot=0x110 ptr=0xc26350
  slot=0x118 ptr=0xc27390
  slot=0x120 ptr=0xc273c0
  slot=0x128 ptr=0xc21830
  slot=0x130 ptr=0xc21860
  slot=0x138 ptr=0xc2a960
  slot=0x140 ptr=0xc31990
  slot=0x148 ptr=0xc2a420
  slot=0x150 ptr=0xc26920
  slot=0x158 ptr=0xc2e3c0
CANDIDATE index=3 address_point_va=0x308c230 file_off=0x308c230 qt_metacall_slot_index=5
  vtable_offset=0x130 target=0xc21830
  vtable_offset=0x138 target=0xc21860
  vtable_offset=0x140 target=0xc2a960
  vtable_offset=0x148 target=0xc31990
  vtable_offset=0x150 target=0xc2a420
  vtable_offset=0x158 target=0xc26920
  slot=0x0 ptr=0xc31bb0
  slot=0x18 ptr=0xdedea0
  slot=0x20 ptr=0xdfe4e0
  slot=0x28 ptr=0xe004d0
  slot=0x30 ptr=0xc3efc0
  slot=0x38 ptr=0xc3f180
  slot=0x78 ptr=0xc3a190
  slot=0x80 ptr=0xc21510
  slot=0x88 ptr=0xc21520
  slot=0x90 ptr=0xc21530
  slot=0x98 ptr=0xc21660
  slot=0xa0 ptr=0xc293d0
  slot=0xa8 ptr=0xc26700
  slot=0xb0 ptr=0xc26cf0
  slot=0xb8 ptr=0xc21700
  slot=0xc0 ptr=0xc27280
  slot=0xc8 ptr=0xc21740
  slot=0xd0 ptr=0xc21690
  slot=0xd8 ptr=0xc26df0
  slot=0xe0 ptr=0xc27020
  slot=0xe8 ptr=0xc26940
  slot=0xf0 ptr=0xc216d0
  slot=0xf8 ptr=0xc268b0
  slot=0x100 ptr=0xc27350
  slot=0x108 ptr=0xc21810
  slot=0x110 ptr=0xc21820
  slot=0x118 ptr=0xc26350
  slot=0x120 ptr=0xc27390
  slot=0x128 ptr=0xc273c0
  slot=0x130 ptr=0xc21830
  slot=0x138 ptr=0xc21860
  slot=0x140 ptr=0xc2a960
  slot=0x148 ptr=0xc31990
  slot=0x150 ptr=0xc2a420
  slot=0x158 ptr=0xc26920
CANDIDATE index=4 address_point_va=0x308c228 file_off=0x308c228 qt_metacall_slot_index=6
  vtable_offset=0x130 target=0xc273c0
  vtable_offset=0x138 target=0xc21830
  vtable_offset=0x140 target=0xc21860
  vtable_offset=0x148 target=0xc2a960
  vtable_offset=0x150 target=0xc31990
  vtable_offset=0x158 target=0xc2a420
  slot=0x0 ptr=0xc21500
  slot=0x8 ptr=0xc31bb0
  slot=0x20 ptr=0xdedea0
  slot=0x28 ptr=0xdfe4e0
  slot=0x30 ptr=0xe004d0
  slot=0x38 ptr=0xc3efc0
  slot=0x40 ptr=0xc3f180
  slot=0x80 ptr=0xc3a190
  slot=0x88 ptr=0xc21510
  slot=0x90 ptr=0xc21520
  slot=0x98 ptr=0xc21530
  slot=0xa0 ptr=0xc21660
  slot=0xa8 ptr=0xc293d0
  slot=0xb0 ptr=0xc26700
  slot=0xb8 ptr=0xc26cf0
  slot=0xc0 ptr=0xc21700
  slot=0xc8 ptr=0xc27280
  slot=0xd0 ptr=0xc21740
  slot=0xd8 ptr=0xc21690
  slot=0xe0 ptr=0xc26df0
  slot=0xe8 ptr=0xc27020
  slot=0xf0 ptr=0xc26940
  slot=0xf8 ptr=0xc216d0
  slot=0x100 ptr=0xc268b0
  slot=0x108 ptr=0xc27350
  slot=0x110 ptr=0xc21810
  slot=0x118 ptr=0xc21820
  slot=0x120 ptr=0xc26350
  slot=0x128 ptr=0xc27390
  slot=0x130 ptr=0xc273c0
  slot=0x138 ptr=0xc21830
  slot=0x140 ptr=0xc21860
  slot=0x148 ptr=0xc2a960
  slot=0x150 ptr=0xc31990
  slot=0x158 ptr=0xc2a420
CANDIDATE index=5 address_point_va=0x308c220 file_off=0x308c220 qt_metacall_slot_index=7
  vtable_offset=0x130 target=0xc27390
  vtable_offset=0x138 target=0xc273c0
  vtable_offset=0x140 target=0xc21830
  vtable_offset=0x148 target=0xc21860
  vtable_offset=0x150 target=0xc2a960
  vtable_offset=0x158 target=0xc31990
  slot=0x0 ptr=0xb1a310
  slot=0x8 ptr=0xc21500
  slot=0x10 ptr=0xc31bb0
  slot=0x28 ptr=0xdedea0
  slot=0x30 ptr=0xdfe4e0
  slot=0x38 ptr=0xe004d0
  slot=0x40 ptr=0xc3efc0
  slot=0x48 ptr=0xc3f180
  slot=0x88 ptr=0xc3a190
  slot=0x90 ptr=0xc21510
  slot=0x98 ptr=0xc21520
  slot=0xa0 ptr=0xc21530
  slot=0xa8 ptr=0xc21660
  slot=0xb0 ptr=0xc293d0
  slot=0xb8 ptr=0xc26700
  slot=0xc0 ptr=0xc26cf0
  slot=0xc8 ptr=0xc21700
  slot=0xd0 ptr=0xc27280
  slot=0xd8 ptr=0xc21740
  slot=0xe0 ptr=0xc21690
  slot=0xe8 ptr=0xc26df0
  slot=0xf0 ptr=0xc27020
  slot=0xf8 ptr=0xc26940
  slot=0x100 ptr=0xc216d0
  slot=0x108 ptr=0xc268b0
  slot=0x110 ptr=0xc27350
  slot=0x118 ptr=0xc21810
  slot=0x120 ptr=0xc21820
  slot=0x128 ptr=0xc26350
  slot=0x130 ptr=0xc27390
  slot=0x138 ptr=0xc273c0
  slot=0x140 ptr=0xc21830
  slot=0x148 ptr=0xc21860
  slot=0x150 ptr=0xc2a960
  slot=0x158 ptr=0xc31990
CANDIDATE index=6 address_point_va=0x308c218 file_off=0x308c218 qt_metacall_slot_index=8
  vtable_offset=0x130 target=0xc26350
  vtable_offset=0x138 target=0xc27390
  vtable_offset=0x140 target=0xc273c0
  vtable_offset=0x148 target=0xc21830
  vtable_offset=0x150 target=0xc21860
  vtable_offset=0x158 target=0xc2a960
  slot=0x0 ptr=0xc274a0
  slot=0x8 ptr=0xb1a310
  slot=0x10 ptr=0xc21500
  slot=0x18 ptr=0xc31bb0
  slot=0x30 ptr=0xdedea0
  slot=0x38 ptr=0xdfe4e0
  slot=0x40 ptr=0xe004d0
  slot=0x48 ptr=0xc3efc0
  slot=0x50 ptr=0xc3f180
  slot=0x90 ptr=0xc3a190
  slot=0x98 ptr=0xc21510
  slot=0xa0 ptr=0xc21520
  slot=0xa8 ptr=0xc21530
  slot=0xb0 ptr=0xc21660
  slot=0xb8 ptr=0xc293d0
  slot=0xc0 ptr=0xc26700
  slot=0xc8 ptr=0xc26cf0
  slot=0xd0 ptr=0xc21700
  slot=0xd8 ptr=0xc27280
  slot=0xe0 ptr=0xc21740
  slot=0xe8 ptr=0xc21690
  slot=0xf0 ptr=0xc26df0
  slot=0xf8 ptr=0xc27020
  slot=0x100 ptr=0xc26940
  slot=0x108 ptr=0xc216d0
  slot=0x110 ptr=0xc268b0
  slot=0x118 ptr=0xc27350
  slot=0x120 ptr=0xc21810
  slot=0x128 ptr=0xc21820
  slot=0x130 ptr=0xc26350
  slot=0x138 ptr=0xc27390
  slot=0x140 ptr=0xc273c0
  slot=0x148 ptr=0xc21830
  slot=0x150 ptr=0xc21860
  slot=0x158 ptr=0xc2a960

handler_slot_target_count=12
HANDLER_SLOT_TARGET_DISASSEMBLY
TARGET 0xc31990
  c31990:	41 55                	push   %r13
  c31992:	41 54                	push   %r12
  c31994:	55                   	push   %rbp
  c31995:	48 89 fd             	mov    %rdi,%rbp
  c31998:	53                   	push   %rbx
  c31999:	48 89 f3             	mov    %rsi,%rbx
  c3199c:	48 83 ec 08          	sub    $0x8,%rsp
  c319a0:	48 8b 3e             	mov    (%rsi),%rdi
  c319a3:	48 8b 07             	mov    (%rdi),%rax
  c319a6:	ff 10                	call   *(%rax)
  c319a8:	3d d5 00 00 00       	cmp    $0xd5,%eax
  c319ad:	74 51                	je     c31a00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x931d0>
  c319af:	48 8b 3b             	mov    (%rbx),%rdi
  c319b2:	48 8b 07             	mov    (%rdi),%rax
  c319b5:	ff 10                	call   *(%rax)
  c319b7:	3d d6 00 00 00       	cmp    $0xd6,%eax
  c319bc:	74 22                	je     c319e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x931b0>
  c319be:	48 8b 3b             	mov    (%rbx),%rdi
  c319c1:	48 8b 07             	mov    (%rdi),%rax
  c319c4:	ff 10                	call   *(%rax)
  c319c6:	3d d7 00 00 00       	cmp    $0xd7,%eax
  c319cb:	0f 84 9f 00 00 00    	je     c31a70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x93240>
  c319d1:	48 83 c4 08          	add    $0x8,%rsp
  c319d5:	5b                   	pop    %rbx
  c319d6:	5d                   	pop    %rbp
  c319d7:	41 5c                	pop    %r12
  c319d9:	41 5d                	pop    %r13
  c319db:	c3                   	ret
  c319dc:	0f 1f 40 00          	nopl   0x0(%rax)
  c319e0:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c319e4:	48 8b 80 98 00 00 00 	mov    0x98(%rax),%rax
  c319eb:	48 83 c4 08          	add    $0x8,%rsp
  c319ef:	48 89 ef             	mov    %rbp,%rdi
  c319f2:	5b                   	pop    %rbx
  c319f3:	5d                   	pop    %rbp
  c319f4:	41 5c                	pop    %r12
  c319f6:	41 5d                	pop    %r13
  c319f8:	ff e0                	jmp    *%rax
  c319fa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c31a00:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c31a04:	48 8d 15 f5 4c ff ff 	lea    -0xb30b(%rip),%rdx        # c26700 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87ed0>
  c31a0b:	48 8b 80 90 00 00 00 	mov    0x90(%rax),%rax
  c31a12:	48 39 d0             	cmp    %rdx,%rax
  c31a15:	75 d4                	jne    c319eb <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x931bb>
  c31a17:	48 8b 7d 20          	mov    0x20(%rbp),%rdi
  c31a1b:	48 8d 0d 3e 16 01 00 	lea    0x1163e(%rip),%rcx        # c43060 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xa4830>
  c31a22:	48 8b 07             	mov    (%rdi),%rax
  c31a25:	48 8b 90 80 00 00 00 	mov    0x80(%rax),%rdx
  c31a2c:	48 39 ca             	cmp    %rcx,%rdx
  c31a2f:	0f 85 33 01 00 00    	jne    c31b68 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x93338>
  c31a35:	48 ba ff ff ff ff ff 	movabs $0x7fffffffffffffff,%rdx
  c31a3c:	ff ff 7f 
  c31a3f:	48 39 57 50          	cmp    %rdx,0x50(%rdi)
  c31a43:	75 8c                	jne    c319d1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x931a1>
  c31a45:	ff 50 70             	call   *0x70(%rax)
  c31a48:	48 83 c4 08          	add    $0x8,%rsp
  c31a4c:	48 89 ef             	mov    %rbp,%rdi
  c31a4f:	31 c9                	xor    %ecx,%ecx
  c31a51:	5b                   	pop    %rbx
  c31a52:	ba 0a 00 00 00       	mov    $0xa,%edx
  c31a57:	48 8d 35 62 a9 45 02 	lea    0x245a962(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c31a5e:	5d                   	pop    %rbp
  c31a5f:	41 5c                	pop    %r12
  c31a61:	41 5d                	pop    %r13
  c31a63:	e9 58 d3 8a ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c31a68:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  c31a6f:	00 
  c31a70:	48 8b 3b             	mov    (%rbx),%rdi
  c31a73:	45 31 e4             	xor    %r12d,%r12d
  c31a76:	48 85 ff             	test   %rdi,%rdi
  c31a79:	74 45                	je     c31ac0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x93290>
  c31a7b:	31 c9                	xor    %ecx,%ecx
  c31a7d:	48 8d 15 7c 96 45 02 	lea    0x245967c(%rip),%rdx        # 308b100 <vtable for __cxxabiv1::__si_class_type_info@CXXABI_1.3>
  c31a84:	48 8d 35 d5 e9 43 02 	lea    0x243e9d5(%rip),%rsi        # 3070460 <vtable for __cxxabiv1::__class_type_info@CXXABI_1.3>
  c31a8b:	e8 f0 e3 8a ff       	call   4dfe80 <__dynamic_cast@plt>
  c31a90:	48 89 c7             	mov    %rax,%rdi
  c31a93:	48 85 c0             	test   %rax,%rax
  c31a96:	74 28                	je     c31ac0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x93290>
  c31a98:	4c 8b 63 08          	mov    0x8(%rbx),%r12
  c31a9c:	4d 85 e4             	test   %r12,%r12
  c31a9f:	74 1f                	je     c31ac0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x93290>
  c31aa1:	48 8b 05 00 e0 4f 02 	mov    0x24fe000(%rip),%rax        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
  c31aa8:	80 38 00             	cmpb   $0x0,(%rax)
  c31aab:	0f 84 dd 00 00 00    	je     c31b8e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x9335e>
  c31ab1:	41 83 44 24 08 01    	addl   $0x1,0x8(%r12)
  c31ab7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c31abe:	00 00 
  c31ac0:	48 8b 55 20          	mov    0x20(%rbp),%rdx
  c31ac4:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c31ac8:	4c 8b 6f 10          	mov    0x10(%rdi),%r13
TARGET 0xc2a420
  c2a420:	41 56                	push   %r14
  c2a422:	48 8d 15 57 72 ff ff 	lea    -0x8da9(%rip),%rdx        # c21680 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x82e50>
  c2a429:	41 54                	push   %r12
  c2a42b:	55                   	push   %rbp
  c2a42c:	48 89 fd             	mov    %rdi,%rbp
  c2a42f:	53                   	push   %rbx
  c2a430:	48 81 ec a8 00 00 00 	sub    $0xa8,%rsp
  c2a437:	48 8b 7f 10          	mov    0x10(%rdi),%rdi
  c2a43b:	48 8b 07             	mov    (%rdi),%rax
  c2a43e:	48 8b 40 78          	mov    0x78(%rax),%rax
  c2a442:	48 39 d0             	cmp    %rdx,%rax
  c2a445:	0f 85 45 04 00 00    	jne    c2a890 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c060>
  c2a44b:	48 8d 9f 98 02 00 00 	lea    0x298(%rdi),%rbx
  c2a452:	0f b6 03             	movzbl (%rbx),%eax
  c2a455:	f3 0f 6f 5b 10       	movdqu 0x10(%rbx),%xmm3
  c2a45a:	48 8b 53 20          	mov    0x20(%rbx),%rdx
  c2a45e:	88 44 24 10          	mov    %al,0x10(%rsp)
  c2a462:	8b 43 02             	mov    0x2(%rbx),%eax
  c2a465:	89 44 24 12          	mov    %eax,0x12(%rsp)
  c2a469:	48 8b 43 08          	mov    0x8(%rbx),%rax
  c2a46d:	48 89 54 24 30       	mov    %rdx,0x30(%rsp)
  c2a472:	48 89 44 24 18       	mov    %rax,0x18(%rsp)
  c2a477:	48 8b 43 10          	mov    0x10(%rbx),%rax
  c2a47b:	0f 29 5c 24 20       	movaps %xmm3,0x20(%rsp)
  c2a480:	48 85 c0             	test   %rax,%rax
  c2a483:	74 04                	je     c2a489 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bc59>
  c2a485:	f0 83 00 01          	lock addl $0x1,(%rax)
  c2a489:	4c 8d 64 24 38       	lea    0x38(%rsp),%r12
  c2a48e:	48 8d 73 28          	lea    0x28(%rbx),%rsi
  c2a492:	4c 89 e7             	mov    %r12,%rdi
  c2a495:	e8 46 2b 8b ff       	call   4dcfe0 <QDateTime::QDateTime(QDateTime const&)@plt>
  c2a49a:	f3 0f 6f 63 30       	movdqu 0x30(%rbx),%xmm4
  c2a49f:	48 8b 53 40          	mov    0x40(%rbx),%rdx
  c2a4a3:	48 8b 43 30          	mov    0x30(%rbx),%rax
  c2a4a7:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
  c2a4ac:	0f 29 64 24 40       	movaps %xmm4,0x40(%rsp)
  c2a4b1:	48 85 c0             	test   %rax,%rax
  c2a4b4:	74 04                	je     c2a4ba <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bc8a>
  c2a4b6:	f0 83 00 01          	lock addl $0x1,(%rax)
  c2a4ba:	8b 43 48             	mov    0x48(%rbx),%eax
  c2a4bd:	f3 0f 6f 6b 50       	movdqu 0x50(%rbx),%xmm5
  c2a4c2:	48 8b 53 60          	mov    0x60(%rbx),%rdx
  c2a4c6:	89 44 24 58          	mov    %eax,0x58(%rsp)
  c2a4ca:	48 8b 43 50          	mov    0x50(%rbx),%rax
  c2a4ce:	48 89 54 24 70       	mov    %rdx,0x70(%rsp)
  c2a4d3:	0f 29 6c 24 60       	movaps %xmm5,0x60(%rsp)
  c2a4d8:	48 85 c0             	test   %rax,%rax
  c2a4db:	74 04                	je     c2a4e1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bcb1>
  c2a4dd:	f0 83 00 01          	lock addl $0x1,(%rax)
  c2a4e1:	8b 43 68             	mov    0x68(%rbx),%eax
  c2a4e4:	f3 0f 6f 73 70       	movdqu 0x70(%rbx),%xmm6
  c2a4e9:	48 8b 93 80 00 00 00 	mov    0x80(%rbx),%rdx
  c2a4f0:	89 44 24 78          	mov    %eax,0x78(%rsp)
  c2a4f4:	48 8b 43 70          	mov    0x70(%rbx),%rax
  c2a4f8:	48 89 94 24 90 00 00 	mov    %rdx,0x90(%rsp)
  c2a4ff:	00 
  c2a500:	0f 29 b4 24 80 00 00 	movaps %xmm6,0x80(%rsp)
  c2a507:	00 
  c2a508:	48 85 c0             	test   %rax,%rax
  c2a50b:	74 04                	je     c2a511 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bce1>
  c2a50d:	f0 83 00 01          	lock addl $0x1,(%rax)
  c2a511:	8b 83 88 00 00 00    	mov    0x88(%rbx),%eax
  c2a517:	48 8b 5d 20          	mov    0x20(%rbp),%rbx
  c2a51b:	48 8d 0d 2e 6c 0c 00 	lea    0xc6c2e(%rip),%rcx        # cf1150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x152920>
  c2a522:	89 84 24 98 00 00 00 	mov    %eax,0x98(%rsp)
  c2a529:	48 8b 03             	mov    (%rbx),%rax
  c2a52c:	48 8b 50 38          	mov    0x38(%rax),%rdx
  c2a530:	48 39 ca             	cmp    %rcx,%rdx
  c2a533:	0f 85 27 03 00 00    	jne    c2a860 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c030>
  c2a539:	f2 0f 10 43 38       	movsd  0x38(%rbx),%xmm0
  c2a53e:	66 0f ef c9          	pxor   %xmm1,%xmm1
  c2a542:	66 0f 2f c1          	comisd %xmm1,%xmm0
  c2a546:	76 64                	jbe    c2a5ac <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bd7c>
  c2a548:	48 8b 40 48          	mov    0x48(%rax),%rax
  c2a54c:	48 8d 15 2d 6d 0c 00 	lea    0xc6d2d(%rip),%rdx        # cf1280 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x152a50>
  c2a553:	f2 0f 11 43 40       	movsd  %xmm0,0x40(%rbx)
  c2a558:	48 39 d0             	cmp    %rdx,%rax
  c2a55b:	0f 85 df 03 00 00    	jne    c2a940 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c110>
  c2a561:	48 8b 7b 08          	mov    0x8(%rbx),%rdi
  c2a565:	48 8b 07             	mov    (%rdi),%rax
  c2a568:	ff 50 18             	call   *0x18(%rax)
  c2a56b:	f2 0f 10 43 38       	movsd  0x38(%rbx),%xmm0
  c2a570:	66 0f ef c9          	pxor   %xmm1,%xmm1
  c2a574:	48 8b 53 28          	mov    0x28(%rbx),%rdx
  c2a578:	48 89 43 28          	mov    %rax,0x28(%rbx)
  c2a57c:	66 0f 2e c1          	ucomisd %xmm1,%xmm0
  c2a580:	7a 02                	jp     c2a584 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bd54>
  c2a582:	74 19                	je     c2a59d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8bd6d>
  c2a584:	48 29 d0             	sub    %rdx,%rax
  c2a587:	66 0f ef c9          	pxor   %xmm1,%xmm1
TARGET 0xc26920
  c26920:	31 c9                	xor    %ecx,%ecx
  c26922:	ba 0c 00 00 00       	mov    $0xc,%edx
  c26927:	48 8d 35 92 5a 46 02 	lea    0x2465a92(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c2692e:	e9 8d 84 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26933:	90                   	nop
  c26934:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
  c2693b:	00 00 00 00 
  c2693f:	90                   	nop
  c26940:	55                   	push   %rbp
  c26941:	53                   	push   %rbx
  c26942:	48 89 fb             	mov    %rdi,%rbx
  c26945:	48 83 ec 18          	sub    $0x18,%rsp
  c26949:	48 8b 6f 20          	mov    0x20(%rdi),%rbp
  c2694d:	66 0f 2e 45 38       	ucomisd 0x38(%rbp),%xmm0
  c26952:	7a 0c                	jp     c26960 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88130>
  c26954:	75 0a                	jne    c26960 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88130>
  c26956:	48 83 c4 18          	add    $0x18,%rsp
  c2695a:	5b                   	pop    %rbx
  c2695b:	5d                   	pop    %rbp
  c2695c:	c3                   	ret
  c2695d:	0f 1f 00             	nopl   (%rax)
  c26960:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c26964:	48 8d 15 15 a9 0c 00 	lea    0xca915(%rip),%rdx        # cf1280 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x152a50>
  c2696b:	48 8b 40 48          	mov    0x48(%rax),%rax
  c2696f:	48 39 d0             	cmp    %rdx,%rax
  c26972:	75 6c                	jne    c269e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x881b0>
  c26974:	48 8b 7d 08          	mov    0x8(%rbp),%rdi
  c26978:	f2 0f 11 44 24 08    	movsd  %xmm0,0x8(%rsp)
  c2697e:	48 8b 07             	mov    (%rdi),%rax
  c26981:	ff 50 18             	call   *0x18(%rax)
  c26984:	f2 0f 10 4d 38       	movsd  0x38(%rbp),%xmm1
  c26989:	66 0f ef d2          	pxor   %xmm2,%xmm2
  c2698d:	48 8b 55 28          	mov    0x28(%rbp),%rdx
  c26991:	f2 0f 10 44 24 08    	movsd  0x8(%rsp),%xmm0
  c26997:	48 89 45 28          	mov    %rax,0x28(%rbp)
  c2699b:	66 0f 2e ca          	ucomisd %xmm2,%xmm1
  c2699f:	7a 02                	jp     c269a3 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88173>
  c269a1:	74 19                	je     c269bc <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8818c>
  c269a3:	48 29 d0             	sub    %rdx,%rax
  c269a6:	66 0f ef d2          	pxor   %xmm2,%xmm2
  c269aa:	f2 48 0f 2a d0       	cvtsi2sd %rax,%xmm2
  c269af:	f2 0f 59 d1          	mulsd  %xmm1,%xmm2
  c269b3:	f2 48 0f 2c c2       	cvttsd2si %xmm2,%rax
  c269b8:	48 01 45 30          	add    %rax,0x30(%rbp)
  c269bc:	f2 0f 11 45 38       	movsd  %xmm0,0x38(%rbp)
  c269c1:	48 83 c4 18          	add    $0x18,%rsp
  c269c5:	48 89 df             	mov    %rbx,%rdi
  c269c8:	31 c9                	xor    %ecx,%ecx
  c269ca:	ba 0d 00 00 00       	mov    $0xd,%edx
  c269cf:	5b                   	pop    %rbx
  c269d0:	48 8d 35 e9 59 46 02 	lea    0x24659e9(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c269d7:	5d                   	pop    %rbp
  c269d8:	e9 e3 83 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c269dd:	0f 1f 00             	nopl   (%rax)
  c269e0:	48 89 ef             	mov    %rbp,%rdi
  c269e3:	ff d0                	call   *%rax
  c269e5:	eb da                	jmp    c269c1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88191>
  c269e7:	90                   	nop
  c269e8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  c269ef:	00 
  c269f0:	41 54                	push   %r12
  c269f2:	55                   	push   %rbp
  c269f3:	53                   	push   %rbx
  c269f4:	48 89 fb             	mov    %rdi,%rbx
  c269f7:	48 83 ec 10          	sub    $0x10,%rsp
  c269fb:	48 8b bf a8 00 00 00 	mov    0xa8(%rdi),%rdi
  c26a02:	48 85 ff             	test   %rdi,%rdi
  c26a05:	0f 84 15 01 00 00    	je     c26b20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x882f0>
  c26a0b:	80 7f 08 00          	cmpb   $0x0,0x8(%rdi)
  c26a0f:	4c 8d 25 aa 59 46 02 	lea    0x24659aa(%rip),%r12        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c26a16:	75 60                	jne    c26a78 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88248>
  c26a18:	48 8b 03             	mov    (%rbx),%rax
  c26a1b:	48 8d 15 1e ff ff ff 	lea    -0xe2(%rip),%rdx        # c26940 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88110>
  c26a22:	f2 0f 10 47 10       	movsd  0x10(%rdi),%xmm0
  c26a27:	48 8b 80 d0 00 00 00 	mov    0xd0(%rax),%rax
  c26a2e:	48 39 d0             	cmp    %rdx,%rax
  c26a31:	0f 85 f9 00 00 00    	jne    c26b30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88300>
  c26a37:	48 8b 6b 20          	mov    0x20(%rbx),%rbp
  c26a3b:	66 0f 2e 45 38       	ucomisd 0x38(%rbp),%xmm0
  c26a40:	7a 5e                	jp     c26aa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88270>
  c26a42:	75 5c                	jne    c26aa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88270>
  c26a44:	48 c7 83 a8 00 00 00 	movq   $0x0,0xa8(%rbx)
  c26a4b:	00 00 00 00 
  c26a4f:	be 18 00 00 00       	mov    $0x18,%esi
  c26a54:	e8 a7 7e 8b ff       	call   4de900 <operator delete(void*, unsigned long)@plt>
  c26a59:	48 83 c4 10          	add    $0x10,%rsp
  c26a5d:	4c 89 e6             	mov    %r12,%rsi
  c26a60:	48 89 df             	mov    %rbx,%rdi
  c26a63:	31 c9                	xor    %ecx,%ecx
  c26a65:	5b                   	pop    %rbx
TARGET 0xc2e3c0
  c2e3c0:	41 57                	push   %r15
  c2e3c2:	41 56                	push   %r14
  c2e3c4:	41 55                	push   %r13
  c2e3c6:	41 54                	push   %r12
  c2e3c8:	49 89 f4             	mov    %rsi,%r12
  c2e3cb:	55                   	push   %rbp
  c2e3cc:	48 89 fd             	mov    %rdi,%rbp
  c2e3cf:	53                   	push   %rbx
  c2e3d0:	48 83 ec 58          	sub    $0x58,%rsp
  c2e3d4:	0f b6 05 95 79 56 02 	movzbl 0x2567995(%rip),%eax        # 3195d70 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3908>
  c2e3db:	84 c0                	test   %al,%al
  c2e3dd:	0f 84 e5 00 00 00    	je     c2e4c8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc98>
  c2e3e3:	4d 8b 04 24          	mov    (%r12),%r8
  c2e3e7:	48 8b 05 62 79 56 02 	mov    0x2567962(%rip),%rax        # 3195d50 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38e8>
  c2e3ee:	41 8b 70 38          	mov    0x38(%r8),%esi
  c2e3f2:	48 85 c0             	test   %rax,%rax
  c2e3f5:	74 53                	je     c2e44a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc1a>
  c2e3f7:	4c 8d 2d 4a 79 56 02 	lea    0x256794a(%rip),%r13        # 3195d48 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38e0>
  c2e3fe:	4c 89 ef             	mov    %r13,%rdi
  c2e401:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c2e408:	48 8b 50 10          	mov    0x10(%rax),%rdx
  c2e40c:	48 8b 48 18          	mov    0x18(%rax),%rcx
  c2e410:	3b 70 20             	cmp    0x20(%rax),%esi
  c2e413:	7f 18                	jg     c2e42d <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fbfd>
  c2e415:	48 89 c7             	mov    %rax,%rdi
  c2e418:	48 85 d2             	test   %rdx,%rdx
  c2e41b:	74 23                	je     c2e440 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc10>
  c2e41d:	48 89 d0             	mov    %rdx,%rax
  c2e420:	48 8b 50 10          	mov    0x10(%rax),%rdx
  c2e424:	48 8b 48 18          	mov    0x18(%rax),%rcx
  c2e428:	3b 70 20             	cmp    0x20(%rax),%esi
  c2e42b:	7e e8                	jle    c2e415 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fbe5>
  c2e42d:	48 85 c9             	test   %rcx,%rcx
  c2e430:	74 0e                	je     c2e440 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc10>
  c2e432:	48 89 c8             	mov    %rcx,%rax
  c2e435:	eb d1                	jmp    c2e408 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fbd8>
  c2e437:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2e43e:	00 00 
  c2e440:	4c 39 ef             	cmp    %r13,%rdi
  c2e443:	74 05                	je     c2e44a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc1a>
  c2e445:	39 77 20             	cmp    %esi,0x20(%rdi)
  c2e448:	7e 46                	jle    c2e490 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc60>
  c2e44a:	81 fe b4 00 00 00    	cmp    $0xb4,%esi
  c2e450:	0f 84 ba 01 00 00    	je     c2e610 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fde0>
  c2e456:	48 89 e1             	mov    %rsp,%rcx
  c2e459:	ba 02 00 00 00       	mov    $0x2,%edx
  c2e45e:	48 89 ef             	mov    %rbp,%rdi
  c2e461:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
  c2e466:	48 8d 35 53 df 45 02 	lea    0x245df53(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c2e46d:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
  c2e474:	00 
  c2e475:	e8 46 09 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c2e47a:	48 83 c4 58          	add    $0x58,%rsp
  c2e47e:	5b                   	pop    %rbx
  c2e47f:	5d                   	pop    %rbp
  c2e480:	41 5c                	pop    %r12
  c2e482:	41 5d                	pop    %r13
  c2e484:	41 5e                	pop    %r14
  c2e486:	41 5f                	pop    %r15
  c2e488:	c3                   	ret
  c2e489:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c2e490:	48 8b 85 a8 00 00 00 	mov    0xa8(%rbp),%rax
  c2e497:	48 85 c0             	test   %rax,%rax
  c2e49a:	74 06                	je     c2e4a2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc72>
  c2e49c:	80 78 09 00          	cmpb   $0x0,0x9(%rax)
  c2e4a0:	75 d8                	jne    c2e47a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc4a>
  c2e4a2:	48 8b 45 20          	mov    0x20(%rbp),%rax
  c2e4a6:	f2 0f 10 40 38       	movsd  0x38(%rax),%xmm0
  c2e4ab:	66 0f 2f 05 35 58 13 	comisd 0x1135835(%rip),%xmm0        # 1d63ce8 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x7a728>
  c2e4b2:	01 
  c2e4b3:	76 95                	jbe    c2e44a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fc1a>
  c2e4b5:	48 83 c4 58          	add    $0x58,%rsp
  c2e4b9:	5b                   	pop    %rbx
  c2e4ba:	5d                   	pop    %rbp
  c2e4bb:	41 5c                	pop    %r12
  c2e4bd:	41 5d                	pop    %r13
  c2e4bf:	41 5e                	pop    %r14
  c2e4c1:	41 5f                	pop    %r15
  c2e4c3:	c3                   	ret
  c2e4c4:	0f 1f 40 00          	nopl   0x0(%rax)
  c2e4c8:	48 8d 3d a1 78 56 02 	lea    0x25678a1(%rip),%rdi        # 3195d70 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3908>
  c2e4cf:	e8 fc c9 8a ff       	call   4daed0 <__cxa_guard_acquire@plt>
  c2e4d4:	85 c0                	test   %eax,%eax
  c2e4d6:	0f 84 07 ff ff ff    	je     c2e3e3 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8fbb3>
  c2e4dc:	48 8b 05 fd 57 13 01 	mov    0x11357fd(%rip),%rax        # 1d63ce0 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x7a720>
  c2e4e3:	45 31 ff             	xor    %r15d,%r15d
  c2e4e6:	4c 8d 2d 5b 78 56 02 	lea    0x256785b(%rip),%r13        # 3195d48 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38e0>
  c2e4ed:	c7 05 51 78 56 02 00 	movl   $0x0,0x2567851(%rip)        # 3195d48 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38e0>
  c2e4f4:	00 00 00 
  c2e4f7:	48 c7 05 4e 78 56 02 	movq   $0x0,0x256784e(%rip)        # 3195d50 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38e8>
TARGET 0xc269f0
  c269f0:	41 54                	push   %r12
  c269f2:	55                   	push   %rbp
  c269f3:	53                   	push   %rbx
  c269f4:	48 89 fb             	mov    %rdi,%rbx
  c269f7:	48 83 ec 10          	sub    $0x10,%rsp
  c269fb:	48 8b bf a8 00 00 00 	mov    0xa8(%rdi),%rdi
  c26a02:	48 85 ff             	test   %rdi,%rdi
  c26a05:	0f 84 15 01 00 00    	je     c26b20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x882f0>
  c26a0b:	80 7f 08 00          	cmpb   $0x0,0x8(%rdi)
  c26a0f:	4c 8d 25 aa 59 46 02 	lea    0x24659aa(%rip),%r12        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c26a16:	75 60                	jne    c26a78 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88248>
  c26a18:	48 8b 03             	mov    (%rbx),%rax
  c26a1b:	48 8d 15 1e ff ff ff 	lea    -0xe2(%rip),%rdx        # c26940 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88110>
  c26a22:	f2 0f 10 47 10       	movsd  0x10(%rdi),%xmm0
  c26a27:	48 8b 80 d0 00 00 00 	mov    0xd0(%rax),%rax
  c26a2e:	48 39 d0             	cmp    %rdx,%rax
  c26a31:	0f 85 f9 00 00 00    	jne    c26b30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88300>
  c26a37:	48 8b 6b 20          	mov    0x20(%rbx),%rbp
  c26a3b:	66 0f 2e 45 38       	ucomisd 0x38(%rbp),%xmm0
  c26a40:	7a 5e                	jp     c26aa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88270>
  c26a42:	75 5c                	jne    c26aa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88270>
  c26a44:	48 c7 83 a8 00 00 00 	movq   $0x0,0xa8(%rbx)
  c26a4b:	00 00 00 00 
  c26a4f:	be 18 00 00 00       	mov    $0x18,%esi
  c26a54:	e8 a7 7e 8b ff       	call   4de900 <operator delete(void*, unsigned long)@plt>
  c26a59:	48 83 c4 10          	add    $0x10,%rsp
  c26a5d:	4c 89 e6             	mov    %r12,%rsi
  c26a60:	48 89 df             	mov    %rbx,%rdi
  c26a63:	31 c9                	xor    %ecx,%ecx
  c26a65:	5b                   	pop    %rbx
  c26a66:	ba 08 00 00 00       	mov    $0x8,%edx
  c26a6b:	5d                   	pop    %rbp
  c26a6c:	41 5c                	pop    %r12
  c26a6e:	e9 4d 83 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26a73:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
  c26a78:	48 8b 7b 20          	mov    0x20(%rbx),%rdi
  c26a7c:	48 8b 07             	mov    (%rdi),%rax
  c26a7f:	ff 50 70             	call   *0x70(%rax)
  c26a82:	48 89 df             	mov    %rbx,%rdi
  c26a85:	31 c9                	xor    %ecx,%ecx
  c26a87:	ba 0a 00 00 00       	mov    $0xa,%edx
  c26a8c:	4c 89 e6             	mov    %r12,%rsi
  c26a8f:	e8 2c 83 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26a94:	48 8b bb a8 00 00 00 	mov    0xa8(%rbx),%rdi
  c26a9b:	e9 78 ff ff ff       	jmp    c26a18 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x881e8>
  c26aa0:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c26aa4:	48 8d 15 d5 a7 0c 00 	lea    0xca7d5(%rip),%rdx        # cf1280 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x152a50>
  c26aab:	48 8b 40 48          	mov    0x48(%rax),%rax
  c26aaf:	48 39 d0             	cmp    %rdx,%rax
  c26ab2:	0f 85 88 00 00 00    	jne    c26b40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88310>
  c26ab8:	48 8b 7d 08          	mov    0x8(%rbp),%rdi
  c26abc:	f2 0f 11 44 24 08    	movsd  %xmm0,0x8(%rsp)
  c26ac2:	48 8b 07             	mov    (%rdi),%rax
  c26ac5:	ff 50 18             	call   *0x18(%rax)
  c26ac8:	f2 0f 10 4d 38       	movsd  0x38(%rbp),%xmm1
  c26acd:	66 0f ef d2          	pxor   %xmm2,%xmm2
  c26ad1:	48 8b 55 28          	mov    0x28(%rbp),%rdx
  c26ad5:	f2 0f 10 44 24 08    	movsd  0x8(%rsp),%xmm0
  c26adb:	48 89 45 28          	mov    %rax,0x28(%rbp)
  c26adf:	66 0f 2e ca          	ucomisd %xmm2,%xmm1
  c26ae3:	7a 6b                	jp     c26b50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88320>
  c26ae5:	75 69                	jne    c26b50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88320>
  c26ae7:	f2 0f 11 45 38       	movsd  %xmm0,0x38(%rbp)
  c26aec:	31 c9                	xor    %ecx,%ecx
  c26aee:	ba 0d 00 00 00       	mov    $0xd,%edx
  c26af3:	4c 89 e6             	mov    %r12,%rsi
  c26af6:	48 89 df             	mov    %rbx,%rdi
  c26af9:	e8 c2 82 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26afe:	48 8b bb a8 00 00 00 	mov    0xa8(%rbx),%rdi
  c26b05:	48 c7 83 a8 00 00 00 	movq   $0x0,0xa8(%rbx)
  c26b0c:	00 00 00 00 
  c26b10:	48 85 ff             	test   %rdi,%rdi
  c26b13:	0f 84 40 ff ff ff    	je     c26a59 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88229>
  c26b19:	e9 31 ff ff ff       	jmp    c26a4f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8821f>
  c26b1e:	66 90                	xchg   %ax,%ax
  c26b20:	48 83 c4 10          	add    $0x10,%rsp
  c26b24:	5b                   	pop    %rbx
  c26b25:	5d                   	pop    %rbp
  c26b26:	41 5c                	pop    %r12
  c26b28:	c3                   	ret
  c26b29:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c26b30:	48 89 df             	mov    %rbx,%rdi
  c26b33:	ff d0                	call   *%rax
  c26b35:	eb c7                	jmp    c26afe <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x882ce>
  c26b37:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c26b3e:	00 00 
  c26b40:	48 89 ef             	mov    %rbp,%rdi
  c26b43:	ff d0                	call   *%rax
  c26b45:	eb a5                	jmp    c26aec <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x882bc>
  c26b47:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
TARGET 0xc26b70
  c26b70:	41 54                	push   %r12
  c26b72:	48 8d 0d 87 fb ff ff 	lea    -0x479(%rip),%rcx        # c26700 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87ed0>
  c26b79:	55                   	push   %rbp
  c26b7a:	53                   	push   %rbx
  c26b7b:	48 8b 07             	mov    (%rdi),%rax
  c26b7e:	48 89 fb             	mov    %rdi,%rbx
  c26b81:	48 8b 90 90 00 00 00 	mov    0x90(%rax),%rdx
  c26b88:	48 39 ca             	cmp    %rcx,%rdx
  c26b8b:	0f 85 3f 01 00 00    	jne    c26cd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x884a0>
  c26b91:	48 8b 7f 20          	mov    0x20(%rdi),%rdi
  c26b95:	48 8d 35 c4 c4 01 00 	lea    0x1c4c4(%rip),%rsi        # c43060 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xa4830>
  c26b9c:	48 8b 17             	mov    (%rdi),%rdx
  c26b9f:	48 8b 8a 80 00 00 00 	mov    0x80(%rdx),%rcx
  c26ba6:	48 39 f1             	cmp    %rsi,%rcx
  c26ba9:	0f 85 d9 00 00 00    	jne    c26c88 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88458>
  c26baf:	48 b9 ff ff ff ff ff 	movabs $0x7fffffffffffffff,%rcx
  c26bb6:	ff ff 7f 
  c26bb9:	48 39 4f 50          	cmp    %rcx,0x50(%rdi)
  c26bbd:	0f 84 d2 00 00 00    	je     c26c95 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88465>
  c26bc3:	4c 8d 25 f6 57 46 02 	lea    0x24657f6(%rip),%r12        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c26bca:	48 8b 80 d0 00 00 00 	mov    0xd0(%rax),%rax
  c26bd1:	48 8d 15 68 fd ff ff 	lea    -0x298(%rip),%rdx        # c26940 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88110>
  c26bd8:	48 39 d0             	cmp    %rdx,%rax
  c26bdb:	0f 85 df 00 00 00    	jne    c26cc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88490>
  c26be1:	48 8b 6b 20          	mov    0x20(%rbx),%rbp
  c26be5:	66 0f ef c0          	pxor   %xmm0,%xmm0
  c26be9:	66 0f 2e 45 38       	ucomisd 0x38(%rbp),%xmm0
  c26bee:	7a 20                	jp     c26c10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x883e0>
  c26bf0:	75 1e                	jne    c26c10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x883e0>
  c26bf2:	4c 89 e6             	mov    %r12,%rsi
  c26bf5:	48 89 df             	mov    %rbx,%rdi
  c26bf8:	31 c9                	xor    %ecx,%ecx
  c26bfa:	5b                   	pop    %rbx
  c26bfb:	ba 05 00 00 00       	mov    $0x5,%edx
  c26c00:	5d                   	pop    %rbp
  c26c01:	41 5c                	pop    %r12
  c26c03:	e9 b8 81 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26c08:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  c26c0f:	00 
  c26c10:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c26c14:	48 8d 15 65 a6 0c 00 	lea    0xca665(%rip),%rdx        # cf1280 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x152a50>
  c26c1b:	48 8b 40 48          	mov    0x48(%rax),%rax
  c26c1f:	48 39 d0             	cmp    %rdx,%rax
  c26c22:	0f 85 b8 00 00 00    	jne    c26ce0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x884b0>
  c26c28:	48 8b 7d 08          	mov    0x8(%rbp),%rdi
  c26c2c:	48 8b 07             	mov    (%rdi),%rax
  c26c2f:	ff 50 18             	call   *0x18(%rax)
  c26c32:	f2 0f 10 45 38       	movsd  0x38(%rbp),%xmm0
  c26c37:	66 0f ef c9          	pxor   %xmm1,%xmm1
  c26c3b:	48 8b 55 28          	mov    0x28(%rbp),%rdx
  c26c3f:	48 89 45 28          	mov    %rax,0x28(%rbp)
  c26c43:	66 0f 2e c1          	ucomisd %xmm1,%xmm0
  c26c47:	7a 02                	jp     c26c4b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8841b>
  c26c49:	74 19                	je     c26c64 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88434>
  c26c4b:	48 29 d0             	sub    %rdx,%rax
  c26c4e:	66 0f ef c9          	pxor   %xmm1,%xmm1
  c26c52:	f2 48 0f 2a c8       	cvtsi2sd %rax,%xmm1
  c26c57:	f2 0f 59 c8          	mulsd  %xmm0,%xmm1
  c26c5b:	f2 48 0f 2c c1       	cvttsd2si %xmm1,%rax
  c26c60:	48 01 45 30          	add    %rax,0x30(%rbp)
  c26c64:	48 c7 45 38 00 00 00 	movq   $0x0,0x38(%rbp)
  c26c6b:	00 
  c26c6c:	31 c9                	xor    %ecx,%ecx
  c26c6e:	ba 0d 00 00 00       	mov    $0xd,%edx
  c26c73:	4c 89 e6             	mov    %r12,%rsi
  c26c76:	48 89 df             	mov    %rbx,%rdi
  c26c79:	e8 42 81 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26c7e:	e9 6f ff ff ff       	jmp    c26bf2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x883c2>
  c26c83:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
  c26c88:	ff d1                	call   *%rcx
  c26c8a:	84 c0                	test   %al,%al
  c26c8c:	74 44                	je     c26cd2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x884a2>
  c26c8e:	48 8b 7b 20          	mov    0x20(%rbx),%rdi
  c26c92:	48 8b 17             	mov    (%rdi),%rdx
  c26c95:	ff 52 70             	call   *0x70(%rdx)
  c26c98:	4c 8d 25 21 57 46 02 	lea    0x2465721(%rip),%r12        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c26c9f:	31 c9                	xor    %ecx,%ecx
  c26ca1:	48 89 df             	mov    %rbx,%rdi
  c26ca4:	ba 0a 00 00 00       	mov    $0xa,%edx
  c26ca9:	4c 89 e6             	mov    %r12,%rsi
  c26cac:	e8 0f 81 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c26cb1:	48 8b 03             	mov    (%rbx),%rax
  c26cb4:	e9 11 ff ff ff       	jmp    c26bca <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8839a>
  c26cb9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c26cc0:	66 0f ef c0          	pxor   %xmm0,%xmm0
  c26cc4:	48 89 df             	mov    %rbx,%rdi
  c26cc7:	ff d0                	call   *%rax
  c26cc9:	e9 24 ff ff ff       	jmp    c26bf2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x883c2>
  c26cce:	66 90                	xchg   %ax,%ax
  c26cd0:	ff d2                	call   *%rdx
TARGET 0xc2a960
  c2a960:	55                   	push   %rbp
  c2a961:	53                   	push   %rbx
  c2a962:	48 89 fb             	mov    %rdi,%rbx
  c2a965:	48 83 ec 08          	sub    $0x8,%rsp
  c2a969:	0f b6 05 c0 b3 56 02 	movzbl 0x256b3c0(%rip),%eax        # 3195d30 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38c8>
  c2a970:	84 c0                	test   %al,%al
  c2a972:	74 3c                	je     c2a9b0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c180>
  c2a974:	48 8b 7b 10          	mov    0x10(%rbx),%rdi
  c2a978:	48 85 ff             	test   %rdi,%rdi
  c2a97b:	0f 84 06 01 00 00    	je     c2aa87 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c257>
  c2a981:	48 8b 07             	mov    (%rdi),%rax
  c2a984:	48 8d 15 f5 6c ff ff 	lea    -0x930b(%rip),%rdx        # c21680 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x82e50>
  c2a98b:	48 8b 40 78          	mov    0x78(%rax),%rax
  c2a98f:	48 39 d0             	cmp    %rdx,%rax
  c2a992:	0f 85 00 01 00 00    	jne    c2aa98 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c268>
  c2a998:	48 83 c4 08          	add    $0x8,%rsp
  c2a99c:	48 8d 87 98 02 00 00 	lea    0x298(%rdi),%rax
  c2a9a3:	5b                   	pop    %rbx
  c2a9a4:	5d                   	pop    %rbp
  c2a9a5:	c3                   	ret
  c2a9a6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2a9ad:	00 00 00 
  c2a9b0:	48 8d 2d 79 b3 56 02 	lea    0x256b379(%rip),%rbp        # 3195d30 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38c8>
  c2a9b7:	48 89 ef             	mov    %rbp,%rdi
  c2a9ba:	e8 11 05 8b ff       	call   4daed0 <__cxa_guard_acquire@plt>
  c2a9bf:	85 c0                	test   %eax,%eax
  c2a9c1:	74 b1                	je     c2a974 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c144>
  c2a9c3:	31 f6                	xor    %esi,%esi
  c2a9c5:	48 8d 15 98 c0 47 01 	lea    0x147c098(%rip),%rdx        # 20a6a64 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x5304>
  c2a9cc:	48 8d 3d dd b2 56 02 	lea    0x256b2dd(%rip),%rdi        # 3195cb0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3848>
  c2a9d3:	c6 05 c6 b2 56 02 00 	movb   $0x0,0x256b2c6(%rip)        # 3195ca0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3838>
  c2a9da:	c7 05 be b2 56 02 00 	movl   $0x0,0x256b2be(%rip)        # 3195ca2 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x383a>
  c2a9e1:	00 00 00 
  c2a9e4:	48 c7 05 b9 b2 56 02 	movq   $0x0,0x256b2b9(%rip)        # 3195ca8 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3840>
  c2a9eb:	00 00 00 00 
  c2a9ef:	e8 1c 48 8b ff       	call   4df210 <QString::fromUtf8(QByteArrayView)@plt>
  c2a9f4:	48 8d 3d cd b2 56 02 	lea    0x256b2cd(%rip),%rdi        # 3195cc8 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3860>
  c2a9fb:	e8 50 3a 8b ff       	call   4de450 <QDateTime::QDateTime()@plt>
  c2aa00:	66 0f ef c0          	pxor   %xmm0,%xmm0
  c2aa04:	48 8d 15 1d 87 50 02 	lea    0x250871d(%rip),%rdx        # 3133128 <typeinfo for QSGRectangleNode@@Base+0xaca0>
  c2aa0b:	48 8d 35 8e b2 56 02 	lea    0x256b28e(%rip),%rsi        # 3195ca0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3838>
  c2aa12:	48 8d 3d 57 6b bb ff 	lea    -0x4494a9(%rip),%rdi        # 7e1570 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, unsigned long, std::allocator<char> const&)@@Base+0xafc50>
  c2aa19:	0f 29 05 b0 b2 56 02 	movaps %xmm0,0x256b2b0(%rip)        # 3195cd0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3868>
  c2aa20:	0f 29 05 c9 b2 56 02 	movaps %xmm0,0x256b2c9(%rip)        # 3195cf0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3888>
  c2aa27:	0f 29 05 e2 b2 56 02 	movaps %xmm0,0x256b2e2(%rip)        # 3195d10 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38a8>
  c2aa2e:	48 c7 05 a7 b2 56 02 	movq   $0x0,0x256b2a7(%rip)        # 3195ce0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3878>
  c2aa35:	00 00 00 00 
  c2aa39:	c7 05 a5 b2 56 02 00 	movl   $0x0,0x256b2a5(%rip)        # 3195ce8 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3880>
  c2aa40:	00 00 00 
  c2aa43:	48 c7 05 b2 b2 56 02 	movq   $0x0,0x256b2b2(%rip)        # 3195d00 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3898>
  c2aa4a:	00 00 00 00 
  c2aa4e:	c7 05 b0 b2 56 02 00 	movl   $0x0,0x256b2b0(%rip)        # 3195d08 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38a0>
  c2aa55:	00 00 00 
  c2aa58:	48 c7 05 bd b2 56 02 	movq   $0x0,0x256b2bd(%rip)        # 3195d20 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38b8>
  c2aa5f:	00 00 00 00 
  c2aa63:	c7 05 bb b2 56 02 00 	movl   $0x0,0x256b2bb(%rip)        # 3195d28 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x38c0>
  c2aa6a:	00 00 00 
  c2aa6d:	e8 ce fc 8a ff       	call   4da740 <__cxa_atexit@plt>
  c2aa72:	48 89 ef             	mov    %rbp,%rdi
  c2aa75:	e8 36 4f 8b ff       	call   4df9b0 <__cxa_guard_release@plt>
  c2aa7a:	48 8b 7b 10          	mov    0x10(%rbx),%rdi
  c2aa7e:	48 85 ff             	test   %rdi,%rdi
  c2aa81:	0f 85 fa fe ff ff    	jne    c2a981 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8c151>
  c2aa87:	48 83 c4 08          	add    $0x8,%rsp
  c2aa8b:	48 8d 05 0e b2 56 02 	lea    0x256b20e(%rip),%rax        # 3195ca0 <guard variable for QMetaType::registerMutableViewImpl<QList<QObject*>, QIterable<QMetaSequence> >(std::function<bool (void*, void*)>, QMetaType, QMetaType)::unregister@@Base+0x3838>
  c2aa92:	5b                   	pop    %rbx
  c2aa93:	5d                   	pop    %rbp
  c2aa94:	c3                   	ret
  c2aa95:	0f 1f 00             	nopl   (%rax)
  c2aa98:	48 83 c4 08          	add    $0x8,%rsp
  c2aa9c:	5b                   	pop    %rbx
  c2aa9d:	5d                   	pop    %rbp
  c2aa9e:	ff e0                	jmp    *%rax
  c2aaa0:	48 89 c3             	mov    %rax,%rbx
  c2aaa3:	e9 38 0f 91 ff       	jmp    53b9e0 <std::runtime_error::~runtime_error()@plt+0x5b710>
  c2aaa8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
  c2aaaf:	00 
  c2aab0:	41 55                	push   %r13
  c2aab2:	48 8d 15 37 7e 08 00 	lea    0x87e37(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c2aab9:	41 54                	push   %r12
  c2aabb:	49 89 f4             	mov    %rsi,%r12
  c2aabe:	55                   	push   %rbp
  c2aabf:	48 89 fd             	mov    %rdi,%rbp
  c2aac2:	53                   	push   %rbx
  c2aac3:	48 83 ec 28          	sub    $0x28,%rsp
  c2aac7:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c2aacb:	8b 76 30             	mov    0x30(%rsi),%esi
  c2aace:	48 8b 07             	mov    (%rdi),%rax
  c2aad1:	48 8b 40 28          	mov    0x28(%rax),%rax
  c2aad5:	48 39 d0             	cmp    %rdx,%rax
TARGET 0xc21860
  c21860:	48 83 bf a8 00 00 00 	cmpq   $0x0,0xa8(%rdi)
  c21867:	00 
  c21868:	0f 95 c0             	setne  %al
  c2186b:	c3                   	ret
  c2186c:	0f 1f 40 00          	nopl   0x0(%rax)
  c21870:	41 54                	push   %r12
  c21872:	4c 8d 25 77 10 09 00 	lea    0x91077(%rip),%r12        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c21879:	55                   	push   %rbp
  c2187a:	48 89 fd             	mov    %rdi,%rbp
  c2187d:	53                   	push   %rbx
  c2187e:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21882:	48 89 f3             	mov    %rsi,%rbx
  c21885:	8b 76 18             	mov    0x18(%rsi),%esi
  c21888:	48 8b 07             	mov    (%rdi),%rax
  c2188b:	48 8b 40 28          	mov    0x28(%rax),%rax
  c2188f:	4c 39 e0             	cmp    %r12,%rax
  c21892:	75 34                	jne    c218c8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83098>
  c21894:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21898:	40 0f be f6          	movsbl %sil,%esi
  c2189c:	e8 1f c7 8b ff       	call   4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c218a1:	48 8b 7d 18          	mov    0x18(%rbp),%rdi
  c218a5:	8b 73 1c             	mov    0x1c(%rbx),%esi
  c218a8:	48 8b 07             	mov    (%rdi),%rax
  c218ab:	48 8b 40 28          	mov    0x28(%rax),%rax
  c218af:	4c 39 e0             	cmp    %r12,%rax
  c218b2:	75 1c                	jne    c218d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x830a0>
  c218b4:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218b8:	5b                   	pop    %rbx
  c218b9:	40 0f be f6          	movsbl %sil,%esi
  c218bd:	5d                   	pop    %rbp
  c218be:	41 5c                	pop    %r12
  c218c0:	e9 fb c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c218c5:	0f 1f 00             	nopl   (%rax)
  c218c8:	40 0f b6 f6          	movzbl %sil,%esi
  c218cc:	ff d0                	call   *%rax
  c218ce:	eb d1                	jmp    c218a1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83071>
  c218d0:	5b                   	pop    %rbx
  c218d1:	40 0f b6 f6          	movzbl %sil,%esi
  c218d5:	5d                   	pop    %rbp
  c218d6:	41 5c                	pop    %r12
  c218d8:	ff e0                	jmp    *%rax
  c218da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c218e0:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218e4:	48 8d 15 05 10 09 00 	lea    0x91005(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c218eb:	8b 76 18             	mov    0x18(%rsi),%esi
  c218ee:	48 8b 07             	mov    (%rdi),%rax
  c218f1:	48 8b 40 28          	mov    0x28(%rax),%rax
  c218f5:	48 39 d0             	cmp    %rdx,%rax
  c218f8:	75 16                	jne    c21910 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x830e0>
  c218fa:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218fe:	40 0f be f6          	movsbl %sil,%esi
  c21902:	e9 b9 c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c21907:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2190e:	00 00 
  c21910:	40 0f b6 f6          	movzbl %sil,%esi
  c21914:	ff e0                	jmp    *%rax
  c21916:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2191d:	00 00 00 
  c21920:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21924:	48 8d 15 c5 0f 09 00 	lea    0x90fc5(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c2192b:	8b 76 18             	mov    0x18(%rsi),%esi
  c2192e:	48 8b 07             	mov    (%rdi),%rax
  c21931:	48 8b 40 28          	mov    0x28(%rax),%rax
  c21935:	48 39 d0             	cmp    %rdx,%rax
  c21938:	75 16                	jne    c21950 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83120>
  c2193a:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c2193e:	40 0f be f6          	movsbl %sil,%esi
  c21942:	e9 79 c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c21947:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2194e:	00 00 
  c21950:	40 0f b6 f6          	movzbl %sil,%esi
  c21954:	ff e0                	jmp    *%rax
  c21956:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2195d:	00 00 00 
  c21960:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21964:	48 8d 15 85 0f 09 00 	lea    0x90f85(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c2196b:	8b 76 18             	mov    0x18(%rsi),%esi
  c2196e:	48 8b 07             	mov    (%rdi),%rax
  c21971:	48 8b 40 28          	mov    0x28(%rax),%rax
  c21975:	48 39 d0             	cmp    %rdx,%rax
  c21978:	75 16                	jne    c21990 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83160>
  c2197a:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c2197e:	40 0f be f6          	movsbl %sil,%esi
  c21982:	e9 39 c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c21987:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2198e:	00 00 
  c21990:	40 0f b6 f6          	movzbl %sil,%esi
  c21994:	ff e0                	jmp    *%rax
  c21996:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2199d:	00 00 00 
TARGET 0xc21830
  c21830:	48 8b 87 80 00 00 00 	mov    0x80(%rdi),%rax
  c21837:	66 0f ef c0          	pxor   %xmm0,%xmm0
  c2183b:	48 85 c0             	test   %rax,%rax
  c2183e:	74 1a                	je     c2185a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x8302a>
  c21840:	66 0f ef c0          	pxor   %xmm0,%xmm0
  c21844:	66 0f ef c9          	pxor   %xmm1,%xmm1
  c21848:	f2 48 0f 2a 87 88 00 	cvtsi2sdq 0x88(%rdi),%xmm0
  c2184f:	00 00 
  c21851:	f2 48 0f 2a c8       	cvtsi2sd %rax,%xmm1
  c21856:	f2 0f 5e c1          	divsd  %xmm1,%xmm0
  c2185a:	c3                   	ret
  c2185b:	90                   	nop
  c2185c:	0f 1f 40 00          	nopl   0x0(%rax)
  c21860:	48 83 bf a8 00 00 00 	cmpq   $0x0,0xa8(%rdi)
  c21867:	00 
  c21868:	0f 95 c0             	setne  %al
  c2186b:	c3                   	ret
  c2186c:	0f 1f 40 00          	nopl   0x0(%rax)
  c21870:	41 54                	push   %r12
  c21872:	4c 8d 25 77 10 09 00 	lea    0x91077(%rip),%r12        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c21879:	55                   	push   %rbp
  c2187a:	48 89 fd             	mov    %rdi,%rbp
  c2187d:	53                   	push   %rbx
  c2187e:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21882:	48 89 f3             	mov    %rsi,%rbx
  c21885:	8b 76 18             	mov    0x18(%rsi),%esi
  c21888:	48 8b 07             	mov    (%rdi),%rax
  c2188b:	48 8b 40 28          	mov    0x28(%rax),%rax
  c2188f:	4c 39 e0             	cmp    %r12,%rax
  c21892:	75 34                	jne    c218c8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83098>
  c21894:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21898:	40 0f be f6          	movsbl %sil,%esi
  c2189c:	e8 1f c7 8b ff       	call   4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c218a1:	48 8b 7d 18          	mov    0x18(%rbp),%rdi
  c218a5:	8b 73 1c             	mov    0x1c(%rbx),%esi
  c218a8:	48 8b 07             	mov    (%rdi),%rax
  c218ab:	48 8b 40 28          	mov    0x28(%rax),%rax
  c218af:	4c 39 e0             	cmp    %r12,%rax
  c218b2:	75 1c                	jne    c218d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x830a0>
  c218b4:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218b8:	5b                   	pop    %rbx
  c218b9:	40 0f be f6          	movsbl %sil,%esi
  c218bd:	5d                   	pop    %rbp
  c218be:	41 5c                	pop    %r12
  c218c0:	e9 fb c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c218c5:	0f 1f 00             	nopl   (%rax)
  c218c8:	40 0f b6 f6          	movzbl %sil,%esi
  c218cc:	ff d0                	call   *%rax
  c218ce:	eb d1                	jmp    c218a1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83071>
  c218d0:	5b                   	pop    %rbx
  c218d1:	40 0f b6 f6          	movzbl %sil,%esi
  c218d5:	5d                   	pop    %rbp
  c218d6:	41 5c                	pop    %r12
  c218d8:	ff e0                	jmp    *%rax
  c218da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c218e0:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218e4:	48 8d 15 05 10 09 00 	lea    0x91005(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c218eb:	8b 76 18             	mov    0x18(%rsi),%esi
  c218ee:	48 8b 07             	mov    (%rdi),%rax
  c218f1:	48 8b 40 28          	mov    0x28(%rax),%rax
  c218f5:	48 39 d0             	cmp    %rdx,%rax
  c218f8:	75 16                	jne    c21910 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x830e0>
  c218fa:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c218fe:	40 0f be f6          	movsbl %sil,%esi
  c21902:	e9 b9 c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c21907:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2190e:	00 00 
  c21910:	40 0f b6 f6          	movzbl %sil,%esi
  c21914:	ff e0                	jmp    *%rax
  c21916:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2191d:	00 00 00 
  c21920:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21924:	48 8d 15 c5 0f 09 00 	lea    0x90fc5(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c2192b:	8b 76 18             	mov    0x18(%rsi),%esi
  c2192e:	48 8b 07             	mov    (%rdi),%rax
  c21931:	48 8b 40 28          	mov    0x28(%rax),%rax
  c21935:	48 39 d0             	cmp    %rdx,%rax
  c21938:	75 16                	jne    c21950 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x83120>
  c2193a:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c2193e:	40 0f be f6          	movsbl %sil,%esi
  c21942:	e9 79 c6 8b ff       	jmp    4ddfc0 <QDataStream::operator<<(signed char)@plt>
  c21947:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2194e:	00 00 
  c21950:	40 0f b6 f6          	movzbl %sil,%esi
  c21954:	ff e0                	jmp    *%rax
  c21956:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2195d:	00 00 00 
  c21960:	48 8b 7f 18          	mov    0x18(%rdi),%rdi
  c21964:	48 8d 15 85 0f 09 00 	lea    0x90f85(%rip),%rdx        # cb28f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1140c0>
  c2196b:	8b 76 18             	mov    0x18(%rsi),%esi
TARGET 0xc273c0
  c273c0:	48 8b 97 88 00 00 00 	mov    0x88(%rdi),%rdx
  c273c7:	48 8b 06             	mov    (%rsi),%rax
  c273ca:	48 39 d0             	cmp    %rdx,%rax
  c273cd:	74 71                	je     c27440 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c10>
  c273cf:	55                   	push   %rbp
  c273d0:	48 89 f5             	mov    %rsi,%rbp
  c273d3:	48 8d 35 b6 ff ff ff 	lea    -0x4a(%rip),%rsi        # c27390 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88b60>
  c273da:	53                   	push   %rbx
  c273db:	48 89 fb             	mov    %rdi,%rbx
  c273de:	48 83 ec 08          	sub    $0x8,%rsp
  c273e2:	48 8b 0f             	mov    (%rdi),%rcx
  c273e5:	48 8b 89 08 01 00 00 	mov    0x108(%rcx),%rcx
  c273ec:	48 39 f1             	cmp    %rsi,%rcx
  c273ef:	0f 85 83 00 00 00    	jne    c27478 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c48>
  c273f5:	48 3b 87 80 00 00 00 	cmp    0x80(%rdi),%rax
  c273fc:	7f 4a                	jg     c27448 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c18>
  c273fe:	48 39 c2             	cmp    %rax,%rdx
  c27401:	7c 0d                	jl     c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27403:	48 83 c4 08          	add    $0x8,%rsp
  c27407:	5b                   	pop    %rbx
  c27408:	5d                   	pop    %rbp
  c27409:	c3                   	ret
  c2740a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c27410:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27414:	48 89 df             	mov    %rbx,%rdi
  c27417:	31 c9                	xor    %ecx,%ecx
  c27419:	ba 06 00 00 00       	mov    $0x6,%edx
  c2741e:	48 8d 35 9b 4f 46 02 	lea    0x2464f9b(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c27425:	48 89 83 88 00 00 00 	mov    %rax,0x88(%rbx)
  c2742c:	48 83 c4 08          	add    $0x8,%rsp
  c27430:	5b                   	pop    %rbx
  c27431:	5d                   	pop    %rbp
  c27432:	e9 89 79 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c27437:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2743e:	00 00 
  c27440:	c3                   	ret
  c27441:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c27448:	48 89 87 80 00 00 00 	mov    %rax,0x80(%rdi)
  c2744f:	ba 06 00 00 00       	mov    $0x6,%edx
  c27454:	31 c9                	xor    %ecx,%ecx
  c27456:	48 8d 35 63 4f 46 02 	lea    0x2464f63(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c2745d:	e8 5e 79 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c27462:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27466:	48 8b 93 88 00 00 00 	mov    0x88(%rbx),%rdx
  c2746d:	48 39 c2             	cmp    %rax,%rdx
  c27470:	7d 91                	jge    c27403 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88bd3>
  c27472:	eb 9c                	jmp    c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27474:	0f 1f 40 00          	nopl   0x0(%rax)
  c27478:	48 89 ee             	mov    %rbp,%rsi
  c2747b:	ff d1                	call   *%rcx
  c2747d:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27481:	48 8b 93 88 00 00 00 	mov    0x88(%rbx),%rdx
  c27488:	48 39 c2             	cmp    %rax,%rdx
  c2748b:	0f 8d 72 ff ff ff    	jge    c27403 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88bd3>
  c27491:	e9 7a ff ff ff       	jmp    c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27496:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2749d:	00 00 00 
  c274a0:	41 57                	push   %r15
  c274a2:	41 56                	push   %r14
  c274a4:	49 89 d6             	mov    %rdx,%r14
  c274a7:	41 55                	push   %r13
  c274a9:	41 54                	push   %r12
  c274ab:	49 89 f4             	mov    %rsi,%r12
  c274ae:	55                   	push   %rbp
  c274af:	89 cd                	mov    %ecx,%ebp
  c274b1:	53                   	push   %rbx
  c274b2:	48 89 fb             	mov    %rdi,%rbx
  c274b5:	48 81 ec 98 00 00 00 	sub    $0x98,%rsp
  c274bc:	48 8b bf 10 02 00 00 	mov    0x210(%rdi),%rdi
  c274c3:	48 8b 07             	mov    (%rdi),%rax
  c274c6:	ff 50 10             	call   *0x10(%rax)
  c274c9:	48 8b bb 40 02 00 00 	mov    0x240(%rbx),%rdi
  c274d0:	49 89 c5             	mov    %rax,%r13
  c274d3:	b8 01 00 00 00       	mov    $0x1,%eax
  c274d8:	48 85 ff             	test   %rdi,%rdi
  c274db:	0f 84 61 01 00 00    	je     c27642 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88e12>
  c274e1:	4c 8d 3d 18 19 32 00 	lea    0x321918(%rip),%r15        # f48e00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3aa5d0>
  c274e8:	e9 ee 00 00 00       	jmp    c275db <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88dab>
  c274ed:	0f 1f 00             	nopl   (%rax)
  c274f0:	48 8b 76 48          	mov    0x48(%rsi),%rsi
  c274f4:	48 8d 7c 24 30       	lea    0x30(%rsp),%rdi
  c274f9:	48 8b 06             	mov    (%rsi),%rax
  c274fc:	ff 50 28             	call   *0x28(%rax)
  c274ff:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
  c27504:	48 89 83 60 03 00 00 	mov    %rax,0x360(%rbx)
  c2750b:	49 39 04 24          	cmp    %rax,(%r12)
  c2750f:	0f 8e 8d 01 00 00    	jle    c276a2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88e72>
  c27515:	48 8b bb 40 02 00 00 	mov    0x240(%rbx),%rdi
  c2751c:	48 8d 15 7d ea 31 00 	lea    0x31ea7d(%rip),%rdx        # f45fa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3a7770>
  c27523:	48 8b 07             	mov    (%rdi),%rax
TARGET 0xc27390
  c27390:	48 8b 06             	mov    (%rsi),%rax
  c27393:	48 3b 87 80 00 00 00 	cmp    0x80(%rdi),%rax
  c2739a:	7f 04                	jg     c273a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88b70>
  c2739c:	c3                   	ret
  c2739d:	0f 1f 00             	nopl   (%rax)
  c273a0:	48 89 87 80 00 00 00 	mov    %rax,0x80(%rdi)
  c273a7:	31 c9                	xor    %ecx,%ecx
  c273a9:	ba 06 00 00 00       	mov    $0x6,%edx
  c273ae:	48 8d 35 0b 50 46 02 	lea    0x246500b(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c273b5:	e9 06 7a 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c273ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c273c0:	48 8b 97 88 00 00 00 	mov    0x88(%rdi),%rdx
  c273c7:	48 8b 06             	mov    (%rsi),%rax
  c273ca:	48 39 d0             	cmp    %rdx,%rax
  c273cd:	74 71                	je     c27440 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c10>
  c273cf:	55                   	push   %rbp
  c273d0:	48 89 f5             	mov    %rsi,%rbp
  c273d3:	48 8d 35 b6 ff ff ff 	lea    -0x4a(%rip),%rsi        # c27390 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88b60>
  c273da:	53                   	push   %rbx
  c273db:	48 89 fb             	mov    %rdi,%rbx
  c273de:	48 83 ec 08          	sub    $0x8,%rsp
  c273e2:	48 8b 0f             	mov    (%rdi),%rcx
  c273e5:	48 8b 89 08 01 00 00 	mov    0x108(%rcx),%rcx
  c273ec:	48 39 f1             	cmp    %rsi,%rcx
  c273ef:	0f 85 83 00 00 00    	jne    c27478 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c48>
  c273f5:	48 3b 87 80 00 00 00 	cmp    0x80(%rdi),%rax
  c273fc:	7f 4a                	jg     c27448 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88c18>
  c273fe:	48 39 c2             	cmp    %rax,%rdx
  c27401:	7c 0d                	jl     c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27403:	48 83 c4 08          	add    $0x8,%rsp
  c27407:	5b                   	pop    %rbx
  c27408:	5d                   	pop    %rbp
  c27409:	c3                   	ret
  c2740a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
  c27410:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27414:	48 89 df             	mov    %rbx,%rdi
  c27417:	31 c9                	xor    %ecx,%ecx
  c27419:	ba 06 00 00 00       	mov    $0x6,%edx
  c2741e:	48 8d 35 9b 4f 46 02 	lea    0x2464f9b(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c27425:	48 89 83 88 00 00 00 	mov    %rax,0x88(%rbx)
  c2742c:	48 83 c4 08          	add    $0x8,%rsp
  c27430:	5b                   	pop    %rbx
  c27431:	5d                   	pop    %rbp
  c27432:	e9 89 79 8b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c27437:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  c2743e:	00 00 
  c27440:	c3                   	ret
  c27441:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c27448:	48 89 87 80 00 00 00 	mov    %rax,0x80(%rdi)
  c2744f:	ba 06 00 00 00       	mov    $0x6,%edx
  c27454:	31 c9                	xor    %ecx,%ecx
  c27456:	48 8d 35 63 4f 46 02 	lea    0x2464f63(%rip),%rsi        # 308c3c0 <QObject::staticMetaObject@Qt_6>
  c2745d:	e8 5e 79 8b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
  c27462:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27466:	48 8b 93 88 00 00 00 	mov    0x88(%rbx),%rdx
  c2746d:	48 39 c2             	cmp    %rax,%rdx
  c27470:	7d 91                	jge    c27403 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88bd3>
  c27472:	eb 9c                	jmp    c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27474:	0f 1f 40 00          	nopl   0x0(%rax)
  c27478:	48 89 ee             	mov    %rbp,%rsi
  c2747b:	ff d1                	call   *%rcx
  c2747d:	48 8b 45 00          	mov    0x0(%rbp),%rax
  c27481:	48 8b 93 88 00 00 00 	mov    0x88(%rbx),%rdx
  c27488:	48 39 c2             	cmp    %rax,%rdx
  c2748b:	0f 8d 72 ff ff ff    	jge    c27403 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88bd3>
  c27491:	e9 7a ff ff ff       	jmp    c27410 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88be0>
  c27496:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2749d:	00 00 00 
  c274a0:	41 57                	push   %r15
  c274a2:	41 56                	push   %r14
  c274a4:	49 89 d6             	mov    %rdx,%r14
  c274a7:	41 55                	push   %r13
  c274a9:	41 54                	push   %r12
  c274ab:	49 89 f4             	mov    %rsi,%r12
  c274ae:	55                   	push   %rbp
  c274af:	89 cd                	mov    %ecx,%ebp
  c274b1:	53                   	push   %rbx
  c274b2:	48 89 fb             	mov    %rdi,%rbx
  c274b5:	48 81 ec 98 00 00 00 	sub    $0x98,%rsp
  c274bc:	48 8b bf 10 02 00 00 	mov    0x210(%rdi),%rdi
  c274c3:	48 8b 07             	mov    (%rdi),%rax
  c274c6:	ff 50 10             	call   *0x10(%rax)
  c274c9:	48 8b bb 40 02 00 00 	mov    0x240(%rbx),%rdi
  c274d0:	49 89 c5             	mov    %rax,%r13
  c274d3:	b8 01 00 00 00       	mov    $0x1,%eax
  c274d8:	48 85 ff             	test   %rdi,%rdi
  c274db:	0f 84 61 01 00 00    	je     c27642 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88e12>
  c274e1:	4c 8d 3d 18 19 32 00 	lea    0x321918(%rip),%r15        # f48e00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x3aa5d0>
  c274e8:	e9 ee 00 00 00       	jmp    c275db <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x88dab>
  c274ed:	0f 1f 00             	nopl   (%rax)
TARGET 0xc26350
  c26350:	41 57                	push   %r15
  c26352:	4c 8d 7f 58          	lea    0x58(%rdi),%r15
  c26356:	41 56                	push   %r14
  c26358:	41 55                	push   %r13
  c2635a:	49 89 f5             	mov    %rsi,%r13
  c2635d:	41 54                	push   %r12
  c2635f:	49 89 fc             	mov    %rdi,%r12
  c26362:	55                   	push   %rbp
  c26363:	89 d5                	mov    %edx,%ebp
  c26365:	53                   	push   %rbx
  c26366:	48 83 ec 18          	sub    $0x18,%rsp
  c2636a:	48 8b 47 60          	mov    0x60(%rdi),%rax
  c2636e:	48 85 c0             	test   %rax,%rax
  c26371:	0f 84 e9 00 00 00    	je     c26460 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87c30>
  c26377:	48 8b 36             	mov    (%rsi),%rsi
  c2637a:	4c 89 fb             	mov    %r15,%rbx
  c2637d:	0f 1f 00             	nopl   (%rax)
  c26380:	48 8b 50 10          	mov    0x10(%rax),%rdx
  c26384:	48 8b 48 18          	mov    0x18(%rax),%rcx
  c26388:	48 3b 70 20          	cmp    0x20(%rax),%rsi
  c2638c:	7f 19                	jg     c263a7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87b77>
  c2638e:	48 89 c3             	mov    %rax,%rbx
  c26391:	48 85 d2             	test   %rdx,%rdx
  c26394:	74 22                	je     c263b8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87b88>
  c26396:	48 89 d0             	mov    %rdx,%rax
  c26399:	48 8b 50 10          	mov    0x10(%rax),%rdx
  c2639d:	48 8b 48 18          	mov    0x18(%rax),%rcx
  c263a1:	48 3b 70 20          	cmp    0x20(%rax),%rsi
  c263a5:	7e e7                	jle    c2638e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87b5e>
  c263a7:	48 85 c9             	test   %rcx,%rcx
  c263aa:	74 0c                	je     c263b8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87b88>
  c263ac:	48 89 c8             	mov    %rcx,%rax
  c263af:	eb cf                	jmp    c26380 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87b50>
  c263b1:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
  c263b8:	49 39 df             	cmp    %rbx,%r15
  c263bb:	0f 84 9f 00 00 00    	je     c26460 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87c30>
  c263c1:	48 3b 73 20          	cmp    0x20(%rbx),%rsi
  c263c5:	7d 7d                	jge    c26444 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87c14>
  c263c7:	bf 30 00 00 00       	mov    $0x30,%edi
  c263cc:	e8 9f 92 8b ff       	call   4df670 <operator new(unsigned long)@plt>
  c263d1:	4d 8b 6d 00          	mov    0x0(%r13),%r13
  c263d5:	c7 40 28 00 00 00 00 	movl   $0x0,0x28(%rax)
  c263dc:	49 89 c6             	mov    %rax,%r14
  c263df:	4c 89 68 20          	mov    %r13,0x20(%rax)
  c263e3:	48 8b 53 20          	mov    0x20(%rbx),%rdx
  c263e7:	4c 89 e9             	mov    %r13,%rcx
  c263ea:	49 39 d5             	cmp    %rdx,%r13
  c263ed:	0f 8d 25 01 00 00    	jge    c26518 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87ce8>
  c263f3:	4d 8b 44 24 68       	mov    0x68(%r12),%r8
  c263f8:	49 39 d8             	cmp    %rbx,%r8
  c263fb:	4c 89 04 24          	mov    %r8,(%rsp)
  c263ff:	74 24                	je     c26425 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87bf5>
  c26401:	48 89 df             	mov    %rbx,%rdi
  c26404:	e8 e7 7b 8b ff       	call   4ddff0 <std::_Rb_tree_decrement(std::_Rb_tree_node_base*)@plt>
  c26409:	4c 8b 04 24          	mov    (%rsp),%r8
  c2640d:	4c 3b 68 20          	cmp    0x20(%rax),%r13
  c26411:	48 89 c6             	mov    %rax,%rsi
  c26414:	0f 8e 90 01 00 00    	jle    c265aa <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87d7a>
  c2641a:	48 83 78 18 00       	cmpq   $0x0,0x18(%rax)
  c2641f:	0f 84 bc 00 00 00    	je     c264e1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87cb1>
  c26425:	48 89 de             	mov    %rbx,%rsi
  c26428:	bf 01 00 00 00       	mov    $0x1,%edi
  c2642d:	4c 89 f3             	mov    %r14,%rbx
  c26430:	48 89 f2             	mov    %rsi,%rdx
  c26433:	4c 89 f9             	mov    %r15,%rcx
  c26436:	4c 89 f6             	mov    %r14,%rsi
  c26439:	e8 a2 75 8b ff       	call   4dd9e0 <std::_Rb_tree_insert_and_rebalance(bool, std::_Rb_tree_node_base*, std::_Rb_tree_node_base*, std::_Rb_tree_node_base&)@plt>
  c2643e:	49 83 44 24 78 01    	addq   $0x1,0x78(%r12)
  c26444:	89 6b 28             	mov    %ebp,0x28(%rbx)
  c26447:	48 83 c4 18          	add    $0x18,%rsp
  c2644b:	5b                   	pop    %rbx
  c2644c:	5d                   	pop    %rbp
  c2644d:	41 5c                	pop    %r12
  c2644f:	41 5d                	pop    %r13
  c26451:	41 5e                	pop    %r14
  c26453:	41 5f                	pop    %r15
  c26455:	c3                   	ret
  c26456:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
  c2645d:	00 00 00 
  c26460:	bf 30 00 00 00       	mov    $0x30,%edi
  c26465:	e8 06 92 8b ff       	call   4df670 <operator new(unsigned long)@plt>
  c2646a:	49 83 7c 24 78 00    	cmpq   $0x0,0x78(%r12)
  c26470:	49 89 c6             	mov    %rax,%r14
  c26473:	49 8b 45 00          	mov    0x0(%r13),%rax
  c26477:	41 c7 46 28 00 00 00 	movl   $0x0,0x28(%r14)
  c2647e:	00 
  c2647f:	49 89 46 20          	mov    %rax,0x20(%r14)
  c26483:	74 0f                	je     c26494 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x87c64>
  c26485:	49 8b 74 24 70       	mov    0x70(%r12),%rsi
  c2648a:	49 8b 46 20          	mov    0x20(%r14),%rax

METHOD_NAME_FILE_OFFSETS
handleFullMapMessage=0x1cd8c2f
handleLeftColumnMessage=0x1cd8c87
handleRightColumnMessage=0x1cd8ce8
handleTopRowMessage=0x1cd8d4c
handleBottomRowMessage=0x1cd8da1
handleTopFloorMessage=0x1cd8dff
handleBottomFloorMessage=0x1cd8e5a
handleFieldDataMessage=0x1cd8ebe
handleCreateOnMapMessage=0x1cd8f1c
handleChangeOnMapMessage=0x1cd8f80
handleDeleteOnMapMessage=0x1cd8fe4
handleAmbientLightMessage=0x1cd9048
handleTibiaTimeMessage=0x1cd90af
receivedFullMapMessage=0x1ccbab9
receivedLeftColumnMessage=0x1ccbb13
receivedRightColumnMessage=0x1ccbb76
receivedTopRowMessage=0x1ccbbdc
receivedBottomRowMessage=0x1ccbc33
receivedTopFloorMessage=0x1ccbe55
receivedBottomFloorMessage=0x1ccbeb2
receivedFieldDataMessage=0x1ccbc93
receivedCreateOnMapMessage=0x1ccbfe4
receivedChangeOnMapMessage=0x1ccbf18
receivedDeleteOnMapMessage=0x1ccbf7e
receivedAmbientLightMessage=0x1ccbcf3
receivedTibiaTimeMessage=0x1cce706
```

<!-- END GENERATED WORLDMAP VTABLE -->

<!-- BEGIN GENERATED WORLDMAP QMETAOBJECT -->

## Generated Worldmap QMetaObject evidence

Source: GitHub Actions run `31579228118` on `oteryn-synology-staging`. Sanitized static metadata only.

```text
static_metacall=0xde9ca0
static_metacall_qword_hits=2
HIT index=0 off=0x28dab8 va=0x28dab8 section=.rela.dyn
  qword delta=-0x40 value=0x308c3c8 -> .data.rel.ro:off=0x308c3c8
  qword delta=-0x38 value=0x8 -> .comment:off=0x318e258
  qword delta=-0x30 value=0x1ce1434 -> .rodata:off=0x1ce1434
  qword delta=-0x28 value=0x308c3d0 -> .data.rel.ro:off=0x308c3d0
  qword delta=-0x20 value=0x8 -> .comment:off=0x318e258
  qword delta=-0x18 value=0x1ce1180 -> .rodata:off=0x1ce1180
  qword delta=-0x10 value=0x308c3d8 -> .data.rel.ro:off=0x308c3d8
  qword delta=-0x8 value=0x8 -> .comment:off=0x318e258
  qword delta=+0x0 value=0xde9ca0 -> .text:off=0xde9ca0
  qword delta=+0x8 value=0x308c3e8 -> .data.rel.ro:off=0x308c3e8
  qword delta=+0x10 value=0x8 -> .comment:off=0x318e258
  qword delta=+0x18 value=0x308b440 -> .data.rel.ro:off=0x308b440
  qword delta=+0x20 value=0x308c400 -> .data.rel.ro:off=0x308c400
  qword delta=+0x28 value=0x8 -> .comment:off=0x318e258
  qword delta=+0x30 value=0x30775a0 -> .data.rel.ro:off=0x30775a0
  qword delta=+0x38 value=0x308c408 -> .data.rel.ro:off=0x308c408
  qword delta=+0x40 value=0x8 -> .comment:off=0x318e258
  qword delta=+0x48 value=0xd1ccd0 -> .text:off=0xd1ccd0
HIT index=1 off=0x308c3d8 va=0x308c3d8 section=.data.rel.ro
  qword delta=-0x40 value=0xc269f0 -> .text:off=0xc269f0
  qword delta=-0x38 value=0xc26b70 -> .text:off=0xc26b70
  qword delta=-0x30 value=0x0 -> .comment:off=0x318e250
  qword delta=-0x28 value=0x0 -> .comment:off=0x318e250
  qword delta=-0x20 value=0x0 -> .comment:off=0x318e250
  qword delta=-0x18 value=0x0 -> .comment:off=0x318e250
  qword delta=-0x10 value=0x1ce1434 -> .rodata:off=0x1ce1434
  qword delta=-0x8 value=0x1ce1180 -> .rodata:off=0x1ce1180
  qword delta=+0x0 value=0xde9ca0 -> .text:off=0xde9ca0
  qword delta=+0x8 value=0x0 -> .comment:off=0x318e250
  qword delta=+0x10 value=0x308b440 -> .data.rel.ro:off=0x308b440
  qword delta=+0x18 value=0x0 -> .comment:off=0x318e250
  qword delta=+0x20 value=0x0 -> .comment:off=0x318e250
  qword delta=+0x28 value=0x30775a0 -> .data.rel.ro:off=0x30775a0
  qword delta=+0x30 value=0xd1ccd0 -> .text:off=0xd1ccd0
  qword delta=+0x38 value=0xd2af60 -> .text:off=0xd2af60
  qword delta=+0x40 value=0xd2f300 -> .text:off=0xd2f300
  qword delta=+0x48 value=0x844350 -> .text:off=0x844350

KNOWN_CLASS_NAME_OFFSETS
0x1cd67a0
0x1cd8bb4

QMETAOBJECT_CANDIDATES=18
CANDIDATE 0 score=14 base_off=0x308c3c8 base_va=0x308c3c8 section=.data.rel.ro metacall_delta=0x10 annotations=q1=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0];q7=small_u32_header:[0, 0, 29991616, 0, 50820800, 0, 0, 0]
  q0=0x1ce1434 -> .rodata:0x1ce1434
  q1=0x1ce1180 -> .rodata:0x1ce1180
  q2=0xde9ca0 -> .text:0xde9ca0
  q3=0x0 -> .comment:0x318e250
  q4=0x308b440 -> .data.rel.ro:0x308b440
  q5=0x0 -> .comment:0x318e250
  q6=0x0 -> .comment:0x318e250
  q7=0x30775a0 -> .data.rel.ro:0x30775a0
CANDIDATE 1 score=14 base_off=0x308c3d0 base_va=0x308c3d0 section=.data.rel.ro metacall_delta=0x8 annotations=q0=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0];q6=small_u32_header:[0, 0, 29991616, 0, 50820800, 0, 0, 0]
  q0=0x1ce1180 -> .rodata:0x1ce1180
  q1=0xde9ca0 -> .text:0xde9ca0
  q2=0x0 -> .comment:0x318e250
  q3=0x308b440 -> .data.rel.ro:0x308b440
  q4=0x0 -> .comment:0x318e250
  q5=0x0 -> .comment:0x318e250
  q6=0x30775a0 -> .data.rel.ro:0x30775a0
  q7=0xd1ccd0 -> .text:0xd1ccd0
CANDIDATE 2 score=11 base_off=0x28da78 base_va=0x28da78 section=.rela.dyn metacall_delta=0x40 annotations=q5=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x308c3c8 -> .data.rel.ro:0x308c3c8
  q1=0x8 -> .comment:0x318e258
  q2=0x1ce1434 -> .rodata:0x1ce1434
  q3=0x308c3d0 -> .data.rel.ro:0x308c3d0
  q4=0x8 -> .comment:0x318e258
  q5=0x1ce1180 -> .rodata:0x1ce1180
  q6=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q7=0x8 -> .comment:0x318e258
CANDIDATE 3 score=11 base_off=0x28da80 base_va=0x28da80 section=.rela.dyn metacall_delta=0x38 annotations=q4=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x8 -> .comment:0x318e258
  q1=0x1ce1434 -> .rodata:0x1ce1434
  q2=0x308c3d0 -> .data.rel.ro:0x308c3d0
  q3=0x8 -> .comment:0x318e258
  q4=0x1ce1180 -> .rodata:0x1ce1180
  q5=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q6=0x8 -> .comment:0x318e258
  q7=0xde9ca0 -> .text:0xde9ca0
CANDIDATE 4 score=11 base_off=0x28da88 base_va=0x28da88 section=.rela.dyn metacall_delta=0x30 annotations=q3=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x1ce1434 -> .rodata:0x1ce1434
  q1=0x308c3d0 -> .data.rel.ro:0x308c3d0
  q2=0x8 -> .comment:0x318e258
  q3=0x1ce1180 -> .rodata:0x1ce1180
  q4=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q5=0x8 -> .comment:0x318e258
  q6=0xde9ca0 -> .text:0xde9ca0
  q7=0x308c3e8 -> .data.rel.ro:0x308c3e8
CANDIDATE 5 score=11 base_off=0x28da90 base_va=0x28da90 section=.rela.dyn metacall_delta=0x28 annotations=q2=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x308c3d0 -> .data.rel.ro:0x308c3d0
  q1=0x8 -> .comment:0x318e258
  q2=0x1ce1180 -> .rodata:0x1ce1180
  q3=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q4=0x8 -> .comment:0x318e258
  q5=0xde9ca0 -> .text:0xde9ca0
  q6=0x308c3e8 -> .data.rel.ro:0x308c3e8
  q7=0x8 -> .comment:0x318e258
CANDIDATE 6 score=11 base_off=0x28da98 base_va=0x28da98 section=.rela.dyn metacall_delta=0x20 annotations=q1=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x8 -> .comment:0x318e258
  q1=0x1ce1180 -> .rodata:0x1ce1180
  q2=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q3=0x8 -> .comment:0x318e258
  q4=0xde9ca0 -> .text:0xde9ca0
  q5=0x308c3e8 -> .data.rel.ro:0x308c3e8
  q6=0x8 -> .comment:0x318e258
  q7=0x308b440 -> .data.rel.ro:0x308b440
CANDIDATE 7 score=11 base_off=0x28daa0 base_va=0x28daa0 section=.rela.dyn metacall_delta=0x18 annotations=q0=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0x1ce1180 -> .rodata:0x1ce1180
  q1=0x308c3d8 -> .data.rel.ro:0x308c3d8
  q2=0x8 -> .comment:0x318e258
  q3=0xde9ca0 -> .text:0xde9ca0
  q4=0x308c3e8 -> .data.rel.ro:0x308c3e8
  q5=0x8 -> .comment:0x318e258
  q6=0x308b440 -> .data.rel.ro:0x308b440
  q7=0x308c400 -> .data.rel.ro:0x308c400
CANDIDATE 8 score=11 base_off=0x28dab0 base_va=0x28dab0 section=.rela.dyn metacall_delta=0x8 annotations=q7=small_u32_header:[0, 0, 29991616, 0, 50820800, 0, 0, 0]
  q0=0x8 -> .comment:0x318e258
  q1=0xde9ca0 -> .text:0xde9ca0
  q2=0x308c3e8 -> .data.rel.ro:0x308c3e8
  q3=0x8 -> .comment:0x318e258
  q4=0x308b440 -> .data.rel.ro:0x308b440
  q5=0x308c400 -> .data.rel.ro:0x308c400
  q6=0x8 -> .comment:0x318e258
  q7=0x30775a0 -> .data.rel.ro:0x30775a0
CANDIDATE 9 score=11 base_off=0x28dab8 base_va=0x28dab8 section=.rela.dyn metacall_delta=0x0 annotations=q6=small_u32_header:[0, 0, 29991616, 0, 50820800, 0, 0, 0]
  q0=0xde9ca0 -> .text:0xde9ca0
  q1=0x308c3e8 -> .data.rel.ro:0x308c3e8
  q2=0x8 -> .comment:0x318e258
  q3=0x308b440 -> .data.rel.ro:0x308b440
  q4=0x308c400 -> .data.rel.ro:0x308c400
  q5=0x8 -> .comment:0x318e258
  q6=0x30775a0 -> .data.rel.ro:0x30775a0
  q7=0x308c408 -> .data.rel.ro:0x308c408
CANDIDATE 10 score=11 base_off=0x308c398 base_va=0x308c398 section=.data.rel.ro metacall_delta=0x40 annotations=q7=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0xc269f0 -> .text:0xc269f0
  q1=0xc26b70 -> .text:0xc26b70
  q2=0x0 -> .comment:0x318e250
  q3=0x0 -> .comment:0x318e250
  q4=0x0 -> .comment:0x318e250
  q5=0x0 -> .comment:0x318e250
  q6=0x1ce1434 -> .rodata:0x1ce1434
  q7=0x1ce1180 -> .rodata:0x1ce1180
CANDIDATE 11 score=11 base_off=0x308c3a0 base_va=0x308c3a0 section=.data.rel.ro metacall_delta=0x38 annotations=q6=small_u32_header:[13, 0, 0, 0, 20, 14, 0, 0]
  q0=0xc26b70 -> .text:0xc26b70
  q1=0x0 -> .comment:0x318e250
  q2=0x0 -> .comment:0x318e250
  q3=0x0 -> .comment:0x318e250
  q4=0x0 -> .comment:0x318e250
  q5=0x1ce1434 -> .rodata:0x1ce1434
  q6=0x1ce1180 -> .rodata:0x1ce1180
  q7=0xde9ca0 -> .text:0xde9ca0

TOP_CANDIDATE_REFERENCED_DATA
REF qvalue=0x1ce1434 section=.rodata off=0x1ce1434
  U32 00000118 00000026 0000013f 00000011 00000151 00000000 00000152 0000002a 0000017d 0000000b 00000189 0000001d 000001a7 00000016 000001be 0000000a 000001c9 00000017 000001e1 0000000b 000001ed 0000001b 00000209 0000001d 00000227 0000001a 00000242 0000002a 0000026d 00000012 00000280 0000001a 0000029b 0000002a 000002c6 00000012 000002d9 00000006 000002e0 00000008 000002e9 00000016 00000300 0000001b 0000031c 00000013 00000330 00000016
  STR +0x118 tibia::sessiondump::TSessiondumpPlayer
  STR +0x13f publishGameAction
  STR +0x152 std::shared_ptr<tibia::input::IGameAction>
  STR +0x17d pGameAction
  STR +0x189 preprocessingMessageAvailable
  STR +0x1a7 TSessiondumpPacketInfo
  STR +0x1be PacketInfo
  STR +0x1c9 TSessiondumpMessageInfo
  STR +0x1e1 MessageInfo
  STR +0x1ed std::function<QByteArray()>
  STR +0x209 RawMessageBytesReaderFunction
  STR +0x227 gameserverMessageAvailable
  STR +0x242 tibia::protobuf::TGameserverMessagePointer
  STR +0x26d pGameserverMessage
  STR +0x280 gameclientMessageAvailable
  STR +0x29b tibia::protobuf::TGameclientMessagePointer
  STR +0x2c6 pGameclientMessage
  STR +0x2d9 paused
  STR +0x2e0 finished
  STR +0x2e9 sessiondumpInfoChanged
  STR +0x300 sessiondumpPacketsProcessed
  STR +0x31c skipToMarkerChanged
  STR +0x330 requestResetOfStorages
  STR +0x347 sessiondumpProcessingChanged
  STR +0x364 sessiondumpLoadingError
  STR +0x37c ErrorMessage
  STR +0x389 finishedLoadingSessiondump
  STR +0x3a4 currentPlayspeedChanged
  STR +0x3bc handleGameAction
  STR +0x3cd onSessiondumpMetadataLoaded
REF qvalue=0x1ce1180 section=.rodata off=0x1ce1180
  U32 0000000d 00000000 00000000 00000000 00000014 0000000e 00000000 00000000 00000000 00000000 00000000 00000000 00000000 0000000e 00000001 00000001 00000086 00000002 00000106 00000001 00000005 00000003 00000089 00000002 00000006 00000003 0000000c 00000001 00000090 00000002 00000006 00000007 0000000f 00000001 00000093 00000002 00000006 00000009 00000012 00000000 00000096 00000002 00000106 0000000b 00000013 00000000 00000097 00000002
  STR +0x3cc tibia::sessiondump::TSessiondumpPlayer
  STR +0x3f3 publishGameAction
  STR +0x406 std::shared_ptr<tibia::input::IGameAction>
  STR +0x431 pGameAction
  STR +0x43d preprocessingMessageAvailable
  STR +0x45b TSessiondumpPacketInfo
  STR +0x472 PacketInfo
  STR +0x47d TSessiondumpMessageInfo
  STR +0x495 MessageInfo
  STR +0x4a1 std::function<QByteArray()>
  STR +0x4bd RawMessageBytesReaderFunction
  STR +0x4db gameserverMessageAvailable
  STR +0x4f6 tibia::protobuf::TGameserverMessagePointer
  STR +0x521 pGameserverMessage
  STR +0x534 gameclientMessageAvailable
  STR +0x54f tibia::protobuf::TGameclientMessagePointer
  STR +0x57a pGameclientMessage
  STR +0x58d paused
  STR +0x594 finished
  STR +0x59d sessiondumpInfoChanged
  STR +0x5b4 sessiondumpPacketsProcessed
  STR +0x5d0 skipToMarkerChanged
  STR +0x5e4 requestResetOfStorages
  STR +0x5fb sessiondumpProcessingChanged
  STR +0x618 sessiondumpLoadingError
  STR +0x630 ErrorMessage
  STR +0x63d finishedLoadingSessiondump
  STR +0x658 currentPlayspeedChanged
  STR +0x670 handleGameAction
  STR +0x681 onSessiondumpMetadataLoaded
REF qvalue=0xde9ca0 section=.text off=0xde9ca0
  U32 2475f685 0f13fa83 00011a87 358d4800 00fa1d78 8348d289 634828ec 01489604 0fe0fff0 0000441f 0f05fe83 0000f685 518b4800 318b4808 a10d8d48 48fffffd 3948028b d5840fc8 48000000 fdbe0d8d 3948ffff d5840fc8 48000000 fdee0d8d 3948ffff d5840fc8 48000000 fe0e0d8d 3948ffff d5840fc8 48000000 fe2e0d8d 3948ffff e5840fc8 48000000 fe3e0d8d 3948ffff e5840fc8 48000000 fe4e0d8d 3948ffff e5840fc8 48000000 fe5e0d8d 3948ffff dd840fc8 48000003 fe6e0d8d
  STR +0x1e9 52%*
  STR +0x2a9 5r$*
  STR +0x2ce 5M$*
  STR +0x2f9 5"$*
  STR +0x3b9 5b#*
  STR +0x3d9 5B#*
  STR +0x4f6 5e^,
REF qvalue=0x0 section=.comment off=0x318e250
  U32 3a434347 65442820 6e616962 2e323120 2d302e32 642b3431 32316265 20293175 322e3231 0000302e 7368732e 61747274 692e0062 7265746e 6e2e0070 2e65746f 2e756e67 706f7270 79747265 6f6e2e00 672e6574 622e756e 646c6975 0064692d 746f6e2e 42412e65 61742d49 672e0067 682e756e 00687361 6e79642e 006d7973 6e79642e 00727473 756e672e 7265762e 6e6f6973 6e672e00 65762e75 6f697372 00725f6e 6c65722e 79642e61 722e006e 2e616c65 00746c70 696e692e 702e0074
  STR +0x0 GCC: (Debian 12.2.0-14+deb12u1) 12.2.0
  STR +0x28 .shstrtab
  STR +0x32 .interp
  STR +0x3a .note.gnu.property
  STR +0x4d .note.gnu.build-id
  STR +0x60 .note.ABI-tag
  STR +0x6e .gnu.hash
  STR +0x78 .dynsym
  STR +0x80 .dynstr
  STR +0x88 .gnu.version
  STR +0x95 .gnu.version_r
  STR +0xa4 .rela.dyn
  STR +0xae .rela.plt
  STR +0xb8 .init
  STR +0xbe .plt.got
  STR +0xc7 .text
  STR +0xcd malloc_hook
  STR +0xd9 .fini
  STR +0xdf .rodata
  STR +0xe7 protodesc_cold
  STR +0xf6 .eh_frame_hdr
  STR +0x104 .eh_frame
  STR +0x10e .gcc_except_table
  STR +0x120 .tdata
  STR +0x127 .tbss
  STR +0x12d .init_array
  STR +0x139 .fini_array
  STR +0x145 .data.rel.ro
  STR +0x152 .dynamic
  STR +0x15b .got.plt
REF qvalue=0x308b440 section=.data.rel.ro off=0x308b440
  U32 0314b320 00000000 02f5ec40 00000000 031399e0 00000000 02f5ec40 00000000 0314b2a0 00000000 0314b220 00000000 0314b1a0 00000000 02f5ec40 00000000 03149360 00000000 02f5ec40 00000000 031493e0 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 00000000 00000000 02f5ec40 00000000 02f5ec40 00000000 02f5ec40 00000000 031399e0 00000000
REF qvalue=0x30775a0 section=.data.rel.ro off=0x30775a0
  U32 00000000 00000000 01c9a2c0 00000000 030776c0 00000000 00000000 00000000 01c9a300 00000000 03077548 00000000 00000000 00000000 01c9a3a0 00000000 00000000 00000000 01c9a400 00000000 03077500 00000000 00000000 00000000 01c9a460 00000000 03077518 00000000 00000000 00000000 01c9a4a0 00000000 00000000 00000002 030777f0 00000000 00000002 00000000 03077530 00000000 00000802 00000000 00000000 00000000 01c9a4c0 00000000 03077548 00000000
REF qvalue=0xd1ccd0 section=.text off=0xd1ccd0
  U32 087f8b48 387f8348 e9057400 ff7c0200 f9058d48 c3022597 00841f0f 00000000 087f8b48 387f8348 e9057400 ff7c01e0 39058d48 c3023791 00841f0f 00000000 087f8b48 387f8348 e9057400 ff7c01c0 b9058d48 c30236ef 00841f0f 00000000 087f8b48 387f8348 e9057400 ff7c01a0 59058d48 c3022597 00841f0f 00000000 087f8b48 387f8348 e9057400 ff7c0180 f9058d48 c3022596 00841f0f 00000000 087f8b48 387f8348 e9057400 ff7c0160 f9058d48 c3022532 00841f0f 00000000
  STR +0x2f0 ATUH
  STR +0x3d7 []A\
  STR +0x542 oGXH
  STR +0x547 WhfH
REF qvalue=0x308c3c8 section=.data.rel.ro off=0x308c3c8
  U32 01ce1434 00000000 01ce1180 00000000 00de9ca0 00000000 00000000 00000000 0308b440 00000000 00000000 00000000 00000000 00000000 030775a0 00000000 00d1ccd0 00000000 00d2af60 00000000 00d2f300 00000000 00844350 00000000 00844680 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 007ec950 00000000 008319f0 00000000 007e06d0 00000000 007e6b70 00000000
REF qvalue=0x8 section=.comment off=0x318e258
  U32 6e616962 2e323120 2d302e32 642b3431 32316265 20293175 322e3231 0000302e 7368732e 61747274 692e0062 7265746e 6e2e0070 2e65746f 2e756e67 706f7270 79747265 6f6e2e00 672e6574 622e756e 646c6975 0064692d 746f6e2e 42412e65 61742d49 672e0067 682e756e 00687361 6e79642e 006d7973 6e79642e 00727473 756e672e 7265762e 6e6f6973 6e672e00 65762e75 6f697372 00725f6e 6c65722e 79642e61 722e006e 2e616c65 00746c70 696e692e 702e0074 672e746c 2e00746f
  STR +0x0 bian 12.2.0-14+deb12u1) 12.2.0
  STR +0x20 .shstrtab
  STR +0x2a .interp
  STR +0x32 .note.gnu.property
  STR +0x45 .note.gnu.build-id
  STR +0x58 .note.ABI-tag
  STR +0x66 .gnu.hash
  STR +0x70 .dynsym
  STR +0x78 .dynstr
  STR +0x80 .gnu.version
  STR +0x8d .gnu.version_r
  STR +0x9c .rela.dyn
  STR +0xa6 .rela.plt
  STR +0xb0 .init
  STR +0xb6 .plt.got
  STR +0xbf .text
  STR +0xc5 malloc_hook
  STR +0xd1 .fini
  STR +0xd7 .rodata
  STR +0xdf protodesc_cold
  STR +0xee .eh_frame_hdr
  STR +0xfc .eh_frame
  STR +0x106 .gcc_except_table
  STR +0x118 .tdata
  STR +0x11f .tbss
  STR +0x125 .init_array
  STR +0x131 .fini_array
  STR +0x13d .data.rel.ro
  STR +0x14a .dynamic
  STR +0x153 .got.plt
REF qvalue=0x308c3d0 section=.data.rel.ro off=0x308c3d0
  U32 01ce1180 00000000 00de9ca0 00000000 00000000 00000000 0308b440 00000000 00000000 00000000 00000000 00000000 030775a0 00000000 00d1ccd0 00000000 00d2af60 00000000 00d2f300 00000000 00844350 00000000 00844680 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 007ec950 00000000 008319f0 00000000 007e06d0 00000000 007e6b70 00000000 007e0b60 00000000
REF qvalue=0x308c3d8 section=.data.rel.ro off=0x308c3d8
  U32 00de9ca0 00000000 00000000 00000000 0308b440 00000000 00000000 00000000 00000000 00000000 030775a0 00000000 00d1ccd0 00000000 00d2af60 00000000 00d2f300 00000000 00844350 00000000 00844680 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 007ec950 00000000 008319f0 00000000 007e06d0 00000000 007e6b70 00000000 007e0b60 00000000 006afe60 00000000
```

<!-- END GENERATED WORLDMAP QMETAOBJECT -->
