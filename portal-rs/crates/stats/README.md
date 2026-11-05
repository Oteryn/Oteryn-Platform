# Statistics core

Enable worker polling with `PORTAL_STATS_ENABLED=true` (off by default). The
existing worker schedules `stats.poll` every minute. Only current-epoch subject
projections covered by their family completeness watermark are consumed. Epoch,
revision and observation time form a per-subject cursor; effects and cursors
commit together under a transaction advisory lock. World observations are
bucketed into five-minute UTC intervals. Missing observations create gaps,
never synthetic samples. Monthly partitions are created for the observation
month and two following months, and for the current month on every poll.

Daily snapshots and events have no expiry. Samples older than 90 days are
aggregated and deleted atomically, one UTC day per transaction. Daily world
aggregates retain count, sum, minimum and maximum (average is sum / count).
Late observations merge into an existing aggregate without losing prior data.

## Source boundary

The accepted ingest currently supplies profile name, world, level and guild,
character deaths and world online lists. Experience, skills, achievement points,
online session minutes and bazaar settlements are not in that contract. Missing
metrics are SQL NULL, not zero. `Snapshot` accepts future normalized metrics;
`source_event` accepts deaths and bazaar sales with stable producer keys. Sales
are never inferred from profile changes. Wiring these missing sources requires
their accepted producer contracts in later work; this crate does not extend the
ingest wire contract. Empty profiles do not manufacture a daily row.

Snapshot comparisons use the most recent earlier daily row. Repeated transitions
with the same day and values deduplicate; this is daily history, not a lossless
stream of every intraday transition. The upstream latest-only projection table
cannot recover revisions overwritten between polls.

## Read policy

`history` and `top_growth` require `Visibility`. Implementations must deny unknown
characters and resolve current Platform profile preferences. There is no default
allow-all implementation. Hidden characters return no rows; online minutes,
skills, guild events and death events require separate permissions. Internal
snapshot identity fields are not exposed by the daily read type. Filtering occurs
before the growth limit, so a hidden leader cannot suppress a visible result.
Growth requires known experience at both exact endpoints, sorts numerically and
breaks ties by character reference. Daily snapshots are retained indefinitely.

`store` write functions are internal infrastructure APIs for trusted adapters;
call them only behind the feature flag, in a transaction serialized with advisory
lock 65001. They do not authorize public presentation. No pages are included.
No query macros are used; runtime SQL builds with `SQLX_OFFLINE=true`.

## Tests

Set `PORTAL_TEST_DATABASE_URL` to a disposable PostgreSQL 18.6 database and run
`cargo test --locked -p oteryn-stats`. Each database test creates an isolated schema
and removes it on success. When unset, database tests print a skip notice.
