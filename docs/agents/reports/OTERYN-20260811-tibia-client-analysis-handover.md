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

<!-- BEGIN EXACT WORLDMAP QMETAOBJECT -->

## Exact Worldmap QMetaObject evidence

Source: GitHub Actions run `31579494156` on `oteryn-synology-staging`. Sanitized metadata/disassembly only.

```text
worldmap_metaobject_matches=13
MATCH index=0 score=180 qmetaobject_va=0x3087800 file_off=0x3087800
  superdata=0x0 stringdata=0x1cd8a54 metadata=0x1cd8820 static_metacall=0xdf2a60 related=0x0 metatypes=0x2f6ab00 extra=0x0
  metadata_header=13 0 0 0 14 14 0 0 0 0 0 0 0 1
  STRING_RUNS
    +0x4 0
    +0x1c *
    +0x34 3
    +0x4c 6
    +0x64 7
    +0x7c 2
    +0x94 5
    +0xac 4
    +0xc4 7
    +0xdc 5
    +0xf4 7
    +0x10c 7
    +0x124 7
    +0x13c 8
    +0x154 5
    +0x160 tibia::worldmap::TWorldmapProtocolMessageHandler
    +0x191 publishGameAction
    +0x1a4 std::shared_ptr<tibia::input::IGameAction>
    +0x1cf pGameAction
    +0x1db handleFullMapMessage
    +0x1f0 tibia::protobuf::protocol::GameserverMessageFullMap
    +0x224 FullMapMessage
    +0x233 handleLeftColumnMessage
    +0x24b tibia::protobuf::protocol::GameserverMessageLeftColumn
    +0x282 LeftColumnMessage
    +0x294 handleRightColumnMessage
    +0x2ad tibia::protobuf::protocol::GameserverMessageRightColumn
    +0x2e5 RightColumnMessage
    +0x2f8 handleTopRowMessage
    +0x30c tibia::protobuf::protocol::GameserverMessageTopRow
    +0x33f TopRowMessage
    +0x34d handleBottomRowMessage
    +0x364 tibia::protobuf::protocol::GameserverMessageBottomRow
    +0x39a BottomRowMessage
    +0x3ab handleTopFloorMessage
    +0x3c1 tibia::protobuf::protocol::GameserverMessageTopFloor
    +0x3f6 TopFloorMessage
    +0x406 handleBottomFloorMessage
    +0x41f tibia::protobuf::protocol::GameserverMessageBottomFloor
    +0x457 BottomFloorMessage
    +0x46a handleFieldDataMessage
    +0x481 tibia::protobuf::protocol::GameserverMessageFieldData
    +0x4b7 FieldDataMessage
    +0x4c8 handleCreateOnMapMessage
    +0x4e1 tibia::protobuf::protocol::GameserverMessageCreateOnMap
    +0x519 CreateOnMapMessage
    +0x52c handleChangeOnMapMessage
    +0x545 tibia::protobuf::protocol::GameserverMessageChangeOnMap
    +0x57d ChangeOnMapMessage
    +0x590 handleDeleteOnMapMessage
    +0x5a9 tibia::protobuf::protocol::GameserverMessageDeleteOnMap
    +0x5e1 DeleteOnMapMessage
    +0x5f4 handleAmbientLightMessage
    +0x60e tibia::protobuf::protocol::GameserverMessageAmbientLight
    +0x647 AmbientLightMessage
    +0x65b handleTibiaTimeMessage
    +0x672 tibia::protobuf::protocol::GameserverMessageTibiaTime
    +0x6a8 TibiaTimeMessage
    +0x71c +
    +0x72c (
    +0x730 +
    +0x734 T
    +0x73c f
    +0x744 g
    +0x748 *
    +0x754 tibia::creatures::TBattleListRenderProvider
    +0x780 publishGameAction
    +0x793 std::shared_ptr<tibia::input::IGameAction>
    +0x7be pGameAction
    +0x81c +
    +0x82c (
    +0x834 G
    +0x83c Y
    +0x844 Z
    +0x848 *
    +0x854 tibia::effects::TEffectStorage
    +0x873 publishGameAction
    +0x886 std::shared_ptr<tibia::input::IGameAction>
    +0x8b1 pGameAction
    +0x90c 2
    +0x924 5
    +0x93c 8
    +0x954 ;
    +0x96c >
    +0x984 ?
    +0x994 +
    +0x9a0 +
    +0x9ac +
    +0x9b8 +
    +0x9c4 +
    +0x9c8 +
    +0x9d8 p
    +0x9dc /
    +0x9f4 *
    +0xa14 2
    +0xa24 E
    +0xa2c $
    +0xa34 &
    +0xa3c *
    +0xa48 tibia::skillwheel::TSkillWheelGameActionHandler
    +0xa78 publishGameAction
    +0xa8b std::shared_ptr<tibia::input::IGameAction>
    +0xab6 pGameAction
    +0xac2 handleGameAction
    +0xad3 handleCreatureGameAction
    +0xaec std::shared_ptr<tibia::input::TGameActionCreature>
    +0xb1f handleSaveSkillWheelGameAction
    +0xb3e std::shared_ptr<tibia::input::gameactions::TGameActionSaveSkillWheel>
    +0xb84 handleRequestOnwSkillWheelGameAction
    +0xba9 onPlayerCreatureAddedToCreatureStorage
    +0xbd0 std::weak_ptr<tibia::creatures::TCreature>
    +0xbfb pPlayerWeak
    +0xc4c 2
    +0xc64 7
    +0xc7c <
    +0xc94 ?
    +0xcac B
    +0xcc4 E
    +0xcd4 +
    +0xce8 +
    +0xcfc +
    +0xd08 +
    +0xd14 +
    +0xd20 +
    +0xd30 x
    +0xd34  
    +0xd4c +
    +0xd84 *
    +0xd94 &
    +0xd9c *
    +0xda8 tibia::chat::TChatChannelStorage
    +0xdc9 newChatChannelOpened
    +0xddf std::shared_ptr<TChatChannelIdentifierBase>
    +0xe0b pIdentifier
    +0xe17 ForceMakeCurrent
    +0xe28 chatChannelReopend
    +0xe3b chatChannelRemoved
    +0xe4e entryAddedToChatChannel
    +0xe66 publishGameAction
    +0xe78 std::shared_ptr<tibia::input::IGameAction>
    +0xea3 pGameAction
    +0xeaf onPlayerCreatureAddedToCreatureStorage
    +0xed6 std::weak_ptr<tibia::creatures::TCreature>
    +0xf01 pPlayer
    +0xf0c tibia::sound::TObjectSoundController
    +0xf9c +
    +0xfbc H
    +0xfc0 $
    +0xfc4 m
    +0xfc8  
    +0xff8 !
    +0x1004 tibia::sound::TObjectSoundController
    +0x1029 ambienceObjectStreamCountChanged
    +0x104b tibia::sound::TSoundEffectID
    +0x1068 SoundEffectID
    +0x1076 size_t
    +0x107d Count
    +0x1083 tibia::worldmap::TWorldMapExtentX
    +0x10a5 NearestDistance
    +0x10cc N5tibia5input30TGenericChatGameActionProviderE
    +0x110c St23_Sp_counted_ptr_inplaceIN5tibia5input30TGenericChatGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x118c N5tibia9creatures24TPartyGameActionProviderE
    +0x11cc St23_Sp_counted_ptr_inplaceIN5tibia9creatures24TPartyGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x124c N5tibia9creatures32TInspectPlayerGameActionProviderE
    +0x128c St23_Sp_counted_ptr_inplaceIN5tibia9creatures32TInspectPlayerGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x130c N5tibia5input32TGenericObjectGameActionProviderE
    +0x134c St23_Sp_counted_ptr_inplaceIN5tibia5input32TGenericObjectGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x13cc N5tibia9creatures29TBattleListGameActionProviderE
    +0x140c St23_Sp_counted_ptr_inplaceIN5tibia9creatures29TBattleListGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x148c N5tibia8worldmap27TWorldMapGameActionProviderE
    +0x14cc St23_Sp_counted_ptr_inplaceIN5tibia8worldmap27TWorldMapGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x154c N5tibia9container28TContainerGameActionProviderE
    +0x158c St23_Sp_counted_ptr_inplaceIN5tibia9container28TContainerGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x160c N5tibia4chat23TChatGameActionProviderE
    +0x164c St23_Sp_counted_ptr_inplaceIN5tibia4chat23TChatGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x16cc N5tibia9creatures28TVipWidgetGameActionProviderE
    +0x170c St23_Sp_counted_ptr_inplaceIN5tibia9creatures28TVipWidgetGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x178c N5tibia5input25TGenericGameActionHandlerE
    +0x17cc St23_Sp_counted_ptr_inplaceIN5tibia5input25TGenericGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x184c N5tibia9creatures27TCreaturesGameActionHandlerE
    +0x188c St23_Sp_counted_ptr_inplaceIN5tibia9creatures27TCreaturesGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x190c N5tibia4game24TPlayerGameActionHandlerE
    +0x194c St23_Sp_counted_ptr_inplaceIN5tibia4game24TPlayerGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x19cc N5tibia9container27TContainerGameActionHandlerE
    +0x1a0c St23_Sp_counted_ptr_inplaceIN5tibia9container27TContainerGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1a8c N5tibia8worldmap26TWorldMapGameActionHandlerE
    +0x1acc St23_Sp_counted_ptr_inplaceIN5tibia8worldmap26TWorldMapGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1b4c N5tibia5input25TUseWithGameActionHandlerE
    +0x1b8c St23_Sp_counted_ptr_inplaceIN5tibia5input25TUseWithGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1c0c N5tibia5input32TCrossHairSpellGameActionHandlerE
    +0x1c4c St23_Sp_counted_ptr_inplaceIN5tibia5input32TCrossHairSpellGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1ccc N5tibia4chat22TChatGameActionHandlerE
    +0x1d0c St23_Sp_counted_ptr_inplaceIN5tibia4chat22TChatGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1d8c N5tibia9creatures21TVipGameActionHandlerE
    +0x1dcc St23_Sp_counted_ptr_inplaceIN5tibia9creatures21TVipGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1e4c N5tibia5trade29TPlayerTradeGameActionHandlerE
    +0x1e8c St23_Sp_counted_ptr_inplaceIN5tibia5trade29TPlayerTradeGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1f0c N5tibia5store23TStoreGameActionHandlerE
    +0x1f4c St23_Sp_counted_ptr_inplaceIN5tibia5store23TStoreGameActionHandlerESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1fcc N5tibia4prey22TPreyGameActionHandlerE
  STATIC_METACALL_DISASSEMBLY
    
    /data/client-15.32.df7b29/bin/client:     file format elf64-x86-64
    
    
    Disassembly of section .text:
    
    0000000000df2a60 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254230>:
      df2a60:	85 f6                	test   %esi,%esi
      df2a62:	75 2c                	jne    df2a90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254260>
      df2a64:	83 fa 0d             	cmp    $0xd,%edx
      df2a67:	77 3c                	ja     df2aa5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254275>
      df2a69:	41 55                	push   %r13
      df2a6b:	48 8d 35 9e 92 f9 00 	lea    0xf9929e(%rip),%rsi        # 1d8bd10 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0xa2750>
      df2a72:	89 d2                	mov    %edx,%edx
      df2a74:	41 54                	push   %r12
      df2a76:	55                   	push   %rbp
      df2a77:	53                   	push   %rbx
      df2a78:	48 89 fb             	mov    %rdi,%rbx
      df2a7b:	48 83 ec 58          	sub    $0x58,%rsp
      df2a7f:	48 63 04 96          	movslq (%rsi,%rdx,4),%rax
      df2a83:	48 01 f0             	add    %rsi,%rax
      df2a86:	ff e0                	jmp    *%rax
      df2a88:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
      df2a8f:	00 
      df2a90:	83 fe 05             	cmp    $0x5,%esi
      df2a93:	75 10                	jne    df2aa5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254275>
      df2a95:	48 8b 41 08          	mov    0x8(%rcx),%rax
      df2a99:	48 8d 15 70 b0 ff ff 	lea    -0x4f90(%rip),%rdx        # dedb10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x24f2e0>
      df2aa0:	48 39 10             	cmp    %rdx,(%rax)
      df2aa3:	74 0b                	je     df2ab0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254280>
      df2aa5:	c3                   	ret
      df2aa6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
      df2aad:	00 00 00 
      df2ab0:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
      df2ab5:	75 ee                	jne    df2aa5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254275>
      df2ab7:	48 8b 01             	mov    (%rcx),%rax
      df2aba:	c7 00 00 00 00 00    	movl   $0x0,(%rax)
      df2ac0:	c3                   	ret
      df2ac1:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      df2ac8:	48 8b 41 08          	mov    0x8(%rcx),%rax
      df2acc:	48 8b 7f 20          	mov    0x20(%rdi),%rdi
      df2ad0:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2ad4:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2ad8:	8b 50 18             	mov    0x18(%rax),%edx
      df2adb:	8b 40 1c             	mov    0x1c(%rax),%eax
      df2ade:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2ae3:	48 8b 07             	mov    (%rdi),%rax
      df2ae6:	f3 0f 5e 0d fa a8 f7 	divss  0xf7a8fa(%rip),%xmm1        # 1d6d3e8 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x83e28>
      df2aed:	00 
      df2aee:	f3 48 0f 2a c2       	cvtsi2ss %rdx,%xmm0
      df2af3:	48 8d 15 d6 f5 eb ff 	lea    -0x140a2a(%rip),%rdx        # cb20d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1138a0>
      df2afa:	f3 0f 5e 05 e2 a8 f7 	divss  0xf7a8e2(%rip),%xmm0        # 1d6d3e4 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x83e24>
      df2b01:	00 
      df2b02:	48 8b 80 80 00 00 00 	mov    0x80(%rax),%rax
      df2b09:	f3 0f 58 c1          	addss  %xmm1,%xmm0
      df2b0d:	48 39 d0             	cmp    %rdx,%rax
      df2b10:	0f 85 51 02 00 00    	jne    df2d67 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254537>
      df2b16:	f3 0f 59 05 be 86 f3 	mulss  0xf386be(%rip),%xmm0        # 1d2b1dc <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x41c1c>
      df2b1d:	00 
      df2b1e:	0f 2f 05 f7 6c b6 01 	comiss 0x1b66cf7(%rip),%xmm0        # 295981c <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x8b80bc>
      df2b25:	0f 83 25 02 00 00    	jae    df2d50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254520>
      df2b2b:	f3 48 0f 2c d0       	cvttss2si %xmm0,%rdx
      df2b30:	48 8b 47 40          	mov    0x40(%rdi),%rax
      df2b34:	31 c9                	xor    %ecx,%ecx
      df2b36:	48 8d 35 23 6e 2a 02 	lea    0x22a6e23(%rip),%rsi        # 3099960 <QObject::staticMetaObject@Qt_6>
      df2b3d:	48 29 d0             	sub    %rdx,%rax
      df2b40:	31 d2                	xor    %edx,%edx
      df2b42:	48 89 47 48          	mov    %rax,0x48(%rdi)
      df2b46:	48 83 c4 58          	add    $0x58,%rsp
      df2b4a:	5b                   	pop    %rbx
      df2b4b:	5d                   	pop    %rbp
      df2b4c:	41 5c                	pop    %r12
      df2b4e:	41 5d                	pop    %r13
      df2b50:	e9 6b c2 6e ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      df2b55:	0f 1f 00             	nopl   (%rax)
      df2b58:	48 8b 41 08          	mov    0x8(%rcx),%rax
      df2b5c:	31 d2                	xor    %edx,%edx
      df2b5e:	48 89 e1             	mov    %rsp,%rcx
      df2b61:	48 8d 35 98 4c 29 02 	lea    0x2294c98(%rip),%rsi        # 3087800 <QObject::staticMetaObject@Qt_6>
      df2b68:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
      df2b6f:	00 
      df2b70:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
      df2b75:	e8 46 c2 6e ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      df2b7a:	48 83 c4 58          	add    $0x58,%rsp
      df2b7e:	5b                   	pop    %rbx
      df2b7f:	5d                   	pop    %rbp
      df2b80:	41 5c                	pop    %r12
      df2b82:	41 5d                	pop    %r13
      df2b84:	c3                   	ret
      df2b85:	0f 1f 00             	nopl   (%rax)
      df2b88:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2b8c:	48 83 c4 58          	add    $0x58,%rsp
      df2b90:	5b                   	pop    %rbx
      df2b91:	5d                   	pop    %rbp
      df2b92:	41 5c                	pop    %r12
      df2b94:	41 5d                	pop    %r13
      df2b96:	e9 35 9d ef ff       	jmp    cec8d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x14e0a0>
      df2b9b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2ba0:	4c 8b 69 08          	mov    0x8(%rcx),%r13
      df2ba4:	83 af 98 00 00 00 01 	subl   $0x1,0x98(%rdi)
      df2bab:	48 89 e5             	mov    %rsp,%rbp
      df2bae:	45 31 e4             	xor    %r12d,%r12d
      df2bb1:	e8 ba 8b ee ff       	call   cdb770 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13cf40>
      df2bb6:	83 bb a0 00 00 00 07 	cmpl   $0x7,0xa0(%rbx)
      df2bbd:	48 8d b3 90 00 00 00 	lea    0x90(%rbx),%rsi
      df2bc4:	48 89 ef             	mov    %rbp,%rdi
      df2bc7:	41 0f 9f c4          	setg   %r12b
      df2bcb:	e8 10 38 dd ff       	call   bc63e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x27bb0>
      df2bd0:	49 8b 45 18          	mov    0x18(%r13),%rax
      df2bd4:	48 8d 15 85 7c 34 02 	lea    0x2347c85(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
      df2bdb:	44 89 e1             	mov    %r12d,%ecx
      df2bde:	48 89 df             	mov    %rbx,%rdi
      df2be1:	48 85 c0             	test   %rax,%rax
      df2be4:	48 0f 44 c2          	cmove  %rdx,%rax
      df2be8:	48 89 ea             	mov    %rbp,%rdx
      df2beb:	48 8b 70 28          	mov    0x28(%rax),%rsi
      df2bef:	e8 8c 5e bb 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
      df2bf4:	48 83 c4 58          	add    $0x58,%rsp
      df2bf8:	5b                   	pop    %rbx
      df2bf9:	5d                   	pop    %rbp
      df2bfa:	41 5c                	pop    %r12
      df2bfc:	41 5d                	pop    %r13
      df2bfe:	c3                   	ret
      df2bff:	90                   	nop
      df2c00:	4c 8b 69 08          	mov    0x8(%rcx),%r13
      df2c04:	83 87 98 00 00 00 01 	addl   $0x1,0x98(%rdi)
      df2c0b:	48 89 e5             	mov    %rsp,%rbp
      df2c0e:	45 31 e4             	xor    %r12d,%r12d
      df2c11:	e8 5a 8b ee ff       	call   cdb770 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13cf40>
      df2c16:	83 bb a0 00 00 00 07 	cmpl   $0x7,0xa0(%rbx)
      df2c1d:	48 8d b3 90 00 00 00 	lea    0x90(%rbx),%rsi
      df2c24:	48 89 ef             	mov    %rbp,%rdi
      df2c27:	41 0f 9f c4          	setg   %r12b
      df2c2b:	e8 40 38 dd ff       	call   bc6470 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x27c40>
      df2c30:	eb 9e                	jmp    df2bd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2543a0>
      df2c32:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df2c38:	4c 8b 69 08          	mov    0x8(%rcx),%r13
      df2c3c:	83 af 9c 00 00 00 01 	subl   $0x1,0x9c(%rdi)
      df2c43:	48 89 e5             	mov    %rsp,%rbp
      df2c46:	45 31 e4             	xor    %r12d,%r12d
      df2c49:	e8 22 8b ee ff       	call   cdb770 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13cf40>
      df2c4e:	83 bb a0 00 00 00 07 	cmpl   $0x7,0xa0(%rbx)
      df2c55:	48 8d b3 90 00 00 00 	lea    0x90(%rbx),%rsi
      df2c5c:	48 89 ef             	mov    %rbp,%rdi
      df2c5f:	41 0f 9f c4          	setg   %r12b
      df2c63:	e8 98 38 dd ff       	call   bc6500 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x27cd0>
      df2c68:	e9 63 ff ff ff       	jmp    df2bd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2543a0>
      df2c6d:	0f 1f 00             	nopl   (%rax)
      df2c70:	4c 8b 69 08          	mov    0x8(%rcx),%r13
      df2c74:	83 87 9c 00 00 00 01 	addl   $0x1,0x9c(%rdi)
      df2c7b:	48 89 e5             	mov    %rsp,%rbp
      df2c7e:	45 31 e4             	xor    %r12d,%r12d
      df2c81:	e8 ea 8a ee ff       	call   cdb770 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13cf40>
      df2c86:	83 bb a0 00 00 00 07 	cmpl   $0x7,0xa0(%rbx)
      df2c8d:	48 8d b3 90 00 00 00 	lea    0x90(%rbx),%rsi
      df2c94:	48 89 ef             	mov    %rbp,%rdi
      df2c97:	41 0f 9f c4          	setg   %r12b
      df2c9b:	e8 f0 38 dd ff       	call   bc6590 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x27d60>
      df2ca0:	e9 2b ff ff ff       	jmp    df2bd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2543a0>
      df2ca5:	0f 1f 00             	nopl   (%rax)
      df2ca8:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2cac:	48 83 c4 58          	add    $0x58,%rsp
      df2cb0:	5b                   	pop    %rbx
      df2cb1:	5d                   	pop    %rbp
      df2cb2:	41 5c                	pop    %r12
      df2cb4:	41 5d                	pop    %r13
      df2cb6:	e9 d5 8f ee ff       	jmp    cdbc90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13d460>
      df2cbb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2cc0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2cc4:	48 83 c4 58          	add    $0x58,%rsp
      df2cc8:	5b                   	pop    %rbx
      df2cc9:	5d                   	pop    %rbp
      df2cca:	41 5c                	pop    %r12
      df2ccc:	41 5d                	pop    %r13
      df2cce:	e9 5d 91 ee ff       	jmp    cdbe30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13d600>
      df2cd3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2cd8:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2cdc:	48 83 c4 58          	add    $0x58,%rsp
      df2ce0:	5b                   	pop    %rbx
      df2ce1:	5d                   	pop    %rbp
      df2ce2:	41 5c                	pop    %r12
      df2ce4:	41 5d                	pop    %r13
      df2ce6:	e9 a5 04 ee ff       	jmp    cd3190 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x134960>
      df2ceb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2cf0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2cf4:	48 83 c4 58          	add    $0x58,%rsp
      df2cf8:	5b                   	pop    %rbx
      df2cf9:	5d                   	pop    %rbp
      df2cfa:	41 5c                	pop    %r12
      df2cfc:	41 5d                	pop    %r13
      df2cfe:	e9 6d 9f ef ff       	jmp    cecc70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x14e440>
      df2d03:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2d08:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2d0c:	48 83 c4 58          	add    $0x58,%rsp
      df2d10:	5b                   	pop    %rbx
      df2d11:	5d                   	pop    %rbp
      df2d12:	41 5c                	pop    %r12
      df2d14:	41 5d                	pop    %r13
      df2d16:	e9 25 a2 ef ff       	jmp    cecf40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x14e710>
      df2d1b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2d20:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2d24:	48 83 c4 58          	add    $0x58,%rsp
      df2d28:	5b                   	pop    %rbx
      df2d29:	5d                   	pop    %rbp
      df2d2a:	41 5c                	pop    %r12
      df2d2c:	41 5d                	pop    %r13
      df2d2e:	e9 ed 20 ee ff       	jmp    cd4e20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1365f0>
      df2d33:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2d38:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2d3c:	48 83 c4 58          	add    $0x58,%rsp
      df2d40:	5b                   	pop    %rbx
      df2d41:	5d                   	pop    %rbp
      df2d42:	41 5c                	pop    %r12
      df2d44:	41 5d                	pop    %r13
      df2d46:	e9 75 05 ee ff       	jmp    cd32c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x134a90>
      df2d4b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2d50:	f3 0f 5c 05 c4 6a b6 	subss  0x1b66ac4(%rip),%xmm0        # 295981c <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x8b80bc>
      df2d57:	01 
      df2d58:	f3 48 0f 2c d0       	cvttss2si %xmm0,%rdx
      df2d5d:	48 0f ba fa 3f       	btc    $0x3f,%rdx
      df2d62:	e9 c9 fd ff ff       	jmp    df2b30 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254300>
      df2d67:	48 83 c4 58          	add    $0x58,%rsp
      df2d6b:	5b                   	pop    %rbx
      df2d6c:	5d                   	pop    %rbp
      df2d6d:	41 5c                	pop    %r12
      df2d6f:	41 5d                	pop    %r13
      df2d71:	ff e0                	jmp    *%rax
      df2d73:	90                   	nop
      df2d74:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
      df2d7b:	00 00 00 00 
      df2d7f:	90                   	nop
      df2d80:	85 f6                	test   %esi,%esi
      df2d82:	75 1c                	jne    df2da0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254570>
      df2d84:	85 d2                	test   %edx,%edx
      df2d86:	74 28                	je     df2db0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254580>
      df2d88:	83 fa 01             	cmp    $0x1,%edx
      df2d8b:	75 1b                	jne    df2da8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254578>
      df2d8d:	48 8b 07             	mov    (%rdi),%rax
      df2d90:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      df2d94:	ff 60 60             	jmp    *0x60(%rax)
      df2d97:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      df2d9e:	00 00 
      df2da0:	c3                   	ret
      df2da1:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      df2da8:	c3                   	ret
      df2da9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      df2db0:	e9 0b 6c ee ff       	jmp    cd99c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x13b190>
      df2db5:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
      df2dbc:	00 00 00 00 
      df2dc0:	53                   	push   %rbx
      df2dc1:	48 8b 42 08          	mov    0x8(%rdx),%rax
      df2dc5:	48 89 f3             	mov    %rsi,%rbx
      df2dc8:	31 c9                	xor    %ecx,%ecx
      df2dca:	48 8b 32             	mov    (%rdx),%rsi
      df2dcd:	48 89 df             	mov    %rbx,%rdi
      df2dd0:	48 89 c2             	mov    %rax,%rdx
      df2dd3:	e8 e8 a0 6e ff       	call   4dcec0 <QDebug::putByteArray(char const*, unsigned long, QDebug::Latin1Content)@plt>
      df2dd8:	48 8b 3b             	mov    (%rbx),%rdi
      df2ddb:	80 7f 30 00          	cmpb   $0x0,0x30(%rdi)
      df2ddf:	75 07                	jne    df2de8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2545b8>
      df2de1:	5b                   	pop    %rbx
      df2de2:	c3                   	ret
      df2de3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2de8:	be 20 00 00 00       	mov    $0x20,%esi
      df2ded:	5b                   	pop    %rbx
      df2dee:	e9 bd 75 6e ff       	jmp    4da3b0 <QTextStream::operator<<(char)@plt>
      df2df3:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
      df2dfa:	00 00 00 00 
      df2dfe:	66 90                	xchg   %ax,%ax
      df2e00:	53                   	push   %rbx
      df2e01:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2e05:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2e09:	48 89 d3             	mov    %rdx,%rbx
      df2e0c:	48 83 ec 10          	sub    $0x10,%rsp
      df2e10:	8b 06                	mov    (%rsi),%eax
      df2e12:	f3 48 0f 2a c0       	cvtsi2ss %rax,%xmm0
      df2e17:	8b 46 04             	mov    0x4(%rsi),%eax
      df2e1a:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2e1f:	f3 0f 5e c1          	divss  %xmm1,%xmm0
      df2e23:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2e27:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
      df2e2b:	e8 70 bb 6e ff       	call   4de9a0 <fmax@plt>
      df2e30:	48 8b 05 79 d9 b3 01 	mov    0x1b3d979(%rip),%rax        # 29307b0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x88f050>
      df2e37:	66 48 0f 6e c8       	movq   %rax,%xmm1
      df2e3c:	e8 6f 81 6e ff       	call   4dafb0 <fmin@plt>
      df2e41:	8b 03                	mov    (%rbx),%eax
      df2e43:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2e47:	f2 0f 11 44 24 08    	movsd  %xmm0,0x8(%rsp)
      df2e4d:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2e51:	f3 48 0f 2a c0       	cvtsi2ss %rax,%xmm0
      df2e56:	8b 43 04             	mov    0x4(%rbx),%eax
      df2e59:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2e5e:	f3 0f 5e c1          	divss  %xmm1,%xmm0
      df2e62:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2e66:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
      df2e6a:	e8 31 bb 6e ff       	call   4de9a0 <fmax@plt>
      df2e6f:	48 8b 05 3a d9 b3 01 	mov    0x1b3d93a(%rip),%rax        # 29307b0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x88f050>
      df2e76:	66 48 0f 6e c8       	movq   %rax,%xmm1
      df2e7b:	e8 30 81 6e ff       	call   4dafb0 <fmin@plt>
      df2e80:	f2 0f 10 54 24 08    	movsd  0x8(%rsp),%xmm2
      df2e86:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2e8a:	ba 00 00 00 00       	mov    $0x0,%edx
      df2e8f:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
      df2e93:	f2 0f 5a ca          	cvtsd2ss %xmm2,%xmm1
      df2e97:	0f 2e c1             	ucomiss %xmm1,%xmm0
      df2e9a:	0f 9b c0             	setnp  %al
      df2e9d:	0f 45 c2             	cmovne %edx,%eax
      df2ea0:	48 83 c4 10          	add    $0x10,%rsp
      df2ea4:	5b                   	pop    %rbx
      df2ea5:	c3                   	ret
      df2ea6:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
      df2ead:	00 00 00 
      df2eb0:	53                   	push   %rbx
      df2eb1:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2eb5:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2eb9:	48 89 d3             	mov    %rdx,%rbx
      df2ebc:	48 83 ec 10          	sub    $0x10,%rsp
      df2ec0:	8b 06                	mov    (%rsi),%eax
      df2ec2:	f3 48 0f 2a c0       	cvtsi2ss %rax,%xmm0
      df2ec7:	8b 46 04             	mov    0x4(%rsi),%eax
      df2eca:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2ecf:	f3 0f 5e c1          	divss  %xmm1,%xmm0
      df2ed3:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2ed7:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
      df2edb:	e8 c0 ba 6e ff       	call   4de9a0 <fmax@plt>
      df2ee0:	48 8b 05 c9 d8 b3 01 	mov    0x1b3d8c9(%rip),%rax        # 29307b0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x88f050>
      df2ee7:	66 48 0f 6e c8       	movq   %rax,%xmm1
      df2eec:	e8 bf 80 6e ff       	call   4dafb0 <fmin@plt>
      df2ef1:	8b 03                	mov    (%rbx),%eax
      df2ef3:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2ef7:	f2 0f 11 44 24 08    	movsd  %xmm0,0x8(%rsp)
      df2efd:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2f01:	f3 48 0f 2a c0       	cvtsi2ss %rax,%xmm0
      df2f06:	8b 43 04             	mov    0x4(%rbx),%eax
      df2f09:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2f0e:	f3 0f 5e c1          	divss  %xmm1,%xmm0
      df2f12:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2f16:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
      df2f1a:	e8 81 ba 6e ff       	call   4de9a0 <fmax@plt>
      df2f1f:	48 8b 05 8a d8 b3 01 	mov    0x1b3d88a(%rip),%rax        # 29307b0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x88f050>
      df2f26:	66 48 0f 6e c8       	movq   %rax,%xmm1
      df2f2b:	e8 80 80 6e ff       	call   4dafb0 <fmin@plt>
      df2f30:	f2 0f 10 54 24 08    	movsd  0x8(%rsp),%xmm2
      df2f36:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2f3a:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
      df2f3e:	f2 0f 5a ca          	cvtsd2ss %xmm2,%xmm1
      df2f42:	0f 2f c1             	comiss %xmm1,%xmm0
      df2f45:	0f 97 c0             	seta   %al
      df2f48:	48 83 c4 10          	add    $0x10,%rsp
      df2f4c:	5b                   	pop    %rbx
      df2f4d:	c3                   	ret
      df2f4e:	66 90                	xchg   %ax,%ax
      df2f50:	8b 02                	mov    (%rdx),%eax
      df2f52:	66 0f ef c0          	pxor   %xmm0,%xmm0
      df2f56:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2f5a:	53                   	push   %rbx
      df2f5b:	48 89 f3             	mov    %rsi,%rbx
      df2f5e:	f3 48 0f 2a c0       	cvtsi2ss %rax,%xmm0
      df2f63:	8b 42 04             	mov    0x4(%rdx),%eax
      df2f66:	f3 48 0f 2a c8       	cvtsi2ss %rax,%xmm1
      df2f6b:	f3 0f 5e c1          	divss  %xmm1,%xmm0
      df2f6f:	66 0f ef c9          	pxor   %xmm1,%xmm1
      df2f73:	f3 0f 5a c0          	cvtss2sd %xmm0,%xmm0
      df2f77:	e8 24 ba 6e ff       	call   4de9a0 <fmax@plt>
      df2f7c:	f2 0f 10 0d 2c d8 b3 	movsd  0x1b3d82c(%rip),%xmm1        # 29307b0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0x88f050>
      df2f83:	01 
      df2f84:	e8 27 80 6e ff       	call   4dafb0 <fmin@plt>
      df2f89:	48 8b 3b             	mov    (%rbx),%rdi
      df2f8c:	f2 0f 5a c0          	cvtsd2ss %xmm0,%xmm0
      df2f90:	e8 6b b0 6e ff       	call   4de000 <QTextStream::operator<<(float)@plt>
      df2f95:	48 8b 3b             	mov    (%rbx),%rdi
      df2f98:	80 7f 30 00          	cmpb   $0x0,0x30(%rdi)
      df2f9c:	75 02                	jne    df2fa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x254770>
      df2f9e:	5b                   	pop    %rbx
      df2f9f:	c3                   	ret
      df2fa0:	be 20 00 00 00       	mov    $0x20,%esi
      df2fa5:	5b                   	pop    %rbx
      df2fa6:	e9 05 74 6e ff       	jmp    4da3b0 <QTextStream::operator<<(char)@plt>
      df2fab:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      df2fb0:	48 89 f7             	mov    %rsi,%rdi
      df2fb3:	8b 32                	mov    (%rdx),%esi
      df2fb5:	e9 36 7b 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df2fba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df2fc0:	48 89 f7             	mov    %rsi,%rdi
      df2fc3:	8b 32                	mov    (%rdx),%esi
      df2fc5:	e9 26 7b 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df2fca:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df2fd0:	48 89 f7             	mov    %rsi,%rdi
      df2fd3:	8b 32                	mov    (%rdx),%esi
      df2fd5:	e9 16 7b 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df2fda:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df2fe0:	48 89 f7             	mov    %rsi,%rdi
      df2fe3:	8b 32                	mov    (%rdx),%esi
      df2fe5:	e9 06 7b 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df2fea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df2ff0:	48 89 f7             	mov    %rsi,%rdi
      df2ff3:	8b 32                	mov    (%rdx),%esi
      df2ff5:	e9 f6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df2ffa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3000:	48 89 f7             	mov    %rsi,%rdi
      df3003:	8b 32                	mov    (%rdx),%esi
      df3005:	e9 e6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df300a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3010:	48 89 f7             	mov    %rsi,%rdi
      df3013:	8b 32                	mov    (%rdx),%esi
      df3015:	e9 d6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df301a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3020:	48 89 f7             	mov    %rsi,%rdi
      df3023:	8b 32                	mov    (%rdx),%esi
      df3025:	e9 c6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df302a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3030:	48 89 f7             	mov    %rsi,%rdi
      df3033:	8b 32                	mov    (%rdx),%esi
      df3035:	e9 b6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df303a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3040:	48 89 f7             	mov    %rsi,%rdi
      df3043:	8b 32                	mov    (%rdx),%esi
      df3045:	e9 a6 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df304a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3050:	48 89 f7             	mov    %rsi,%rdi
      df3053:	8b 32                	mov    (%rdx),%esi
      df3055:	e9 96 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df305a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3060:	48 89 f7             	mov    %rsi,%rdi
      df3063:	8b 32                	mov    (%rdx),%esi
      df3065:	e9 86 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df306a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3070:	48 89 f7             	mov    %rsi,%rdi
      df3073:	8b 32                	mov    (%rdx),%esi
      df3075:	e9 76 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df307a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3080:	48 89 f7             	mov    %rsi,%rdi
      df3083:	8b 32                	mov    (%rdx),%esi
      df3085:	e9 66 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df308a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3090:	48 89 f7             	mov    %rsi,%rdi
      df3093:	8b 32                	mov    (%rdx),%esi
      df3095:	e9 56 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df309a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30a0:	48 89 f7             	mov    %rsi,%rdi
      df30a3:	8b 32                	mov    (%rdx),%esi
      df30a5:	e9 46 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30aa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30b0:	48 89 f7             	mov    %rsi,%rdi
      df30b3:	8b 32                	mov    (%rdx),%esi
      df30b5:	e9 36 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30c0:	48 89 f7             	mov    %rsi,%rdi
      df30c3:	8b 32                	mov    (%rdx),%esi
      df30c5:	e9 26 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30ca:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30d0:	48 89 f7             	mov    %rsi,%rdi
      df30d3:	8b 32                	mov    (%rdx),%esi
      df30d5:	e9 16 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30e0:	48 89 f7             	mov    %rsi,%rdi
      df30e3:	8b 32                	mov    (%rdx),%esi
      df30e5:	e9 06 7a 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30ea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df30f0:	48 89 f7             	mov    %rsi,%rdi
      df30f3:	8b 32                	mov    (%rdx),%esi
      df30f5:	e9 f6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df30fa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3100:	48 89 f7             	mov    %rsi,%rdi
      df3103:	8b 32                	mov    (%rdx),%esi
      df3105:	e9 e6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df310a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3110:	48 89 f7             	mov    %rsi,%rdi
      df3113:	8b 32                	mov    (%rdx),%esi
      df3115:	e9 d6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df311a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3120:	48 89 f7             	mov    %rsi,%rdi
      df3123:	8b 32                	mov    (%rdx),%esi
      df3125:	e9 c6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df312a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3130:	48 89 f7             	mov    %rsi,%rdi
      df3133:	8b 32                	mov    (%rdx),%esi
      df3135:	e9 b6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df313a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3140:	48 89 f7             	mov    %rsi,%rdi
      df3143:	8b 32                	mov    (%rdx),%esi
      df3145:	e9 a6 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df314a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3150:	48 89 f7             	mov    %rsi,%rdi
      df3153:	8b 32                	mov    (%rdx),%esi
      df3155:	e9 96 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df315a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3160:	48 89 f7             	mov    %rsi,%rdi
      df3163:	8b 32                	mov    (%rdx),%esi
      df3165:	e9 86 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df316a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3170:	48 89 f7             	mov    %rsi,%rdi
      df3173:	8b 32                	mov    (%rdx),%esi
      df3175:	e9 76 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df317a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3180:	48 89 f7             	mov    %rsi,%rdi
      df3183:	8b 32                	mov    (%rdx),%esi
      df3185:	e9 66 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df318a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3190:	48 89 f7             	mov    %rsi,%rdi
      df3193:	8b 32                	mov    (%rdx),%esi
      df3195:	e9 56 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df319a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31a0:	48 89 f7             	mov    %rsi,%rdi
      df31a3:	8b 32                	mov    (%rdx),%esi
      df31a5:	e9 46 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31aa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31b0:	48 89 f7             	mov    %rsi,%rdi
      df31b3:	8b 32                	mov    (%rdx),%esi
      df31b5:	e9 36 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31c0:	48 89 f7             	mov    %rsi,%rdi
      df31c3:	8b 32                	mov    (%rdx),%esi
      df31c5:	e9 26 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31ca:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31d0:	48 89 f7             	mov    %rsi,%rdi
      df31d3:	8b 32                	mov    (%rdx),%esi
      df31d5:	e9 16 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31e0:	48 89 f7             	mov    %rsi,%rdi
      df31e3:	8b 32                	mov    (%rdx),%esi
      df31e5:	e9 06 79 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31ea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df31f0:	48 89 f7             	mov    %rsi,%rdi
      df31f3:	8b 32                	mov    (%rdx),%esi
      df31f5:	e9 f6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df31fa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3200:	48 89 f7             	mov    %rsi,%rdi
      df3203:	8b 32                	mov    (%rdx),%esi
      df3205:	e9 e6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df320a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3210:	48 89 f7             	mov    %rsi,%rdi
      df3213:	8b 32                	mov    (%rdx),%esi
      df3215:	e9 d6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df321a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3220:	48 89 f7             	mov    %rsi,%rdi
      df3223:	8b 32                	mov    (%rdx),%esi
      df3225:	e9 c6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df322a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3230:	48 89 f7             	mov    %rsi,%rdi
      df3233:	8b 32                	mov    (%rdx),%esi
      df3235:	e9 b6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df323a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3240:	48 89 f7             	mov    %rsi,%rdi
      df3243:	8b 32                	mov    (%rdx),%esi
      df3245:	e9 a6 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df324a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3250:	48 89 f7             	mov    %rsi,%rdi
      df3253:	8b 32                	mov    (%rdx),%esi
      df3255:	e9 96 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df325a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3260:	48 89 f7             	mov    %rsi,%rdi
      df3263:	8b 32                	mov    (%rdx),%esi
      df3265:	e9 86 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df326a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3270:	48 89 f7             	mov    %rsi,%rdi
      df3273:	8b 32                	mov    (%rdx),%esi
      df3275:	e9 76 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df327a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3280:	48 89 f7             	mov    %rsi,%rdi
      df3283:	8b 32                	mov    (%rdx),%esi
      df3285:	e9 66 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df328a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3290:	48 89 f7             	mov    %rsi,%rdi
      df3293:	8b 32                	mov    (%rdx),%esi
      df3295:	e9 56 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df329a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32a0:	48 89 f7             	mov    %rsi,%rdi
      df32a3:	8b 32                	mov    (%rdx),%esi
      df32a5:	e9 46 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32aa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32b0:	48 89 f7             	mov    %rsi,%rdi
      df32b3:	8b 32                	mov    (%rdx),%esi
      df32b5:	e9 36 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32ba:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32c0:	48 89 f7             	mov    %rsi,%rdi
      df32c3:	8b 32                	mov    (%rdx),%esi
      df32c5:	e9 26 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32ca:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32d0:	48 89 f7             	mov    %rsi,%rdi
      df32d3:	8b 32                	mov    (%rdx),%esi
      df32d5:	e9 16 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32e0:	48 89 f7             	mov    %rsi,%rdi
      df32e3:	8b 32                	mov    (%rdx),%esi
      df32e5:	e9 06 78 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32ea:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df32f0:	48 89 f7             	mov    %rsi,%rdi
      df32f3:	8b 32                	mov    (%rdx),%esi
      df32f5:	e9 f6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df32fa:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3300:	48 89 f7             	mov    %rsi,%rdi
      df3303:	8b 32                	mov    (%rdx),%esi
      df3305:	e9 e6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df330a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3310:	48 89 f7             	mov    %rsi,%rdi
      df3313:	8b 32                	mov    (%rdx),%esi
      df3315:	e9 d6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df331a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3320:	48 89 f7             	mov    %rsi,%rdi
      df3323:	8b 32                	mov    (%rdx),%esi
      df3325:	e9 c6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df332a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3330:	48 89 f7             	mov    %rsi,%rdi
      df3333:	8b 32                	mov    (%rdx),%esi
      df3335:	e9 b6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df333a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3340:	48 89 f7             	mov    %rsi,%rdi
      df3343:	8b 32                	mov    (%rdx),%esi
      df3345:	e9 a6 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df334a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      df3350:	48 89 f7             	mov    %rsi,%rdi
      df3353:	8b 32                	mov    (%rdx),%esi
      df3355:	e9 96 77 6e ff       	jmp    4daaf0 <QDataStream::operator<<(int)@plt>
      df335a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
MATCH index=1 score=180 qmetaobject_va=0x30877c0 file_off=0x30877c0
  superdata=0x0 stringdata=0x1cd8268 metadata=0x1cd8060 static_metacall=0xd05f20 related=0x0 metatypes=0x2f6aa20 extra=0x0
  metadata_header=13 0 0 0 13 14 0 0 0 0 0 0 0 2
  STRING_RUNS
    +0x4 (
    +0x34 *
    +0x4c 0
    +0x64 3
    +0x7c 7
    +0x94 :
    +0xa4 8
    +0xbc 4
    +0xd4 :
    +0xec 8
    +0x104 :
    +0x114 *
    +0x11c &
    +0x124 *
    +0x130 tibia::chat::TChatProtocolMessageHandler
    +0x159 currentlyAvailableChannels
    +0x175 TChatChannelIdentifierList
    +0x190 ChannelList
    +0x19c publishGameAction
    +0x1ae std::shared_ptr<tibia::input::IGameAction>
    +0x1d9 pGameAction
    +0x1e5 handleTalkMessage
    +0x1f7 tibia::protobuf::protocol::GameserverMessageTalk
    +0x228 TalkMessage
    +0x234 handleMessageMessage
    +0x249 tibia::protobuf::protocol::GameserverMessageMessage
    +0x27d MessageMessage
    +0x28c handleOpenChannelMessage
    +0x2a5 tibia::protobuf::protocol::GameserverMessageOpenChannel
    +0x2dd OpenChannelMessage
    +0x2f0 handleOpenOwnChannelMessage
    +0x30c tibia::protobuf::protocol::GameserverMessageOpenOwnChannel
    +0x347 handleCloseChannelMessage
    +0x361 tibia::protobuf::protocol::GameserverMessageCloseChannel
    +0x39a CloseChannelMessage
    +0x3ae handleChannelsMessage
    +0x3c4 tibia::protobuf::protocol::GameserverMessageChannels
    +0x3f9 ChannelsMessage
    +0x409 handlePrivateChannelMessage
    +0x425 tibia::protobuf::protocol::GameserverMessagePrivateChannel
    +0x460 PrivateChannel
    +0x46f handleChannelEventMessage
    +0x489 tibia::protobuf::protocol::GameserverMessageChannelEvent
    +0x4c2 ChannelEvent
    +0x4cf handleNpcTalkPartersMessage
    +0x4eb tibia::protobuf::protocol::GameserverMessageNpcTalkParters
    +0x526 Message
    +0x52e onChatProtocolMessageHandlerOptionsChanged
    +0x559 onPlayerCreatureAddedToCreatureStorage
    +0x580 std::weak_ptr<tibia::creatures::TCreature>
    +0x5ab pPlayer
    +0x5f8 b
    +0x610 e
    +0x628 h
    +0x640 k
    +0x658 n
    +0x670 q
    +0x688 t
    +0x6a0 w
    +0x6b8 z
    +0x6d0 }
    +0x6e0  
    +0x6f8 #
    +0x710 &
    +0x728 )
    +0x740 +
    +0x74c +
    +0x758 +
    +0x764 +
    +0x770 +
    +0x77c +
    +0x788 +
    +0x794 +
    +0x7a0 +
    +0x7ac +
    +0x7b8 +
    +0x7bc !
    +0x7c0 "
    +0x7c4 +
    +0x7c8 $
    +0x7cc %
    +0x7d0 +
    +0x7d4 '
    +0x7d8 (
    +0x7dc +
    +0x7e0 *
    +0x7e4 +
    +0x7f0 0
    +0x808 *
    +0x820 3
    +0x838 6
    +0x850 7
    +0x868 2
    +0x880 5
    +0x898 4
    +0x8b0 7
    +0x8c8 5
    +0x8e0 7
    +0x8f8 7
    +0x910 7
    +0x928 8
    +0x940 5
    +0x94c tibia::worldmap::TWorldmapProtocolMessageHandler
    +0x97d publishGameAction
    +0x990 std::shared_ptr<tibia::input::IGameAction>
    +0x9bb pGameAction
    +0x9c7 handleFullMapMessage
    +0x9dc tibia::protobuf::protocol::GameserverMessageFullMap
    +0xa10 FullMapMessage
    +0xa1f handleLeftColumnMessage
    +0xa37 tibia::protobuf::protocol::GameserverMessageLeftColumn
    +0xa6e LeftColumnMessage
    +0xa80 handleRightColumnMessage
    +0xa99 tibia::protobuf::protocol::GameserverMessageRightColumn
    +0xad1 RightColumnMessage
    +0xae4 handleTopRowMessage
    +0xaf8 tibia::protobuf::protocol::GameserverMessageTopRow
    +0xb2b TopRowMessage
    +0xb39 handleBottomRowMessage
    +0xb50 tibia::protobuf::protocol::GameserverMessageBottomRow
    +0xb86 BottomRowMessage
    +0xb97 handleTopFloorMessage
    +0xbad tibia::protobuf::protocol::GameserverMessageTopFloor
    +0xbe2 TopFloorMessage
    +0xbf2 handleBottomFloorMessage
    +0xc0b tibia::protobuf::protocol::GameserverMessageBottomFloor
    +0xc43 BottomFloorMessage
    +0xc56 handleFieldDataMessage
    +0xc6d tibia::protobuf::protocol::GameserverMessageFieldData
    +0xca3 FieldDataMessage
    +0xcb4 handleCreateOnMapMessage
    +0xccd tibia::protobuf::protocol::GameserverMessageCreateOnMap
    +0xd05 CreateOnMapMessage
    +0xd18 handleChangeOnMapMessage
    +0xd31 tibia::protobuf::protocol::GameserverMessageChangeOnMap
    +0xd69 ChangeOnMapMessage
    +0xd7c handleDeleteOnMapMessage
    +0xd95 tibia::protobuf::protocol::GameserverMessageDeleteOnMap
    +0xdcd DeleteOnMapMessage
    +0xde0 handleAmbientLightMessage
    +0xdfa tibia::protobuf::protocol::GameserverMessageAmbientLight
    +0xe33 AmbientLightMessage
    +0xe47 handleTibiaTimeMessage
    +0xe5e tibia::protobuf::protocol::GameserverMessageTibiaTime
    +0xe94 TibiaTimeMessage
    +0xf08 +
    +0xf18 (
    +0xf1c +
    +0xf20 T
    +0xf28 f
    +0xf30 g
    +0xf34 *
    +0xf40 tibia::creatures::TBattleListRenderProvider
    +0xf6c publishGameAction
    +0xf7f std::shared_ptr<tibia::input::IGameAction>
    +0xfaa pGameAction
    +0x1008 +
    +0x1018 (
    +0x1020 G
    +0x1028 Y
    +0x1030 Z
    +0x1034 *
    +0x1040 tibia::effects::TEffectStorage
    +0x105f publishGameAction
    +0x1072 std::shared_ptr<tibia::input::IGameAction>
    +0x109d pGameAction
    +0x10f8 2
    +0x1110 5
    +0x1128 8
    +0x1140 ;
    +0x1158 >
    +0x1170 ?
    +0x1180 +
    +0x118c +
    +0x1198 +
    +0x11a4 +
    +0x11b0 +
    +0x11b4 +
    +0x11c4 p
    +0x11c8 /
    +0x11e0 *
    +0x1200 2
    +0x1210 E
    +0x1218 $
    +0x1220 &
    +0x1228 *
    +0x1234 tibia::skillwheel::TSkillWheelGameActionHandler
    +0x1264 publishGameAction
    +0x1277 std::shared_ptr<tibia::input::IGameAction>
    +0x12a2 pGameAction
    +0x12ae handleGameAction
    +0x12bf handleCreatureGameAction
    +0x12d8 std::shared_ptr<tibia::input::TGameActionCreature>
    +0x130b handleSaveSkillWheelGameAction
    +0x132a std::shared_ptr<tibia::input::gameactions::TGameActionSaveSkillWheel>
    +0x1370 handleRequestOnwSkillWheelGameAction
    +0x1395 onPlayerCreatureAddedToCreatureStorage
    +0x13bc std::weak_ptr<tibia::creatures::TCreature>
    +0x13e7 pPlayerWeak
    +0x1438 2
    +0x1450 7
    +0x1468 <
    +0x1480 ?
    +0x1498 B
    +0x14b0 E
    +0x14c0 +
    +0x14d4 +
    +0x14e8 +
    +0x14f4 +
    +0x1500 +
    +0x150c +
    +0x151c x
    +0x1520  
    +0x1538 +
    +0x1570 *
    +0x1580 &
    +0x1588 *
    +0x1594 tibia::chat::TChatChannelStorage
    +0x15b5 newChatChannelOpened
    +0x15cb std::shared_ptr<TChatChannelIdentifierBase>
    +0x15f7 pIdentifier
    +0x1603 ForceMakeCurrent
    +0x1614 chatChannelReopend
    +0x1627 chatChannelRemoved
    +0x163a entryAddedToChatChannel
    +0x1652 publishGameAction
    +0x1664 std::shared_ptr<tibia::input::IGameAction>
    +0x168f pGameAction
    +0x169b onPlayerCreatureAddedToCreatureStorage
    +0x16c2 std::weak_ptr<tibia::creatures::TCreature>
    +0x16ed pPlayer
    +0x16f8 tibia::sound::TObjectSoundController
    +0x1788 +
    +0x17a8 H
    +0x17ac $
    +0x17b0 m
    +0x17b4  
    +0x17e4 !
    +0x17f0 tibia::sound::TObjectSoundController
    +0x1815 ambienceObjectStreamCountChanged
    +0x1837 tibia::sound::TSoundEffectID
    +0x1854 SoundEffectID
    +0x1862 size_t
    +0x1869 Count
    +0x186f tibia::worldmap::TWorldMapExtentX
    +0x1891 NearestDistance
    +0x18b8 N5tibia5input30TGenericChatGameActionProviderE
    +0x18f8 St23_Sp_counted_ptr_inplaceIN5tibia5input30TGenericChatGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1978 N5tibia9creatures24TPartyGameActionProviderE
    +0x19b8 St23_Sp_counted_ptr_inplaceIN5tibia9creatures24TPartyGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1a38 N5tibia9creatures32TInspectPlayerGameActionProviderE
    +0x1a78 St23_Sp_counted_ptr_inplaceIN5tibia9creatures32TInspectPlayerGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1af8 N5tibia5input32TGenericObjectGameActionProviderE
    +0x1b38 St23_Sp_counted_ptr_inplaceIN5tibia5input32TGenericObjectGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1bb8 N5tibia9creatures29TBattleListGameActionProviderE
    +0x1bf8 St23_Sp_counted_ptr_inplaceIN5tibia9creatures29TBattleListGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1c78 N5tibia8worldmap27TWorldMapGameActionProviderE
    +0x1cb8 St23_Sp_counted_ptr_inplaceIN5tibia8worldmap27TWorldMapGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1d38 N5tibia9container28TContainerGameActionProviderE
    +0x1d78 St23_Sp_counted_ptr_inplaceIN5tibia9container28TContainerGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1df8 N5tibia4chat23TChatGameActionProviderE
    +0x1e38 St23_Sp_counted_ptr_inplaceIN5tibia4chat23TChatGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1eb8 N5tibia9creatures28TVipWidgetGameActionProviderE
    +0x1ef8 St23_Sp_counted_ptr_inplaceIN5tibia9creatures28TVipWidgetGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1f78 N5tibia5input25TGenericGameActionHandlerE
  STATIC_METACALL_DISASSEMBLY
    
    /data/client-15.32.df7b29/bin/client:     file format elf64-x86-64
    
    
    Disassembly of section .text:
    
    0000000000d05f20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1676f0>:
      d05f20:	85 f6                	test   %esi,%esi
      d05f22:	75 24                	jne    d05f48 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167718>
      d05f24:	83 fa 0c             	cmp    $0xc,%edx
      d05f27:	77 4d                	ja     d05f76 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167746>
      d05f29:	55                   	push   %rbp
      d05f2a:	48 8d 35 3b 7f 06 01 	lea    0x1067f3b(%rip),%rsi        # 1d6de6c <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x848ac>
      d05f31:	89 d2                	mov    %edx,%edx
      d05f33:	53                   	push   %rbx
      d05f34:	48 83 ec 18          	sub    $0x18,%rsp
      d05f38:	48 63 04 96          	movslq (%rsi,%rdx,4),%rax
      d05f3c:	48 01 f0             	add    %rsi,%rax
      d05f3f:	ff e0                	jmp    *%rax
      d05f41:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d05f48:	83 fe 05             	cmp    $0x5,%esi
      d05f4b:	75 29                	jne    d05f76 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167746>
      d05f4d:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d05f51:	48 8b 31             	mov    (%rcx),%rsi
      d05f54:	48 8d 0d c5 d8 fe ff 	lea    -0x1273b(%rip),%rcx        # cf3820 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x154ff0>
      d05f5b:	48 8b 10             	mov    (%rax),%rdx
      d05f5e:	48 39 ca             	cmp    %rcx,%rdx
      d05f61:	74 1d                	je     d05f80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167750>
      d05f63:	48 8d 0d e6 d8 fe ff 	lea    -0x1271a(%rip),%rcx        # cf3850 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x155020>
      d05f6a:	48 39 ca             	cmp    %rcx,%rdx
      d05f6d:	75 07                	jne    d05f76 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167746>
      d05f6f:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
      d05f74:	74 1a                	je     d05f90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167760>
      d05f76:	c3                   	ret
      d05f77:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d05f7e:	00 00 
      d05f80:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
      d05f85:	75 ef                	jne    d05f76 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167746>
      d05f87:	c7 06 00 00 00 00    	movl   $0x0,(%rsi)
      d05f8d:	c3                   	ret
      d05f8e:	66 90                	xchg   %ax,%ax
      d05f90:	c7 06 01 00 00 00    	movl   $0x1,(%rsi)
      d05f96:	c3                   	ret
      d05f97:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d05f9e:	00 00 
      d05fa0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d05fa4:	48 83 c4 18          	add    $0x18,%rsp
      d05fa8:	5b                   	pop    %rbx
      d05fa9:	5d                   	pop    %rbp
      d05faa:	e9 41 01 14 00       	jmp    e460f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2a78c0>
      d05faf:	90                   	nop
      d05fb0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d05fb4:	48 83 c4 18          	add    $0x18,%rsp
      d05fb8:	5b                   	pop    %rbx
      d05fb9:	5d                   	pop    %rbp
      d05fba:	e9 51 40 13 00       	jmp    e3a010 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x29b7e0>
      d05fbf:	90                   	nop
      d05fc0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d05fc4:	48 83 c4 18          	add    $0x18,%rsp
      d05fc8:	5b                   	pop    %rbx
      d05fc9:	5d                   	pop    %rbp
      d05fca:	e9 91 2c 13 00       	jmp    e38c60 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x29a430>
      d05fcf:	90                   	nop
      d05fd0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d05fd4:	48 83 c4 18          	add    $0x18,%rsp
      d05fd8:	5b                   	pop    %rbx
      d05fd9:	5d                   	pop    %rbp
      d05fda:	e9 c1 2e 13 00       	jmp    e38ea0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x29a670>
      d05fdf:	90                   	nop
      d05fe0:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d05fe4:	31 d2                	xor    %edx,%edx
      d05fe6:	48 89 e1             	mov    %rsp,%rcx
      d05fe9:	48 8d 35 d0 17 38 02 	lea    0x23817d0(%rip),%rsi        # 30877c0 <QObject::staticMetaObject@Qt_6>
      d05ff0:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
      d05ff7:	00 
      d05ff8:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
      d05ffd:	e8 be 8d 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d06002:	48 83 c4 18          	add    $0x18,%rsp
      d06006:	5b                   	pop    %rbx
      d06007:	5d                   	pop    %rbp
      d06008:	c3                   	ret
      d06009:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d06010:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06014:	ba 01 00 00 00       	mov    $0x1,%edx
      d06019:	48 89 e1             	mov    %rsp,%rcx
      d0601c:	48 8d 35 9d 17 38 02 	lea    0x238179d(%rip),%rsi        # 30877c0 <QObject::staticMetaObject@Qt_6>
      d06023:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
      d0602a:	00 
      d0602b:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
      d06030:	e8 8b 8d 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d06035:	eb cb                	jmp    d06002 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1677d2>
      d06037:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d0603e:	00 00 
      d06040:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d06044:	48 83 c4 18          	add    $0x18,%rsp
      d06048:	5b                   	pop    %rbx
      d06049:	5d                   	pop    %rbp
      d0604a:	e9 91 30 13 00       	jmp    e390e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x29a8b0>
      d0604f:	90                   	nop
      d06050:	48 83 c4 18          	add    $0x18,%rsp
      d06054:	5b                   	pop    %rbx
      d06055:	5d                   	pop    %rbp
      d06056:	e9 e5 4a 12 00       	jmp    e2ab40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x28c310>
      d0605b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06060:	48 8b 7f 30          	mov    0x30(%rdi),%rdi
      d06064:	48 8b 51 08          	mov    0x8(%rcx),%rdx
      d06068:	48 89 e6             	mov    %rsp,%rsi
      d0606b:	48 8b 07             	mov    (%rdi),%rax
      d0606e:	8b 52 18             	mov    0x18(%rdx),%edx
      d06071:	48 8b 80 d0 00 00 00 	mov    0xd0(%rax),%rax
      d06078:	66 89 14 24          	mov    %dx,(%rsp)
      d0607c:	ff d0                	call   *%rax
      d0607e:	eb 82                	jmp    d06002 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1677d2>
      d06080:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d06084:	48 83 c4 18          	add    $0x18,%rsp
      d06088:	5b                   	pop    %rbx
      d06089:	5d                   	pop    %rbp
      d0608a:	e9 d1 11 14 00       	jmp    e47260 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2a8a30>
      d0608f:	90                   	nop
      d06090:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d06094:	48 83 c4 18          	add    $0x18,%rsp
      d06098:	5b                   	pop    %rbx
      d06099:	5d                   	pop    %rbp
      d0609a:	e9 61 8c 12 00       	jmp    e2ed00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2904d0>
      d0609f:	90                   	nop
      d060a0:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d060a4:	48 83 c4 18          	add    $0x18,%rsp
      d060a8:	5b                   	pop    %rbx
      d060a9:	5d                   	pop    %rbp
      d060aa:	e9 f1 e2 12 00       	jmp    e343a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x295b70>
      d060af:	90                   	nop
      d060b0:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d060b4:	f3 0f 6f 00          	movdqu (%rax),%xmm0
      d060b8:	0f 29 04 24          	movaps %xmm0,(%rsp)
      d060bc:	48 8b 5c 24 08       	mov    0x8(%rsp),%rbx
      d060c1:	48 85 db             	test   %rbx,%rbx
      d060c4:	74 2a                	je     d060f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1678c0>
      d060c6:	48 8b 05 db 99 42 02 	mov    0x24299db(%rip),%rax        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
      d060cd:	80 38 00             	cmpb   $0x0,(%rax)
      d060d0:	74 2b                	je     d060fd <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1678cd>
      d060d2:	83 43 0c 01          	addl   $0x1,0xc(%rbx)
      d060d6:	48 89 e6             	mov    %rsp,%rsi
      d060d9:	e8 22 1b 14 00       	call   e47c00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2a93d0>
      d060de:	48 83 c4 18          	add    $0x18,%rsp
      d060e2:	48 89 df             	mov    %rbx,%rdi
      d060e5:	5b                   	pop    %rbx
      d060e6:	5d                   	pop    %rbp
      d060e7:	e9 f4 38 9b ff       	jmp    6b99e0 <std::runtime_error::~runtime_error()@plt+0x1d9710>
      d060ec:	0f 1f 40 00          	nopl   0x0(%rax)
      d060f0:	48 89 e6             	mov    %rsp,%rsi
      d060f3:	e8 08 1b 14 00       	call   e47c00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2a93d0>
      d060f8:	e9 05 ff ff ff       	jmp    d06002 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1677d2>
      d060fd:	f0 83 43 0c 01       	lock addl $0x1,0xc(%rbx)
      d06102:	48 8b 5c 24 08       	mov    0x8(%rsp),%rbx
      d06107:	48 89 e6             	mov    %rsp,%rsi
      d0610a:	e8 f1 1a 14 00       	call   e47c00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x2a93d0>
      d0610f:	48 85 db             	test   %rbx,%rbx
      d06112:	75 ca                	jne    d060de <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1678ae>
      d06114:	e9 e9 fe ff ff       	jmp    d06002 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1677d2>
      d06119:	48 89 c5             	mov    %rax,%rbp
      d0611c:	e9 04 6b 84 ff       	jmp    54cc25 <std::runtime_error::~runtime_error()@plt+0x6c955>
      d06121:	48 89 c5             	mov    %rax,%rbp
      d06124:	e9 f7 6a 84 ff       	jmp    54cc20 <std::runtime_error::~runtime_error()@plt+0x6c950>
      d06129:	90                   	nop
      d0612a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d06130:	09 f2                	or     %esi,%edx
      d06132:	74 04                	je     d06138 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167908>
      d06134:	c3                   	ret
      d06135:	0f 1f 00             	nopl   (%rax)
      d06138:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d0613c:	41 54                	push   %r12
      d0613e:	55                   	push   %rbp
      d0613f:	53                   	push   %rbx
      d06140:	48 8b 68 08          	mov    0x8(%rax),%rbp
      d06144:	48 89 fb             	mov    %rdi,%rbx
      d06147:	48 8b 10             	mov    (%rax),%rdx
      d0614a:	48 85 ed             	test   %rbp,%rbp
      d0614d:	74 71                	je     d061c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167990>
      d0614f:	4c 8b 25 52 99 42 02 	mov    0x2429952(%rip),%r12        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
      d06156:	41 80 3c 24 00       	cmpb   $0x0,(%r12)
      d0615b:	0f 84 8f 00 00 00    	je     d061f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1679c0>
      d06161:	8b 45 0c             	mov    0xc(%rbp),%eax
      d06164:	48 89 57 28          	mov    %rdx,0x28(%rdi)
      d06168:	83 c0 01             	add    $0x1,%eax
      d0616b:	48 8b 7b 30          	mov    0x30(%rbx),%rdi
      d0616f:	8d 50 01             	lea    0x1(%rax),%edx
      d06172:	89 55 0c             	mov    %edx,0xc(%rbp)
      d06175:	48 85 ff             	test   %rdi,%rdi
      d06178:	74 26                	je     d061a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167970>
      d0617a:	8b 47 0c             	mov    0xc(%rdi),%eax
      d0617d:	8d 50 ff             	lea    -0x1(%rax),%edx
      d06180:	89 57 0c             	mov    %edx,0xc(%rdi)
      d06183:	83 f8 01             	cmp    $0x1,%eax
      d06186:	75 06                	jne    d0618e <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x16795e>
      d06188:	48 8b 07             	mov    (%rdi),%rax
      d0618b:	ff 50 18             	call   *0x18(%rax)
      d0618e:	48 89 6b 30          	mov    %rbp,0x30(%rbx)
      d06192:	48 85 ed             	test   %rbp,%rbp
      d06195:	0f 85 95 00 00 00    	jne    d06230 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167a00>
      d0619b:	5b                   	pop    %rbx
      d0619c:	5d                   	pop    %rbp
      d0619d:	41 5c                	pop    %r12
      d0619f:	c3                   	ret
      d061a0:	48 89 6b 30          	mov    %rbp,0x30(%rbx)
      d061a4:	89 45 0c             	mov    %eax,0xc(%rbp)
      d061a7:	83 fa 01             	cmp    $0x1,%edx
      d061aa:	75 ef                	jne    d0619b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x16796b>
      d061ac:	48 8b 45 00          	mov    0x0(%rbp),%rax
      d061b0:	5b                   	pop    %rbx
      d061b1:	48 89 ef             	mov    %rbp,%rdi
      d061b4:	5d                   	pop    %rbp
      d061b5:	41 5c                	pop    %r12
      d061b7:	48 8b 40 18          	mov    0x18(%rax),%rax
      d061bb:	ff e0                	jmp    *%rax
      d061bd:	0f 1f 00             	nopl   (%rax)
      d061c0:	48 89 57 28          	mov    %rdx,0x28(%rdi)
      d061c4:	48 8b 7f 30          	mov    0x30(%rdi),%rdi
      d061c8:	48 85 ff             	test   %rdi,%rdi
      d061cb:	74 ce                	je     d0619b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x16796b>
      d061cd:	4c 8b 25 d4 98 42 02 	mov    0x24298d4(%rip),%r12        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
      d061d4:	41 80 3c 24 00       	cmpb   $0x0,(%r12)
      d061d9:	75 9f                	jne    d0617a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x16794a>
      d061db:	b8 ff ff ff ff       	mov    $0xffffffff,%eax
      d061e0:	f0 0f c1 47 0c       	lock xadd %eax,0xc(%rdi)
      d061e5:	eb 9c                	jmp    d06183 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167953>
      d061e7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d061ee:	00 00 
      d061f0:	48 8d 45 0c          	lea    0xc(%rbp),%rax
      d061f4:	f0 83 45 0c 01       	lock addl $0x1,0xc(%rbp)
      d061f9:	41 80 3c 24 00       	cmpb   $0x0,(%r12)
      d061fe:	48 89 57 28          	mov    %rdx,0x28(%rdi)
      d06202:	75 47                	jne    d0624b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167a1b>
      d06204:	f0 83 45 0c 01       	lock addl $0x1,0xc(%rbp)
      d06209:	48 8b 7f 30          	mov    0x30(%rdi),%rdi
      d0620d:	48 85 ff             	test   %rdi,%rdi
      d06210:	75 c2                	jne    d061d4 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1679a4>
      d06212:	48 89 6b 30          	mov    %rbp,0x30(%rbx)
      d06216:	41 80 3c 24 00       	cmpb   $0x0,(%r12)
      d0621b:	75 23                	jne    d06240 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167a10>
      d0621d:	ba ff ff ff ff       	mov    $0xffffffff,%edx
      d06222:	f0 0f c1 10          	lock xadd %edx,(%rax)
      d06226:	e9 7c ff ff ff       	jmp    d061a7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167977>
      d0622b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06230:	48 8d 45 0c          	lea    0xc(%rbp),%rax
      d06234:	eb e0                	jmp    d06216 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1679e6>
      d06236:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
      d0623d:	00 00 00 
      d06240:	8b 55 0c             	mov    0xc(%rbp),%edx
      d06243:	8d 42 ff             	lea    -0x1(%rdx),%eax
      d06246:	e9 59 ff ff ff       	jmp    d061a4 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167974>
      d0624b:	8b 45 0c             	mov    0xc(%rbp),%eax
      d0624e:	e9 18 ff ff ff       	jmp    d0616b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x16793b>
      d06253:	90                   	nop
      d06254:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
      d0625b:	00 00 00 00 
      d0625f:	90                   	nop
      d06260:	85 f6                	test   %esi,%esi
      d06262:	75 24                	jne    d06288 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167a58>
      d06264:	83 fa 2b             	cmp    $0x2b,%edx
      d06267:	77 71                	ja     d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d06269:	55                   	push   %rbp
      d0626a:	48 8d 35 2f 7c 06 01 	lea    0x1067c2f(%rip),%rsi        # 1d6dea0 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x848e0>
      d06271:	89 d2                	mov    %edx,%edx
      d06273:	53                   	push   %rbx
      d06274:	48 89 fb             	mov    %rdi,%rbx
      d06277:	48 83 ec 48          	sub    $0x48,%rsp
      d0627b:	48 63 04 96          	movslq (%rsi,%rdx,4),%rax
      d0627f:	48 01 f0             	add    %rsi,%rax
      d06282:	ff e0                	jmp    *%rax
      d06284:	0f 1f 40 00          	nopl   0x0(%rax)
      d06288:	83 fe 05             	cmp    $0x5,%esi
      d0628b:	75 4d                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d0628d:	48 8b 51 08          	mov    0x8(%rcx),%rdx
      d06291:	48 8b 31             	mov    (%rcx),%rsi
      d06294:	48 8d 0d e5 d8 fe ff 	lea    -0x1271b(%rip),%rcx        # cf3b80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x155350>
      d0629b:	48 8b 02             	mov    (%rdx),%rax
      d0629e:	48 39 c8             	cmp    %rcx,%rax
      d062a1:	74 3d                	je     d062e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167ab0>
      d062a3:	48 8d 0d 06 d9 fe ff 	lea    -0x126fa(%rip),%rcx        # cf3bb0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x155380>
      d062aa:	48 39 c8             	cmp    %rcx,%rax
      d062ad:	74 41                	je     d062f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167ac0>
      d062af:	48 8d 0d 1a d9 fe ff 	lea    -0x126e6(%rip),%rcx        # cf3bd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1553a0>
      d062b6:	48 39 c8             	cmp    %rcx,%rax
      d062b9:	74 45                	je     d06300 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167ad0>
      d062bb:	48 8d 0d 4e d9 fe ff 	lea    -0x126b2(%rip),%rcx        # cf3c10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1553e0>
      d062c2:	48 39 c8             	cmp    %rcx,%rax
      d062c5:	74 49                	je     d06310 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167ae0>
      d062c7:	48 8d 0d 22 d8 fe ff 	lea    -0x127de(%rip),%rcx        # cf3af0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1552c0>
      d062ce:	48 39 c8             	cmp    %rcx,%rax
      d062d1:	75 4d                	jne    d06320 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167af0>
      d062d3:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d062d8:	74 66                	je     d06340 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b10>
      d062da:	c3                   	ret
      d062db:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d062e0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d062e5:	75 f3                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d062e7:	c7 06 00 00 00 00    	movl   $0x0,(%rsi)
      d062ed:	c3                   	ret
      d062ee:	66 90                	xchg   %ax,%ax
      d062f0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d062f5:	75 e3                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d062f7:	c7 06 01 00 00 00    	movl   $0x1,(%rsi)
      d062fd:	c3                   	ret
      d062fe:	66 90                	xchg   %ax,%ax
      d06300:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d06305:	75 d3                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d06307:	c7 06 02 00 00 00    	movl   $0x2,(%rsi)
      d0630d:	c3                   	ret
      d0630e:	66 90                	xchg   %ax,%ax
      d06310:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d06315:	75 c3                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d06317:	c7 06 03 00 00 00    	movl   $0x3,(%rsi)
      d0631d:	c3                   	ret
      d0631e:	66 90                	xchg   %ax,%ax
      d06320:	48 8d 0d 29 d9 fe ff 	lea    -0x126d7(%rip),%rcx        # cf3c50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x155420>
      d06327:	48 39 c8             	cmp    %rcx,%rax
      d0632a:	75 ae                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d0632c:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d06331:	75 a7                	jne    d062da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167aaa>
      d06333:	c7 06 05 00 00 00    	movl   $0x5,(%rsi)
      d06339:	c3                   	ret
      d0633a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d06340:	c7 06 04 00 00 00    	movl   $0x4,(%rsi)
      d06346:	c3                   	ret
      d06347:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d0634e:	00 00 
      d06350:	e8 db d9 9e ff       	call   6f3d30 <std::runtime_error::~runtime_error()@plt+0x213a60>
      d06355:	48 89 df             	mov    %rbx,%rdi
      d06358:	e8 73 94 9e ff       	call   6ef7d0 <std::runtime_error::~runtime_error()@plt+0x20f500>
      d0635d:	84 c0                	test   %al,%al
      d0635f:	75 32                	jne    d06393 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b63>
      d06361:	48 83 c4 48          	add    $0x48,%rsp
      d06365:	5b                   	pop    %rbx
      d06366:	5d                   	pop    %rbp
      d06367:	c3                   	ret
      d06368:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
      d0636f:	00 
      d06370:	48 8b 07             	mov    (%rdi),%rax
      d06373:	48 8d 15 86 98 9e ff 	lea    -0x61677a(%rip),%rdx        # 6efc00 <std::runtime_error::~runtime_error()@plt+0x20f930>
      d0637a:	48 8b 80 d0 00 00 00 	mov    0xd0(%rax),%rax
      d06381:	48 39 d0             	cmp    %rdx,%rax
      d06384:	0f 85 5c 01 00 00    	jne    d064e6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167cb6>
      d0638a:	e8 41 94 9e ff       	call   6ef7d0 <std::runtime_error::~runtime_error()@plt+0x20f500>
      d0638f:	84 c0                	test   %al,%al
      d06391:	74 ce                	je     d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d06393:	48 83 c4 48          	add    $0x48,%rsp
      d06397:	48 89 df             	mov    %rbx,%rdi
      d0639a:	31 c9                	xor    %ecx,%ecx
      d0639c:	ba 01 00 00 00       	mov    $0x1,%edx
      d063a1:	5b                   	pop    %rbx
      d063a2:	48 8d 35 f7 ba 25 02 	lea    0x225baf7(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d063a9:	5d                   	pop    %rbp
      d063aa:	e9 11 8a 7d ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d063af:	90                   	nop
      d063b0:	48 8b bf 90 09 00 00 	mov    0x990(%rdi),%rdi
      d063b7:	48 83 c4 48          	add    $0x48,%rsp
      d063bb:	5b                   	pop    %rbx
      d063bc:	5d                   	pop    %rbp
      d063bd:	e9 ae fd da ff       	jmp    ab6170 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, unsigned long, std::allocator<char> const&)@@Base+0x384850>
      d063c2:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d063c8:	48 8b 41 10          	mov    0x10(%rcx),%rax
      d063cc:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
      d063d1:	48 89 4c 24 08       	mov    %rcx,0x8(%rsp)
      d063d6:	48 8d 6c 24 20       	lea    0x20(%rsp),%rbp
      d063db:	48 8b 10             	mov    (%rax),%rdx
      d063de:	48 8d 70 08          	lea    0x8(%rax),%rsi
      d063e2:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
      d063e7:	e8 34 ca 9a ff       	call   6b2e20 <std::runtime_error::~runtime_error()@plt+0x1d2b50>
      d063ec:	48 8b 4c 24 08       	mov    0x8(%rsp),%rcx
      d063f1:	48 89 ea             	mov    %rbp,%rdx
      d063f4:	48 89 df             	mov    %rbx,%rdi
      d063f7:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d063fb:	e8 50 6d 9e ff       	call   6ed150 <std::runtime_error::~runtime_error()@plt+0x20ce80>
      d06400:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
      d06405:	48 85 ff             	test   %rdi,%rdi
      d06408:	0f 84 53 ff ff ff    	je     d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d0640e:	e8 cd d6 9a ff       	call   6b3ae0 <std::runtime_error::~runtime_error()@plt+0x1d3810>
      d06413:	e9 49 ff ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d06418:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
      d0641f:	00 
      d06420:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06424:	0f b6 30             	movzbl (%rax),%esi
      d06427:	48 83 c4 48          	add    $0x48,%rsp
      d0642b:	5b                   	pop    %rbx
      d0642c:	5d                   	pop    %rbp
      d0642d:	e9 ee 6d 9e ff       	jmp    6ed220 <std::runtime_error::~runtime_error()@plt+0x20cf50>
      d06432:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d06438:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d0643c:	48 83 c4 48          	add    $0x48,%rsp
      d06440:	5b                   	pop    %rbx
      d06441:	5d                   	pop    %rbp
      d06442:	e9 a9 8c 9e ff       	jmp    6ef0f0 <std::runtime_error::~runtime_error()@plt+0x20ee20>
      d06447:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d0644e:	00 00 
      d06450:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06454:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
      d06459:	48 8d 6c 24 20       	lea    0x20(%rsp),%rbp
      d0645e:	48 8b 10             	mov    (%rax),%rdx
      d06461:	48 8d 70 08          	lea    0x8(%rax),%rsi
      d06465:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
      d0646a:	e8 f1 c8 9e ff       	call   6f2d60 <std::runtime_error::~runtime_error()@plt+0x212a90>
      d0646f:	48 89 ee             	mov    %rbp,%rsi
      d06472:	48 89 df             	mov    %rbx,%rdi
      d06475:	e8 66 e1 9e ff       	call   6f45e0 <std::runtime_error::~runtime_error()@plt+0x214310>
      d0647a:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
      d0647f:	48 85 ff             	test   %rdi,%rdi
      d06482:	0f 84 d9 fe ff ff    	je     d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d06488:	e8 53 35 9b ff       	call   6b99e0 <std::runtime_error::~runtime_error()@plt+0x1d9710>
      d0648d:	e9 cf fe ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d06492:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d06498:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d0649c:	31 d2                	xor    %edx,%edx
      d0649e:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d064a3:	48 8d 35 f6 b9 25 02 	lea    0x225b9f6(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d064aa:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d064b1:	00 00 
      d064b3:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d064b8:	e8 03 89 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d064bd:	e9 9f fe ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d064c2:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d064c8:	48 83 c4 48          	add    $0x48,%rsp
      d064cc:	5b                   	pop    %rbx
      d064cd:	5d                   	pop    %rbp
      d064ce:	e9 2d b2 9f ff       	jmp    701700 <std::runtime_error::~runtime_error()@plt+0x221430>
      d064d3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d064d8:	48 8b bf 38 0a 00 00 	mov    0xa38(%rdi),%rdi
      d064df:	48 8b 07             	mov    (%rdi),%rax
      d064e2:	48 8b 40 70          	mov    0x70(%rax),%rax
      d064e6:	48 83 c4 48          	add    $0x48,%rsp
      d064ea:	5b                   	pop    %rbx
      d064eb:	5d                   	pop    %rbp
      d064ec:	ff e0                	jmp    *%rax
      d064ee:	66 90                	xchg   %ax,%ax
      d064f0:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d064f4:	8b 30                	mov    (%rax),%esi
      d064f6:	48 83 c4 48          	add    $0x48,%rsp
      d064fa:	5b                   	pop    %rbx
      d064fb:	5d                   	pop    %rbp
      d064fc:	e9 2f b7 9f ff       	jmp    701c30 <std::runtime_error::~runtime_error()@plt+0x221960>
      d06501:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d06508:	48 83 c4 48          	add    $0x48,%rsp
      d0650c:	5b                   	pop    %rbx
      d0650d:	5d                   	pop    %rbp
      d0650e:	e9 8d bc 9f ff       	jmp    7021a0 <std::runtime_error::~runtime_error()@plt+0x221ed0>
      d06513:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06518:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d0651c:	8b 30                	mov    (%rax),%esi
      d0651e:	48 83 c4 48          	add    $0x48,%rsp
      d06522:	5b                   	pop    %rbx
      d06523:	5d                   	pop    %rbp
      d06524:	e9 67 7b 9e ff       	jmp    6ee090 <std::runtime_error::~runtime_error()@plt+0x20ddc0>
      d06529:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d06530:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06534:	ba 05 00 00 00       	mov    $0x5,%edx
      d06539:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d0653e:	48 8d 35 5b b9 25 02 	lea    0x225b95b(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d06545:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d0654c:	00 00 
      d0654e:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d06553:	e8 68 88 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d06558:	e9 04 fe ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d0655d:	0f 1f 00             	nopl   (%rax)
      d06560:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06564:	f3 0f 6f 08          	movdqu (%rax),%xmm1
      d06568:	48 8b 10             	mov    (%rax),%rdx
      d0656b:	0f 29 4c 24 20       	movaps %xmm1,0x20(%rsp)
      d06570:	48 8b 40 10          	mov    0x10(%rax),%rax
      d06574:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
      d06579:	48 85 d2             	test   %rdx,%rdx
      d0657c:	74 04                	je     d06582 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167d52>
      d0657e:	f0 83 02 01          	lock addl $0x1,(%rdx)
      d06582:	48 8d 6c 24 20       	lea    0x20(%rsp),%rbp
      d06587:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
      d0658c:	ba 04 00 00 00       	mov    $0x4,%edx
      d06591:	48 89 df             	mov    %rbx,%rdi
      d06594:	48 8d 35 05 b9 25 02 	lea    0x225b905(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d0659b:	48 89 6c 24 18       	mov    %rbp,0x18(%rsp)
      d065a0:	48 c7 44 24 10 00 00 	movq   $0x0,0x10(%rsp)
      d065a7:	00 00 
      d065a9:	e8 12 88 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d065ae:	48 89 ef             	mov    %rbp,%rdi
      d065b1:	e8 0a be 9a ff       	call   6b23c0 <std::runtime_error::~runtime_error()@plt+0x1d20f0>
      d065b6:	e9 a6 fd ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d065bb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d065c0:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d065c4:	ba 03 00 00 00       	mov    $0x3,%edx
      d065c9:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d065ce:	48 8d 35 cb b8 25 02 	lea    0x225b8cb(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d065d5:	8b 00                	mov    (%rax),%eax
      d065d7:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d065de:	00 00 
      d065e0:	89 44 24 10          	mov    %eax,0x10(%rsp)
      d065e4:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
      d065e9:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d065ee:	e8 cd 87 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d065f3:	e9 69 fd ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d065f8:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
      d065ff:	00 
      d06600:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d06604:	48 83 c4 48          	add    $0x48,%rsp
      d06608:	5b                   	pop    %rbx
      d06609:	5d                   	pop    %rbp
      d0660a:	e9 91 74 9f ff       	jmp    6fdaa0 <std::runtime_error::~runtime_error()@plt+0x21d7d0>
      d0660f:	90                   	nop
      d06610:	48 83 c4 48          	add    $0x48,%rsp
      d06614:	5b                   	pop    %rbx
      d06615:	5d                   	pop    %rbp
      d06616:	e9 75 db 9e ff       	jmp    6f4190 <std::runtime_error::~runtime_error()@plt+0x213ec0>
      d0661b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06620:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06624:	ba 02 00 00 00       	mov    $0x2,%edx
      d06629:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d0662e:	48 8d 35 6b b8 25 02 	lea    0x225b86b(%rip),%rsi        # 2f61ea0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xec0740>
      d06635:	48 8b 00             	mov    (%rax),%rax
      d06638:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d0663f:	00 00 
      d06641:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
      d06646:	48 8d 44 24 10       	lea    0x10(%rsp),%rax
      d0664b:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d06650:	e8 6b 87 7d ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d06655:	e9 07 fd ff ff       	jmp    d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d0665a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d06660:	48 83 c4 48          	add    $0x48,%rsp
      d06664:	5b                   	pop    %rbx
      d06665:	5d                   	pop    %rbp
      d06666:	e9 65 8b 9e ff       	jmp    6ef1d0 <std::runtime_error::~runtime_error()@plt+0x20ef00>
      d0666b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06670:	48 8b 9f 48 03 00 00 	mov    0x348(%rdi),%rbx
      d06677:	48 89 df             	mov    %rbx,%rdi
      d0667a:	e8 21 f1 b4 ff       	call   8557a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, unsigned long, std::allocator<char> const&)@@Base+0x123e80>
      d0667f:	48 8b 7b 68          	mov    0x68(%rbx),%rdi
      d06683:	48 83 c4 48          	add    $0x48,%rsp
      d06687:	5b                   	pop    %rbx
      d06688:	5d                   	pop    %rbp
      d06689:	e9 22 6b fe ff       	jmp    ced1b0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x14e980>
      d0668e:	66 90                	xchg   %ax,%ax
      d06690:	48 83 c4 48          	add    $0x48,%rsp
      d06694:	5b                   	pop    %rbx
      d06695:	5d                   	pop    %rbp
      d06696:	e9 a5 43 9e ff       	jmp    6eaa40 <std::runtime_error::~runtime_error()@plt+0x20a770>
      d0669b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d066a0:	48 8b 07             	mov    (%rdi),%rax
      d066a3:	48 8b 80 c8 00 00 00 	mov    0xc8(%rax),%rax
      d066aa:	48 83 c4 48          	add    $0x48,%rsp
      d066ae:	5b                   	pop    %rbx
      d066af:	5d                   	pop    %rbp
      d066b0:	ff e0                	jmp    *%rax
      d066b2:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d066b8:	48 83 c4 48          	add    $0x48,%rsp
      d066bc:	5b                   	pop    %rbx
      d066bd:	5d                   	pop    %rbp
      d066be:	e9 5d 59 9f ff       	jmp    6fc020 <std::runtime_error::~runtime_error()@plt+0x21bd50>
      d066c3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d066c8:	e8 93 b2 9e ff       	call   6f1960 <std::runtime_error::~runtime_error()@plt+0x211690>
      d066cd:	48 83 c4 48          	add    $0x48,%rsp
      d066d1:	48 89 df             	mov    %rbx,%rdi
      d066d4:	5b                   	pop    %rbx
      d066d5:	5d                   	pop    %rbp
      d066d6:	e9 d5 b7 9e ff       	jmp    6f1eb0 <std::runtime_error::~runtime_error()@plt+0x211be0>
      d066db:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d066e0:	48 83 c4 48          	add    $0x48,%rsp
      d066e4:	5b                   	pop    %rbx
      d066e5:	5d                   	pop    %rbp
      d066e6:	e9 45 7a 9e ff       	jmp    6ee130 <std::runtime_error::~runtime_error()@plt+0x20de60>
      d066eb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d066f0:	48 83 c4 48          	add    $0x48,%rsp
      d066f4:	5b                   	pop    %rbx
      d066f5:	5d                   	pop    %rbp
      d066f6:	e9 a5 95 9e ff       	jmp    6efca0 <std::runtime_error::~runtime_error()@plt+0x20f9d0>
      d066fb:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06700:	48 83 c4 48          	add    $0x48,%rsp
      d06704:	5b                   	pop    %rbx
      d06705:	5d                   	pop    %rbp
      d06706:	e9 95 db 9e ff       	jmp    6f42a0 <std::runtime_error::~runtime_error()@plt+0x213fd0>
      d0670b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06710:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d06714:	bf 20 00 00 00       	mov    $0x20,%edi
      d06719:	48 63 28             	movslq (%rax),%rbp
      d0671c:	e8 4f 8f 7d ff       	call   4df670 <operator new(unsigned long)@plt>
      d06721:	48 8d 3d 38 d1 9d ff 	lea    -0x622ec8(%rip),%rdi        # 6e3860 <std::runtime_error::~runtime_error()@plt+0x203590>
      d06728:	31 f6                	xor    %esi,%esi
      d0672a:	48 89 da             	mov    %rbx,%rdx
      d0672d:	48 69 ed 40 42 0f 00 	imul   $0xf4240,%rbp,%rbp
      d06734:	c7 00 01 00 00 00    	movl   $0x1,(%rax)
      d0673a:	48 89 c1             	mov    %rax,%rcx
      d0673d:	48 8d 05 5c db 9e ff 	lea    -0x6124a4(%rip),%rax        # 6f42a0 <std::runtime_error::~runtime_error()@plt+0x213fd0>
      d06744:	66 48 0f 6e c7       	movq   %rdi,%xmm0
      d06749:	66 48 0f 6e d0       	movq   %rax,%xmm2
      d0674e:	48 c7 41 18 00 00 00 	movq   $0x0,0x18(%rcx)
      d06755:	00 
      d06756:	66 0f 6c c2          	punpcklqdq %xmm2,%xmm0
      d0675a:	48 81 fd ff 93 35 77 	cmp    $0x773593ff,%rbp
      d06761:	0f 11 41 08          	movups %xmm0,0x8(%rcx)
      d06765:	48 89 ef             	mov    %rbp,%rdi
      d06768:	40 0f 9f c6          	setg   %sil
      d0676c:	48 83 c4 48          	add    $0x48,%rsp
      d06770:	5b                   	pop    %rbx
      d06771:	5d                   	pop    %rbp
      d06772:	e9 d9 92 7d ff       	jmp    4dfa50 <QTimer::singleShotImpl(std::chrono::duration<long, std::ratio<1l, 1000000000l> >, Qt::TimerType, QObject const*, QtPrivate::QSlotObjectBase*)@plt>
      d06777:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d0677e:	00 00 
      d06780:	48 83 c4 48          	add    $0x48,%rsp
      d06784:	5b                   	pop    %rbx
      d06785:	5d                   	pop    %rbp
      d06786:	e9 f5 0f 9e ff       	jmp    6e7780 <std::runtime_error::~runtime_error()@plt+0x2074b0>
      d0678b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d06790:	48 83 c4 48          	add    $0x48,%rsp
      d06794:	5b                   	pop    %rbx
      d06795:	5d                   	pop    %rbp
      d06796:	e9 55 11 9e ff       	jmp    6e78f0 <std::runtime_error::~runtime_error()@plt+0x207620>
      d0679b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d067a0:	48 83 c4 48          	add    $0x48,%rsp
      d067a4:	5b                   	pop    %rbx
      d067a5:	5d                   	pop    %rbp
      d067a6:	e9 b5 bd 9f ff       	jmp    702560 <std::runtime_error::~runtime_error()@plt+0x222290>
      d067ab:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d067b0:	48 8b 87 18 04 00 00 	mov    0x418(%rdi),%rax
      d067b7:	48 85 c0             	test   %rax,%rax
      d067ba:	74 0d                	je     d067c9 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167f99>
      d067bc:	80 b8 ec 00 00 00 00 	cmpb   $0x0,0xec(%rax)
      d067c3:	0f 85 25 01 00 00    	jne    d068ee <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1680be>
      d067c9:	48 89 df             	mov    %rbx,%rdi
      d067cc:	e8 2f af 9f ff       	call   701700 <std::runtime_error::~runtime_error()@plt+0x221430>
      d067d1:	48 8b bb d0 08 00 00 	mov    0x8d0(%rbx),%rdi
      d067d8:	48 8b 47 70          	mov    0x70(%rdi),%rax
      d067dc:	48 85 c0             	test   %rax,%rax
      d067df:	74 13                	je     d067f4 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167fc4>
      d067e1:	48 8b 10             	mov    (%rax),%rdx
      d067e4:	48 89 c7             	mov    %rax,%rdi
      d067e7:	ff 92 80 00 00 00    	call   *0x80(%rdx)
      d067ed:	48 8b bb d0 08 00 00 	mov    0x8d0(%rbx),%rdi
      d067f4:	80 bf 60 06 00 00 00 	cmpb   $0x0,0x660(%rdi)
      d067fb:	0f 84 60 fb ff ff    	je     d06361 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x167b31>
      d06801:	48 83 c4 48          	add    $0x48,%rsp
      d06805:	5b                   	pop    %rbx
      d06806:	5d                   	pop    %rbp
      d06807:	e9 44 e8 a6 ff       	jmp    775050 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, unsigned long, std::allocator<char> const&)@@Base+0x43730>
      d0680c:	0f 1f 40 00          	nopl   0x0(%rax)
      d06810:	48 83 c4 48          	add    $0x48,%rsp
      d06814:	5b                   	pop    %rbx
      d06815:	5d                   	pop    %rbp
      d06816:	e9 65 7c 9f ff       	jmp    6fe480 <std::runtime_error::~runtime_error()@plt+0x21e1b0>
      d0681b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
MATCH index=2 score=180 qmetaobject_va=0x3087780 file_off=0x3087780
  superdata=0x0 stringdata=0x1cd7f44 metadata=0x1cd7ec0 static_metacall=0xd20800 related=0x0 metatypes=0x2f6a9e0 extra=0x0
  metadata_header=13 0 0 0 2 14 0 0 0 0 0 0 0 1
  STRING_RUNS
    +0x0 @
    +0x4 -
    +0x8 n
    +0x1c *
    +0x34 5
    +0x40 tibia::game::TGameEventProtocolMessageHandler
    +0x6e publishGameAction
    +0x81 std::shared_ptr<tibia::input::IGameAction>
    +0xac pGameAction
    +0xb8 handleGameEventMessage
    +0xcf tibia::protobuf::protocol::GameserverMessageGameEvent
    +0x105 GameEventMessage
    +0x15c \
    +0x174 _
    +0x18c b
    +0x1a4 e
    +0x1bc h
    +0x1d4 k
    +0x1ec n
    +0x204 q
    +0x21c t
    +0x234 w
    +0x24c z
    +0x25c "
    +0x264 }
    +0x274 #
    +0x27c ~
    +0x28c +
    +0x298 +
    +0x2a4 +
    +0x2b0 +
    +0x2bc +
    +0x2c8 +
    +0x2d4 +
    +0x2e0 +
    +0x2ec +
    +0x2f8 +
    +0x304 +
    +0x308  
    +0x30c !
    +0x310 +
    +0x314 +
    +0x318 $
    +0x31c %
    +0x328 (
    +0x358 *
    +0x370 0
    +0x388 3
    +0x3a0 7
    +0x3b8 :
    +0x3c8 8
    +0x3e0 4
    +0x3f8 :
    +0x410 8
    +0x428 :
    +0x438 *
    +0x440 &
    +0x448 *
    +0x454 tibia::chat::TChatProtocolMessageHandler
    +0x47d currentlyAvailableChannels
    +0x499 TChatChannelIdentifierList
    +0x4b4 ChannelList
    +0x4c0 publishGameAction
    +0x4d2 std::shared_ptr<tibia::input::IGameAction>
    +0x4fd pGameAction
    +0x509 handleTalkMessage
    +0x51b tibia::protobuf::protocol::GameserverMessageTalk
    +0x54c TalkMessage
    +0x558 handleMessageMessage
    +0x56d tibia::protobuf::protocol::GameserverMessageMessage
    +0x5a1 MessageMessage
    +0x5b0 handleOpenChannelMessage
    +0x5c9 tibia::protobuf::protocol::GameserverMessageOpenChannel
    +0x601 OpenChannelMessage
    +0x614 handleOpenOwnChannelMessage
    +0x630 tibia::protobuf::protocol::GameserverMessageOpenOwnChannel
    +0x66b handleCloseChannelMessage
    +0x685 tibia::protobuf::protocol::GameserverMessageCloseChannel
    +0x6be CloseChannelMessage
    +0x6d2 handleChannelsMessage
    +0x6e8 tibia::protobuf::protocol::GameserverMessageChannels
    +0x71d ChannelsMessage
    +0x72d handlePrivateChannelMessage
    +0x749 tibia::protobuf::protocol::GameserverMessagePrivateChannel
    +0x784 PrivateChannel
    +0x793 handleChannelEventMessage
    +0x7ad tibia::protobuf::protocol::GameserverMessageChannelEvent
    +0x7e6 ChannelEvent
    +0x7f3 handleNpcTalkPartersMessage
    +0x80f tibia::protobuf::protocol::GameserverMessageNpcTalkParters
    +0x84a Message
    +0x852 onChatProtocolMessageHandlerOptionsChanged
    +0x87d onPlayerCreatureAddedToCreatureStorage
    +0x8a4 std::weak_ptr<tibia::creatures::TCreature>
    +0x8cf pPlayer
    +0x91c b
    +0x934 e
    +0x94c h
    +0x964 k
    +0x97c n
    +0x994 q
    +0x9ac t
    +0x9c4 w
    +0x9dc z
    +0x9f4 }
    +0xa04  
    +0xa1c #
    +0xa34 &
    +0xa4c )
    +0xa64 +
    +0xa70 +
    +0xa7c +
    +0xa88 +
    +0xa94 +
    +0xaa0 +
    +0xaac +
    +0xab8 +
    +0xac4 +
    +0xad0 +
    +0xadc +
    +0xae0 !
    +0xae4 "
    +0xae8 +
    +0xaec $
    +0xaf0 %
    +0xaf4 +
    +0xaf8 '
    +0xafc (
    +0xb00 +
    +0xb04 *
    +0xb08 +
    +0xb14 0
    +0xb2c *
    +0xb44 3
    +0xb5c 6
    +0xb74 7
    +0xb8c 2
    +0xba4 5
    +0xbbc 4
    +0xbd4 7
    +0xbec 5
    +0xc04 7
    +0xc1c 7
    +0xc34 7
    +0xc4c 8
    +0xc64 5
    +0xc70 tibia::worldmap::TWorldmapProtocolMessageHandler
    +0xca1 publishGameAction
    +0xcb4 std::shared_ptr<tibia::input::IGameAction>
    +0xcdf pGameAction
    +0xceb handleFullMapMessage
    +0xd00 tibia::protobuf::protocol::GameserverMessageFullMap
    +0xd34 FullMapMessage
    +0xd43 handleLeftColumnMessage
    +0xd5b tibia::protobuf::protocol::GameserverMessageLeftColumn
    +0xd92 LeftColumnMessage
    +0xda4 handleRightColumnMessage
    +0xdbd tibia::protobuf::protocol::GameserverMessageRightColumn
    +0xdf5 RightColumnMessage
    +0xe08 handleTopRowMessage
    +0xe1c tibia::protobuf::protocol::GameserverMessageTopRow
    +0xe4f TopRowMessage
    +0xe5d handleBottomRowMessage
    +0xe74 tibia::protobuf::protocol::GameserverMessageBottomRow
    +0xeaa BottomRowMessage
    +0xebb handleTopFloorMessage
    +0xed1 tibia::protobuf::protocol::GameserverMessageTopFloor
    +0xf06 TopFloorMessage
    +0xf16 handleBottomFloorMessage
    +0xf2f tibia::protobuf::protocol::GameserverMessageBottomFloor
    +0xf67 BottomFloorMessage
    +0xf7a handleFieldDataMessage
    +0xf91 tibia::protobuf::protocol::GameserverMessageFieldData
    +0xfc7 FieldDataMessage
    +0xfd8 handleCreateOnMapMessage
    +0xff1 tibia::protobuf::protocol::GameserverMessageCreateOnMap
    +0x1029 CreateOnMapMessage
    +0x103c handleChangeOnMapMessage
    +0x1055 tibia::protobuf::protocol::GameserverMessageChangeOnMap
    +0x108d ChangeOnMapMessage
    +0x10a0 handleDeleteOnMapMessage
    +0x10b9 tibia::protobuf::protocol::GameserverMessageDeleteOnMap
    +0x10f1 DeleteOnMapMessage
    +0x1104 handleAmbientLightMessage
    +0x111e tibia::protobuf::protocol::GameserverMessageAmbientLight
    +0x1157 AmbientLightMessage
    +0x116b handleTibiaTimeMessage
    +0x1182 tibia::protobuf::protocol::GameserverMessageTibiaTime
    +0x11b8 TibiaTimeMessage
    +0x122c +
    +0x123c (
    +0x1240 +
    +0x1244 T
    +0x124c f
    +0x1254 g
    +0x1258 *
    +0x1264 tibia::creatures::TBattleListRenderProvider
    +0x1290 publishGameAction
    +0x12a3 std::shared_ptr<tibia::input::IGameAction>
    +0x12ce pGameAction
    +0x132c +
    +0x133c (
    +0x1344 G
    +0x134c Y
    +0x1354 Z
    +0x1358 *
    +0x1364 tibia::effects::TEffectStorage
    +0x1383 publishGameAction
    +0x1396 std::shared_ptr<tibia::input::IGameAction>
    +0x13c1 pGameAction
    +0x141c 2
    +0x1434 5
    +0x144c 8
    +0x1464 ;
    +0x147c >
    +0x1494 ?
    +0x14a4 +
    +0x14b0 +
    +0x14bc +
    +0x14c8 +
    +0x14d4 +
    +0x14d8 +
    +0x14e8 p
    +0x14ec /
    +0x1504 *
    +0x1524 2
    +0x1534 E
    +0x153c $
    +0x1544 &
    +0x154c *
    +0x1558 tibia::skillwheel::TSkillWheelGameActionHandler
    +0x1588 publishGameAction
    +0x159b std::shared_ptr<tibia::input::IGameAction>
    +0x15c6 pGameAction
    +0x15d2 handleGameAction
    +0x15e3 handleCreatureGameAction
    +0x15fc std::shared_ptr<tibia::input::TGameActionCreature>
    +0x162f handleSaveSkillWheelGameAction
    +0x164e std::shared_ptr<tibia::input::gameactions::TGameActionSaveSkillWheel>
    +0x1694 handleRequestOnwSkillWheelGameAction
    +0x16b9 onPlayerCreatureAddedToCreatureStorage
    +0x16e0 std::weak_ptr<tibia::creatures::TCreature>
    +0x170b pPlayerWeak
    +0x175c 2
    +0x1774 7
    +0x178c <
    +0x17a4 ?
    +0x17bc B
    +0x17d4 E
    +0x17e4 +
    +0x17f8 +
    +0x180c +
    +0x1818 +
    +0x1824 +
    +0x1830 +
    +0x1840 x
    +0x1844  
    +0x185c +
    +0x1894 *
    +0x18a4 &
    +0x18ac *
    +0x18b8 tibia::chat::TChatChannelStorage
    +0x18d9 newChatChannelOpened
    +0x18ef std::shared_ptr<TChatChannelIdentifierBase>
    +0x191b pIdentifier
    +0x1927 ForceMakeCurrent
    +0x1938 chatChannelReopend
    +0x194b chatChannelRemoved
    +0x195e entryAddedToChatChannel
    +0x1976 publishGameAction
    +0x1988 std::shared_ptr<tibia::input::IGameAction>
    +0x19b3 pGameAction
    +0x19bf onPlayerCreatureAddedToCreatureStorage
    +0x19e6 std::weak_ptr<tibia::creatures::TCreature>
    +0x1a11 pPlayer
    +0x1a1c tibia::sound::TObjectSoundController
    +0x1aac +
    +0x1acc H
    +0x1ad0 $
    +0x1ad4 m
    +0x1ad8  
    +0x1b08 !
    +0x1b14 tibia::sound::TObjectSoundController
    +0x1b39 ambienceObjectStreamCountChanged
    +0x1b5b tibia::sound::TSoundEffectID
    +0x1b78 SoundEffectID
    +0x1b86 size_t
    +0x1b8d Count
    +0x1b93 tibia::worldmap::TWorldMapExtentX
    +0x1bb5 NearestDistance
    +0x1bdc N5tibia5input30TGenericChatGameActionProviderE
    +0x1c1c St23_Sp_counted_ptr_inplaceIN5tibia5input30TGenericChatGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1c9c N5tibia9creatures24TPartyGameActionProviderE
    +0x1cdc St23_Sp_counted_ptr_inplaceIN5tibia9creatures24TPartyGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1d5c N5tibia9creatures32TInspectPlayerGameActionProviderE
    +0x1d9c St23_Sp_counted_ptr_inplaceIN5tibia9creatures32TInspectPlayerGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1e1c N5tibia5input32TGenericObjectGameActionProviderE
    +0x1e5c St23_Sp_counted_ptr_inplaceIN5tibia5input32TGenericObjectGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1edc N5tibia9creatures29TBattleListGameActionProviderE
    +0x1f1c St23_Sp_counted_ptr_inplaceIN5tibia9creatures29TBattleListGameActionProviderESaIvELN9__gnu_cxx12_Lock_policyE2EE
    +0x1f9c N5tibia8worldmap27TWorldMapGameActionProviderE
  STATIC_METACALL_DISASSEMBLY
    
    /data/client-15.32.df7b29/bin/client:     file format elf64-x86-64
    
    
    Disassembly of section .text:
    
    0000000000d20800 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x181fd0>:
      d20800:	85 f6                	test   %esi,%esi
      d20802:	75 1c                	jne    d20820 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x181ff0>
      d20804:	85 d2                	test   %edx,%edx
      d20806:	74 40                	je     d20848 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182018>
      d20808:	83 fa 01             	cmp    $0x1,%edx
      d2080b:	75 33                	jne    d20840 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182010>
      d2080d:	48 8b 71 08          	mov    0x8(%rcx),%rsi
      d20811:	e9 ca 6c b1 ff       	jmp    8374e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string(char const*, unsigned long, std::allocator<char> const&)@@Base+0x105bc0>
      d20816:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
      d2081d:	00 00 00 
      d20820:	83 fe 05             	cmp    $0x5,%esi
      d20823:	75 10                	jne    d20835 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182005>
      d20825:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d20829:	48 8d 15 30 8f ff ff 	lea    -0x70d0(%rip),%rdx        # d19760 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17af30>
      d20830:	48 39 10             	cmp    %rdx,(%rax)
      d20833:	74 43                	je     d20878 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182048>
      d20835:	c3                   	ret
      d20836:	66 2e 0f 1f 84 00 00 	cs nopw 0x0(%rax,%rax,1)
      d2083d:	00 00 00 
      d20840:	c3                   	ret
      d20841:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20848:	48 83 ec 18          	sub    $0x18,%rsp
      d2084c:	48 8b 41 08          	mov    0x8(%rcx),%rax
      d20850:	31 d2                	xor    %edx,%edx
      d20852:	48 8d 35 27 6f 36 02 	lea    0x2366f27(%rip),%rsi        # 3087780 <QObject::staticMetaObject@Qt_6>
      d20859:	48 89 e1             	mov    %rsp,%rcx
      d2085c:	48 c7 04 24 00 00 00 	movq   $0x0,(%rsp)
      d20863:	00 
      d20864:	48 89 44 24 08       	mov    %rax,0x8(%rsp)
      d20869:	e8 52 e5 7b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d2086e:	48 83 c4 18          	add    $0x18,%rsp
      d20872:	c3                   	ret
      d20873:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d20878:	48 83 78 08 00       	cmpq   $0x0,0x8(%rax)
      d2087d:	75 b6                	jne    d20835 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182005>
      d2087f:	48 8b 01             	mov    (%rcx),%rax
      d20882:	c7 00 00 00 00 00    	movl   $0x0,(%rax)
      d20888:	c3                   	ret
      d20889:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20890:	66 0f ef c0          	pxor   %xmm0,%xmm0
      d20894:	48 8d 05 95 c7 37 02 	lea    0x237c795(%rip),%rax        # 309d030 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0xd050>
      d2089b:	48 c7 46 08 00 00 00 	movq   $0x0,0x8(%rsi)
      d208a2:	00 
      d208a3:	48 89 06             	mov    %rax,(%rsi)
      d208a6:	48 c7 46 30 00 00 00 	movq   $0x0,0x30(%rsi)
      d208ad:	00 
      d208ae:	0f 11 46 10          	movups %xmm0,0x10(%rsi)
      d208b2:	0f 11 46 20          	movups %xmm0,0x20(%rsi)
      d208b6:	48 39 d6             	cmp    %rdx,%rsi
      d208b9:	0f 84 91 00 00 00    	je     d20950 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182120>
      d208bf:	55                   	push   %rbp
      d208c0:	48 89 d5             	mov    %rdx,%rbp
      d208c3:	53                   	push   %rbx
      d208c4:	48 89 f3             	mov    %rsi,%rbx
      d208c7:	48 83 ec 18          	sub    $0x18,%rsp
      d208cb:	48 8b 42 08          	mov    0x8(%rdx),%rax
      d208cf:	48 89 c2             	mov    %rax,%rdx
      d208d2:	48 83 e2 fe          	and    $0xfffffffffffffffe,%rdx
      d208d6:	a8 01                	test   $0x1,%al
      d208d8:	75 7e                	jne    d20958 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182128>
      d208da:	48 85 d2             	test   %rdx,%rdx
      d208dd:	74 21                	je     d20900 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1820d0>
      d208df:	48 89 df             	mov    %rbx,%rdi
      d208e2:	e8 89 86 a8 00       	call   17a8f70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xc0a740>
      d208e7:	48 89 ee             	mov    %rbp,%rsi
      d208ea:	48 89 df             	mov    %rbx,%rdi
      d208ed:	e8 0e c7 a8 00       	call   17ad000 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xc0e7d0>
      d208f2:	48 83 c4 18          	add    $0x18,%rsp
      d208f6:	5b                   	pop    %rbx
      d208f7:	5d                   	pop    %rbp
      d208f8:	c3                   	ret
      d208f9:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20900:	48 89 43 08          	mov    %rax,0x8(%rbx)
      d20904:	8b 45 10             	mov    0x10(%rbp),%eax
      d20907:	48 c7 45 08 00 00 00 	movq   $0x0,0x8(%rbp)
      d2090e:	00 
      d2090f:	89 43 10             	mov    %eax,0x10(%rbx)
      d20912:	f3 0f 6f 4d 18       	movdqu 0x18(%rbp),%xmm1
      d20917:	c7 45 10 00 00 00 00 	movl   $0x0,0x10(%rbp)
      d2091e:	f3 0f 6f 43 18       	movdqu 0x18(%rbx),%xmm0
      d20923:	0f 11 4b 18          	movups %xmm1,0x18(%rbx)
      d20927:	48 8b 7d 28          	mov    0x28(%rbp),%rdi
      d2092b:	0f 11 45 18          	movups %xmm0,0x18(%rbp)
      d2092f:	48 8b 73 28          	mov    0x28(%rbx),%rsi
      d20933:	8b 4b 30             	mov    0x30(%rbx),%ecx
      d20936:	48 89 7b 28          	mov    %rdi,0x28(%rbx)
      d2093a:	8b 7d 30             	mov    0x30(%rbp),%edi
      d2093d:	89 7b 30             	mov    %edi,0x30(%rbx)
      d20940:	48 89 75 28          	mov    %rsi,0x28(%rbp)
      d20944:	89 4d 30             	mov    %ecx,0x30(%rbp)
      d20947:	48 83 c4 18          	add    $0x18,%rsp
      d2094b:	5b                   	pop    %rbx
      d2094c:	5d                   	pop    %rbp
      d2094d:	c3                   	ret
      d2094e:	66 90                	xchg   %ax,%ax
      d20950:	c3                   	ret
      d20951:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20958:	48 8b 12             	mov    (%rdx),%rdx
      d2095b:	e9 7a ff ff ff       	jmp    d208da <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1820aa>
      d20960:	55                   	push   %rbp
      d20961:	48 8d 05 c8 c6 37 02 	lea    0x237c6c8(%rip),%rax        # 309d030 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0xd050>
      d20968:	53                   	push   %rbx
      d20969:	48 89 f3             	mov    %rsi,%rbx
      d2096c:	48 83 ec 08          	sub    $0x8,%rsp
      d20970:	48 89 06             	mov    %rax,(%rsi)
      d20973:	f6 46 08 01          	testb  $0x1,0x8(%rsi)
      d20977:	75 77                	jne    d209f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821c0>
      d20979:	48 8d 05 a0 3d 43 02 	lea    0x2433da0(%rip),%rax        # 3154720 <typeinfo for QSGRectangleNode@@Base+0x2c298>
      d20980:	48 39 c3             	cmp    %rax,%rbx
      d20983:	74 63                	je     d209e8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821b8>
      d20985:	48 8b 6b 18          	mov    0x18(%rbx),%rbp
      d20989:	48 85 ed             	test   %rbp,%rbp
      d2098c:	74 29                	je     d209b7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182187>
      d2098e:	48 8d 05 93 e5 36 02 	lea    0x236e593(%rip),%rax        # 308ef28 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xfed7c8>
      d20995:	48 89 45 00          	mov    %rax,0x0(%rbp)
      d20999:	f6 45 08 01          	testb  $0x1,0x8(%rbp)
      d2099d:	0f 85 7d 00 00 00    	jne    d20a20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821f0>
      d209a3:	8b 45 1c             	mov    0x1c(%rbp),%eax
      d209a6:	85 c0                	test   %eax,%eax
      d209a8:	75 66                	jne    d20a10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821e0>
      d209aa:	be 20 00 00 00       	mov    $0x20,%esi
      d209af:	48 89 ef             	mov    %rbp,%rdi
      d209b2:	e8 49 df 7b ff       	call   4de900 <operator delete(void*, unsigned long)@plt>
      d209b7:	48 8b 5b 20          	mov    0x20(%rbx),%rbx
      d209bb:	48 85 db             	test   %rbx,%rbx
      d209be:	74 28                	je     d209e8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821b8>
      d209c0:	48 8d 05 41 b8 37 02 	lea    0x237b841(%rip),%rax        # 309c208 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0xc228>
      d209c7:	48 89 03             	mov    %rax,(%rbx)
      d209ca:	f6 43 08 01          	testb  $0x1,0x8(%rbx)
      d209ce:	75 30                	jne    d20a00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821d0>
      d209d0:	48 83 c4 08          	add    $0x8,%rsp
      d209d4:	48 89 df             	mov    %rbx,%rdi
      d209d7:	be 20 00 00 00       	mov    $0x20,%esi
      d209dc:	5b                   	pop    %rbx
      d209dd:	5d                   	pop    %rbp
      d209de:	e9 1d df 7b ff       	jmp    4de900 <operator delete(void*, unsigned long)@plt>
      d209e3:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d209e8:	48 83 c4 08          	add    $0x8,%rsp
      d209ec:	5b                   	pop    %rbx
      d209ed:	5d                   	pop    %rbp
      d209ee:	c3                   	ret
      d209ef:	90                   	nop
      d209f0:	48 8d 7e 08          	lea    0x8(%rsi),%rdi
      d209f4:	e8 57 b7 a5 00       	call   177c150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd920>
      d209f9:	e9 7b ff ff ff       	jmp    d20979 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182149>
      d209fe:	66 90                	xchg   %ax,%ax
      d20a00:	48 8d 7b 08          	lea    0x8(%rbx),%rdi
      d20a04:	e8 47 b7 a5 00       	call   177c150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd920>
      d20a09:	eb c5                	jmp    d209d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1821a0>
      d20a0b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d20a10:	48 89 ef             	mov    %rbp,%rdi
      d20a13:	e8 38 8b ab 00       	call   17d9550 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xc3ad20>
      d20a18:	eb 90                	jmp    d209aa <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18217a>
      d20a1a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20a20:	48 8d 7d 08          	lea    0x8(%rbp),%rdi
      d20a24:	e8 27 b7 a5 00       	call   177c150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd920>
      d20a29:	e9 75 ff ff ff       	jmp    d209a3 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182173>
      d20a2e:	66 90                	xchg   %ax,%ax
      d20a30:	41 55                	push   %r13
      d20a32:	48 8d 05 97 4f 38 02 	lea    0x2384f97(%rip),%rax        # 30a59d0 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x159f0>
      d20a39:	66 0f ef c0          	pxor   %xmm0,%xmm0
      d20a3d:	41 54                	push   %r12
      d20a3f:	55                   	push   %rbp
      d20a40:	48 89 d5             	mov    %rdx,%rbp
      d20a43:	53                   	push   %rbx
      d20a44:	48 89 f3             	mov    %rsi,%rbx
      d20a47:	48 83 ec 08          	sub    $0x8,%rsp
      d20a4b:	48 c7 46 08 00 00 00 	movq   $0x0,0x8(%rsi)
      d20a52:	00 
      d20a53:	48 89 06             	mov    %rax,(%rsi)
      d20a56:	0f 11 46 10          	movups %xmm0,0x10(%rsi)
      d20a5a:	48 8b 72 08          	mov    0x8(%rdx),%rsi
      d20a5e:	40 f6 c6 01          	test   $0x1,%sil
      d20a62:	74 16                	je     d20a7a <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18224a>
      d20a64:	48 89 f0             	mov    %rsi,%rax
      d20a67:	83 e0 01             	and    $0x1,%eax
      d20a6a:	75 2c                	jne    d20a98 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182268>
      d20a6c:	ff d0                	call   *%rax
      d20a6e:	48 89 c6             	mov    %rax,%rsi
      d20a71:	48 8d 7b 08          	lea    0x8(%rbx),%rdi
      d20a75:	e8 06 32 df 00       	call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
      d20a7a:	83 7d 1c 01          	cmpl   $0x1,0x1c(%rbp)
      d20a7e:	c7 43 1c 00 00 00 00 	movl   $0x0,0x1c(%rbx)
      d20a85:	74 21                	je     d20aa8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182278>
      d20a87:	48 83 c4 08          	add    $0x8,%rsp
      d20a8b:	5b                   	pop    %rbx
      d20a8c:	5d                   	pop    %rbp
      d20a8d:	41 5c                	pop    %r12
      d20a8f:	41 5d                	pop    %r13
      d20a91:	c3                   	ret
      d20a92:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20a98:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
      d20a9c:	48 83 c6 08          	add    $0x8,%rsi
      d20aa0:	eb cf                	jmp    d20a71 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182241>
      d20aa2:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20aa8:	48 8b 43 08          	mov    0x8(%rbx),%rax
      d20aac:	c7 43 1c 01 00 00 00 	movl   $0x1,0x1c(%rbx)
      d20ab3:	49 89 c5             	mov    %rax,%r13
      d20ab6:	49 83 e5 fe          	and    $0xfffffffffffffffe,%r13
      d20aba:	a8 01                	test   $0x1,%al
      d20abc:	0f 85 d7 00 00 00    	jne    d20b99 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182369>
      d20ac2:	4d 85 ed             	test   %r13,%r13
      d20ac5:	0f 84 97 00 00 00    	je     d20b62 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182332>
      d20acb:	be 20 00 00 00       	mov    $0x20,%esi
      d20ad0:	4c 89 ef             	mov    %r13,%rdi
      d20ad3:	e8 38 77 d8 00       	call   1aa8210 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x282980>
      d20ad8:	4c 89 68 08          	mov    %r13,0x8(%rax)
      d20adc:	49 89 c4             	mov    %rax,%r12
      d20adf:	48 8d 05 72 4e 38 02 	lea    0x2384e72(%rip),%rax        # 30a5958 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x15978>
      d20ae6:	49 89 04 24          	mov    %rax,(%r12)
      d20aea:	49 c7 44 24 10 00 00 	movq   $0x0,0x10(%r12)
      d20af1:	00 00 
      d20af3:	41 c7 44 24 18 00 00 	movl   $0x0,0x18(%r12)
      d20afa:	00 00 
      d20afc:	83 7d 1c 01          	cmpl   $0x1,0x1c(%rbp)
      d20b00:	4c 89 63 10          	mov    %r12,0x10(%rbx)
      d20b04:	48 8d 05 95 a6 43 02 	lea    0x243a695(%rip),%rax        # 315b1a0 <typeinfo for QSGRectangleNode@@Base+0x32d18>
      d20b0b:	75 04                	jne    d20b11 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1822e1>
      d20b0d:	48 8b 45 10          	mov    0x10(%rbp),%rax
      d20b11:	f6 40 10 01          	testb  $0x1,0x10(%rax)
      d20b15:	74 0f                	je     d20b26 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1822f6>
      d20b17:	0f b6 50 18          	movzbl 0x18(%rax),%edx
      d20b1b:	41 83 4c 24 10 01    	orl    $0x1,0x10(%r12)
      d20b21:	41 88 54 24 18       	mov    %dl,0x18(%r12)
      d20b26:	48 8b 40 08          	mov    0x8(%rax),%rax
      d20b2a:	a8 01                	test   $0x1,%al
      d20b2c:	0f 84 55 ff ff ff    	je     d20a87 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182257>
      d20b32:	48 89 c2             	mov    %rax,%rdx
      d20b35:	83 e2 01             	and    $0x1,%edx
      d20b38:	75 1e                	jne    d20b58 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182328>
      d20b3a:	ff d2                	call   *%rdx
      d20b3c:	48 89 c6             	mov    %rax,%rsi
      d20b3f:	48 83 c4 08          	add    $0x8,%rsp
      d20b43:	49 8d 7c 24 08       	lea    0x8(%r12),%rdi
      d20b48:	5b                   	pop    %rbx
      d20b49:	5d                   	pop    %rbp
      d20b4a:	41 5c                	pop    %r12
      d20b4c:	41 5d                	pop    %r13
      d20b4e:	e9 2d 31 df 00       	jmp    1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
      d20b53:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d20b58:	48 83 e0 fe          	and    $0xfffffffffffffffe,%rax
      d20b5c:	48 8d 70 08          	lea    0x8(%rax),%rsi
      d20b60:	eb dd                	jmp    d20b3f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18230f>
      d20b62:	bf 20 00 00 00       	mov    $0x20,%edi
      d20b67:	e8 04 eb 7b ff       	call   4df670 <operator new(unsigned long)@plt>
      d20b6c:	48 c7 40 08 00 00 00 	movq   $0x0,0x8(%rax)
      d20b73:	00 
      d20b74:	49 89 c4             	mov    %rax,%r12
      d20b77:	48 8d 05 da 4d 38 02 	lea    0x2384dda(%rip),%rax        # 30a5958 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x15978>
      d20b7e:	49 89 04 24          	mov    %rax,(%r12)
      d20b82:	49 c7 44 24 10 00 00 	movq   $0x0,0x10(%r12)
      d20b89:	00 00 
      d20b8b:	41 c7 44 24 18 00 00 	movl   $0x0,0x18(%r12)
      d20b92:	00 00 
      d20b94:	e9 63 ff ff ff       	jmp    d20afc <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1822cc>
      d20b99:	4d 8b 6d 00          	mov    0x0(%r13),%r13
      d20b9d:	e9 20 ff ff ff       	jmp    d20ac2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182292>
      d20ba2:	66 66 2e 0f 1f 84 00 	data16 cs nopw 0x0(%rax,%rax,1)
      d20ba9:	00 00 00 00 
      d20bad:	0f 1f 00             	nopl   (%rax)
      d20bb0:	48 8d 05 19 4e 38 02 	lea    0x2384e19(%rip),%rax        # 30a59d0 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x159f0>
      d20bb7:	66 0f ef c0          	pxor   %xmm0,%xmm0
      d20bbb:	48 c7 46 08 00 00 00 	movq   $0x0,0x8(%rsi)
      d20bc2:	00 
      d20bc3:	0f 11 46 10          	movups %xmm0,0x10(%rsi)
      d20bc7:	48 89 06             	mov    %rax,(%rsi)
      d20bca:	c7 46 1c 00 00 00 00 	movl   $0x0,0x1c(%rsi)
      d20bd1:	48 39 d6             	cmp    %rdx,%rsi
      d20bd4:	74 6a                	je     d20c40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182410>
      d20bd6:	55                   	push   %rbp
      d20bd7:	48 89 d5             	mov    %rdx,%rbp
      d20bda:	53                   	push   %rbx
      d20bdb:	48 89 f3             	mov    %rsi,%rbx
      d20bde:	48 83 ec 08          	sub    $0x8,%rsp
      d20be2:	48 8b 42 08          	mov    0x8(%rdx),%rax
      d20be6:	48 89 c2             	mov    %rax,%rdx
      d20be9:	48 83 e2 fe          	and    $0xfffffffffffffffe,%rdx
      d20bed:	a8 01                	test   $0x1,%al
      d20bef:	75 57                	jne    d20c48 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182418>
      d20bf1:	48 85 d2             	test   %rdx,%rdx
      d20bf4:	74 1a                	je     d20c10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1823e0>
      d20bf6:	48 89 df             	mov    %rbx,%rdi
      d20bf9:	e8 42 01 a6 00       	call   1780d40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbe2510>
      d20bfe:	48 89 ee             	mov    %rbp,%rsi
      d20c01:	48 89 df             	mov    %rbx,%rdi
      d20c04:	e8 57 39 a6 00       	call   1784560 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbe5d30>
      d20c09:	48 83 c4 08          	add    $0x8,%rsp
      d20c0d:	5b                   	pop    %rbx
      d20c0e:	5d                   	pop    %rbp
      d20c0f:	c3                   	ret
      d20c10:	48 89 43 08          	mov    %rax,0x8(%rbx)
      d20c14:	48 8b 55 10          	mov    0x10(%rbp),%rdx
      d20c18:	48 8b 43 10          	mov    0x10(%rbx),%rax
      d20c1c:	48 c7 45 08 00 00 00 	movq   $0x0,0x8(%rbp)
      d20c23:	00 
      d20c24:	48 89 53 10          	mov    %rdx,0x10(%rbx)
      d20c28:	48 89 45 10          	mov    %rax,0x10(%rbp)
      d20c2c:	8b 45 1c             	mov    0x1c(%rbp),%eax
      d20c2f:	89 43 1c             	mov    %eax,0x1c(%rbx)
      d20c32:	c7 45 1c 00 00 00 00 	movl   $0x0,0x1c(%rbp)
      d20c39:	48 83 c4 08          	add    $0x8,%rsp
      d20c3d:	5b                   	pop    %rbx
      d20c3e:	5d                   	pop    %rbp
      d20c3f:	c3                   	ret
      d20c40:	c3                   	ret
      d20c41:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20c48:	48 8b 12             	mov    (%rdx),%rdx
      d20c4b:	eb a4                	jmp    d20bf1 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1823c1>
      d20c4d:	0f 1f 00             	nopl   (%rax)
      d20c50:	48 8d 05 79 8f 3a 02 	lea    0x23a8f79(%rip),%rax        # 30c9bd0 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x39bf0>
      d20c57:	48 c7 46 08 00 00 00 	movq   $0x0,0x8(%rsi)
      d20c5e:	00 
      d20c5f:	48 89 06             	mov    %rax,(%rsi)
      d20c62:	48 c7 46 10 00 00 00 	movq   $0x0,0x10(%rsi)
      d20c69:	00 
      d20c6a:	c7 46 18 00 00 00 00 	movl   $0x0,0x18(%rsi)
      d20c71:	48 39 d6             	cmp    %rdx,%rsi
      d20c74:	0f 84 96 00 00 00    	je     d20d10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1824e0>
      d20c7a:	55                   	push   %rbp
      d20c7b:	48 89 d5             	mov    %rdx,%rbp
      d20c7e:	53                   	push   %rbx
      d20c7f:	48 89 f3             	mov    %rsi,%rbx
      d20c82:	48 83 ec 08          	sub    $0x8,%rsp
      d20c86:	48 8b 42 08          	mov    0x8(%rdx),%rax
      d20c8a:	48 89 c2             	mov    %rax,%rdx
      d20c8d:	48 83 e2 fe          	and    $0xfffffffffffffffe,%rdx
      d20c91:	a8 01                	test   $0x1,%al
      d20c93:	0f 85 7f 00 00 00    	jne    d20d18 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1824e8>
      d20c99:	48 85 d2             	test   %rdx,%rdx
      d20c9c:	74 42                	je     d20ce0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1824b0>
      d20c9e:	48 89 df             	mov    %rbx,%rdi
      d20ca1:	e8 4a 7d a3 00       	call   17589f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbba1c0>
      d20ca6:	f6 45 10 01          	testb  $0x1,0x10(%rbp)
      d20caa:	74 0a                	je     d20cb6 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182486>
      d20cac:	8b 45 18             	mov    0x18(%rbp),%eax
      d20caf:	83 4b 10 01          	orl    $0x1,0x10(%rbx)
      d20cb3:	89 43 18             	mov    %eax,0x18(%rbx)
      d20cb6:	48 8b 75 08          	mov    0x8(%rbp),%rsi
      d20cba:	40 f6 c6 01          	test   $0x1,%sil
      d20cbe:	74 45                	je     d20d05 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1824d5>
      d20cc0:	48 89 f0             	mov    %rsi,%rax
      d20cc3:	83 e0 01             	and    $0x1,%eax
      d20cc6:	75 58                	jne    d20d20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1824f0>
      d20cc8:	ff d0                	call   *%rax
      d20cca:	48 89 c6             	mov    %rax,%rsi
      d20ccd:	48 8d 7b 08          	lea    0x8(%rbx),%rdi
      d20cd1:	e8 aa 2f df 00       	call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
      d20cd6:	48 83 c4 08          	add    $0x8,%rsp
      d20cda:	5b                   	pop    %rbx
      d20cdb:	5d                   	pop    %rbp
      d20cdc:	c3                   	ret
      d20cdd:	0f 1f 00             	nopl   (%rax)
      d20ce0:	48 89 43 08          	mov    %rax,0x8(%rbx)
      d20ce4:	8b 45 10             	mov    0x10(%rbp),%eax
      d20ce7:	48 c7 45 08 00 00 00 	movq   $0x0,0x8(%rbp)
      d20cee:	00 
      d20cef:	89 43 10             	mov    %eax,0x10(%rbx)
      d20cf2:	8b 55 18             	mov    0x18(%rbp),%edx
      d20cf5:	c7 45 10 00 00 00 00 	movl   $0x0,0x10(%rbp)
      d20cfc:	8b 43 18             	mov    0x18(%rbx),%eax
      d20cff:	89 53 18             	mov    %edx,0x18(%rbx)
      d20d02:	89 45 18             	mov    %eax,0x18(%rbp)
      d20d05:	48 83 c4 08          	add    $0x8,%rsp
      d20d09:	5b                   	pop    %rbx
      d20d0a:	5d                   	pop    %rbp
      d20d0b:	c3                   	ret
      d20d0c:	0f 1f 40 00          	nopl   0x0(%rax)
      d20d10:	c3                   	ret
      d20d11:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20d18:	48 8b 12             	mov    (%rdx),%rdx
      d20d1b:	e9 79 ff ff ff       	jmp    d20c99 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182469>
      d20d20:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
      d20d24:	48 83 c6 08          	add    $0x8,%rsi
      d20d28:	eb a3                	jmp    d20ccd <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18249d>
      d20d2a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20d30:	48 8d 05 09 8e 3a 02 	lea    0x23a8e09(%rip),%rax        # 30c9b40 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x39b60>
      d20d37:	48 c7 46 08 00 00 00 	movq   $0x0,0x8(%rsi)
      d20d3e:	00 
      d20d3f:	48 89 06             	mov    %rax,(%rsi)
      d20d42:	48 c7 46 10 00 00 00 	movq   $0x0,0x10(%rsi)
      d20d49:	00 
      d20d4a:	c7 46 18 01 00 00 00 	movl   $0x1,0x18(%rsi)
      d20d51:	48 39 d6             	cmp    %rdx,%rsi
      d20d54:	0f 84 96 00 00 00    	je     d20df0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1825c0>
      d20d5a:	55                   	push   %rbp
      d20d5b:	48 89 d5             	mov    %rdx,%rbp
      d20d5e:	53                   	push   %rbx
      d20d5f:	48 89 f3             	mov    %rsi,%rbx
      d20d62:	48 83 ec 08          	sub    $0x8,%rsp
      d20d66:	48 8b 42 08          	mov    0x8(%rdx),%rax
      d20d6a:	48 89 c2             	mov    %rax,%rdx
      d20d6d:	48 83 e2 fe          	and    $0xfffffffffffffffe,%rdx
      d20d71:	a8 01                	test   $0x1,%al
      d20d73:	0f 85 7f 00 00 00    	jne    d20df8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1825c8>
      d20d79:	48 85 d2             	test   %rdx,%rdx
      d20d7c:	74 42                	je     d20dc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182590>
      d20d7e:	48 89 df             	mov    %rbx,%rdi
      d20d81:	e8 8a 6c a6 00       	call   1787a10 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbe91e0>
      d20d86:	f6 45 10 01          	testb  $0x1,0x10(%rbp)
      d20d8a:	74 0a                	je     d20d96 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182566>
      d20d8c:	8b 45 18             	mov    0x18(%rbp),%eax
      d20d8f:	83 4b 10 01          	orl    $0x1,0x10(%rbx)
      d20d93:	89 43 18             	mov    %eax,0x18(%rbx)
      d20d96:	48 8b 75 08          	mov    0x8(%rbp),%rsi
      d20d9a:	40 f6 c6 01          	test   $0x1,%sil
      d20d9e:	74 45                	je     d20de5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1825b5>
      d20da0:	48 89 f0             	mov    %rsi,%rax
      d20da3:	83 e0 01             	and    $0x1,%eax
      d20da6:	75 58                	jne    d20e00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1825d0>
      d20da8:	ff d0                	call   *%rax
      d20daa:	48 89 c6             	mov    %rax,%rsi
      d20dad:	48 8d 7b 08          	lea    0x8(%rbx),%rdi
      d20db1:	e8 ca 2e df 00       	call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
      d20db6:	48 83 c4 08          	add    $0x8,%rsp
      d20dba:	5b                   	pop    %rbx
      d20dbb:	5d                   	pop    %rbp
      d20dbc:	c3                   	ret
      d20dbd:	0f 1f 00             	nopl   (%rax)
      d20dc0:	48 89 43 08          	mov    %rax,0x8(%rbx)
      d20dc4:	8b 45 10             	mov    0x10(%rbp),%eax
      d20dc7:	48 c7 45 08 00 00 00 	movq   $0x0,0x8(%rbp)
      d20dce:	00 
      d20dcf:	89 43 10             	mov    %eax,0x10(%rbx)
      d20dd2:	8b 55 18             	mov    0x18(%rbp),%edx
      d20dd5:	c7 45 10 00 00 00 00 	movl   $0x0,0x10(%rbp)
      d20ddc:	8b 43 18             	mov    0x18(%rbx),%eax
      d20ddf:	89 53 18             	mov    %edx,0x18(%rbx)
      d20de2:	89 45 18             	mov    %eax,0x18(%rbp)
      d20de5:	48 83 c4 08          	add    $0x8,%rsp
      d20de9:	5b                   	pop    %rbx
      d20dea:	5d                   	pop    %rbp
      d20deb:	c3                   	ret
      d20dec:	0f 1f 40 00          	nopl   0x0(%rax)
      d20df0:	c3                   	ret
      d20df1:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20df8:	48 8b 12             	mov    (%rdx),%rdx
      d20dfb:	e9 79 ff ff ff       	jmp    d20d79 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182549>
      d20e00:	48 83 e6 fe          	and    $0xfffffffffffffffe,%rsi
      d20e04:	48 83 c6 08          	add    $0x8,%rsi
      d20e08:	eb a3                	jmp    d20dad <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18257d>
      d20e0a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20e10:	48 89 c8             	mov    %rcx,%rax
      d20e13:	85 f6                	test   %esi,%esi
      d20e15:	75 29                	jne    d20e40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182610>
      d20e17:	83 fa 18             	cmp    $0x18,%edx
      d20e1a:	0f 87 3f 01 00 00    	ja     d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20e20:	55                   	push   %rbp
      d20e21:	48 8d 0d b8 08 05 01 	lea    0x10508b8(%rip),%rcx        # 1d716e0 <typeinfo name for QMetaType::registerConverter<QList<QObject*>, QIterable<QMetaSequence>, QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> > >(QtPrivate::QSequentialIterableConvertFunctor<QList<QObject*> >)::{lambda(void const*, void*)#1}@@Base+0x88120>
      d20e28:	89 d2                	mov    %edx,%edx
      d20e2a:	53                   	push   %rbx
      d20e2b:	48 83 ec 48          	sub    $0x48,%rsp
      d20e2f:	48 63 14 91          	movslq (%rcx,%rdx,4),%rdx
      d20e33:	48 01 ca             	add    %rcx,%rdx
      d20e36:	ff e2                	jmp    *%rdx
      d20e38:	0f 1f 84 00 00 00 00 	nopl   0x0(%rax,%rax,1)
      d20e3f:	00 
      d20e40:	83 fe 05             	cmp    $0x5,%esi
      d20e43:	0f 85 16 01 00 00    	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20e49:	48 8b 51 08          	mov    0x8(%rcx),%rdx
      d20e4d:	48 8b 31             	mov    (%rcx),%rsi
      d20e50:	48 8d 0d 39 89 ff ff 	lea    -0x76c7(%rip),%rcx        # d19790 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17af60>
      d20e57:	48 8b 02             	mov    (%rdx),%rax
      d20e5a:	48 39 c8             	cmp    %rcx,%rax
      d20e5d:	0f 84 f5 00 00 00    	je     d20f58 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182728>
      d20e63:	48 8d 0d 56 89 ff ff 	lea    -0x76aa(%rip),%rcx        # d197c0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17af90>
      d20e6a:	48 39 c8             	cmp    %rcx,%rax
      d20e6d:	0f 84 ed 00 00 00    	je     d20f60 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182730>
      d20e73:	48 8d 0d 76 89 ff ff 	lea    -0x768a(%rip),%rcx        # d197f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17afc0>
      d20e7a:	48 39 c8             	cmp    %rcx,%rax
      d20e7d:	0f 84 ed 00 00 00    	je     d20f70 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182740>
      d20e83:	48 8d 0d 96 89 ff ff 	lea    -0x766a(%rip),%rcx        # d19820 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17aff0>
      d20e8a:	48 39 c8             	cmp    %rcx,%rax
      d20e8d:	0f 84 ed 00 00 00    	je     d20f80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182750>
      d20e93:	48 8d 0d b6 89 ff ff 	lea    -0x764a(%rip),%rcx        # d19850 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b020>
      d20e9a:	48 39 c8             	cmp    %rcx,%rax
      d20e9d:	0f 84 fd 00 00 00    	je     d20fa0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182770>
      d20ea3:	48 8d 0d d6 89 ff ff 	lea    -0x762a(%rip),%rcx        # d19880 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b050>
      d20eaa:	48 39 c8             	cmp    %rcx,%rax
      d20ead:	0f 84 fd 00 00 00    	je     d20fb0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182780>
      d20eb3:	48 8d 0d f6 89 ff ff 	lea    -0x760a(%rip),%rcx        # d198b0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b080>
      d20eba:	48 39 c8             	cmp    %rcx,%rax
      d20ebd:	0f 84 fd 00 00 00    	je     d20fc0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182790>
      d20ec3:	48 8d 0d 06 8a ff ff 	lea    -0x75fa(%rip),%rcx        # d198d0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b0a0>
      d20eca:	48 39 c8             	cmp    %rcx,%rax
      d20ecd:	0f 84 5d 06 00 00    	je     d21530 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182d00>
      d20ed3:	48 8d 0d 26 8a ff ff 	lea    -0x75da(%rip),%rcx        # d19900 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b0d0>
      d20eda:	48 39 c8             	cmp    %rcx,%rax
      d20edd:	0f 84 ed 00 00 00    	je     d20fd0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1827a0>
      d20ee3:	48 8d 0d 66 8a ff ff 	lea    -0x759a(%rip),%rcx        # d19950 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b120>
      d20eea:	48 39 c8             	cmp    %rcx,%rax
      d20eed:	0f 84 ed 00 00 00    	je     d20fe0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1827b0>
      d20ef3:	48 8d 0d 76 8a ff ff 	lea    -0x758a(%rip),%rcx        # d19970 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b140>
      d20efa:	48 39 c8             	cmp    %rcx,%rax
      d20efd:	0f 84 f5 00 00 00    	je     d20ff8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1827c8>
      d20f03:	48 8d 0d a6 8a ff ff 	lea    -0x755a(%rip),%rcx        # d199b0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b180>
      d20f0a:	48 39 c8             	cmp    %rcx,%rax
      d20f0d:	0f 84 fd 00 00 00    	je     d21010 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x1827e0>
      d20f13:	48 8d 0d c6 8a ff ff 	lea    -0x753a(%rip),%rcx        # d199e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b1b0>
      d20f1a:	48 39 c8             	cmp    %rcx,%rax
      d20f1d:	0f 84 25 06 00 00    	je     d21548 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182d18>
      d20f23:	48 8d 0d f6 8a ff ff 	lea    -0x750a(%rip),%rcx        # d19a20 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b1f0>
      d20f2a:	48 39 c8             	cmp    %rcx,%rax
      d20f2d:	0f 84 2d 06 00 00    	je     d21560 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182d30>
      d20f33:	48 8d 0d 26 8b ff ff 	lea    -0x74da(%rip),%rcx        # d19a60 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x17b230>
      d20f3a:	48 39 c8             	cmp    %rcx,%rax
      d20f3d:	0f 85 35 06 00 00    	jne    d21578 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182d48>
      d20f43:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20f48:	75 15                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20f4a:	c7 06 0e 00 00 00    	movl   $0xe,(%rsi)
      d20f50:	c3                   	ret
      d20f51:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d20f58:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20f5d:	74 31                	je     d20f90 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182760>
      d20f5f:	c3                   	ret
      d20f60:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20f65:	75 f8                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20f67:	c7 06 01 00 00 00    	movl   $0x1,(%rsi)
      d20f6d:	c3                   	ret
      d20f6e:	66 90                	xchg   %ax,%ax
      d20f70:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20f75:	75 e8                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20f77:	c7 06 02 00 00 00    	movl   $0x2,(%rsi)
      d20f7d:	c3                   	ret
      d20f7e:	66 90                	xchg   %ax,%ax
      d20f80:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20f85:	75 d8                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20f87:	c7 06 03 00 00 00    	movl   $0x3,(%rsi)
      d20f8d:	c3                   	ret
      d20f8e:	66 90                	xchg   %ax,%ax
      d20f90:	c7 06 00 00 00 00    	movl   $0x0,(%rsi)
      d20f96:	c3                   	ret
      d20f97:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d20f9e:	00 00 
      d20fa0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20fa5:	75 b8                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20fa7:	c7 06 04 00 00 00    	movl   $0x4,(%rsi)
      d20fad:	c3                   	ret
      d20fae:	66 90                	xchg   %ax,%ax
      d20fb0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20fb5:	75 a8                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20fb7:	c7 06 05 00 00 00    	movl   $0x5,(%rsi)
      d20fbd:	c3                   	ret
      d20fbe:	66 90                	xchg   %ax,%ax
      d20fc0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20fc5:	75 98                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20fc7:	c7 06 06 00 00 00    	movl   $0x6,(%rsi)
      d20fcd:	c3                   	ret
      d20fce:	66 90                	xchg   %ax,%ax
      d20fd0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20fd5:	75 88                	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20fd7:	c7 06 08 00 00 00    	movl   $0x8,(%rsi)
      d20fdd:	c3                   	ret
      d20fde:	66 90                	xchg   %ax,%ax
      d20fe0:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20fe5:	0f 85 74 ff ff ff    	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d20feb:	c7 06 09 00 00 00    	movl   $0x9,(%rsi)
      d20ff1:	c3                   	ret
      d20ff2:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d20ff8:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d20ffd:	0f 85 5c ff ff ff    	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d21003:	c7 06 0a 00 00 00    	movl   $0xa,(%rsi)
      d21009:	c3                   	ret
      d2100a:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d21010:	48 83 7a 08 00       	cmpq   $0x0,0x8(%rdx)
      d21015:	0f 85 44 ff ff ff    	jne    d20f5f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x18272f>
      d2101b:	c7 06 0b 00 00 00    	movl   $0xb,(%rsi)
      d21021:	c3                   	ret
      d21022:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d21028:	31 c9                	xor    %ecx,%ecx
      d2102a:	ba 0f 00 00 00       	mov    $0xf,%edx
      d2102f:	48 83 c4 48          	add    $0x48,%rsp
      d21033:	48 8d 35 66 45 36 02 	lea    0x2364566(%rip),%rsi        # 30855a0 <QObject::staticMetaObject@Qt_6>
      d2103a:	5b                   	pop    %rbx
      d2103b:	5d                   	pop    %rbp
      d2103c:	e9 7f dd 7b ff       	jmp    4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d21041:	0f 1f 80 00 00 00 00 	nopl   0x0(%rax)
      d21048:	48 8b 70 08          	mov    0x8(%rax),%rsi
      d2104c:	48 8b 07             	mov    (%rdi),%rax
      d2104f:	48 8b 40 68          	mov    0x68(%rax),%rax
      d21053:	48 83 c4 48          	add    $0x48,%rsp
      d21057:	5b                   	pop    %rbx
      d21058:	5d                   	pop    %rbp
      d21059:	ff e0                	jmp    *%rax
      d2105b:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d21060:	48 8b 70 08          	mov    0x8(%rax),%rsi
      d21064:	48 8b 07             	mov    (%rdi),%rax
      d21067:	48 8b 40 60          	mov    0x60(%rax),%rax
      d2106b:	48 83 c4 48          	add    $0x48,%rsp
      d2106f:	5b                   	pop    %rbx
      d21070:	5d                   	pop    %rbp
      d21071:	ff e0                	jmp    *%rax
      d21073:	0f 1f 44 00 00       	nopl   0x0(%rax,%rax,1)
      d21078:	48 8b 40 08          	mov    0x8(%rax),%rax
      d2107c:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d21081:	ba 05 00 00 00       	mov    $0x5,%edx
      d21086:	48 8d 35 13 45 36 02 	lea    0x2364513(%rip),%rsi        # 30855a0 <QObject::staticMetaObject@Qt_6>
      d2108d:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d21094:	00 00 
      d21096:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d2109b:	e8 20 dd 7b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d210a0:	48 83 c4 48          	add    $0x48,%rsp
      d210a4:	5b                   	pop    %rbx
      d210a5:	5d                   	pop    %rbp
      d210a6:	c3                   	ret
      d210a7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
      d210ae:	00 00 
      d210b0:	48 8b 40 08          	mov    0x8(%rax),%rax
      d210b4:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d210b9:	ba 04 00 00 00       	mov    $0x4,%edx
      d210be:	48 8d 35 db 44 36 02 	lea    0x23644db(%rip),%rsi        # 30855a0 <QObject::staticMetaObject@Qt_6>
      d210c5:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d210cc:	00 00 
      d210ce:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
      d210d3:	e8 e8 dc 7b ff       	call   4dedc0 <QMetaObject::activate(QObject*, QMetaObject const*, int, void**)@plt>
      d210d8:	eb c6                	jmp    d210a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x182870>
      d210da:	66 0f 1f 44 00 00    	nopw   0x0(%rax,%rax,1)
      d210e0:	48 8b 40 08          	mov    0x8(%rax),%rax
      d210e4:	48 8d 4c 24 20       	lea    0x20(%rsp),%rcx
      d210e9:	ba 03 00 00 00       	mov    $0x3,%edx
      d210ee:	48 8d 35 ab 44 36 02 	lea    0x23644ab(%rip),%rsi        # 30855a0 <QObject::staticMetaObject@Qt_6>
      d210f5:	48 c7 44 24 20 00 00 	movq   $0x0,0x20(%rsp)
      d210fc:	00 00 
      d210fe:	48                   	rex.W
      d210ff:	89                   	.byte 0x89
```

<!-- END EXACT WORLDMAP QMETAOBJECT -->

