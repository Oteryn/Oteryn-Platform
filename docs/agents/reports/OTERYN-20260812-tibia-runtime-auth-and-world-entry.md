# OTERYN-20260812 Tibia runtime authentication and world-entry findings

## Scope

This report persists only verified findings from the live Synology runtime work performed on branch `ops/oteryn-tibia-client-analysis-20260811` for the official Tibia Linux client in owned container `oteryn-tibia-client-analysis`.

It intentionally does **not** record credential values, account identifiers, character names, recovery material, proprietary client binaries, or any other secret-bearing data.

## FACT — userspace WARP path is proven

- Cloudflare WARP through `wgcf` + `wireproxy` SOCKS5 works in the owned runtime without relying on Synology TUN/nft support.
- GitHub Actions run `31604103984`, job `94138500351`, proved `warp=on`, changed public egress, and successful proxychains traversal.
- The same run proved the real Tibia client process had no direct remote TCP socket outside local SOCKS `127.0.0.1:25344` and emitted `CLIENT_TCP_USERSPACE_WARP_CONFINEMENT_VERIFIED=true`.
- Push run `31604419752`, job `94139596953`, independently repeated the WARP egress and Tibia TCP confinement proof.
- Canonical `oteryn-staging` container inventory remained unchanged in those runs.

## FACT — Actions secrets became available and account authentication succeeded

- After repository secrets were restored/created in `blakinio/Oteryn-Platform`, rerun attempt 2 of workflow run `31604419752`, job `94144553452`, received non-empty `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD` through GitHub Actions secret injection.
- The credential values were not printed, OCRed, persisted to repository files, or exposed in artifacts.
- The workflow successfully authenticated the test account through the proven userspace WARP path and reached the Tibia `Select Character` screen.
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

## FACT — GUI automation failure mode

- Static click coordinates are not reliable enough for the `Select Character` dialog.
- Attempting `Home` -> `Enter` after focusing the dialog did not reliably enter the world and in a later inspected state returned to the account-login screen.
- OCR also failed to consistently detect a visible `Login/Enter/Play` action control even though the character-selection screen itself was present.
- The reliable checkpoint is the authenticated `Select Character` screen, not the earlier login-form state.

## FACT — current world-entry strategy

The current safe strategy is:

1. Start/restart only the owned Tibia client runtime, never canonical staging services.
2. Authenticate through userspace WARP with Actions secrets.
3. Require OCR confirmation of `Select Character` before attempting a character action.
4. Detect the actual visible character row from runtime text/geometry rather than guessing button coordinates.
5. Activate the character row directly (for example, by double-clicking the detected row) instead of relying on a guessed `Login` button location.
6. Treat world entry as proven only after inspecting a post-action screenshot and confirming a real in-game viewport/state; generic OCR marker counts alone are insufficient.
7. Do not terminate the client after a proven successful entry, so the character can remain logged in for decoded-map runtime capture.

## FACT — current active execution

- Commit `e21a1b57e837d928280348abc29d93b2f36f3c3c` updated `.github/workflows/tibia-client-analysis-trace.yml` to attempt world entry from a proven visible character row and leave the client running after success.
- Workflow run `31611278517`, job `94162935321`, was in progress at the time this report was written, in step `Login account, select visible character row, and enter world`.
- Therefore **world entry and persistent online character state are not yet proven at this checkpoint**.

## Acceptance criteria for the next checkpoint

Do not mark the authenticated-world blocker resolved until all of the following are directly verified:

- authenticated `Select Character` checkpoint is reached;
- a concrete character row is selected from observed runtime geometry/text;
- post-action screenshot visibly shows the actual game world viewport and no account-login or character-selection dialog;
- the Tibia process remains alive after the workflow step exits;
- canonical `oteryn-staging` inventory remains unchanged;
- then continue immediately to decoded `FullMap/FieldData` capture at the already-proven worldmap boundary and normalize one concrete `(x,y,z) -> ordered contents -> appearance/type IDs` sample.
