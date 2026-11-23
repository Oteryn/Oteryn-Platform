# GAP-10 account overview

Players open Account overview from the account menu. Account Settings also links
to Account display settings, which accepts an optional display name and Polish
or English as the preferred language for account communications. The display
name is visible in the private overview and to authorized staff. It is plain
text, trimmed on save, capped at 80 Unicode scalar values, and rejects control
characters. An empty name clears it. A display name grants no account, character
or public profile identity. Only a verified active account with a live session
and no pending closure can save these settings.

## Operator status

| Fact | Readiness |
| --- | --- |
| Website login | The latest successful login from the account security history is shown in UTC. No retained login event displays Unavailable. |
| Roster | Enable `OTERYN_ACCOUNT_OVERVIEW_ENABLED` and `OTERYN_LCFA_ENABLED` and configure `OTERYN_LCFA_SOURCE_AUTHORITY`. A fresh cut displays its verified-through timestamp. Missing snapshots, stale watermarks and invalid snapshots have distinct states. A verified empty roster has its own message. |
| Level and vocation | Enable the default-off `PORTAL_PUBLIC_PROFILES_ENABLED` read flag and configure `PORTAL_PUBLIC_PROJECTIONS_AUTHORITY`. The current G-4 profile and watermark must be fresh. The profile must match the owner's current LCFA character ID, world and name. Each available profile shows its own UTC observation timestamp. |
| Game login | The existing default-off G-11 read controls `OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED` and `OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED` require `PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY`. The overview retrieves the newest visible event in the 30-day history through the owner-only login history route. Empty or stale history displays Unavailable. |
| Character limits | Set `OTERYN_ACTIVE_CHARACTER_LIMIT` and `OTERYN_TOTAL_CHARACTER_LIMIT` to positive integers with total at least active. These are informational Portal policy values. Account-specific Game entitlements require authoritative evidence. |
| Balance | The existing `OTERYN_ACCOUNT_BALANCE_ENABLED` flag controls the wallet balance read. |

All reads use existing Platform projections and identity state. The display
editor stores the existing account fields. The browser reads the summary at
`/{lang}/account/game-logins?view=latest`. These routes reuse the existing database schema and Game wire contract.

## Game-side qualification

Qualify the live LCFA and G-4 producers together for account and character
identity, world changes, freshness and ownership transfers. Qualify G-11
baseline and watermark delivery, including transfers and restore, before relying
on production login recency. Account-specific active and total entitlements
still require a Game contract. Keep the existing ingest and read controls off
until the corresponding source is qualified.
