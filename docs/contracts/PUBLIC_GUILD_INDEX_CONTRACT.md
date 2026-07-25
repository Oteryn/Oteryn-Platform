# Public Guild Index Contract

## Status

`PROVEN READ CONTRACT — IMPLEMENTED BY PR #160`

This contract defines the smallest additional read-only PublicGameData capability selected by `OTERYN-20260724-public-game-statistics`.

## Evidence pin

Canary repository: `blakinio/canary`  
Canary commit: `93413bd53e9a40f0ff3c4f55986036b10be44e0f`

Verified source evidence:

- `schema.sql` defines `guilds`, `guild_membership` and `players`;
- `guilds.id` is the primary key and `guilds.name` is unique;
- `guild_membership.player_id` is the membership primary key and references `players.id`;
- `guild_membership.guild_id` references `guilds.id`;
- `players.deletion = 0` is the established active/listable character state in the broad Canary data contract;
- `src/io/ioguild.cpp` loads persisted guild identity from `guilds` and provides exact-name lookup without a separate visibility or deletion flag.

No Canary repository or schema change is required or authorized.

## Capability

Public route:

```text
GET /guilds
```

The route returns a deterministic, paginated directory of persisted guilds.

### Public field allowlist

Exactly:

- guild name;
- derived active-member count.

The active-member count is the number of `guild_membership` rows for the guild whose joined `players` row has `deletion = 0`.

A guild with no active/listable members remains a valid persisted guild and is shown with a count of zero.

## Explicit exclusions

The index does not expose:

- guild or player database identifiers;
- owner identifiers;
- account identifiers;
- guild bank balance;
- MOTD;
- membership nicknames or ranks;
- invitations or war data;
- deleted-character names;
- ban, namelock or disciplinary data;
- raw membership records;
- session or runtime data.

The product meanings of `guilds.level`, `guilds.points`, `guilds.residence` and `guilds.creationdata` are not required for this capability and remain `UNKNOWN` here. They are not displayed or interpreted.

## Ordering and pagination

- page size: 50;
- primary order: `guilds.name` ascending;
- deterministic tie-breaker: `guilds.id` ascending;
- one bounded page query plus the paginator count query;
- active-member counts are calculated inside the page query, not through per-guild follow-up queries.

Although the current schema makes guild names unique, the ID tie-breaker preserves deterministic behavior if collation or future compatible schema behavior produces equivalent sort values.

## Empty and unavailable behavior

- A successful query with zero guild rows returns HTTP 200 and the explicit empty state `No guilds found.`
- A Canary database query failure returns HTTP 503.
- Dependency failure must never be rendered as an empty guild directory.
- No cache is introduced, so the capability does not extend game-data freshness beyond the database read.

## Read-only enforcement

The query uses the existing dedicated `canary` connection and `oteryn_readonly` credential boundary.

No new table grant is required. The existing database-enforced direct-SELECT allowlist already contains:

- `guilds`;
- `guild_membership`;
- `players`.

The application adds no write method, mutation route or Canary migration.

## Privacy and moderation decision

Guild names are already part of the existing public guild-detail surface and are classified as public game data. The index intentionally avoids free-form MOTD and membership nicknames, minimizing moderation and content-abuse exposure. Deleted characters are excluded from the aggregate and no private account or disciplinary information is queried for output.

## Rejected scope

Latest deaths and kill statistics are not bundled into this capability. Their source-table semantics, killer/assist attribution, privacy treatment and aggregation windows require separate evidence-backed discovery before public exposure.
