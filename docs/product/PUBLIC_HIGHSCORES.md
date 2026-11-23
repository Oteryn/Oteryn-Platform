# Public highscores qualification (GAP-09)

The Community navigation opens `/{lang}/rankings/experience`. The page accepts
a registered world, a skill or ranking category, and a base vocation. The
world selector uses names from the Platform registry. A scope contains one
current-epoch Game snapshot. The page preserves published competition ranks
and tie order, filters for current public visibility, and paginates the
remaining rows at 50 per page. A fresh empty result has its own message. A
missing or expired cut returns the localized unavailable view with HTTP 503
and keeps the selectors available.

## Platform operating status

The existing `PORTAL_PUBLIC_RANKINGS_ENABLED` flag is off by default. The
existing `PORTAL_PUBLIC_PROJECTIONS_ENABLED` ingest remains restricted to
testing and preproduction with its source authority and verified mTLS
identity. The ranking also needs the configured Platform profile-privacy
reader, its ownership feed, and a current `character_profile` family.

Qualification uses a 30-second highscores watermark window and a five-minute
hard maximum for the computation timestamp, including one second of clock
uncertainty in both. The selected computation must precede or equal the
snapshot observation, and the observation must be covered by the current
family watermark. A watermark advances coverage without extending the
computation's lifetime. The page and home block show the computation time.

With the serving flag enabled in a qualification environment, send a GET for
a registered world and an expected category and vocation. HTTP 200 indicates a
qualifying cut and a resolved privacy view. HTTP 404 identifies an unknown
world or category, or a page outside the visible result. HTTP 400 identifies
malformed query parameters or a page beyond the maximum possible snapshot size.
HTTP 503 indicates that the view cannot currently be served.

For a 503, include `oteryn_community::rankings=debug` in the qualification
process's `RUST_LOG` and repeat the probe. The `rankings unavailable` event
records the selected world, category, vocation and read status.
`MissingWorld` and `WorldRegistryLimit` refer to the Platform
registry. `MissingWatermark`, `StaleWatermark`, `MissingSnapshot`,
`StaleSnapshot`, `InvalidSnapshot` and `Conflict` refer to the state of the
Game projection. `Privacy` refers to the visibility reader, `Changed` to a
revision change during the request, and `Database` to a storage failure.
The home block uses the same source checks and has its own diagnostic event.

## Remaining Game qualification

Game must publish ordered world, category and vocation snapshots, including
explicit empty results, through the existing highscores operation. Confirm
that its category and base-vocation identifiers match the Platform options
and that snapshots refresh within five minutes. Confirm that the same-epoch
highscores watermark stays current within 30 seconds and that profiles and
ownership evidence remain current independently. Confirm tied positions and
order across consecutive publications. Exercise producer restart and the
authorized epoch resync in preproduction, then stop publication and observe
the 503 transition on the public page. Production activation requires Game
and deployment-owner approval of the producer, freshness policy and transport.

[Public projection contract](../contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md)
