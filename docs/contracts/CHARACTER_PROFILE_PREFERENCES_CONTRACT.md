# Character Profile Preferences Contract

## Status

Implemented for the Platform-owned Issue #307 slice of parent Issue #277 and re-keyed to the native CharacterId by the public game data packet. This contract does not authorize production deployment.

## Ownership

Oteryn Platform is the primary owner of `character_profile_preferences` and its migration, validation, audit and presentation lifecycle.

Game is the primary owner of the character: identity, name, level, vocation, guild, house, deaths and online status. The Platform never writes those facts. The public character page is built from the Game Public Projections `character_profile` family; the Platform contributes only what the owner chose.

## Ownership proof

An authenticated browser request never proves ownership by supplying an Identity, account or character identifier.

For each edit or update the server must:

1. resolve the authenticated Platform Identity (not terminated, not disabled) and its canonical `account_id`;
2. read the account-character projection for that account with a live watermark (`NativeAccountCharactersReadModel`);
3. match the requested character name, case-insensitively, against the projected characters of that account;
4. fail closed when any step is absent, stale or unreadable: an account without a projection or a character not in it is not found, a stale or unavailable projection is a sanitized unavailable state.

The Platform stores the native CharacterId only after this check. Every later update repeats the proof; a stored row is never ownership proof.

## Preference model

One row may exist per Platform Identity and native CharacterId (`character_id`, unique together with `identity_id`). Absence of a row means the default: the guild row is visible when Game allows it.

A row stores:

- `show_guild`, the only visibility the native page can honour;
- an optional main-character flag used by the account overview.

The legacy player-id column is optional history. Rows written before the native keying carry no `character_id`, are never read by the native page and stay inert; no backfill exists because no mapping from a legacy player id to a native CharacterId is published. The owner saves the native form once to bind a character. The public comment and the other `show_*` toggles are no longer collected or rendered, because the native page has nothing to attach them to.

## Effective privacy

Game's `guild_visible` flag in `character_profile` is the primary decision. The owner preference is an upper bound over it: the guild name and rank are rendered only when both Game and the owner allow them.

Resolution of the owner preference for a public page read:

1. find the account bound to the native CharacterId through the live account-character read model: the consumer is enabled, the watermark is at the highest epoch and inside the freshness window, and the account's snapshot is valid and at that epoch and lists the character; a leftover row of an account whose snapshot is invalid or at a fenced epoch is not ownership;
2. require exactly one such account and exactly one live Identity of that account;
3. read the preference by `(identity_id, character_id)`; no row means visible.

A character that cannot be bound (no projected account, a disabled, stale or epoch-fenced feed, an invalid snapshot, more than one live owner, no live Identity, unreadable projection) fails closed: the guild is hidden. The preference can narrow Game's flag but never widen it.

The account overview shows, for each character the live projection lists, a link to the preference form and the saved main-character marker; a main-character row for a character the projection no longer lists is not shown.

## Main-character concurrency

Main character is optional. At most one preference row per Identity may have `is_main_character = true` through the supported service.

The update service:

- starts a Platform database transaction;
- locks the owning Identity row and the account-character view before changing main-character state;
- clears every other main row (including unbound rows) before selecting the requested character;
- uses the `(identity_id, character_id)` unique key for idempotent row identity;
- retries bounded transaction failures through the database transaction helper.

## Authorization and audit

The management routes require an authenticated Identity and current server-side ownership proof. Foreign, missing or unbound characters are not editable.

Successful updates record `character.profile_preferences_updated`. A transition from non-main to main additionally records `character.main_character_selected`. Events contain the Identity reference and bounded event type only.

## Failure semantics

- invalid input returns localized validation errors and does not write;
- missing or foreign ownership returns not found without exposing account ids;
- a stale or unreadable projection returns a generic unavailable response or form error without SQL details;
- Platform writes are atomic; Game is never mutated;
- deleting a Platform Identity cascades its preference rows;
- a stale preference row for a character the account no longer holds is inert because every edit repeats the ownership proof.

## Explicit exclusions

This contract does not implement or authorize:

- character rename, deletion scheduling, restore or finalization;
- world transfer or Bazaar ownership transfer changes;
- a Platform-owned public comment or per-section house, skills, deaths or kills toggles;
- production activation or a `PRODUCTION_PROVEN` claim.
