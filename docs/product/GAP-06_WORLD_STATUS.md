# GAP-06 public world status

The Platform world registry supplies the canonical UUIDv7 and name. From
**World status**, open a world's **Uptime and incidents** page at
`/{lang}/worlds/{world}/uptime`. An unknown registry UUID returns 404. The
page links to the privacy-filtered online roster when that module is enabled.

## Platform operation

`PORTAL_WORLDS_ENABLED=true` enables the world pages and defaults to off.
`PORTAL_ONLINE_ENABLED=true` separately enables the roster and its link.
Both flags can be turned off independently. Incident notices remain stored
when world pages are disabled.

On the configured staff host with the admin panel enabled, follow
**Manage world incidents** from a world's uptime page. Staff need the
existing panel access and `portal.announcements.manage` permission with MFA.
Enter a short public summary in both languages. One notice per world can be
open at a time. Resolve the open notice when appropriate. Resolution keeps the
original summary and both UTC timestamps visible in the paginated history.
Publish and resolve each write an audit record in the same transaction.

Incident notices describe what the Platform team has communicated. Unavailable
Game measurements remain unavailable regardless of incident state. Public roster
counts refer to the displayed characters whose presence and visibility are
current. Aggregate world population has its own source requirements.

## Game-side qualification

The existing native report describes channel admission readiness. For public
world measurements, Game and Platform still need an accepted producer and
consumer contract covering:

- The current channel membership and scope authority for each registry world.
  Define how fresh channel observations become world health, including degraded
  service, maintenance and confirmed outage.
- A world population aggregate with deduplication, public allowlisting,
  observation time and freshness. An authoritative empty world reports zero.
- Ordered, replayable availability observations and a completeness watermark.
  Define retention and which intervals count toward uptime, including missing
  observations, planned maintenance and failover between owners.
- Producer evidence for steady state, empty worlds, delayed reports, outages,
  partial coverage, owner changes, recovery and privacy suppression. Qualify
  these against the exact Platform candidate before showing live values.

These requirements derive from the
[Runtime Status Projection Contract](../contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md)
and the
[Public Game Data Projection Contract](../contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md).
