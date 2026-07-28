# Platform-driven Character Transfer Contract

## Status

`APPROVED FOR IMPLEMENTATION — OPERATION-SPECIFIC CHARACTER BAZAAR TRANSFER`

This contract defines the only approved Oteryn Platform operation for changing an existing Canary `players.account_id`: Character Bazaar escrow, cancellation return and winner settlement.

Evidence is pinned to read-only `blakinio/canary@885afae89946a56dddc2041bb335b6a900f206c9`, including `schema.sql@4e3d7e4b87ac51d1f664328b5399a78226153dc8` and the inspected login/session/save behavior at that revision. A materially different deployed Canary revision or schema requires revalidation.

## Ownership and authorization

Normal user ownership is established only by the ready immutable Platform binding:

`1 Platform Identity <-> 1 Canary accounts.id`

For seller-to-escrow transfer:

1. caller is the authenticated seller Identity;
2. ready binding resolves the exact seller `accounts.id` server-side;
3. requested `player_id` is loaded and must currently belong to that account;
4. the configured escrow account ID is server configuration, never browser input.

For escrow-to-buyer settlement:

1. winner Identity is stored by the locked Platform auction;
2. winner's current ready binding resolves the exact target account server-side;
3. seller, buyer and escrow account IDs are never accepted from browser fields.

For cancellation return, the target is the auction's immutable seller account captured at listing and revalidated against the seller's ready binding before return.

## Proven Canary behavior

At the evidence revision:

- `players.account_id` is an unsigned foreign key to `accounts.id`;
- active/listable characters use `players.deletion = 0`;
- game-world authentication verifies the requested character belongs to the authenticated account and is not deleted;
- persistent character-dependent data, including inventory, storage, stash, guild membership, house ownership and market rows, is keyed by `player_id`, not duplicated by account;
- the normal player save path does not write `players.account_id`;
- `cluster_sessions` has `PRIMARY KEY(account_id)` and `UNIQUE(player_id)` and is deleted on clean session release;
- a row in `cluster_sessions` represents an acquiring, online, saving, dirty or not-yet-released session boundary and therefore blocks transfer regardless of its status value.

## Bounded operation

Connection:

`canary_character_transfer`

Supported intents:

- `seller_to_escrow`;
- `escrow_to_seller`;
- `escrow_to_buyer`.

The operation changes only `players.account_id`.

It does not modify:

- player name, deletion state, level, skills, outfit, inventory, storage, guild, house or market data;
- accounts credentials, type, premium state or any coin field;
- account or cluster sessions;
- Redis state;
- Canary schema.

## Required transaction

Every transfer executes on the dedicated Canary connection:

1. begin transaction;
2. lock source and target account rows with `SELECT id FROM accounts WHERE id IN (?, ?) ORDER BY id FOR UPDATE`;
3. require both exact accounts to exist;
4. lock the player row selecting the approved snapshot/ownership fields;
5. if current owner equals target, classify as idempotent existing success;
6. otherwise require current owner equals expected source;
7. require `deletion = 0`;
8. query `cluster_sessions` for the player under lock-compatible transaction isolation and reject if any row exists, regardless of status or expiry;
9. for a normal user target, count active characters for the locked target account excluding the transferred player and reject when the count is already 10 or greater;
10. for the configured escrow target, skip the player-facing quota only when intent is exactly `seller_to_escrow`;
11. update `players.account_id` using both player ID and expected source account in the predicate;
12. require exactly one affected row;
13. commit.

Source/target account locking order is ascending numeric ID to reduce deadlock risk. The whole transaction may be retried up to three total attempts only for explicitly recognized deadlock/serialization failures.

## Escrow activation quiescence

One successful seller-to-escrow transaction does not immediately prove there was no game login already between ownership verification and session acquisition.

Therefore:

- the Platform persists the auction as `escrow_pending`;
- it records `escrowed_at` only after the transfer reports success;
- it waits at least the configured quiescence interval;
- reconciliation re-reads exact player ownership and requires no `cluster_sessions` row;
- only then may the auction become `active` and publicly bid-capable.

