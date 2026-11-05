# World statistics

Routes: `/stats/world`, `/pl/stats/world`, `/en/stats/world`.
Required query: `world` (non-nil UUID), `start` and `end` (UTC dates in
YYYY-MM-DD form, inclusive, at most 31 days). Unknown parameters are rejected.
`PORTAL_WORLD_STATS_ENABLED=true` enables routing; it defaults to disabled (404).
The configured privacy adapter denies all representations with localized 503.
An application integrating a current privacy provider can use `http::router`
and implement `http::Policy`; no game request is made during rendering.

The census counts visible daily observations by level, not all registered
characters. Death counts include only characters observed in that world on the
same day with current public death permission. No observation is carried forward.
Missing observations are unavailable, never manufactured zeroes. Reads are
bounded to 100,000 observations and fail unavailable above that limit.

## Source boundaries and remaining work

- T65 samples retain raw online totals without privacy subjects. T25 grants no
  independent public aggregate count. Historical chart data therefore remains
  unavailable rather than publishing counts that include hidden characters.
- Owner-only online sessions await their separate G-8 consumer. World kill
  snapshots use the V2 repository API below; their display remains unavailable.
- T65 daily snapshots do not retain vocation or sex. The pinned Game profile
  includes vocation, but not sex; joining current vocation to historical levels
  would fabricate historical census evidence. These breakdowns remain unavailable.
- A daily snapshot does not establish the world of a death during an intraday
  transfer. Counts are explicitly observations of characters, not authoritative
  totals of deaths occurring in a world.
- The live privacy integration is pending T45; the default adapter fails closed.

References: plan sections 5.7 and 6; public community data contract;
public game data projection contract; T25 online implementation.
Game source inspected read-only at 857f15f376907c592bb28d854fccff18a15cdf87:
`crates/gameplay/native-admission-source/src/public_projections.rs`
(`ProfileEntry` and `OnlineEntry`). No Game fixture was copied.
The Laravel public statistics routes expose deaths and guilds; no world census
or historical online chart route exists to preserve byte-for-byte.

## V2 world/day kills consumer

`kills::ingest` accepts only the exact G-8 `PublishWorldDailyKillsV2` wire,
including the contract's golden world/day fixture. It is a repository API for a
future purpose-authenticated internal listener; it does not install a public
listener or trust browser headers. The caller must supply the configured source
and verified transport context and enforce rate limits before calling it.
Both `public_projections_enabled` and `ingest_enabled` default false.
The existing shared public-feed epoch must be established first; this consumer
cannot raise it independently. Unknown operations, versions, members, noncanonical
encoding and impossible dates/counts are refused. Revisions replace totals;
equal-content retries retain the first observation, and equal-pair conflicts
preserve the first content. Tombstones remove payloads without losing fences.

`kills::purge` is scheduled hourly by the worker, including when features are disabled.
World/day payloads expire 400 days after day end, including on new ingestion;
ordering and digest fences survive erasure to prevent resurrection. There are
no character intervals stored here, so the contract's 30-day interval retention
belongs to the separate owner-only interval consumer.

The separate `presentation_enabled` flag defaults false. `kills::presentation`
returns unavailable even if it is set: Q3 still lacks authoritative boss/summon
classification and physical death dedup evidence. The localized page explicitly
renders monster/boss totals unavailable without reading these payloads.

T69B cannot yet provide the requested public online-player chart or authoritative
world player-death totals. G-8 section 1 explicitly says current population and
player deaths need their own sources; sections 3 and 4 define only owner-only
`character_online_sessions` and `world_daily_kills`. There are no public online
sample or player-death aggregate fixtures or wire definitions in that contract.
Raw owner intervals are never grouped for public display, and the existing
observed deaths retain their explicit partial-observation label. Required next
steps are an accepted additive world-total wire/fixtures and authenticated V2
listener/worker integration before any activation. No Game call occurs in render.
