# GAP-36 statistics qualification

The Polish and English Statistics availability page under World navigation shows the current presentation state and the evidence needed for each report. It links to enabled report pages and to Watch API access when `PORTAL_STATS_WATCHES_ENABLED=true`. Operators can inspect the same status page without an account or access to private projection data.

| Surface | Platform behavior | Remaining evidence |
| --- | --- | --- |
| Guild history and comparison | Select two distinct public guild names and a 1–31 day window. The comparison uses currently visible members and exact endpoint observations for the same cohort. | Qualify the G-4 roster and progression feeds, privacy cuts, and retained history. Complete membership history, wars, dissolved guilds and authoritative guild rankings need separate sources. |
| Growth and character comparison | Validate the measure and bounds. Keep unknown or unavailable series out of results. | Qualify G-4 progression across the requested days, publication continuity, privacy and historical coverage. |
| Online time | Show Unavailable. The owner interval consumer requires verified retained-set coverage and ownership history. | Accept and qualify G-8 interval semantics, Game producer, ownership transition boundary, and resync. Public rankings require a separate exposure decision. |
| Census | The world/date picker uses the Platform world registry. The report requires privacy-qualified observations. | Qualify a current population source and public census dimensions. Census needs a separate source contract. |
| Daily monster and boss totals | Show Unavailable pending qualification of the existing read model. | Accept physical-death counting and classification, UTC cutoffs and corrections. Qualify producer revisions, coverage, late corrections and replay. |
| Watches | Signed-in verified players can issue, replace or revoke session-bound watch API access when the default-off watch flag is enabled. A live bearer can remove that account’s existing subscriptions. Existing fanout delivers to opted-in subscriptions and commits its deduplication receipt with the notification. | Qualify canonical targets and accepted transitions with event-time and current privacy evidence. Connect the qualified event adapter to fanout only after baseline/replay suppression and UI opt-in are qualified together. |

Watch API access uses one opaque credential per browser session, valid for up to one hour. Its value is shown once. Rotation or expiry of the originating session ends access. A disabled watch flag leaves the API unmounted. A verified bearer gets `503` for listing and creating watches while source qualification is unavailable, and `204` for idempotent removal. The bearer is scoped to this endpoint.

G-4 revision 4 accepts the wire and privacy semantics. Activation still requires exact producer/consumer revisions, fixtures and deployment qualification. G-8 revision 2 remains a proposal. Game owns authoritative production, epoch/replay/resync evidence and producer qualification. Platform activation also requires the matching freshness, privacy and event-provenance adapters. Existing comparison and watch dispatch tests exercise the consumer with isolated fixtures.

- [G-4 public projection families](../contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md)
- [G-8 statistics projection families](../contracts/OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md)
