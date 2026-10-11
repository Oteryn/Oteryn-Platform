# ADR 0005 — Greenfield character creation product policy

## Status

Accepted — 2026-07-20

## Context

Phase 5 now has an implemented and tested immutable authorization path from an authenticated Oteryn Platform Identity to exactly one legacy engine `accounts.id` for supported greenfield accounts.

The remaining character-creation blocker is product policy rather than ownership. Current legacy engine source proves schema/load compatibility facts, but it does not define Oteryn's web-facing name policy, starter profile or per-account character limit. SQL defaults, sample characters and incidental login hooks are not product intent by themselves.

This ADR deliberately selects a narrow first product policy so the successor operation contract can define one exact transactional write surface without inventing starter inventory, quest or game-side initialization behavior.

## Decision

### 1. Creation inputs

The first supported character-create operation accepts only:

- `name`;
- `vocation`, from the approved base-vocation allowlist;
- `sex`, from the approved two-value allowlist.

The client does not choose:

- Legacy engine `account_id`;
- town;
- position;
- pronoun;
- level or statistics;
- outfit type or colors;
- skills;
- starter items;
- storage, quest or tutorial state.

The target legacy engine account is always derived server-side from the authenticated Platform Identity's ready immutable binding.

### 2. Canonical character-name policy

Character names are intentionally ASCII-only in the first product version to avoid unproven Unicode normalization, locale and visually-confusable behavior.

Canonicalization and validation are:

1. input may contain only ASCII letters `A-Z`, `a-z` and ASCII space;
2. trim leading and trailing ASCII spaces;
3. collapse each internal run of ASCII spaces to one space;
4. split into words;
5. allow one to three words;
6. each word must contain 2 to 15 ASCII letters;
7. canonical total length must be 3 to 29 characters, including spaces;
8. store each word with first letter uppercase and all remaining letters lowercase.

Examples:

- `alice` -> `Alice`;
- `aLiCe   moon` -> `Alice Moon`;
- tabs, punctuation, apostrophes, hyphens, digits, emoji and non-ASCII letters are rejected in the first slice.

The canonical stored name is the only value submitted to legacy engine.

The legacy engine database unique constraint on `players.name` remains the final durable uniqueness guard. Application preflight checks are UX only and never replace the insert-time unique constraint.

### 3. Reserved and impersonation names

Comparison is case-insensitive after canonicalization.

A name is rejected if any canonical word is exactly one of:

- `Admin`;
- `Administrator`;
- `God`;
- `Gamemaster`;
- `GM`;
- `CM`;
- `Support`;
- `Tutor`;
- `Moderator`;
- `Staff`;
- `System`;
- `Server`;
- `Oteryn`;
- `legacy engine`.

The exact phrases `Game Master` and `Community Manager` are also reserved.

In addition, the first word must not begin with the product/system brand prefixes `Oteryn` or `legacy engine` after case-folding. This prevents names such as `OterynSupport` from becoming player-owned identifiers.

The reserved set is Platform-owned product policy. Future expansion may add names without changing legacy engine schema.

### 4. Allowed vocation choices

The first product version permits only current base vocations proven in legacy engine's vocation configuration:

- `1` — Sorcerer;
- `2` — Druid;
- `3` — Paladin;
- `4` — Knight;
- `9` — Monk.

Creation with vocation `0` (`None`) is not offered by the Platform product flow.

Promoted vocations are rejected at creation:

- `5` — Master Sorcerer;
- `6` — Elder Druid;
- `7` — Royal Paladin;
- `8` — Elite Knight;
- `10` — Exalted Monk.

Promotion remains game progression, not a web character-creation choice.

### 5. Sex and pronoun policy

Creation-time `sex` is limited to legacy engine values `0` or `1`.

`pronoun` is not a creation-time input in the first slice and is persisted as `0`.

A later profile/customization task may introduce pronoun selection only after the product semantics and exact legacy engine enum mapping are explicitly contracted. Character creation does not guess that mapping.

### 6. Canonical starter profile v1

Every new character starts with the following product-selected persisted state:

- `group_id = 1`;
- `account_id =` the authenticated Identity's ready bound legacy engine account ID;
- `level = 8`;
- `experience = 4200`;
- selected base `vocation` from the allowlist above;
- `health = 185`;
- `healthmax = 185`;
- `mana = 90`;
- `manamax = 90`;
- `maglevel = 0`;
- `manaspent = 0`;
- `soul = 100`;
- `cap = 470`;
- `town_id = 8`;
- `posx = 0`, `posy = 0`, `posz = 0`, intentionally using legacy engine's proven temple-position fallback at load time;
- `conditions =` an empty non-null blob;
- `sex =` selected value `0` or `1`;
- `pronoun = 0`;
- `looktype = 136` when `sex = 0`;
- `looktype = 128` when `sex = 1`;
- `lookhead = 114`;
- `lookbody = 120`;
- `looklegs = 132`;
- `lookfeet = 115`;
- `lookaddons = 0`;
- `istutorial = 0`.

