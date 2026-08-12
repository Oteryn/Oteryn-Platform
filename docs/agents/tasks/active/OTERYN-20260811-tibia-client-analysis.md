# OTERYN-20260811 Tibia client analysis runtime

## Objective

Materialize and inspect the current official Linux Tibia client on the Synology Oteryn staging host in an isolated analysis container, identify the decoded map-protocol boundary, and prove a deterministic map record path without modifying canonical Oteryn staging services.

## Scope

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (draft)
- Runner label: `oteryn-staging`
- Verified runner name: `oteryn-synology-staging`
- Owned runtime container: `oteryn-tibia-client-analysis`
- Owned bind path: `/volume1/docker/oteryn/tibia-analysis`
- Ownership labels: `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`

## Safety and lifecycle

Do not modify, restart, stop, remove, clean, or reconfigure canonical `oteryn-staging` Compose services, deployment infrastructure, databases, networks, volumes, or unrelated containers. No blanket Docker cleanup is allowed.

Do not commit proprietary Tibia binaries or extracted assets. Persist only hashes, addresses, bounded disassembly/control-flow evidence, safe workflows/scripts, and research notes. Never persist credentials, tokens, account data, character data, process arguments, or secret-bearing environment/state contents.

The owned analysis container and `/volume1/docker/oteryn/tibia-analysis` remain intentionally retained while this task is active.

## Durable evidence

Read in this order:

1. `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`
2. `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md`

The worldmap report contains the generated bounded trace and deep-trace blocks written by successful self-hosted Actions runs.

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-12T10:02:00Z
branch: ops/oteryn-tibia-client-analysis-20260811
pr: 1006
status: blocked_on_authenticated_world_session
observed_pr_head_before_this_checkpoint: bfddac0974570d3ae4cc637544d1da94f04ff815
context_routes:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
  - docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md
owned_paths:
  - .github/workflows/tibia-client-analysis-one-shot.yml
  - .github/workflows/tibia-client-analysis-continue.yml
  - .github/workflows/tibia-client-analysis-relay.yml
  - .github/workflows/tibia-client-analysis-dispatch.yml
  - .github/workflows/tibia-client-analysis-trace.yml
  - docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md
  - docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
  - docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md
proven:
  - "Current official client is 15.32.df7b29; executable /data/client-15.32.df7b29/bin/client is 51965216 bytes with SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
  - "TWorldmapProtocolMessageHandler exact bodies include FullMap 0xcec8d0, FieldData 0xcd3190, Create 0xcecc70, Change 0xcecf40 and Delete 0xcd4e20."
  - "0x19a8a80 is a shared map-data routine called by FieldData, FullMap/meta-dispatch and multiple directional/floor update paths."
  - "Coordinate protobuf schema is x=field 1 uint32, y=field 2 uint32, z=field 3 uint32."
  - "Run 31585256131 / job 94077482965 completed SUCCESS on oteryn-synology-staging, persisted bounded trace evidence and verified canonical oteryn-staging inventory unchanged."
  - "The central routine indexes repeated map-field entries by array index, computes world coordinates from region dimensions/base coordinates, then iterates each field's repeated content array by monotonically increasing index."
  - "At 0x19a8e21 the content element is selected from 0x8(base,index,8); the inner loop increments the content index and therefore preserves protobuf repeated-field order."
  - "Each selected content element exposes values at generated-object offsets +0x28 and +0x30 and selects a nested payload at +0x10 using helper 0x1ab4e50 with default-instance candidate 0x314b480 before calling map-content builder 0xceca50."
  - "Run 31585575487 / job 94078511849 completed SUCCESS on oteryn-synology-staging, persisted the bounded deep trace and again verified canonical staging unchanged."
  - "Deep-trace session inspection found 0 active Tibia client processes in the owned container, no X11 socket and no Wayland socket; it intentionally inspected no credentials, process arguments, account data, tokens, or secret state contents."
derived:
  - "The static path is sufficient to prove deterministic coordinate traversal and ordered field-content traversal before world-map storage mutation."
  - "0x314b480 remains the high-confidence AppearanceInstance default-instance candidate because it is passed to the protobuf message-selection helper for each ordered field content before 0xceca50 materializes the corresponding runtime map-content object."
  - "The lowest-risk dynamic proof point remains after protobuf translation and at/before TWorldmapProtocolMessageHandler / 0x19a8a80, rather than encrypted TCP reconstruction."
unknown:
  - "A live decoded FullMap/FieldData message has not yet been captured and normalized to a concrete (x,y,z) -> ordered contents -> appearance/type IDs sample."
  - "The exact semantic name of generated-object offsets +0x28 and +0x30 on each field-content protobuf object is not yet proven from static evidence alone."
  - "The exact appearance/type identifier field consumed inside the downstream runtime builder remains to be confirmed against a live message or stronger generated-protobuf type binding."
conflicts: []
validation:
  - command: "GitHub Actions run 31585256131 / job 94077482965"
    result: PASS
    evidence: "Bounded common-map-data trace generated and persisted; runtime identity and staging-preservation checks passed."
  - command: "GitHub Actions run 31585575487 / job 94078511849"
    result: PASS
    evidence: "Deep trace of common tail, map-content builder and protobuf helpers generated and persisted; active-session probe found no running client/display; staging-preservation check passed."
blockers:
  - "Final runtime acceptance requires an authenticated game-world session capable of producing a decoded FullMap/FieldData update. The owned analysis container currently has no running client and no graphical session. Credentials must not be pasted into chat, workflow inputs, scripts or logs."
next_action: "Establish the minimum approved interactive authenticated Tibia world session without exposing credentials; then immediately run one bounded decoded-message capture at the proven worldmap boundary and normalize one message to deterministic (x,y,z) -> ordered contents -> appearance/type IDs evidence."
```
