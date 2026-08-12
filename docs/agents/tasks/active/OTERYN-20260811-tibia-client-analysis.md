# OTERYN-20260811 Tibia client analysis runtime

## Objective

Inspect the current official Linux Tibia client on the Synology Oteryn staging host in the isolated task-owned analysis container, prove the decoded map-protocol boundary and obtain a deterministic live map record path without modifying canonical Oteryn staging services.

## Scope

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (draft)
- Runner label: `oteryn-staging`
- Verified runner: `oteryn-synology-staging`
- Owned runtime container: `oteryn-tibia-client-analysis`
- Owned bind: `/volume1/docker/oteryn/tibia-analysis` -> `/data`
- Ownership labels: `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`
- Graphical runtime: `DISPLAY=:99`, Xvfb `:99`
- Installed client: `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`

## Safety and lifecycle

Do not modify, restart, stop, remove, clean, or reconfigure canonical `oteryn-staging` Compose services, deployment infrastructure, databases, networks, volumes, or unrelated containers. No blanket Docker cleanup.

Do not commit proprietary Tibia binaries/assets, credentials, tokens, session material, account data or character-private data. The owned analysis container and bind remain intentionally retained while this task is active.

The currently authenticated client/session, when still alive, is valuable runtime state. Do not kill/restart/relaunch the client, restart Xvfb, recreate the container, disturb WARP/wireproxy, or relog merely for convenience. Discover the current client PID live; never hard-code an old PID. If an experiment requires terminating the authenticated process, stop and record why first.

## Durable evidence order

1. `docs/agents/prompts/OTERYN-20260811-tibia-client-analysis-continuation.md`
2. `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`
3. `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md`

## Context checkpoint

```yaml
checkpoint_version: 4
updated_at: 2026-08-12T18:00:00Z
branch: ops/oteryn-tibia-client-analysis-20260811
pr: 1006
status: authenticated_world_entry_proven_decoded_capture_pending
runtime:
  runner_label: oteryn-staging
  runner_name: oteryn-synology-staging
  container: oteryn-tibia-client-analysis
  bind: /volume1/docker/oteryn/tibia-analysis:/data
  display: :99
  client_path: /data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client
  client_process: client
  pid_policy: discover_live_with_pgrep_do_not_hardcode
  network_path: client -> proxychains4 -> SOCKS5 127.0.0.1:25344 -> wireproxy/userspace Cloudflare WARP -> Tibia
proven:
  - "Current official client is 15.32.df7b29; original bounded executable /data/client-15.32.df7b29/bin/client is 51965216 bytes with SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
  - "TWorldmapProtocolMessageHandler bodies include FullMap 0xcec8d0, FieldData 0xcd3190, Create 0xcecc70, Change 0xcecf40 and Delete 0xcd4e20."
  - "0x19a8a80 is a shared decoded map-data routine; repeated map fields and each field's repeated contents are traversed by increasing array index, preserving protobuf order while computing world coordinates."
  - "Coordinate protobuf schema is x=field 1 uint32, y=field 2 uint32, z=field 3 uint32."
  - "At 0x19a8e21 a content element is selected; generated-object values occur around +0x28/+0x30, nested payload selection from +0x10 uses helper 0x1ab4e50/default-instance candidate 0x314b480, followed by map-content builder 0xceca50."
  - "Userspace WARP is proven through wireproxy SOCKS5 127.0.0.1:25344. Client launch through proxychains4 can be confined to this path with no direct Tibia TCP/UDP."
  - "The official client launches on Xvfb :99 with Mesa lavapipe/software Vulkan and exposes a 1020x650 Tibia window in the proven runtime."
  - "Repository Actions secrets TIBIA_TEST_EMAIL and TIBIA_TEST_PASSWORD became available to the successful bounded login workflow and were used only via Actions secret injection; values were not printed, OCRed, persisted or committed."
  - "Non-OCR login/world entry is PROVEN by .github/workflows/tibia-client-analysis-cv-world-entry.yml, introduced at bd0bb114b8f812d849228ae325f8a5e2d71f6d62."
  - "Successful world-entry run 31620129239 attempt 1 / job 94192583991 completed successfully; external account history later showed Last Login Aug 12 2026 18:57:15 CEST, consistent with this entry."
  - "Successful rerun 31620129239 attempt 2 / job 94202682934 completed successfully. It proved CLIENT_LOCAL_SOCKS_MAX=7, CLIENT_DIRECT_TCP_SEEN=0, CLIENT_UDP_SEEN=0, CLIENT_SUSTAINED_TUNNELED_SESSION=1."
  - "The successful non-OCR login geometry for the exact 1020x650 layout is approximately email (535,275), password (535,304), Login (590,388); transition away from login is detected by image differencing, not OCR, with the proven >45000 changed-pixel threshold."
  - "First character row activation is proven around (285,193): click + Return, followed after 3 seconds by a deterministic double-click on the same row as bounded fallback."
  - "World presence/action was independently proven in successful attempt 2: VIEWPORT_CHANGED_PIXELS_AFTER_RIGHT=117976, CONTROLLED_MOVE_ACTION_PROVEN=true, PHYSICAL_WORLD_SESSION_AND_ACTION_PROVEN=true; Left was then sent to return to the starting tile."
  - "The successful login workflow intentionally leaves the client process running after job completion. A subsequent active-session workflow found the surviving session with ACTIVE_LOCAL_SOCKS_COUNT=2 and ACTIVE_DIRECT_TCP_COUNT=0."
  - "Qt/Tibia may recreate/replace its X11 window across transitions; old window IDs are not durable. Future interactions must resolve the current visible Tibia window from the live client PID rather than retaining a historical X11 ID."
  - "Classical/Tesseract OCR is not required for successful login/world entry and was unreliable as the primary targeting mechanism."
  - "UI pixel differences alone are not semantic proof. A private-message experiment changed the UI substantially but the user observed the intended message had not been delivered and a movement occurred instead."
  - "Later V2 chat experiment typed '*Glera Mars* zajaczek to faja' into the bottom chat area and preserved active SOCKS, but absent independent delivery confirmation it must not be labelled a proven delivered private message."
derived:
  - "The highest-value live map capture point remains after protobuf translation and at/before TWorldmapProtocolMessageHandler / 0x19a8a80, rather than reconstructing encrypted TCP first."
  - "For future non-OCR control, the desired architecture is decoded GameState for reads plus internal outbound action/message builders for writes; xdotool/CV should become diagnostic/recovery mechanisms rather than the final control interface."
  - "If the current authenticated process is still alive, attaching/instrumenting it after world entry is preferable to starting the client under GDB because startup-under-GDB previously changed timing/UI behavior and failed to reproduce the otherwise proven entry path."
unknown:
  - "Whether the authenticated client process/session is still alive at the start of the next agent session; this must be verified live before any relog/restart."
  - "A live decoded FullMap/FieldData message has not yet been normalized to a concrete (x,y,z) -> ordered contents -> appearance/type IDs sample."
  - "The exact semantic names of generated-object offsets +0x28/+0x30 and the final appearance/type identifier consumed downstream remain unproven."
  - "Protocol-native outbound movement/turn/use/attack/chat message builders have not yet been proven; existing successful movement used xdotool."
  - "The exact amount of OTBM-relevant map coverage recoverable from visible/received/cached official-client state remains to be proven."
conflicts:
  - "Older checkpoint_version 3 and the 2026-08-12T14:01Z section said authentication was blocked because Actions secrets resolved empty. This is superseded by later successful authenticated world-entry runs after the secrets became available."
validation:
  - command: "GitHub Actions run 31620129239 attempt 2 / job 94202682934"
    result: PASS
    evidence: "WARP verified; non-OCR login transition; first-character activation; sustained tunneled session; controlled in-world movement; privacy-safe proof; canonical staging unchanged."
  - command: "GitHub Actions job 94203489987"
    result: PASS_WITH_SEMANTIC_LIMIT
    evidence: "Verified surviving active session and zero direct TCP; private-message semantic outcome was not proven and must not be inferred from pixel change."
  - command: "GitHub Actions run 31621938187 / jobs 94198656638 and 94201117705"
    result: FAIL_FOR_DECODED_CAPTURE
    evidence: "Authentication observed but DECODED_RECORD_COUNT_AFTER_ENTRY=0; GDB/UI path did not reproduce character entry."
  - command: "GitHub Actions run 31624128761"
    result: FAILURE
    evidence: "Attempted live-worldmap attach workflow failed; do not treat it as decoded-map evidence. Inspect exact logs before retrying rather than repeating blindly."
constraints:
  - "Preserve an existing authenticated session whenever technically possible."
  - "Tibia may disconnect an idle character; if long analysis risks timeout, use the smallest verified reversible keepalive, preferably turn-in-place and restore direction, not autonomous wandering."
  - "Before and after invasive runtime work verify live client PID, tunneled connection, zero direct Tibia TCP, and that the character was not intentionally logged out."
  - "The recovery login workflow contains pkill -x client and therefore MUST NOT be rerun while an authenticated client survives."
  - "Do not use UI_CHANGED_PIXELS alone as proof of a semantic game action."
next_action: "First verify whether the authenticated client PID and tunneled world session still survive inside oteryn-tibia-client-analysis. If alive, preserve it and perform one non-destructive attach/instrumentation experiment at the already-proven decoded Worldmap boundary to capture a real (x,y,z) -> ordered contents -> appearance/type IDs record, then detach safely and re-verify the session. Only if the session is genuinely gone, recover it using the proven non-OCR WARP-confined world-entry workflow."
```

