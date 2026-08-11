# OTERYN-20260811 Tibia client analysis runtime

## Objective

Materialize and inspect the current official Linux Tibia client on the Synology Oteryn staging host in an isolated analysis container, then identify the decoded map-protocol boundary suitable for reconstructing world-map data without modifying canonical Oteryn staging services.

## Scope

- Repository: `blakinio/Oteryn-Platform`
- Runner label: `oteryn-staging`
- Verified runner name: `oteryn-synology-staging`
- Environment boundary: Synology staging host
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (draft)
- Owned repository paths:
  - `.github/workflows/tibia-client-analysis-one-shot.yml`
  - `.github/workflows/tibia-client-analysis-continue.yml`
  - `.github/workflows/tibia-client-analysis-relay.yml`
  - `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md`
  - `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`
- Owned runtime identity:
  - container `oteryn-tibia-client-analysis`
  - bind path `/volume1/docker/oteryn/tibia-analysis`
  - labels `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`

## Safety and lifecycle

The workflow and continuation work must not modify, stop, restart, remove, or reconfigure the canonical `oteryn-staging` Compose services, the deploy runner, databases, networks, volumes, or unrelated containers. No blanket Docker cleanup is allowed.

The analysis container and `/volume1/docker/oteryn/tibia-analysis` are intentionally retained because they contain the materialized official client used for reverse engineering. Cleanup must target only the exact owned container/path and only after this task no longer needs the data.

Do not commit proprietary Tibia binaries or extracted client assets to Git. Commit only hashes, paths, metadata, research notes, safe scripts/workflows, and bounded evidence references. Never persist credentials, login tokens, or protected account data.

## Acceptance

- The job runs on exact label `oteryn-staging` and verifies runner identity/host Docker access.
- The owned analysis container is created/reused only when ownership labels match.
- Current official Linux client package metadata is obtained from CipSoft/Tibia infrastructure.
- The current client executable is materialized and verified against the official manifest by size and SHA-256.
- Static inspection identifies the receive/decode/map-processing pipeline and concrete map message types/handlers.
- Canonical `oteryn-staging` services remain unchanged.
- A continuation agent has enough durable evidence to start directly from `TWorldmapProtocolMessageHandler` / decoded protobuf map messages instead of repeating bootstrap work.

## Durable evidence report

Read:

`docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`

It contains the complete current-client identity, package-envelope reverse result, successful run/job evidence, map/protocol type inventory, rejected hypotheses, safety boundaries, and exact continuation objective.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T22:55:00Z
head: 4376b5c16af5fc824bd2b1ce8a3caecd40da1a7c
branch: ops/oteryn-tibia-client-analysis-20260811
pr: 1006
status: ready
context_routes:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
owned_paths:
  - .github/workflows/tibia-client-analysis-one-shot.yml
  - .github/workflows/tibia-client-analysis-continue.yml
  - .github/workflows/tibia-client-analysis-relay.yml
  - docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md
  - docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
proven:
  - "Runner label oteryn-staging resolves to runner name oteryn-synology-staging with host Docker access."
  - "Owned runtime container is oteryn-tibia-client-analysis with persistent data under /volume1/docker/oteryn/tibia-analysis."
  - "Current official client version is 15.32.df7b29."
  - "Materialized executable is /data/client-15.32.df7b29/bin/client, size 51965216 bytes, SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
  - "Executable is a stripped x86-64 ELF64 PIE with Build ID 427ad268e6d482f3ff96c72406a64c432040fecf and entry point 0x6afb50."
  - "Official packed bin/client.lzma has size 10150849 bytes and SHA-256 496c5b3517c0996a1bbd0e76a7738d450f79d0bf4fef140a807044776042dc9b."
  - "The CipSoft package uses an outer envelope; the working inner LZMA stream begins at offset 45 with lc=3, lp=0, pb=2 and 32 MiB dictionary."
  - "GitHub Actions run 31534432923 job 93922033384 completed SUCCESS on oteryn-synology-staging and re-verified executable identity."
  - "The same successful job inspected the binary and verified canonical oteryn-staging container inventory unchanged."
  - "Binary strings/types expose tibia::worldmap::TWorldmapProtocolMessageHandler and handlers handleFullMapMessage, handleLeftColumnMessage, handleRightColumnMessage, handleTopRowMessage, handleBottomRowMessage, handleTopFloorMessage, handleBottomFloorMessage, handleFieldDataMessage, handleCreateOnMapMessage, handleChangeOnMapMessage and handleDeleteOnMapMessage."
  - "Binary strings/types expose decoded protobuf server messages GameserverMessageFullMap, LeftColumn, RightColumn, TopRow, BottomRow, TopFloor, BottomFloor, FieldData, CreateOnMap, ChangeOnMap and DeleteOnMap plus MapFieldData, MapArea, Coordinate and AppearanceInstance."
  - "Binary strings/types expose TGameserverNetworkPacketConnection, TGameserverNetworkPacketRawDataProcessor, TProtocolReader, TProtobufServerMessageTranslator, TProtocolServerMessageProcessor and TProtocolMessageQueue."
  - "Binary strings/types expose TWorldMapStorage, TWorldMapCoordinate, TAppearancesManager, TMinimapProtocolMessageHandler, TMinimapTileStorage and TMinimapDiskIO."
derived:
  - "A preferred map collector can operate after protobuf translation and before or at TWorldmapProtocolMessageHandler, avoiding raw encrypted TCP reconstruction for the first implementation path."
  - "The target normalized record should map world coordinates to ordered field/tile contents and appearance/type identifiers, with optional appearance metadata resolution."
unknown:
  - "Exact code addresses/xrefs and protobuf field layouts for handleFullMapMessage, handleFieldDataMessage, MapFieldData, MapArea, Coordinate and AppearanceInstance are not yet recovered."
  - "Exact lowest-risk runtime hook/interception mechanism has not yet been proven with a live captured map message."
conflicts: []
first_failure:
  marker: none
  evidence: "Latest relevant final inspection run 31534432923 / job 93922033384 is SUCCESS."
rejected_hypotheses:
  - "Uploaded 1.4 MB Tibia/Tibia is the game client: disproved; it is the launcher/updater."
  - "client.lzma is standard LZMA from byte zero: disproved by decoder failures and outer-envelope reverse."
  - "Removing/restoring only a classic 13-byte LZMA header is sufficient: disproved by legal raw-LZMA parameter tests and failed decompression."
  - "Encrypted TCP sniffing is required before semantic map extraction: not supported by current evidence because decoded protobuf map messages and worldmap handlers are directly present in the client."
changed_paths:
  - .github/workflows/tibia-client-analysis-one-shot.yml
  - .github/workflows/tibia-client-analysis-continue.yml
  - .github/workflows/tibia-client-analysis-relay.yml
  - docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md
  - docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
validation:
  - command: "GitHub Actions run 31534432923 / job 93922033384"
    result: PASS
    evidence: "Verify runtime and executable identity, install binutils, inspect binary strings/imports/protocol clues, and verify staging unchanged all completed successfully."
  - command: "Executable manifest identity check"
    result: PASS
    evidence: "/data/client-15.32.df7b29/bin/client size 51965216 and SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
blockers:
  - none
next_action: "Reverse xrefs/code for tibia::worldmap::TWorldmapProtocolMessageHandler::handleFullMapMessage and handleFieldDataMessage, recover MapFieldData/MapArea/Coordinate/AppearanceInstance layouts, then prove one bounded decoded-map capture to deterministic (x,y,z, contents/appearance IDs) output without touching canonical oteryn-staging services."
```
