# OTERYN-20260812 Tibia runtime authentication and world-entry findings

## Scope

This report persists only verified findings from the live Synology runtime work performed on branch `ops/oteryn-tibia-client-analysis-20260811` for the official Tibia Linux client in owned container `oteryn-tibia-client-analysis`.

It intentionally does **not** record credential values, account identifiers, character names, recovery material, proprietary client binaries, or any other secret-bearing data.

## FACT — userspace WARP path is proven

- Cloudflare WARP through `wgcf` + `wireproxy` SOCKS5 works in the owned runtime.
- GitHub Actions run `31604103984`, job `94138500351`, proved `warp=on`, changed public egress, and successful proxychains traversal.
- The same run proved the real Tibia client process had no direct remote TCP socket outside local SOCKS `127.0.0.1:25344` and emitted `CLIENT_TCP_USERSPACE_WARP_CONFINEMENT_VERIFIED=true`.
- Push run `31604419752`, job `94139596953`, independently repeated the WARP egress and Tibia TCP confinement proof.
- Canonical `oteryn-staging` container inventory remained unchanged in those runs.

## FACT — Actions secrets became available and account authentication succeeded

- After repository secrets were restored/created in `blakinio/Oteryn-Platform`, rerun attempt 2 of workflow run `31604419752`, job `94144553452`, received non-empty `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD` through GitHub Actions secret injection.
- The credential values were not printed, OCRed, persisted to repository files, or exposed in artifacts.
- The workflow successfully authenticated the test account through the proven WARP path and reached the Tibia `Select Character` screen.
- Privacy-safe screenshot artifact from that successful authenticated state was uploaded as artifact `9145110530`.

## FACT — recovery warning text

- A narrowly cropped system-message capture was produced by workflow run `31606528343`, job `94146769428`.
- OCR of that bounded strip returned the exact warning:
  - `You need to complete the recovery setup process!`
- This warning is displayed on the character-selection screen.
- User-side validation with the official Windows client proved that this account warning does **not** prevent normal world login; therefore the warning must not be treated as the blocker for Linux world entry.
- No recovery setup or recovery-key generation should be automated as part of this task.

## FACT — prior world-entry success claim was false

- Workflow run `31608703101`, job `94154190380`, emitted `TIBIA_WORLD_ENTRY_PROVEN=true`, but that result was a false positive.
- Manual inspection of the uploaded screenshot showed the `Account Login` screen rather than an in-game world viewport.
- The faulty validator counted generic OCR words such as `store`, `mana`, or similar terms as sufficient game-state evidence; this is not a valid world-entry proof.
- Any durable claim that run `31608703101` proved world entry is superseded by this report.

## FACT — GUI automation problems that caused earlier failures

- Early OCR row detection accidentally matched top-bar text such as `Players online` rather than a character row. This produced unsafe coordinates near the top of the client and was replaced with geometry anchored to the `Select Character` title and `Character` column header.
- Static click coordinates and `Home` -> `Enter` were not reliable enough for the character-selection dialog.
- The workflow was changed to derive the first visible character row from OCR geometry and later to target the first actual character-name token without emitting its text.
- Exact-name targeting was directly verified in run `31618392447`, job `94186782092`: the resolved target was `x=236, y=190` and the character-name geometry was accepted.
- Despite exact-name targeting, the first captured state after activation was already `Account Login`; at `150 ms`, `1 s`, `5 s`, and `15 s` there were no game markers. Therefore the present blocker is not explained by a missed row or obviously wrong click coordinate.

## FACT — activation returns locally before any game-server network transition

- Run `31616078668`, job `94179099943`, classified the retained syscall trace for the activation window.
- During the bounded post-activation window it observed exactly `24` `connect()` calls.
- All `24` were `AF_UNIX`; `AF_INET=0` and `AF_INET6=0`.
- It also observed `24` `AF_UNIX/SOCK_STREAM` socket creations and no Internet-family socket creation in that window.
- No Internet destination ports were present because no Internet-family connection was attempted.
- This proves the current reset occurs before the normal game-server network transition. A game-server refusal, WARP egress failure, TCP timeout, or wrong remote game-server port is therefore not the immediate observed failure mode.

## FACT — proxychains / LD_PRELOAD is not the sufficient cause

- A transparent TUN-to-SOCKS path was built in the owned container so the client could run without proxychains injection / `LD_PRELOAD` while the whole container still exited through the existing WARP SOCKS path.
- The transparent path was directly verified with `warp=on`, and the client was started without `LD_PRELOAD`.
- With that path the same behavior remained: authentication reached `Select Character`, the verified character row was activated, and the client returned to `Account Login` instead of entering the game.
- Therefore proxychains interposition is not sufficient to explain the failure.

## FACT — root execution is not the sufficient cause

