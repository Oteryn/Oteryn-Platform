# OTERYN-20260811 Tibia client analysis runtime

## Objective

Inspect the current official Linux Tibia client on the Synology Oteryn staging host in the isolated task-owned analysis container, prove structured decoded game/map state and native/internal control paths without OCR as the primary mechanism, and determine whether the resulting data can support collision-aware navigation and OTBM-relevant map reconstruction without modifying canonical Oteryn staging services.

**Lane identity:** this task is **Track A = official CipSoft Linux Tibia client**. Do not confuse it with Track B.

## Scope and ownership

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (draft)
- Runner label: `oteryn-staging`
- Verified runner: `oteryn-synology-staging`
- Owned runtime container: `oteryn-tibia-client-analysis`
- Owned bind: `/volume1/docker/oteryn/tibia-analysis:/data`
- Ownership labels: `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`
- Graphical runtime: `DISPLAY=:99`, Xvfb `:99`
- Installed client: `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`
- Client PID policy: discover live with `pgrep -x client`; never hard-code a historical PID.
- Network path: `client -> proxychains4 -> SOCKS5 127.0.0.1:25344 -> wireproxy/userspace Cloudflare WARP -> Tibia`.

Owned paths for this task are limited to task/report documentation and task-specific `.github/workflows/tibia-client-*` research workflows already present on PR #1006. Canonical application/runtime paths remain out of scope.

## Safety and lifecycle

- Do not modify, restart, stop, remove, clean or reconfigure canonical `oteryn-staging` Compose services, deployment infrastructure, databases, networks, volumes or unrelated containers.
- No blanket Docker cleanup.
- Never commit proprietary Tibia binaries/assets, credentials, tokens, cookies, session material, account data or character-private data.
- Login credentials are supplied only by GitHub Actions secrets `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD`; never print or persist their values.
- Preserve an existing authenticated client/session whenever technically possible. Do not relog/restart merely for convenience.
- The recovery workflow contains `pkill -x client`; use it only after live inspection proves that the authenticated session is gone.
- Before/after invasive runtime work verify the live client PID, tunneled local SOCKS connection, zero direct Tibia TCP/UDP and session survival.

## Durable evidence order

1. `docs/agents/reports/OTERYN-20260813-tibia-client-track-a-live-checkpoint.md` — **latest canonical continuation checkpoint**.
2. `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md` — binary/package/protocol reverse history.
3. `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md` — decoded Worldmap dispatch evidence.
4. `docs/agents/prompts/OTERYN-20260811-tibia-client-analysis-continuation.md` — older continuation prompt; live task/report state overrides stale details.

## Context checkpoint

