# OTERYN-20260812 live decoded Worldmap capture

## PROVEN

- Repository: `blakinio/Oteryn-Platform`
- Task branch: `ops/oteryn-tibia-client-analysis-20260811`
- Capture workflow run: `31632627071`
- Capture job: `94234965002`
- Capture commit: `76098dc280fce2c38eeb1ce017247bdf77a8cb1e`
- Runner: `oteryn-synology-staging`
- Container: `oteryn-tibia-client-analysis`
- Live client PID during capture: `17893` (transient; never reuse as a constant)
- PIE base during capture: `0x564bf7949000`
- Runtime breakpoint: `0x564bf92f1ea3`, corresponding to static `0x19a8ea3` inside the already-identified common map-data routine.
- Before attach: `ACTIVE_LOCAL_SOCKS_COUNT=2`, `ACTIVE_DIRECT_TCP_COUNT=0`.
- After attach: `ACTIVE_LOCAL_SOCKS_COUNT_AFTER_ATTACH=2`, `ACTIVE_DIRECT_TCP_COUNT_AFTER_ATTACH=0`.
- `DECODED_CAPTURE_RECORD_COUNT=83`.
- The job completed successfully and emitted `LIVE_SESSION_RETAINED=true` and `LIVE_DECODED_WORLDMAP_CAPTURE_PROVEN=true`.

This is the first verified live decoded runtime sample satisfying the bounded structural target `(x,y,z) -> ordered contents -> decoded generated-object values` without OCR and without restarting/relogging the official client.

## Representative captured records

```text
REC x=32554 y=32510 z=7 order=0 raw28=1 raw30=486
REC x=32554 y=32510 z=7 order=1 raw28=1 raw30=7144
REC x=32554 y=32511 z=7 order=0 raw28=1 raw30=870
REC x=32554 y=32511 z=7 order=1 raw28=1 raw30=4534
REC x=32554 y=32511 z=7 order=2 raw28=1 raw30=4531
REC x=32536 y=32513 z=7 order=0 raw28=1 raw30=4407
REC x=32536 y=32513 z=7 order=1 raw28=1 raw30=313
REC x=32536 y=32513 z=7 order=2 raw28=1 raw30=6379
REC x=32536 y=32513 z=7 order=3 raw28=1 raw30=19394
REC x=32536 y=32513 z=7 order=4 raw28=1 raw30=6217
REC x=32555 y=32509 z=6 order=0 raw28=1 raw30=1168
REC x=32555 y=32509 z=6 order=1 raw28=1 raw30=20661
```

The capture proves multiple ordered contents on one coordinate and decoded records on at least floors `z=6` and `z=7` during the bounded turn-right/turn-left correlation experiment.

## DERIVED

- The live official client exposes deterministic world coordinates and per-coordinate ordered content traversal at the decoded map boundary.
- A normalized model `WorldTile{x,y,z,ordered_contents[]}` can be populated from this boundary.
- This materially strengthens OTBM reconstruction feasibility because tile coordinates and content order no longer depend on OCR or encrypted TCP reconstruction.

## UNKNOWN / claim boundary

- `raw30` is now proven to carry stable decoded per-content numeric values, but its exact semantic identity as an appearance ID, type ID, or another generated field is not yet proven. Do not rename it in durable evidence until tied to the generated protobuf/appearance type.
- `raw28=1` was observed in the captured records, but its semantic name remains unproven.
- Static-vs-dynamic classification, ground/item mapping, blocking/pathability attributes, and complete OTBM attribute mapping remain unproven.
- This capture does not prove complete global map coverage. It proves received runtime map updates across multiple coordinates/floors.
- The turn action used to stimulate map updates was still injected through `xdotool`; protocol-native outbound movement/turn remains UNKNOWN.

## Session preservation

A separate safe check immediately before capture (run `31632580452`, job `94234803949`) proved client PID `17893`, two local SOCKS connections and zero direct TCP. The capture job independently rechecked the same confinement before and after attach. No relogin, client restart, Xvfb restart, WARP/wireproxy restart, or container recreation was performed.

## Next action

Tie `raw30` and `raw28` at `0x19a8ea3` to their concrete generated protobuf/appearance fields and the downstream `0xceca50` map-content builder, then repeat one bounded live capture with the semantic field names. Only after that classify ground/items/dynamic creatures for deterministic OTBM conversion.
