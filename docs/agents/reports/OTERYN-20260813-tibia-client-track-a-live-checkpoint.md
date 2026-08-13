# OTERYN-20260813 Tibia official-client Track A live checkpoint

## Purpose

Durable continuation record for **Track A = the official Linux Tibia client**. This checkpoint supersedes chat-only state from 2026-08-13 and must be read together with `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md` and PR #1006.

Do not confuse this with Track B. Work described here is exclusively the official CipSoft Linux client running in the task-owned analysis container.

## Repository and runtime

- Repository: `blakinio/Oteryn-Platform`
- Branch: `ops/oteryn-tibia-client-analysis-20260811`
- PR: `#1006` (draft)
- Runner label: `oteryn-staging`
- Verified runner: `oteryn-synology-staging`
- Owned container: `oteryn-tibia-client-analysis`
- Owned bind: `/volume1/docker/oteryn/tibia-analysis:/data`
- Display: `:99`
- Official client: `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`
- Network path: `client -> proxychains4 -> SOCKS5 127.0.0.1:25344 -> wireproxy/Cloudflare WARP -> Tibia`
- Client PID is ephemeral. Always resolve it live with `pgrep -x client`; never reuse a historical PID.

## Safety and secret handling

- Never commit or print account email/password, tokens, cookies, session material, private account data, proprietary Tibia binaries or extracted proprietary assets.
- Login credentials exist only as GitHub Actions secrets `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD`; workflows inject them only into the bounded login step and unset them immediately after typing.
- Never touch canonical `oteryn-staging` Compose services, databases, networks, volumes or unrelated containers.
- Preserve a live authenticated session. Do not relog/restart merely for convenience.
- The recovery workflow contains `pkill -x client`; it is permitted only after live inspection proves the authenticated session is gone.
- Before and after runtime experiments verify that the client uses local SOCKS and has zero direct Tibia TCP/UDP.

## PROVEN — recovery/login procedure without OCR

Canonical workflow: `.github/workflows/tibia-client-analysis-cv-world-entry.yml`.

The login path that repeatedly succeeded is:

1. Verify the owned container is running and has labels `com.blakinio.owner=oteryn` and `com.blakinio.purpose=tibia-client-analysis`.
2. Verify `wireproxy` exists and prove WARP before any credential use with `curl --socks5-hostname 127.0.0.1:25344 .../cdn-cgi/trace`; require `warp=on`.
3. Configure `proxychains4` with strict SOCKS5 `127.0.0.1:25344` and proxy DNS.
4. Use Xvfb `DISPLAY=:99` and Mesa lavapipe/software Vulkan (`VK_ICD_FILENAMES`, `VK_DRIVER_FILES`, `LIBGL_ALWAYS_SOFTWARE=1`).
5. Only if the old authenticated client is genuinely gone, terminate stale `client` processes and launch the installed official client through `proxychains4`.
6. Resolve the current largest visible `^Tibia$` X11 window. For the fixed-coordinate recovery path require exact geometry `1020x650`; old X11 window IDs are not durable.
7. Inject `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD` from Actions secrets only into this step. Use the proven geometry:
   - email field approximately `(535,275)`; Ctrl+A, type email;
   - password field approximately `(535,304)`; Ctrl+A, type password;
   - Login button approximately `(590,388)`.
   Immediately unset both secret variables after submission.
8. Detect leaving the login form by screenshot image differencing (`compare -metric AE`), **not OCR**. Proven threshold: more than `45000` changed pixels.
9. Resolve/keep the current Tibia window and activate the first character row around `(285,193)` with click + Return; after about 3 seconds use a bounded double-click at the same location as fallback.
10. Prove world entry by a sustained tunneled client connection plus a real in-world viewport/action response. Authentication/socket existence alone is not sufficient proof.
11. Leave the client process running after successful world entry.

Fresh 2026-08-13 recovery evidence: rerun job `94531254552` of workflow run `31620129239` completed `SUCCESS` and reported the established markers including `LOGIN_SUBMITTED=true`, visual transition, first-character activation, `CLIENT_SUSTAINED_TUNNELED_SESSION=1`, zero direct TCP/UDP, controlled world action and `PHYSICAL_WORLD_SESSION_AND_ACTION_PROVEN=true`.

## PROVEN — decoded Worldmap live capture

Canonical workflow: `.github/workflows/tibia-client-live-worldmap-attach.yml`.

Fresh 2026-08-13 job `94533266345` completed `SUCCESS` against the live authenticated client and retained the session.

Observed evidence:

- `CLIENT_PID=26286` for that bounded run (historical only; do not reuse);
- `PIE_BASE=0x555fd28f5000` for that process;
- decoded breakpoint = runtime PIE base + `0x19a8ea3`;
- `ACTIVE_LOCAL_SOCKS_COUNT=2` before attach;
- `ACTIVE_DIRECT_TCP_COUNT=0` before attach;
- `ACTIVE_LOCAL_SOCKS_COUNT_AFTER_ATTACH=2`;
- `ACTIVE_DIRECT_TCP_COUNT_AFTER_ATTACH=0`;
- `DECODED_CAPTURE_RECORD_COUNT=83`;
- `LIVE_SESSION_RETAINED=true`;
- `LIVE_DECODED_WORLDMAP_CAPTURE_PROVEN=true`.

The capture emits real decoded records in the form:

```text
REC x=<uint32> y=<uint32> z=<uint32> order=<uint32> raw28=<uint32> raw30=<uint32>
```

Examples from the fresh run include tiles around `x=32536..32555`, `y=32508..32521`, floors `z=6` and `z=7`, and ordered multiple contents on individual tiles.

This proves a structured decoded Worldmap source without OCR. It does **not** yet prove the semantic meaning of `raw28`/`raw30`.

