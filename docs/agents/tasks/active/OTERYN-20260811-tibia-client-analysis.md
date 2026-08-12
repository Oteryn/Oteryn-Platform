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
checkpoint_version: 3
updated_at: 2026-08-12T12:01:30Z
branch: ops/oteryn-tibia-client-analysis-20260811
pr: 1006
status: blocked_on_authenticated_world_session
observed_pr_head_before_this_checkpoint: 66834dc93ffa167b8422cea3b55d20bfc683cbb3
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
  - "Current official client is 15.32.df7b29; original bounded executable /data/client-15.32.df7b29/bin/client is 51965216 bytes with SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
  - "TWorldmapProtocolMessageHandler exact bodies include FullMap 0xcec8d0, FieldData 0xcd3190, Create 0xcecc70, Change 0xcecf40 and Delete 0xcd4e20."
  - "0x19a8a80 is a shared map-data routine called by FieldData, FullMap/meta-dispatch and multiple directional/floor update paths."
  - "Coordinate protobuf schema is x=field 1 uint32, y=field 2 uint32, z=field 3 uint32."
  - "Run 31585256131 / job 94077482965 completed SUCCESS on oteryn-synology-staging, persisted bounded trace evidence and verified canonical oteryn-staging inventory unchanged."
  - "The central routine indexes repeated map-field entries by array index, computes world coordinates from region dimensions/base coordinates, then iterates each field's repeated content array by monotonically increasing index."
  - "At 0x19a8e21 the content element is selected from 0x8(base,index,8); the inner loop increments the content index and therefore preserves protobuf repeated-field order."
  - "Each selected content element exposes values at generated-object offsets +0x28 and +0x30 and selects a nested payload at +0x10 using helper 0x1ab4e50 with default-instance candidate 0x314b480 before calling map-content builder 0xceca50."
  - "Run 31585575487 / job 94078511849 completed SUCCESS on oteryn-synology-staging, persisted the bounded deep trace and again verified canonical staging unchanged."
  - "Official launcher was started in the owned container on Xvfb display :99. A bounded screenshot proved the central blue download tile; run 31593652146 / job 94104018357 uploaded artifact 9140141360 and verified canonical staging unchanged."
  - "Run 31593816173 / job 94104541247 clicked the verified download tile at window-relative coordinate 400,193. The launcher downloaded the official runtime, growing /data/home from 1320698 bytes to 388524108 bytes, and materialized /data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client. Canonical staging remained unchanged."
  - "Run 31594068971 / job 94105341913 launched the installed official client successfully. ldd reported no missing dependencies, process PID 5136 remained alive, and the 1020x650 Tibia client window was visible alongside the launcher. Artifact 9140336079 contains the bounded client welcome-screen capture. Canonical staging remained unchanged."
  - "Run 31594315448 / job 94106127186 opened the client's Login form and performed OCR only on allow-listed label words. It identified Email, Password, Remember Email, Account and Login labels without emitting arbitrary field contents. Artifact 9140453170 contains the bounded login-form screenshot. Canonical staging remained unchanged."
  - "The login screenshot directly shows empty Email and Password fields; no authenticated session or saved credential value is present in the fresh owned runtime."
derived:
  - "The static path is sufficient to prove deterministic coordinate traversal and ordered field-content traversal before world-map storage mutation."
  - "0x314b480 remains the high-confidence AppearanceInstance default-instance candidate because it is passed to the protobuf message-selection helper for each ordered field content before 0xceca50 materializes the corresponding runtime map-content object."
  - "The lowest-risk dynamic proof point remains after protobuf translation and at/before TWorldmapProtocolMessageHandler / 0x19a8a80, rather than encrypted TCP reconstruction."
  - "The owned Docker/Xvfb runtime is now sufficient for client-side authenticated dynamic capture once the user completes authentication interactively or through another approved secret-preserving mechanism."
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
    evidence: "Deep trace of common tail, map-content builder and protobuf helpers generated and persisted; staging-preservation check passed."
  - command: "GitHub Actions run 31593816173 / job 94104541247"
    result: PASS
    evidence: "Official launcher installation completed; installed client path materialized; staging-preservation check passed."
  - command: "GitHub Actions run 31594068971 / job 94105341913"
    result: PASS
    evidence: "Installed official client launched with no missing shared-library dependencies and remained alive; screenshot artifact 9140336079 uploaded; staging-preservation check passed."
  - command: "GitHub Actions run 31594315448 / job 94106127186"
    result: PASS
    evidence: "Login form opened; safe-label OCR identified form geometry; screenshot artifact 9140453170 uploaded; staging-preservation check passed."
blockers:
  - "Final runtime acceptance requires an authenticated game-world session capable of producing a decoded FullMap/FieldData update. The official client and graphical login form are now running in the owned container, but the Email and Password fields are empty. No approved credential source or existing authenticated session is available to this task. Credentials must not be committed, logged, placed in workflow inputs, or exposed through OCR output."
next_action: "Complete authentication through a user-controlled or otherwise approved secret-preserving interactive path into the already running owned client; then immediately run one bounded decoded-message capture at the proven worldmap boundary and normalize one message to deterministic (x,y,z) -> ordered contents -> appearance/type IDs evidence."
```