## Proven recovery/login recipe

Use only if live inspection proves that the existing authenticated session is gone.

1. Verify the task-owned container and labels; never touch canonical `oteryn-staging` services.
2. Verify `wireproxy` and `curl --socks5-hostname 127.0.0.1:25344 .../cdn-cgi/trace` reports `warp=on` before credential use.
3. Launch the installed official client through `proxychains4` using SOCKS5 `127.0.0.1:25344`, on Xvfb `:99`, with lavapipe software Vulkan.
4. Resolve the largest visible `^Tibia$` window and require the proven exact `1020x650` geometry for the fixed-coordinate path.
5. Inject Actions secrets only into the bounded login step. Click email `(535,275)`, Ctrl+A/type email; password `(535,304)`, Ctrl+A/type password; click Login `(590,388)`; immediately unset secret variables.
6. Detect a material transition away from the login form using `compare -metric AE`, not OCR; the proven threshold is >45000 changed pixels.
7. Activate the first character around `(285,193)` with click + Return; after 3 seconds use double-click at the same point as bounded fallback.
8. Prove world entry by sustained client TCP exclusively to local SOCKS plus an actual in-world action/viewport response. Do not treat authentication alone as world entry.
9. Leave the client process alive after success.

Canonical implementation: `.github/workflows/tibia-client-analysis-cv-world-entry.yml`.

## Research direction after world entry

Prioritize structured runtime data over OCR:

`server -> session processing -> decoded protobuf/messages -> Worldmap/GameState -> normalized world model`

For outbound actions investigate:

`game action -> internal client action/message builder -> serialization -> session framing/encryption -> TCP`

The immediate map acceptance target remains one real bounded runtime record:

`(x,y,z) -> ordered tile/field contents -> appearance/type IDs`

Only after this is proven should an OTBM exporter or protocol-native control layer be treated as feasible rather than assumed.