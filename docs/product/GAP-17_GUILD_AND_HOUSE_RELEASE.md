# GAP-17 guild and house release

Platform qualification covers `/{pl|en}/guilds`, `/{pl|en}/guilds/{name}`,
`/{pl|en}/houses` and `/{pl|en}/houses/{key}`. Open Guilds in Community or
Houses in World and select a registered world. Guild search and pagination
retain the filter. Guild details are scoped by world and exact public name
when a world is supplied. House details use the world and the catalogue key.
An authoritative empty selection shows the localized empty state. A valid
but unknown world, or an unknown or foreign detail under a current complete
cut, returns 404. Malformed selections return 400.

## Operator check

On the configured admin host, open `/admin/en/world-directories` or
`/admin/pl/world-directories` with a staff MFA session and permissions
`admin.access` and `game.world.read`. The page reports the release decision,
effective page flags and the source/epoch-bound completeness of the guild,
character profile and house feeds. A current watermark describes feed
completeness only. If the release decision is pending, the public pages
continue to show the localized unavailable state, even when the flags and
watermarks are present. The page does not include projection payloads.

Guild presentation uses `PORTAL_GUILDS_ENABLED`. House presentation uses both
`PORTAL_PUBLIC_PROJECTIONS_ENABLED` and `PORTAL_HOUSES_SERVING_ENABLED`.
Both presentations are restricted to testing/preproduction. Existing ingest
requires the parent projection switch, the respective
`PORTAL_GUILDS_INGEST_ENABLED` or `PORTAL_HOUSES_INGEST_ENABLED`, the configured
source authority and the verified publisher identity. These switches default off.

A stale or missing cut returns HTTP 503 and sends no directory records.
Guild counts and rosters cover only members whose stable identity, same-world
membership and current privacy permit display. Occupied houses require
separate proof before owner information can be shown. The configured reader
keeps owner data private. This change adds no house bidding or other write
operations.

## Game evidence required for activation

1. Accept the versioned G-4 guild and house contract for the target environment
   and qualify the authenticated producer with live evidence. The legacy
   [guild index](../contracts/PUBLIC_GUILD_INDEX_CONTRACT.md) covered only names
   and counts, so it cannot qualify the new feeds.
2. Demonstrate full guild snapshots, stable IDs, same-world profile membership
   and privacy suppression, including renames, disbanding and loss of eligibility.
3. Demonstrate housing catalogue keys and vacancies, retirements and owner
   suppression. Any future display of owners needs a qualified identity and
   privacy binding through name handoffs. Vacant houses need no owner identity.
4. Exercise initial publication, heartbeats, replay, resync and epoch
   changes with the [G-4 contract](../contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md).
   Verify successful scoped pages, expired watermarks and privacy withdrawal
   using the actual staging producer before approving the release gate. A
   production rollout needs its own freshness, PKI and resync qualification.
