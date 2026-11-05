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
- Online sessions and monster/boss counters await G-8. No wire shape is assumed.
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