## PROVEN — native/internal action reverse direction

Static workflow `.github/workflows/tibia-client-native-rotate-callgraph.yml` ran successfully as run `31724477571`, job `94529323245`, exact head `f853166c599215a6ab0222bfa15215f9c3e3ffb9`.

Important static facts for this exact client binary SHA-256 `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`:

- `sendRotateEast` body at file VA `0xee3250`;
- it activates Qt signal id `13` through `QMetaObject::activate`;
- movement/action metacall region starts at `0xd1a920`;
- string/type evidence includes:
  - `TProtocolWriter` near `0x1cac340`;
  - `TProtocolClientMessageProcessor` near `0x1cac3e0`;
  - `sendRotateEast` / `GameclientMessageRotateEast` pairs near `0x1cb53a3/0x1cb53b2` and `0x1cd1209/0x1cd1218`;
  - `sendRotateWest` / `GameclientMessageRotateWest` pairs near `0x1cb5456/0x1cb5465` and `0x1cd12bc/0x1cd12cb`;
  - `prepareAndEnqueueGameclientMessage` string at `0x1cd0b47`.

These are string/static-code locations, not automatically function entry points. In particular, `0x1cd0b47` is the string location for the name `prepareAndEnqueueGameclientMessage`; the function entry address remains `UNKNOWN` until xref resolution proves it.

Earlier runtime work on this branch also established that internal movement/rotation paths can be invoked without OCR/mouse/keyboard. The long-term control architecture should therefore be decoded GameState for reads plus native/internal client message/action builders for writes, with X11/CV retained only as recovery/diagnostic fallback.

## PROVEN / OWNER-SOURCE — current navigation correction

A previous decoded-viewport inference estimated the player around `32544,32514,7`. That estimate was **not** an exact player-position decoder and must not be treated as authoritative.

The repository owner subsequently supplied the actual observed position and screenshot evidence:

```json
{
  "position": {
    "x": 32554,
    "y": 32512,
    "z": 7
  }
}
```

The owner observed that the attempted eastward movement stopped because the character ran into a blocking lamp/object near the barrel. Therefore repeated key submission did not imply repeated tile movement.

Requested destination at that point was:

```json
{
  "position": {
    "x": 32572,
    "y": 32498,
    "z": 7
  }
}
```

Geometric delta from the owner-confirmed position is `+18 X, -14 Y`, but no straight-line route is proven.

## CONFLICT / correction — naive movement workflow

The current branch head contains `.github/workflows/tibia-client-two-steps-right.yml` renamed internally to `Tibia Route To Target`, which sends a precomputed batch of `28 Right` followed by `16 Up` inputs.

This strategy is now invalid as pathfinding evidence because a blocked tile can consume input without changing player position. Do not use `ROUTE_INPUTS_COMPLETED=true`, sent-key counts or viewport change alone as proof of arrival.

The workflow should be treated as experimental/stale until replaced or reduced. It must not be used as an autonomous pathfinder.

## DERIVED — required movement controller design

Navigation must be closed-loop and position-confirmed:

```text
read current player XYZ
-> obtain/passability model for neighboring tiles
-> choose one next walkable tile
-> issue one movement action
-> confirm player XYZ changed to the expected tile
-> update map/obstacles
-> repeat
```

Requirements:

- one movement step at a time or another strictly bounded step size;
- distinguish **command sent** from **movement accepted**;
- detect blocked tiles and replan;
- use decoded map/GameState rather than screen pixels for semantic state whenever possible;
- do not infer exact player coordinates solely from the center of a decoded viewport strip;
- only declare arrival when exact player XYZ equals the target.

A* or equivalent graph search becomes appropriate once tile passability can be derived reliably from the decoded world model. Until then, movement should use short probes plus exact position confirmation rather than long blind key sequences.

## PROVEN — evidence semantics that future agents must preserve

- `xdotool key ...` proves input submission, not successful movement.
- viewport pixel change proves visual change, not semantic action completion.
- active SOCKS proves a connection, not necessarily in-world presence.
- decoded Worldmap records prove structured map data, but not exact player XYZ by themselves unless the player-position relationship is separately decoded.
- owner-observed exact position may correct a derived viewport-center estimate; preserve the correction rather than averaging the two.
- `raw28` and `raw30` semantics remain unknown until correlated with concrete appearance/type/content structures.

## UNKNOWN

- Exact live player-position structure/API in GameState independent of viewport inference.
- Exact semantic names and meaning of decoded-content offsets represented as `raw28` and `raw30`.
- Exact function entry for `prepareAndEnqueueGameclientMessage` and the complete outbound framing path.
- ABI/arguments for higher-level native actions such as attack, follow, use-on-object/use-on-creature, move object and container operations.
- A reliable tile passability classifier and collision-aware pathfinder based on decoded map content.
- Exact OTBM coverage obtainable from current visible/received/cached official-client state.

## Next action

Implement/prove an exact **player XYZ read** from decoded GameState/runtime state, independent of viewport-center inference. Then use it to build a one-step closed-loop movement probe that confirms `before XYZ -> command -> after XYZ`, detects a blocked step, and leaves the session alive. Only after that should collision-aware pathfinding toward `32572,32498,7` resume.

## Start-here for the next agent

Read in this order:

```text
AGENTS.md
docs/agents/REPOSITORY_MAP.md
docs/agents/CONTEXT_ROUTING.md
docs/agents/PROMPTING_STANDARD.md
docs/agents/PROMPTING_HANDOVER.md
docs/agents/CONTEXT_HANDOFF.md
docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md
docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md
docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md
docs/agents/reports/OTERYN-20260813-tibia-client-track-a-live-checkpoint.md
```

Then inspect live PR #1006 head and runtime/session state. Do not rely on chat history.