```yaml
checkpoint_version: 5
updated_at: 2026-08-13T19:45:00+02:00
branch: ops/oteryn-tibia-client-analysis-20260811
pr: 1006
status: authenticated_world_and_decoded_worldmap_proven_player_xyz_read_pending
lane: "Track A = official CipSoft Linux Tibia client"

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
  - "Current official client identity remains 15.32.df7b29; executable SHA-256 e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe."
  - "Non-OCR login/world entry is implemented by .github/workflows/tibia-client-analysis-cv-world-entry.yml. Credentials are injected only from Actions secrets TIBIA_TEST_EMAIL/TIBIA_TEST_PASSWORD and are not printed or committed."
  - "Recovery/login requires verified WARP through SOCKS5 127.0.0.1:25344, proxychains4, Xvfb :99, lavapipe software Vulkan, exact 1020x650 window geometry, fixed login coordinates email~(535,275), password~(535,304), Login~(590,388), image-difference login transition >45000 changed pixels, then first-character activation near (285,193) with Return and bounded double-click fallback."
  - "Fresh recovery on 2026-08-13 succeeded: workflow run 31620129239 rerun job 94531254552 returned SUCCESS and proved physical in-world entry/action with sustained tunneled session and zero direct TCP/UDP."
  - "Decoded Worldmap capture is PROVEN at runtime breakpoint PIE+0x19a8ea3. Fresh job 94533266345 succeeded with 83 decoded REC records, local SOCKS retained before/after attach, direct TCP 0 and LIVE_SESSION_RETAINED=true."
  - "Fresh decoded records cover real coordinates around x=32536..32555, y=32508..32521 and floors z=6/7 with ordered per-tile contents encoded as raw28/raw30 values."
  - "Static native rotate callgraph workflow run 31724477571 / job 94529323245 succeeded on exact client hash. sendRotateEast is at file VA 0xee3250 and activates Qt signal id 13; movement/action metacall region starts at 0xd1a920."
  - "The binary contains TProtocolWriter, TProtocolClientMessageProcessor, GameclientMessageRotateEast/West and prepareAndEnqueueGameclientMessage strings; prepareAndEnqueueGameclientMessage string is at 0x1cd0b47, but this is not yet proven to be the function entry."
  - "Earlier task runtime evidence established direct internal movement/rotation invocation without relying on OCR/mouse/keyboard as the final control path."
  - "Owner-provided current-position correction after the failed blind route is x=32554, y=32512, z=7. The owner observed the character stopped on a blocking lamp/object near a barrel."
  - "Requested destination at that point is x=32572, y=32498, z=7."

derived:
  - "The preferred architecture remains decoded GameState/Worldmap for reads plus internal outbound action/message builders for writes; X11/CV is recovery/diagnostic fallback."
  - "The previous viewport-center estimate near 32544,32514,7 was not an exact player-position decoder and is superseded for navigation by the owner-observed exact position 32554,32512,7."
  - "Navigation must be closed-loop: read exact XYZ -> choose one walkable neighbor -> issue one action -> confirm new XYZ -> replan on block -> repeat."
  - "Geometric delta from the corrected current position to the requested destination is +18 X and -14 Y, but no straight-line route is proven."
  - "A* or equivalent pathfinding becomes appropriate only after reliable neighboring-tile passability/collision classification exists."

unknown:
  - "Exact live player-position structure/API in GameState independent of decoded viewport-center inference."
  - "Semantic meaning/names of raw28 and raw30 and the final appearance/type/content identifier mapping."
  - "Exact function entry/xref for prepareAndEnqueueGameclientMessage and complete outbound writer/framing chain."
  - "ABI/arguments for higher-level native attack/follow/use/move-object/container actions."
  - "Reliable decoded tile passability classifier and collision-aware pathfinder."
  - "Exact OTBM-relevant coverage recoverable from visible/received/cached official-client state."

conflicts:
  - "Older checkpoint_version 4 stated live decoded capture was pending. This is superseded by successful job 94533266345 with 83 real decoded records."
  - "Older viewport-derived player estimate around 32544,32514,7 conflicts with the owner-observed exact position 32554,32512,7; do not use the old estimate for navigation."
  - "Current .github/workflows/tibia-client-two-steps-right.yml was changed into a blind batched route (28 Right, 16 Up). Its key-count completion is not pathfinding proof and the workflow must be treated as stale/experimental, not an autonomous navigator."

validation:
  - command: "GitHub Actions rerun job 94531254552 (run 31620129239)"
    result: PASS
    evidence: "WARP-confined non-OCR login, first-character world entry, sustained tunnel, zero direct TCP/UDP, controlled viewport/world action."
  - command: "GitHub Actions job 94533266345"
    result: PASS
    evidence: "83 decoded Worldmap records; local SOCKS retained; direct TCP 0; live session retained."
  - command: "GitHub Actions run 31724477571 / job 94529323245"
    result: PASS
    evidence: "Read-only static native rotate/action callgraph and outbound-message string evidence; live session observed through local SOCKS only."

constraints:
  - "Do not treat sent xdotool keys as successful tile movement."
  - "Do not treat viewport pixel change as semantic action completion."
  - "Do not treat active SOCKS alone as proof of in-world presence."
  - "Do not infer exact player XYZ solely from decoded viewport strip geometry."
  - "Do not use the current blind batched route workflow as a pathfinder."
  - "Preserve live authenticated session whenever possible."

next_action: "Implement/prove an exact player XYZ read from decoded GameState/runtime state, independent of viewport-center inference. Then run a one-step closed-loop movement probe that records before XYZ, issues one movement action, records after XYZ, detects a blocked tile without assuming success, and leaves the authenticated session alive. Only after this proof resume collision-aware navigation toward 32572,32498,7."
```

## Proven recovery/login recipe

Use this only if live inspection proves that the existing authenticated session is gone. Canonical implementation: `.github/workflows/tibia-client-analysis-cv-world-entry.yml`.

1. Verify the task-owned container, labels and canonical staging inventory boundary.
2. Verify wireproxy and prove `warp=on` through SOCKS5 `127.0.0.1:25344` before credential use.
3. Launch the official installed client through `proxychains4` on Xvfb `:99` with lavapipe/software Vulkan.
4. Resolve the current largest visible `^Tibia$` window and require exact `1020x650` geometry for the fixed-coordinate recovery path.
5. Inject Actions secrets only into the bounded login step. Click email `(535,275)`, Ctrl+A/type; password `(535,304)`, Ctrl+A/type; Login `(590,388)`; immediately unset the secret variables.
6. Detect transition away from the login form using image differencing, not OCR; require more than `45000` changed pixels.
7. Activate the first character near `(285,193)` with click + Return; after about 3 seconds use bounded double-click fallback at the same location.
8. Prove physical world entry by sustained client TCP exclusively through local SOCKS plus an actual in-world action/viewport response. Authentication/socket state alone is insufficient.
9. Leave the successful client process running.

## Research direction

Prioritize structured data over OCR:

```text
server -> session processing -> decoded protobuf/messages -> Worldmap/GameState -> normalized world model
```

For writes:

```text
semantic game action -> internal client action/message builder -> GameclientMessage -> client processor/queue -> writer/framing -> server
```

For navigation:

```text
exact player XYZ -> decoded neighboring tiles/passability -> one native move -> exact after XYZ -> replan
```

The immediate acceptance target is no longer merely a decoded tile sample; that is proven. The next acceptance target is an exact, repeatable **player XYZ read** plus one position-confirmed movement step that can distinguish a successful move from collision.