- The official Linux client binary contains BattlEye-related strings, so running the client as root was treated as a plausible local gate.
- Run `31619426456`, job `94190213906`, moved the owned Tibia home to a temporary normal user and launched the client with effective UID `1000`.
- Non-root X access succeeded, account authentication succeeded, the exact character target was verified, and the character was activated.
- At `1 s`, `5 s`, `15 s`, and `45 s` the client was still back on `Account Login` with no game markers.
- The workflow restored ownership on failure.
- Therefore root execution / UID 0 is not sufficient to explain the current blocker.

## FACT — renderer failures were real, but fixing Vulkan alone did not solve world entry

- The Synology host exposed no `/dev/dri` devices to the job, and the owned container also had no `/dev/dri` render node.
- GLX worked through software Mesa/llvmpipe, while initial Vulkan diagnostics failed to create a Vulkan instance.
- Client configuration at `conf/clientoptions.json` contains `options.rendererIndex=0` at the verified baseline.
- Renderer-index probe run `31619046703`, job `94188955126`, showed the client could start for indices `0`, `1`, `2`, and `3`; the coarse startup log continued to contain a Vulkan-instance failure signature in that probe, so the index-to-backend mapping was not fully established there.
- A more targeted `rendererIndex=1` world-entry run `31619043410`, job `94188942946`, directly proved `RENDERER1_FAILED_VULKAN_INSTANCE=false`. That run then stopped on a shell syntax error before it could complete the login attempt, so it was not a valid world-entry result.
- Software Vulkan was then installed in the owned container using Mesa lavapipe. Run `31619426807`, job `94190214746`, proved:
  - lavapipe ICD present;
  - `vulkaninfo` successful;
  - CPU/software Vulkan device identified;
  - `CLIENT_FAILED_VULKAN_INSTANCE=false`;
  - `Account Login` reachable with software Vulkan;
  - account authentication and exact character-name target still successful.
- Even with working software Vulkan, post-activation states at `2 s`, `10 s`, and `30 s` were `Account Login` with no game markers.
- Therefore missing Vulkan initialization was a real runtime defect, but fixing it alone did not resolve the pre-game reset.

## FACT — hypotheses now excluded as sufficient explanations

The following are no longer sufficient standalone explanations for the failed world entry:

- wrong account credentials;
- failure to reach `Select Character`;
- the account recovery-setup warning;
- selecting the wrong obvious GUI region;
- userspace WARP failure;
- proxychains / `LD_PRELOAD` injection;
- game-server TCP refusal/timeout during the observed activation window;
- running the client as root;
- missing Vulkan instance support by itself.

## UNKNOWN — exact local gate that returns the client to Account Login

- The exact client-side condition that causes the immediate return to `Account Login` is still unknown.
- Verified evidence places the transition locally, before any `AF_INET/AF_INET6` connect attempt for the game session.
- No persistent visible error dialog has been captured between `Select Character` and `Account Login`; sub-second captures already show the login screen.
- The client logs seen so far do not contain a clear generic error matching the reset other than the renderer issue that has now been independently corrected without solving entry.
- The next useful evidence must therefore come from a narrower local state/dispatch observation, not another blind change to WARP or character coordinates.

## FACT — current execution strategy

The current safe strategy is:

1. Start/restart only the owned Tibia client runtime, never canonical staging services.
2. Authenticate through the already-proven WARP path with Actions secrets.
3. Require OCR confirmation of `Select Character` before attempting a character action.
4. Resolve the actual first character-name token from runtime geometry without persisting the name.
5. Activate only the verified character target.
6. Treat world entry as proven only after a post-action screenshot shows a real in-game world viewport/state and no account-login/character-selection dialog.
7. Require the Tibia process to remain alive after the workflow step exits.
8. Do not terminate the client after a proven successful entry, so the character remains online for decoded-map runtime capture.
9. Continue immediately to the already-proven decoded `FullMap/FieldData` boundary once world entry is real.

## FACT — current active execution checkpoint

- Commit `7ef53da08a8f5dde7a94bfc888ff2c9fdefc3e42` added a combined non-root + software-Vulkan world-entry test so the two local-environment fixes can be exercised together rather than only independently.
- Workflow run `31620102923` (`Tibia Nonroot Software Vulkan World Entry`) was in progress when this report checkpoint was updated.
- **World entry and persistent online character state are still not proven at this checkpoint.**

## Acceptance criteria for the next checkpoint

Do not mark the authenticated-world blocker resolved until all of the following are directly verified:

- authenticated `Select Character` checkpoint is reached;
- the concrete character-name target is derived from observed runtime geometry;
- post-action screenshot visibly shows the actual game world viewport and no account-login or character-selection dialog;
- the Tibia process remains alive after the workflow step exits;
- the network path remains confined to the approved WARP route;
- canonical `oteryn-staging` inventory remains unchanged;
- then continue immediately to decoded `FullMap/FieldData` capture at the already-proven worldmap boundary and normalize one concrete `(x,y,z) -> ordered contents -> appearance/type IDs` sample.
