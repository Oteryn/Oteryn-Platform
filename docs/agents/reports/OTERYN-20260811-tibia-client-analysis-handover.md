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