Starter classic skills are all level `10` with zero tries. The successor operation contract may either write those values explicitly or rely on the exact currently proven schema defaults, but the resulting persisted state must be verified as `10/0`.

Other fields remain at the exact current legacy engine schema defaults only when the successor contract proves those defaults are compatible with this profile. The product policy is the resulting starter state, not an instruction to trust unspecified future defaults.

### 7. No starter dependent writes in v1

The initial product operation creates no starter:

- inventory items;
- inbox/store-inbox items;
- depot items;
- reward items;
- stash entries;
- player storage values;
- quest state;
- learned spells;
- tutorial records;
- guild data;
- session/runtime rows.

No current generic login hook is treated as an implicit mandatory starter initializer.

If later product requirements need mandatory game-side initialization that cannot be represented safely in the same bounded legacy engine database transaction, that requirement needs a separately authorized legacy engine/datapack contract and implementation.

### 8. Per-account character limit

A supported Platform-owned legacy engine account may have at most **10 active characters**.

For quota purposes:

- rows with `players.deletion = 0` count toward the limit;
- rows with `players.deletion != 0` do not count toward the active-character quota;
- a pending-deletion character's name remains globally unavailable while its `players` row still exists because the legacy engine unique-name constraint remains authoritative.

The limit is enforced inside the character-create legacy engine transaction, not by a preflight count alone.

### 9. Concurrent-limit semantics

Every character-create transaction for one account must serialize on the same legacy engine `accounts.id` row before counting active characters and inserting a player.

Required ordering:

1. resolve the ready Platform-owned binding before opening the shared-write transaction;
2. begin the dedicated legacy engine character-create transaction;
3. lock the exact bound `accounts` row using `SELECT ... FOR UPDATE` or an equivalent proven row-lock mechanism;
4. count `players` rows for that account where `deletion = 0`;
5. fail with a deterministic limit result when the count is already 10;
6. perform canonical-name conflict checks/insertion while retaining the database unique constraint as the final race guard;
7. commit only after the complete approved starter state is durable.

Two concurrent creates for the same account must not both observe slot 10 as available. Different accounts do not require a shared global account lock; global name uniqueness remains serialized by the database unique constraint.

## Security consequences

- Character ownership authorization is always derived from the authenticated Identity's ready immutable binding.
- Browser-supplied `account_id` remains forbidden as authorization evidence.
- The existing `canary` read-only connection and `canary_provisioning` account-create connection must not gain character-write privileges.
- Character creation requires a third dedicated least-privilege operation credential/connection after the successor operation contract defines its exact grants.
- Name canonicalization must run server-side before any availability query or insert.
- The reserved-name policy is enforced server-side and must be covered by regression tests.
- Database unique constraints and row locking remain required even when the UI performs preflight validation.

## Rejected alternatives

### Unicode names in the first slice

Rejected. A stable normalization/confusable/locale policy is not yet part of the product and must not be delegated implicitly to database collation behavior.

### Infer the starter profile from schema defaults

Rejected. This ADR explicitly selects the starter profile. Schema defaults are implementation evidence only and must be revalidated by the operation contract.

### Create promoted vocations directly

Rejected. Promoted vocations represent progression state and are not initial creation choices.

### Add starter items or quest/storage state by imitation

Rejected. No generic mandatory starter initialization contract is proven, and unnecessary dependent writes would enlarge the shared-write privilege surface.

### Unlimited characters per account

Rejected. A hard limit is required for product abuse control and deterministic concurrency semantics.

## Follow-up

The next Phase 5 task is a bounded character-create operation contract. It must prove against current legacy engine:

- the exact `players` insert column set that yields this ADR's starter profile;
- whether any schema-default fields must be written explicitly for deterministic compatibility;
- loadability of `town_id = 8` plus `(0,0,0)` temple fallback for all allowed base vocations and both sex values;
- exact account-row lock and active-character count query;
- exact duplicate-name, same-account retry/idempotency and ambiguous-commit recovery semantics;
- exact dedicated `canary_character_create` column/table grants;
- real MariaDB integration coverage for locking, limit races, unique-name races, privilege denial and player load-shape assumptions.

`CHARACTER CREATION: BLOCKED` until that operation contract is approved and its implementation is tested.
