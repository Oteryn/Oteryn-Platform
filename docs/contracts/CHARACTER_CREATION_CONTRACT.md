# Platform-driven Character Creation Contract

## Status

`IMPLEMENTED — GREENFIELD CHARACTER CREATION ON MAIN`

This contract defines the only approved Phase 5 operation for creating a greenfield Canary `players` row for an authenticated Oteryn Platform Identity under ADR 0005.

PR #41 implemented the operation and was squash-merged to `main` as `9839822b8e445c0e9828e73d2d7767bb237e587f` after final CI #568 and Agent Governance #489 passed.

`CHARACTER CREATION: IMPLEMENTED`

## Ownership and authorization

Canonical ownership model:

`1 Platform Identity <-> 1 Canary accounts.id`

Before any character-create Canary transaction:

1. caller is an authenticated Platform Identity;
2. Platform-owned `identity_canary_accounts` binding exists and is `ready`;
3. binding contains one non-null exact `canary_account_id`;
4. that server-resolved ID becomes the operation `account_id`;
5. browser-supplied account IDs, account names or email values cannot establish or override authorization.

Absent, pending or conflict binding fails closed before the character-write gateway is invoked.

## User-controlled inputs

Only:

- `name`;
- `vocation`;
- `sex`.

ADR 0005 canonicalization/reserved-name policy is applied before shared write.

Allowed vocation IDs:

`1`, `2`, `3`, `4`, `9`.

Allowed sex values:

`0`, `1`.

All other starter values are server-selected. Client-supplied `account_id`, level, town, position, conditions or other starter-state fields are not trusted inputs.

## Exact starter insert

The dedicated principal may insert only these `players` columns:

- `name`;
- `group_id`;
- `account_id`;
- `level`;
- `vocation`;
- `health`;
- `healthmax`;
- `experience`;
- `lookbody`;
- `lookfeet`;
- `lookhead`;
- `looklegs`;
- `looktype`;
- `lookaddons`;
- `maglevel`;
- `mana`;
- `manamax`;
- `manaspent`;
- `soul`;
- `town_id`;
- `posx`;
- `posy`;
- `posz`;
- `conditions`;
- `cap`;
- `sex`;
- `pronoun`;
- `istutorial`;
- `skill_fist`;
- `skill_fist_tries`;
- `skill_club`;
- `skill_club_tries`;
- `skill_sword`;
- `skill_sword_tries`;
- `skill_axe`;
- `skill_axe_tries`;
- `skill_dist`;
- `skill_dist_tries`;
- `skill_shielding`;
- `skill_shielding_tries`;
- `skill_fishing`;
- `skill_fishing_tries`.

No other direct `players` insert column is approved in v1.

Exact server-selected values include:

- canonical `name`;
- `group_id = 1`;
- exact ready bound `account_id`;
- `level = 8`;
- validated vocation;
- `health = healthmax = 185`;
- `experience = 4200`;
- approved starter outfit values with sex-dependent `looktype`;
- `maglevel = 0`;
- `mana = manamax = 90`;
- `manaspent = 0`;
- `soul = 100`;
- `town_id = 8`;
- persisted position `(0,0,0)`;
- non-null empty `conditions` payload;
- `cap = 470`;
- validated sex;
- `pronoun = 0`;
- `istutorial = 0`;
- listed classic skill levels `10` with zero tries.

Other current `players` columns intentionally use current schema defaults/nullable behavior. Real MariaDB tests assert representative relied-upon defaults including active deletion state, save flag, balance, stamina, offline-training state, reward state and forge dust level.

A material Canary schema/default change requires revalidation before deployment.

## No dependent starter writes

The operation does not directly write:

- player items/inboxes/depots/reward containers;
- stash/storage/quest/tutorial side tables;
- learned spells;
- guild tables;
- account or cluster session tables;
- account credentials.

No incidental first-login script is part of the atomic creation contract.

## Operation identity and idempotency

Natural v1 operation identity:

`(authorized Canary accounts.id, canonical players.name)`

Existing exact-name classification after locking the account row:

- same authorized account + active row -> idempotent existing success with the same `players.id` and no mutation;
- another account -> deterministic `name_conflict`;
- same account + non-zero deletion -> deterministic `name_conflict`.

The first successful creation fixes vocation/sex/starter state. Repeated same-name requests do not mutate the existing row.

