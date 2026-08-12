# Continuation prompt — OTERYN-20260811 Tibia client analysis

Copy the prompt below verbatim into a fresh agent session.

---

Read and execute the durable state for task `OTERYN-20260811-tibia-client-analysis` and continue autonomously until the decoded-map runtime capture objective is proven or a genuine authority boundary is reached.

Repository: `blakinio/Oteryn-Platform`
Branch: `ops/oteryn-tibia-client-analysis-20260811`
PR: `#1006` (draft)

Do not reconstruct the investigation from chat. Treat live GitHub/repository/runtime state as authoritative and read applicable governance before mutation, especially root `AGENTS.md`, `docs/agents/PROMPTING_STANDARD.md`, `docs/agents/PROMPTING_HANDOVER.md`, `docs/agents/CONTEXT_HANDOFF.md`, `docs/agents/EXECUTION_RESOURCE_HYGIENE.md`, `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md`, `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`, and `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md`.

Primary objective: prove one real decoded Tibia Worldmap runtime capture equivalent to:

`(x, y, z) -> ordered tile/field contents -> appearance/type IDs`

## PROVEN runtime/client state

- official current client 15.32.df7b29; original bounded executable `/data/client-15.32.df7b29/bin/client`, size 51965216, SHA-256 `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`;
- installed official runtime exists under `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`;
- official client launches successfully under Xvfb `:99` using Mesa software Vulkan/lavapipe;
- `TWorldmapProtocolMessageHandler`: FullMap `0xcec8d0`, FieldData `0xcd3190`, Create `0xcecc70`, Change `0xcecf40`, Delete `0xcd4e20`;
- shared decoded map routine `0x19a8a80` preserves repeated field/content order and computes world coordinates;
- Coordinate protobuf schema is `x=1:uint32`, `y=2:uint32`, `z=3:uint32`;
- per-content path selects nested payload using helper `0x1ab4e50`, default-instance candidate `0x314b480`, then calls map-content builder `0xceca50`;
- lowest-risk dynamic interception remains after protobuf translation and at/before Worldmap handler / `0x19a8a80`, not encrypted TCP.

## PROVEN authentication, egress, world entry, and action path — 2026-08-12

Do NOT rediscover this from OCR. A successful non-OCR world-entry workflow now exists and has succeeded twice.

### Network/egress

- Userspace Cloudflare WARP is operational through `wireproxy` and exposes SOCKS5 at `127.0.0.1:25344` inside the task-owned analysis container.
- Before credential use, verify `curl --socks5-hostname 127.0.0.1:25344 https://www.cloudflare.com/cdn-cgi/trace` reports `warp=on`.
- Launch the Tibia client under `proxychains4` configured with `strict_chain`, `proxy_dns`, and `socks5 127.0.0.1 25344`.
- Successful world-entry run observed client TCP only through local SOCKS: `CLIENT_LOCAL_SOCKS_MAX=7`, `CLIENT_DIRECT_TCP_SEEN=0`, `CLIENT_UDP_SEEN=0`.
- Do not use the household/public IP for login.

### Secrets boundary

- Repository Actions secrets `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD` are authorized only for this bounded test login.
- Inject them only into the GitHub Actions login step / `docker exec` environment.
- Never print, OCR, persist, commit, screenshot, echo, or otherwise expose their values.
- Immediately `unset TIBIA_TEST_EMAIL TIBIA_TEST_PASSWORD` after submitting login.

### Successful login/world-entry implementation

Canonical successful workflow on this branch:

`.github/workflows/tibia-client-analysis-cv-world-entry.yml`

Commit introducing the proven implementation: `bd0bb114b8f812d849228ae325f8a5e2d71f6d62` (`ci: enter Tibia world without OCR targeting`).

Successful run #1:
- workflow run `31620129239`, attempt 1;
- job `94192583991`;
- completed `success` at 2026-08-12 16:57:40 UTC;
- external account history subsequently showed Last Login `Aug 12 2026, 18:57:15 CEST`, independently consistent with this successful world entry.

Successful rerun:
- workflow run `31620129239`, attempt 2;
- job `94202682934`;
- completed `success` at 2026-08-12 17:33:16 UTC.

The proven sequence is:

1. Verify owned container labels/status and WARP.
2. Ensure lavapipe ICD exists (`lvp_icd*.json`).
3. Start official client on Xvfb `:99` through proxychains/WARP.
4. Resolve the largest visible `^Tibia$` X11 window. In the proven runtime it is exactly `1020x650`.
5. Login WITHOUT OCR using fixed geometry already established for this exact layout:
   - email field around `(535,275)`;
   - password field around `(535,304)`;
   - Login button around `(590,388)`.
