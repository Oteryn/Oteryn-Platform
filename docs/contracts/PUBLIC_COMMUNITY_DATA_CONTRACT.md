# Public Community Data Contract

## Ownership

- Canary owns character, skill, guild, house, death and runtime records.
- Oteryn Platform reads only through the dedicated `canary` connection.
- Platform Identity owns public account-association and public status visibility flags.
- Browser input never establishes character ownership, account association, status visibility or guild authority.

## Allowed public reads

### Highscores

Supported categories are fixed in `CommunityDataPolicy`:

- level;
- experience;
- magic level;
- fist, club, sword, axe and distance fighting;
- shielding;
- fishing.

The only supported filters are category and vocation. Scope is explicitly global across configured channels because `players` has no authoritative channel/world ownership column. Pagination is bounded and deterministic.

### Character profile

A public profile may expose:

- name, level and vocation;
- magic level and approved skill values;
- public Canary comment and boss points;
- guild name and rank;
- house name and size only;
- bounded recent deaths;
- bounded player-kill count and recent victim summaries;
- related active characters only when Platform public account association is enabled;
- online/last login/last logout only when Platform public status visibility is enabled.

It must never expose Canary account ID, Platform Identity ID/email, binding state internals, IP data, moderator records, raw participant payloads, house coordinates or runtime lease internals.

### Guilds

Directory search matches bounded public guild names and returns active-member totals. Detail exposes guild name, level, points, leader character name, public message and active member/rank/nickname rows. Raw owner IDs, balances and residence IDs are excluded.

### Deaths and kills

Latest deaths include active victim name, level, cause and timestamp. Character kill summaries count only rows where the named character is the recorded player killer. Raw participant text and unjustified/private enforcement interpretation are excluded.

## Explicit exclusions

- guild administration: no approved Canary write contract;
- world/channel transfer history: no authoritative transfer service or source;
- selectable achievements: no authoritative selection source in the current schema;
- polls: not adopted for the current launch contract;
- public punishment publication: excluded by privacy/moderation policy; enforcement stays account-visible.

## Failure and performance contract

- read failures return localized `503` responses without query text, credentials or schema internals;
- empty and not-found states remain distinct from dependency failure;
- every collection has a deterministic limit, pagination size and order;
- the dedicated principal has direct `SELECT` only on `players`, `guilds`, `guild_membership`, `guild_ranks`, `houses`, `player_deaths`, `channels` and `cluster_sessions`;
- schema-wide grants, roles that cannot be inspected and every write privilege fail verification;
- production indexes and query plans remain deployment evidence to verify before go-live.

## Evidence boundary

Feature tests use isolated read-only SQLite contracts. Dedicated acceptance uses MariaDB 11.8, Redis, the exact PR SHA, direct-table grant verification and zero-retry Chromium desktop/tablet/mobile runs. This evidence is repository/staging-like proof only.