No player UPDATE, ownership reassignment or destructive compensation is permitted for recovery.

## Exact Canary transaction

Using only `canary_character_create`:

1. begin transaction;
2. `SELECT id FROM accounts WHERE id = ? FOR UPDATE` for the authorized account;
3. missing account -> `account_missing`;
4. query exact canonical player name selecting only `id`, `name`, `account_id`, `deletion`;
5. perform idempotent/conflict classification;
6. if no row exists, count active characters for the locked account;
7. count `>= 10` -> `character_limit`;
8. insert exactly the approved starter columns/values;
9. obtain generated player ID;
10. commit and return created result.

Exact-name recovery occurs before quota evaluation so retrying a successfully created tenth character returns the same player rather than a false limit result.

## Concurrency and retry

Same-account create operations serialize on the same account row lock before quota evaluation.

Real MariaDB coverage proves the nine-to-ten concurrent last-slot race results in exactly one new active character and one limit outcome.

Global name uniqueness is protected by the Canary `players.name` unique constraint. Real MariaDB coverage proves two accounts racing the same canonical name produce exactly one committed row and one deterministic conflict outcome.

The implementation retries the whole transaction up to three total attempts only for explicitly recognized deadlock/serialization failures. Permanent authorization, validation, conflict, quota and privilege errors are not retried.

## Result/error contract

Bounded outcomes are equivalent to:

- `created`;
- `existing`;
- `validation_failed`;
- `binding_not_ready`;
- `account_missing`;
- `name_conflict`;
- `character_limit`;
- `dependency_unavailable`.

Raw SQL errors, credentials and database exception details are not exposed to the browser.

## Dedicated least-privilege boundary

Connection:

`canary_character_create`

It does not reuse or broaden:

- `canary` / `oteryn_readonly`;
- `canary_provisioning`.

Approved SELECT surface:

- `accounts(id)`;
- `players(id, name, account_id, deletion)`.

Approved INSERT surface:

- only the exact starter columns listed above.

Forbidden:

- table-level broad SELECT;
- `accounts.password` or unrelated account fields;
- player UPDATE/DELETE;
- unapproved player INSERT columns;
- writes to unrelated player/account/session tables;
- DDL;
- `GRANT OPTION` or administrative privileges.

A reviewed SQL provisioning template defines the deployment grants. Production credentials stay outside Git.

`CanaryCharacterCreateDatabasePrivilegeVerifier` fails closed when required privileges are missing or broader privileges are present.

Deployment verification command:

`php artisan canary:verify-character-create-db-privileges`

## Validation evidence

PR #41 includes:

- name-policy unit tests;
- authenticated feature tests through the real Platform login/session establishment flow;
- binding authorization and browser non-control tests;
- deterministic validation/conflict/idempotent result tests;
- effective-grant verifier tests;
- real MariaDB exact-grant and denied-privilege tests;
- real MariaDB account lock, starter/default persisted state and active-limit tests;
- real MariaDB same-account last-slot race;
- real MariaDB cross-account same-name race;
- committed-row forward recovery coverage.

The initial feature-test 403 was a test-harness mismatch: synthetic authentication state did not establish the production web-session generation marker. Tests were corrected to exercise the real login flow; the security middleware was not bypassed.

Final PR #41 head `1e6291afb29a4c92d1ba690f74c6d23671876107` passed CI #568 and Agent Governance #489 before squash merge.

## Deployment gate

Before enabling character creation in an environment:

1. provision `canary_character_create` out-of-band using the reviewed SQL template;
2. provide credentials through approved secret management;
3. run `php artisan canary:verify-character-create-db-privileges`;
4. fail closed on missing/excessive grants;
5. revalidate if deployed Canary schema materially differs from the evidence baseline.

## Cross-repository decision

No `blakinio/canary` or `opentibiabr/login-server` change was required for this bounded character-create database operation.

The authoritative Platform game-login bridge remains a separate cross-repository dependency and is not implied by character creation.

## Decision

`CHARACTER CREATE OPERATION: IMPLEMENTED`

`CHARACTER CREATION: UNBLOCKED AND PRESENT ON MAIN`

Character deletion/soft deletion and rename remain optional future lifecycle operations and are forbidden until separately contracted, least-privileged and tested.
