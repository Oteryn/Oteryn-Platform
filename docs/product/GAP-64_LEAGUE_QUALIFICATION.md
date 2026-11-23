# GAP-64 league and prediction qualification

The Play menu links to the league in both locales when `OTERYN_LEAGUE_ENABLED=true`.
The flag defaults to off. The existing league worker and Game ingress supply verified
event and guild snapshots. A missing or stale feed returns the localized unavailable
page. A fresh feed with no published events or standings has its own empty state.

An active, email-verified account may save one immutable choice per world event.
The option list and deadline come from the current Game publication. The Portal stops
accepting predictions one second before the published lock time to account for clock
uncertainty. It closes the form immediately on a verified LOCKED state. A choice
remains private to its account and available while the verified event is displayed.
The Portal displays one point for a correct CLOSED outcome and zero for an incorrect
one. Provisional decisions remain unscored, and void events carry no score. Scores
are derived from the saved choice and the verified final result on every read.
There are no stakes, awards, Wallet changes or Game commands. Choices expire 90 days
after the published event end and are removed in bounded cleanup batches. Guild
rankings come from the independently verified Game guild-season family.

## Operator status

Check the HTTP status of `/en/league` or `/pl/league` for end-to-end readiness.
Use the league worker's `league.ingest` job status to confirm it is processing the
feed. The following read-only query shows the consumer state in the Portal database.
An absent row means the worker has not installed the feed state.

```sql
SELECT epoch, recovery_generation,
       CASE
         WHEN conflicted THEN 'conflicted'
         WHEN event_complete = 0 OR season_complete = 0 THEN 'awaiting publication'
         WHEN floor(extract(epoch FROM clock_timestamp()))
                 - least(event_complete, season_complete) + 1 > 30 THEN 'stale'
         WHEN NOT ready THEN 'awaiting reconciliation'
         ELSE 'ready'
       END AS feed_status,
       to_timestamp(NULLIF(event_complete, 0)) AS events_complete_through,
       to_timestamp(NULLIF(season_complete, 0)) AS seasons_complete_through
FROM league_feed_state
WHERE authority = 'oteryn:game:primary';
```

A ready database row is one input to the HTTP check. Serving also verifies current
deployment trust, the independent recovery fence, and the admitted publication
records. An existing scheduled or locked cursor needs a fresh Game observation
after rollout before it qualifies for the prediction view. Already admitted terminal
outcomes and standings keep their existing read path.

## Game-side qualification

Confirm the production publisher and dedicated mTLS identity using the existing
[league feed runbook](../../portal-rs/crates/minigames/src/league/README.md).
Capture an actual predictable occurrence from scheduled publication through lock,
decision and final freeze, plus a guild-season standings refresh. Verify that the
producer keeps occurrence identity, options and lock time fixed, preserves correction
lineage, and advances both watermarks after Platform acknowledgment. Verify that
production observations and registrations match the intended world and that the
round-trip operates inside the freshness budget. Keep the rollout flag off until
this qualification is accepted. The local Playwright spec uses the disposable
producer identity and does not constitute production evidence.
