# GAP-60 Atlas and place recommendations

The World menu opens **Bestiary and bosses** and **Hunting, bestiary and boss places**.
The bestiary landing page uses the logged-in account to list characters with proven
ownership. The private bestiary page can show progress, boss cooldowns, locations
and ranked recommendations when supplied with a qualified snapshot. Wiki place
cards accept purpose, party size and sort filters. Missing evidence is shown as
unavailable. An empty result from a qualified source has its own message.

## Operator status

Open **Game evidence status** in the Wiki staff panel, under `/admin/wiki`.
It uses the existing Wiki staff permission and session checks. It reports whether
each data flag is off or on and awaiting qualification. Both are off by default.

- `OTERYN_ATLAS_BESTIARY_ENABLED` gates character discovery and the owner-only
  bestiary detail route. A configured deployment still needs the approved
  journal and map adapter to serve a snapshot.
- `PORTAL_WIKI_PLACES_ENABLED` gates loading public place cards. The place
  selector remains reachable and returns HTTP 503 until the public feed is qualified.

## Evidence required for activation

- Game must publish G-11 bestiary and boss-cooldown baselines and watermarks with
  accepted definition revisions, including how threshold and boss-category
  changes advance revisions. See sections 5.1 and 5.2 of the
  [progress contract](../contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md)
  and sections 5.4 and 5.5 of the
  [definition mapping contract](../contracts/OTERYN_V2_GAME_JOURNAL_DEFINITION_MAPPING_CONTRACT.md).
- Game and Platform must qualify the join between a character's current
  receipt-derived ownership cut, visible definitions and spawn facts from a
  single ready world version. The adapter must recheck transfer, restore and
  erasure fences around the read and prove freshness for all supplied data.
  Only the approved fixed-at-reset boss policy can supply a boss location.
- Personal completion estimates require qualified pace evidence for that
  character and level band. Wiki recommendations need an approved public
  place and editorial binding, plus delayed, privacy-qualified rates for any
  metrics or party sizes shown. See the
  [journal sessions contract](../contracts/OTERYN_V2_GAME_JOURNAL_SESSIONS_CONTRACT_PROPOSAL.md).
  Current authenticated solo cohorts do not provide a public feed.
- Once the inputs are accepted, wire the production adapters and qualify the
  signed-in flow with a Game producer. Include transfer, restore, stale data
  and missing-baseline cases in the qualification.

These portal routes are read-only. There is no database migration or new
Game wire contract.
