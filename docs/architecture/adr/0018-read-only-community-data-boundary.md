# ADR 0018: Read-only community data boundary

- Status: Accepted
- Date: 2026-07-29

## Context

Oteryn needs richer public rankings, character profiles, latest deaths, kill statistics and guild discovery. The authoritative Canary schema stores global characters, skills, comments, houses, guild membership, deaths and current runtime leases, but it does not store per-channel character ownership, selectable achievements or an approved world-transfer history. Canary also owns guild membership and rank mutations.

Platform already owns private-by-default account-association and status flags. Public presentation must apply those flags without exposing Identity email, binding IDs, Canary account IDs, private status or moderator-only enforcement data.

## Decision

Oteryn Platform extends the dedicated read-only Canary connection with direct-table `SELECT` grants for `houses` and `player_deaths`. All highscore columns come from a fixed server-side allowlist. Rankings support category and vocation filters and explicitly use a global scope because the authoritative character schema has no per-channel ownership dimension.

Public character profiles expose only approved server-backed values: name, level, vocation, magic level, selected skills, public comment, boss points, guild/rank, public house summary, bounded recent deaths and bounded player-kill statistics. Platform resolves a ready Identity-to-Canary binding server-side and discloses related characters or status timestamps only when the corresponding Identity privacy flag is enabled.

Guild directory/search and detail remain read-only. Platform does not offer guild membership, invitation, rank or message mutations until a separate least-privilege Canary operation contract is approved. Transfer history is not applicable until an authoritative transfer service exists. Polls are not adopted for the current launch contract. Enforcement remains account-visible only and is not published publicly.

Dependency failures return localized `503` states without SQL details or fabricated fallback data. Queries use deterministic limits, ordering and documented index expectations.

## Consequences

- Issue #280 can close without a Canary write path or invented channel/achievement/transfer data;
- existing account privacy controls govern public association and status disclosure;
- guild administration, world transfer, selectable achievements, polls and public enforcement require separate product, ownership and security decisions;
- the Canary read-only principal gains two explicit table grants but no schema-wide or write privilege;
- repository and isolated staging-like evidence do not establish production deployment or `PRODUCTION_PROVEN`.
