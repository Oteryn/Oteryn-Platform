# GAP-48 player journal

Players open **Private weekly notes** in the Account menu. A note belongs to one
account and one completed Monday-to-Monday UTC week. Saving a blank note removes
it. The JSON attachment contains the signed-in account's own notes and supports
an optional UUIDv7 selector for a single note. The editor and download remain
available during journal source outages. Notes are retained until the owner
deletes them or the account is erased. The existing source-linked character and
session annotations have separate authorization and retention.

## Status in the portal

| Surface | Ready | Unavailable |
| --- | --- | --- |
| Atlas → Hunting places | HTTP 200 with delayed eligible cells or an explicit empty state. | HTTP 503. If the aggregate switch is enabled, the localized page keeps participation controls available. |
| Account → Journal reports | HTTP 200 for a covered completed week. JSON and CSV downloads use the same coverage check. | HTTP 503. The enabled report page links to the independent weekly-note editor. |
| Account → Private weekly notes | HTTP 200 for a signed-in active, verified owner, including a blank editor before the first save. | HTTP 503 on a storage outage. |
| Weekly email | The existing service requires both report flags and recipient consent. | The shipped server leaves scheduling and delivery disabled. |

For an operator smoke check, sign in with a test account and request the first
two surfaces with `Accept: text/html`. A generic 503 is the default when each
feature flag is absent. On a healthy deployment with the two flags enabled,
the localized 503 states indicate unresolved journal data. The weekly-note
route is independent of both switches. Responses are private and uncached.

The PostgreSQL playerdata tests exercise the normal contribution ingest and
publish APIs with a controlled source. A cell needs ten distinct sessions and
five participating accounts, with a full UTC day plus another 24 hours before
publication. A subsequent change can withdraw the day's release. The release
record holds a digest and withdrawal state. The retention worker expires the
record with its contribution window. An operator with read-only Platform
database access can check release state without loading contribution payloads.

```sql
SELECT utc_day, CASE WHEN withdrawn THEN 'withdrawn' ELSE 'released' END AS status
FROM pd_places_publication_days
ORDER BY utc_day DESC;
```

An absent day has no recorded release. The authenticated route is the check for
current serving readiness.

## Game and joint qualification

Keep `OTERYN_PLAYERDATA_PLACES_AGGREGATE` and `OTERYN_JOURNAL_REPORTS_ENABLED`
off by default. The shipped process uses unresolved source adapters. Live
activation requires an accepted durable-logout and ordered journal producer,
qualified replay and restore fencing, complete retained week coverage (including
known-empty weeks), and accepted catalogue and NPC valuation evidence. The
Platform integration must bind the adapters to that evidence and jointly qualify
cohort privacy, transfer and invalidation behavior before switching on reads.
Any deployment of weekly email also needs a qualified source and scheduling.
See the [G-9 contract proposal](../../../docs/contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md).