If a session row appears, the listing remains fail-closed in recovery. The character must not be auctioned while session state is ambiguous.

## Public-safe listing snapshot

Before the character enters escrow, the operation may read only these `players` fields for the immutable Platform snapshot:

- `id`, `name`, `account_id`, `deletion`;
- `level`, `vocation`, `experience`, `sex`, `maglevel`;
- `skill_fist`, `skill_club`, `skill_sword`, `skill_axe`, `skill_dist`, `skill_shielding`, `skill_fishing`;
- `looktype`, `lookaddons`, `lookhead`, `lookbody`, `looklegs`, `lookfeet`;
- `town_id`, `lastlogin`, `lastlogout`.

The persisted public snapshot excludes `account_id`, IP data, conditions, credentials, sessions and arbitrary blobs.

Snapshot expansion requires contract and grant expansion before code uses additional columns.

## Result contract

Bounded outcomes:

- `transferred`;
- `already_transferred`;
- `source_account_missing`;
- `target_account_missing`;
- `character_missing`;
- `ownership_conflict`;
- `character_deleted`;
- `character_online_or_session_active`;
- `target_character_limit`;
- `invalid_escrow_configuration`;
- `dependency_unavailable`.

Raw SQL errors, credentials, account identifiers and exception text are not exposed to public responses.

## Dedicated least-privilege grants

Approved SELECT:

- `accounts(id)`;
- `players` on exactly the ownership/snapshot fields listed above;
- `cluster_sessions(player_id, account_id, status, expires_at)`.

Approved UPDATE:

- `players(account_id)` only.

Forbidden:

- table-level `SELECT` or `UPDATE`;
- INSERT or DELETE on any Canary table;
- account password, email, coin or security-field access;
- player deletion/name/inventory/state mutation;
- session writes;
- DDL, administrative privileges or `GRANT OPTION`.

`database/provisioning/canary-character-transfer.sql.template` is the reviewed deployment template. Production credentials remain outside Git. `php artisan canary:verify-character-transfer-db-privileges` must fail closed on missing or excessive privileges.

## Cross-database saga

Platform wallet and auction rows cannot commit atomically with Canary. Marketplace actions therefore use deterministic operation IDs and durable state:

- listing: Platform pending row -> Canary escrow -> delayed ownership/session reconciliation -> active;
- cancellation: mark cancellation pending -> Canary return -> Platform terminal state;
- settlement: mark settlement pending -> Canary winner transfer -> Platform wallet settlement -> completed.

Reconciliation must inspect actual player owner and append-only wallet idempotency keys before retrying a step. It must never reverse or duplicate a wallet effect based only on a timeout.

## Validation requirements

Before launch:

- unit tests for result classification and snapshot allowlisting;
- feature tests for server-resolved ownership and public error redaction;
- real MariaDB tests for exact grants, forbidden privileges, owner/account/player lock ordering, active-session rejection, target quota and idempotent retry;
- a composed interruption/reconciliation test for transfer-success-before-Platform-commit and Platform-pending-before-transfer;
- browser acceptance for pending, active, cancellation, winning and recovery-visible states;
- deployment verification against the exact Canary schema/version and configured escrow account.

## Deployment prerequisites

1. provision a non-login Canary escrow account out of band;
2. inject its numeric ID as `MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID`;
3. provision the dedicated transfer principal from the reviewed SQL template;
4. inject credentials through approved secret management;
5. run the privilege verifier;
6. verify the escrow account exists, is not bound to an Identity and has no usable public credential path;
7. run marketplace reconciliation before exposing auction routes after deployment;
8. fail closed if any prerequisite is unavailable.

## Decision

`CHARACTER BAZAAR TRANSFER: APPROVED ONLY THROUGH THIS CONTRACT`

No general character reassignment, account transfer/import, rename, deletion or arbitrary administrator ownership edit is approved.
