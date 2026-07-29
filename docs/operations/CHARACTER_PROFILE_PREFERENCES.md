# Character Profile Preferences Operations

## Scope

This runbook covers the Platform-owned public comment, per-character visibility and optional main-character preference delivered by Issue #307. It does not cover character rename, deletion, restore, world transfer, achievements or production deployment.

## Dependencies

The surface requires:

- the Platform database and completed `character_profile_preferences` migration;
- an authenticated non-terminated Platform Identity;
- a ready immutable `IdentityCanaryAccount` binding;
- the existing read-only Canary connection and approved `players`/community-data grants;
- normal Platform session, CSRF, throttle and audit infrastructure.

No new Canary write credential is required or permitted.

## Normal behavior

1. The owner opens Account Center and selects **Manage public profile** for an active character.
2. The Platform re-resolves the ready binding and character ownership from Canary.
3. The owner stores a bounded plain-text comment, visibility flags and optional main-character selection.
4. The Platform commits the preference and bounded audit events atomically.
5. Public profile reads combine current Canary facts with the Platform preference and account-level privacy upper bounds.

Absence of a preference row preserves the previously delivered profile behavior. A saved row deliberately owns the public comment and explicit field visibility.

## Failure diagnosis

### Owner receives not found

Check, without exposing identifiers to the user:

- Identity is active and authenticated;
- `identity_canary_accounts.status = ready`;
- the binding has a valid `canary_account_id`;
- the named character is active (`deletion = 0`);
- the current server-read `players.account_id` matches the ready binding.

A stale preference row does not grant access and must not be used to bypass current ownership.

### Owner receives unavailable

Check:

- Platform database health;
- read-only Canary connectivity and exact SELECT grants;
- application logs using request correlation, without returning SQL details;
- whether community-data outage/recovery acceptance reproduces the failure.

Do not add a generic Canary UPDATE credential as a workaround.

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

### Public field appears unexpectedly

Evaluate both layers:

- account association/status can be shown only when the Identity account-level flag is enabled and the character preference permits it;
- guild, house, skills, deaths and kill statistics follow their per-character flags;
- related-character lists also exclude siblings whose association flag is disabled;
- missing preference rows use the compatibility defaults documented in the contract.

Never infer visibility from a browser-supplied Identity, account or player ID.

## Audit review

Expected event types:

- `character.profile_preferences_updated` for each successful preference update;
- `character.main_character_selected` when a character transitions from non-main to main.

Audit rows intentionally do not contain comment text, visibility values or Canary identifiers. Use the owner Identity, timestamp and correlated request logs for investigation.

## Rollback

Application rollback must remain compatible with the additive table. The migration down path drops only Platform-owned `character_profile_preferences`.

Before destructive rollback in a controlled non-production environment, export the Platform preference rows if evidence or restoration is required. Rolling back this slice does not require and must not perform Canary changes.

## Validation

Required evidence before merge:

- Pint, PHPStan and full feature suite;
- focused owner/non-owner, validation, escaped rendering and privacy tests;
- real-MariaDB two-process main-character race leaving exactly one main row;
- zero-retry Chromium desktop/tablet/mobile owner and public-profile lifecycle in EN/PL;
- strict portal and product-ledger validation;
- production-like migration, privilege, backup/restore and rollback validation.

All evidence is repository or isolated staging-like proof. It is not `PRODUCTION_PROVEN`.
