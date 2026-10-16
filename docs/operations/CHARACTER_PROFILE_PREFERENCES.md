# Character Profile Preferences Operations

## Scope

This runbook covers the Platform-owned guild-visibility and main-character preference of the native public character page, keyed by the native CharacterId. It does not cover character rename, deletion, restore, world transfer, achievements or production deployment.

## Dependencies

The surface requires:

- the Platform database with the `character_profile_preferences` migrations, including the native-keying migration;
- an authenticated, non-terminated, non-disabled Platform Identity with a canonical `account_id`;
- a live account-character projection (`native_account_character_*` tables with a fresh watermark) that lists the character;
- normal Platform session, CSRF, throttle and audit infrastructure.

No Game write credential is required or permitted.

## Normal behavior

1. The owner opens Account Center and selects **Manage public profile** for a character.
2. The Platform re-resolves ownership from the account-character projection.
3. The owner stores the guild toggle and the optional main-character selection.
4. The Platform commits the preference and bounded audit events atomically.
5. The public character page shows the guild only when Game's `guild_visible` and the owner preference both allow it.

A character with no saved row keeps the default (guild visible if Game allows it). A character the projection cannot bind to a live Identity hides the guild.

## Failure diagnosis

### Owner receives not found

Check, without exposing identifiers to the user:

- Identity is active and authenticated;
- the Identity `account_id` has a row in `native_account_character_snapshots`;
- `native_account_character_rows` lists the named character for that account.

A stale preference row does not grant access and must not be used to bypass current ownership.

### Owner receives unavailable

Check the account-character projection feed: `native_account_character_projection_state` must show a watermark at the highest epoch with `complete_through` inside the freshness window, and the snapshot must not be marked invalid. Also check Platform database health and the application logs by request correlation.

### Guild hidden unexpectedly

Evaluate both layers:

- Game's `guild_visible` in the `character_profile` snapshot (a hidden guild is Game's decision);
- the owner row `(identity_id, character_id)` with `show_guild = 0`;
- whether the character binds to a live Identity through the account-character projection (an unbound character hides the guild by design).

### Multiple main characters suspected

The supported service locks the Platform Identity row and clears another main before selecting a new one. Verify:

```sql
SELECT identity_id, COUNT(*) AS main_count
FROM character_profile_preferences
WHERE is_main_character = 1
GROUP BY identity_id
HAVING COUNT(*) > 1;
```

The query should return no rows. If legacy/manual data violates the invariant, disable direct database writes, preserve evidence and reconcile through a reviewed repair task. Do not silently choose a row in production.

### Guild appears unexpectedly

The guild renders only when Game's `guild_visible` is true and the owner preference allows it. Never infer visibility from a browser-supplied Identity, account or character ID.

## Audit review

Expected event types:

- `character.profile_preferences_updated` for each successful preference update;
- `character.main_character_selected` when a character transitions from non-main to main.

Audit rows intentionally do not contain visibility values or character identifiers. Use the owner Identity, timestamp and correlated request logs for investigation.

## Rollback

The native-keying migration makes the legacy player-id column nullable and adds the `(identity_id, character_id)` unique key. Its down path refuses while any row has no legacy player id; export the Platform preference rows before a destructive rollback in a controlled non-production environment. Rolling back this slice performs no Game change.

## Validation

Required evidence before merge:

- Pint, PHPStan and full feature suite;
- focused owner/non-owner, unavailable-projection, guild upper-bound and fail-closed binding tests;
- real-MariaDB two-process main-character race leaving exactly one main row;
- strict portal and product-ledger validation;
- production-like migration, backup/restore and rollback validation.

All evidence is repository or isolated staging-like proof. It is not `PRODUCTION_PROVEN`.
