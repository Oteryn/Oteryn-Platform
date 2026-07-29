# Character Profile Preferences Contract

## Status

Implemented for the Platform-owned Issue #307 slice of parent Issue #277. This contract does not authorize Canary mutation or production deployment.

## Ownership

Oteryn Platform is the primary owner of `character_profile_preferences` and its migration, validation, audit and presentation lifecycle.

Canary remains the primary owner of:

- character identity, name and current `players.account_id` ownership;
- level, vocation, skills, guild, house, deaths, kills and runtime status;
- the legacy Canary character comment.

The Platform reads the approved Canary fields through the existing read-only principal. It does not update `players.comment`, `players.account_id` or any other Canary field in this contract.

## Ownership proof

An authenticated browser request never proves ownership by supplying an Identity, account or player identifier.

For each edit or update the server must:

1. resolve the authenticated Platform Identity;
2. load its ready immutable `IdentityCanaryAccount` binding;
3. resolve the active character by name through the read-only Canary repository;
4. compare the server-read `players.account_id` with the bound Canary account ID;
5. fail closed with a public-safe not-found or unavailable state when any proof step is absent or unreadable.

The Platform stores the stable Canary player ID only after this check. Every later update repeats current ownership verification; stored preference rows are not ownership proof.

## Preference model

One row may exist per Platform Identity and Canary player ID. Absence of a row preserves the delivered public-profile behavior.

A row may store:

- a nullable Platform-owned public comment of at most 500 characters;
- field visibility for account association, status, guild, house, skills, deaths and player-kill statistics;
- an optional main-character flag.

The Platform comment is plain text and is always escaped by the public template. Once a preference row exists, a null comment deliberately presents the public no-comment state rather than falling back to the Canary comment.

## Effective privacy

Account-level `public_account_association` and `public_status_visible` remain upper bounds. A character preference can narrow those disclosures but cannot make them visible while the account-level flag is disabled.

Guild, house, skills, deaths and kill statistics are independently suppressible per character. Suppressed sections render localized private states and avoid unnecessary Canary reads where practical.

Related-character lists require the viewed character's effective account-association visibility and exclude sibling characters whose own preference disables association. Private account IDs and internal binding identifiers are never emitted.

## Main-character concurrency

Main character is optional. At most one preference row per Identity may have `is_main_character = true` through the supported service.

The update service:

- starts a Platform database transaction;
- locks the owning Identity row before reading or changing main-character state;
- clears another current main row before selecting the requested character;
- uses the `(identity_id, canary_player_id)` uniqueness constraint for idempotent row identity;
- retries bounded transaction failures through the database transaction helper.

The Community Data acceptance workflow runs two separate PHP processes against real MariaDB, selecting two characters for the same Identity concurrently, and fails unless exactly one main row remains.

## Authorization and audit

The management routes require an authenticated Identity and current server-side ownership proof. Foreign, deleted, missing or unbound characters are not editable.

Successful updates record `character.profile_preferences_updated`. A transition from non-main to main additionally records `character.main_character_selected`. Events contain the Identity reference and bounded event type only; public comments and character data are not copied into security-event metadata.

## Failure semantics

- invalid input returns localized validation errors and does not write;
- missing or foreign ownership returns not found without exposing account IDs;
- Canary read failure returns a generic unavailable response or form error without SQL details;
- Platform writes are atomic; Canary is never mutated;
- deleting a Platform Identity cascades its preference rows;
- a stale preference row for a character no longer owned is inert because every edit repeats current ownership proof and public projection joins through the current ready binding.

## Explicit exclusions

This contract does not implement or authorize:

- character rename;
- deletion scheduling, restore or finalization;
- world/channel transfer;
- Bazaar ownership transfer changes;
- selected achievements without an authoritative source;
- guild, house, skills, deaths or comment mutation in Canary;
- production activation or a `PRODUCTION_PROVEN` claim.
