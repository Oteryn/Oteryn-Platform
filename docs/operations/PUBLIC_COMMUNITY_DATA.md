# Public Community Data Operations

## Runtime dependencies

Public community pages use:

- the Platform database for Identity privacy flags and ready Identity-to-Canary bindings;
- the dedicated read-only Canary MariaDB principal for character, guild, house and death data;
- the dedicated Canary runtime Redis principal only for current online/server state.

No page in this lifecycle writes to Canary.

## Required Canary grants

Render and execute `database/provisioning/canary-readonly.sql.template` only through a reviewed deployment procedure. The resulting principal must have direct `SELECT` on exactly:

- `players`;
- `guilds`;
- `guild_membership`;
- `guild_ranks`;
- `houses`;
- `player_deaths`;
- `channels`;
- `cluster_sessions`.

After configuration, run:

```bash
php artisan canary:verify-db-privileges
```

Do not copy raw `SHOW GRANTS` output into tickets or logs because some database variants include authentication metadata.

## Smoke checks

Use a known non-sensitive public fixture or production-safe character and verify:

1. `/highscores?category=level&scope=global` renders a bounded ranking;
2. a valid `/characters/{name}` page renders approved public fields and respects privacy flags;
3. `/deaths` distinguishes records from an empty state;
4. `/guilds?q=...` returns bounded results and a no-match state;
5. Polish localized routes render Polish copy;
6. revoking one required table grant in an isolated environment returns a sanitized `503`, and restoring it recovers the page.

Never perform grant revocation or mutation probes against production without an approved maintenance plan.

## Monitoring and incident response

Track HTTP `503` rates for community routes and correlate them with structured request IDs. A dependency failure must not produce SQL text or credentials in the response. During an incident:

- confirm Platform database health before investigating privacy-binding reads;
- verify the dedicated Canary credential and direct table grants without widening them;
- verify MariaDB reachability and query latency;
- verify Redis only for online/server state; deaths, profiles, guilds and highscores do not depend on Redis;
- restore the missing dependency or grant rather than replacing authoritative values with cached guesses.

## Privacy review

When changing public profile fields, inspect both the Canary source and Platform privacy policy. Do not expose:

- Identity email or internal IDs;
- Canary account IDs;
- binding/provisioning details;
- IP addresses or runtime lease values;
- moderator-only notes or account enforcement records;
- raw death participant payloads;
- house coordinates, owner IDs or financial fields.

## Rollback

Application rollback is code-only because this slice adds no database migration and performs no Canary mutation. If the prior application revision does not read `houses` or `player_deaths`, its operation remains compatible with the expanded read-only grants. Removing grants requires a separate security/deployment review and must not happen while any deployed revision still requires those tables.

## Evidence classification

Exact-SHA CI and isolated MariaDB/Redis/browser acceptance establish repository or staging-like confidence only. Production deployment identity, grants, latency, observability and recovery remain unverified until the production gate records them.