6. Detect transition away from login by image differencing (`compare -metric AE`), not OCR. Proven threshold: more than `45000` changed pixels.
7. First character row is around `(285,193)`. Activate with click + Return, then after 3 s perform a deterministic double-click on the same row as bounded fallback.
8. Treat sustained tunneled runtime TCP plus an in-world visual action as world-entry evidence; do not infer world entry merely from authentication.
9. Wait for world render, capture viewport, send `Right`, wait 2 s, and require a material viewport change. In successful attempt 2: `VIEWPORT_CHANGED_PIXELS_AFTER_RIGHT=117976`.
10. Send `Left` to return the character to the starting tile. Successful run emitted `CONTROLLED_MOVE_ACTION_PROVEN=true` and `PHYSICAL_WORLD_SESSION_AND_ACTION_PROVEN=true`.
11. Leave the client process running; the GitHub Actions step exiting does not intentionally terminate the Tibia client. Subsequent workflow observed an active session with two local SOCKS connections and zero direct TCP.

Important implementation detail: later GDB experiments showed that assuming an X11 window ID remains valid across UI transitions is unsafe. Qt/Tibia may replace/recreate the visible window. For future interaction, resolve the current visible Tibia window by the live client PID instead of retaining an old X11 ID. The simple CV world-entry workflow succeeded because its exact flow/window happened to remain usable; more general automation must re-resolve as needed.

### What failed / do not repeat blindly

- Full-screen/classical Tesseract OCR was unreliable for login/character-selection targeting. It is no longer required for successful login/world entry.
- `UI_CHANGED_PIXELS` alone is NOT proof that a requested semantic action occurred. A private-message experiment proved this: a key sequence changed the screen but the user observed that no message arrived and the character moved. Verify semantic outcomes separately.
- GDB V4 worldmap run `31621938187` (job `94198656638`, rerun job `94201117705`) authenticated successfully (`AUTH_TCP_OBSERVED=1`) but failed to enter/capture world state: `DECODED_RECORD_COUNT_AFTER_ENTRY=0`. The decoded-map breakpoint being ready does not prove that character activation succeeded.
- The GDB workflow's strict `resolve_window()` by PID/1020x650 failed after character activation because the window lifecycle changed. Do not interpret that as authentication failure.

### Additional active-session interaction experiments

- After successful world entry, job `94203489987` verified `ACTIVE_LOCAL_SOCKS_COUNT=2`, `ACTIVE_DIRECT_TCP_COUNT=0` and resolved the live Tibia window. Its `Ctrl+O` private-message attempt is NOT semantically proven and must not be used as evidence of successful chat sending.
- A later V2 chat workflow `.github/workflows/tibia-send-private-message-v2.yml` used the active session, focused the bottom chat area and typed `*Glera Mars* zajaczek to faja`, with active SOCKS preserved. Image differencing proved text/UI changes but, unless externally confirmed, this still must not be labelled a delivered private message. Do not confuse visual reaction with server-side delivery.

## Current decoded-map objective

World entry itself is now PROVEN. The remaining primary task is decoded Worldmap runtime capture.

1. Reuse the proven non-OCR login/world-entry path above rather than the failed GDB character-selection variant.
2. Instrument the already-proven decoded Worldmap boundary, preferably `0x19a8a80` or an immediately adjacent deterministic point after protobuf translation and before `TWorldMapStorage` mutation.
3. The instrumentation must not destabilize or alter the proven UI activation path. Prefer attaching/instrumenting only after physical world entry has been independently established if startup-under-GDB changes timing/window behavior.
4. Capture one bounded real FullMap/FieldData (or equivalent relevant map update) and normalize it to deterministic evidence: `(x,y,z) -> ordered contents -> appearance/type IDs`.
5. Tie `0x314b480` and relevant generated-object offsets to concrete protobuf semantics if live capture permits.
6. Persist safe bounded evidence in `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md` and update the active task checkpoint with PROVEN / DERIVED / UNKNOWN / CONFLICT.
7. Verify canonical staging unchanged after runtime work.
8. Do not declare decoded-map completion without the real decoded-message capture proof.

## Owned runtime

- runner label `oteryn-staging`; verified runner `oteryn-synology-staging`;
- container `oteryn-tibia-client-analysis`;
- persistent bind `/volume1/docker/oteryn/tibia-analysis` -> `/data`;
- required labels `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`;
- canonical `oteryn-staging` Compose services are strictly out of scope for stop/recreate/reconfigure/cleanup.

## Safety

- Never modify, restart, stop, recreate, clean or otherwise disturb canonical `oteryn-staging` services, deployment infrastructure, databases, networks, volumes or unrelated containers.
- No blanket Docker cleanup.
- Before every owned-container mutation, verify both ownership labels.
- Do not commit Tibia binaries/assets, credentials, tokens, session material, account information or protected character data.
- Screenshots after authentication must be privacy-safe/redacted and must not reveal account/character/private information unless strictly required and explicitly safe.
- Do not mistake pixel differences, socket presence, or key injection by themselves for semantic proof; state exactly what each observation proves.

Do not stop merely because a workflow, commit or checkpoint was created. Continue autonomously until the decoded-map runtime capture is proven or a genuine authority/environment boundary is reached. If blocked, leave exactly one concrete `next_action` in durable repository state.