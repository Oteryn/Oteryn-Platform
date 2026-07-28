# Oteryn Platform Data Ownership

## Purpose

Prevent accidental coupling and unsafe shared-database writes between Oteryn Platform, login-server and Canary.

## Rule

Every persistent data set has exactly one documented **primary owner**. Other components may read or write only through an explicitly documented contract.

At bootstrap, exact Canary table ownership is not yet proven. Do not infer ownership from MyAAC or generic TFS conventions.

## Ownership categories

### Platform-owned

Oteryn Platform controls schema, migrations and lifecycle.

Expected examples:

- CMS/news content;
- platform roles/permissions if not mapped to a shared account field;
- audit events;
- MFA metadata;
- platform-specific user preferences;
- platform notification metadata;
- Character Bazaar auctions, bids, watchlists, saga state and immutable listing snapshots;
- Oteryn Coins wallet accounts, reservations and append-oriented ledger entries;
- future payment-provider records.

### Canary-owned

Canary controls schema/semantics. Oteryn Platform is read-only unless a write contract explicitly permits operations.

Potential examples may include game runtime/world state and gameplay-owned player data. Exact tables must be discovered from the actual Oteryn Canary repository/database schema.

### Shared-contract data

Both components require access, but one component remains the semantic owner and the contract defines allowed operations.

Confirmed operation-specific examples:

- greenfield Canary account provisioning;
- greenfield Canary character creation;
- Character Bazaar reassignment of the single `players.account_id` ownership field through `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`.

Other candidates requiring discovery include:

- guild membership;
- bans/account status;
- game login sessions/tokens.

No candidate is confirmed solely by this document.

## Write policy

For every shared write path document:

1. primary owner;
2. caller/component;
3. exact table/model/fields;
4. validation rules;
5. authorization rule;
6. transaction boundary;
7. concurrency/locking behavior;
8. side effects/cache/session invalidation;
9. compatibility/version assumptions;
10. rollback/migration implications.

## Read policy

Public read features such as highscores or character profiles may query Canary-compatible data through dedicated read/query services.

Rules:

- use explicit selected columns rather than depending on arbitrary whole-row shapes when practical;
- handle missing/deprecated fields deliberately;
- document freshness/cache expectations;
- avoid N+1/mass-query patterns;
- do not accidentally turn read models into mutation-capable domain models.

## Database credentials

Target production direction:

- platform migration owner: allowed only for platform-owned schema and approved migrations;
- application runtime credential: least privileges required by the platform;
- read-only game-data credential where architecture permits separation;
- operation-specific Canary write credentials for provisioning, character creation and Character Bazaar transfer;
- Canary uses its own credential;
- no shared root/admin database credential in application configuration.

The Character Bazaar transfer principal is restricted to approved column-level reads on `accounts`, `players` and `cluster_sessions`, plus `UPDATE (account_id)` on `players`. It cannot read credentials/coins, update other player fields or write sessions.

Exact credential split depends on final deployment/database topology.

## Migrations

### Platform-owned schema

Laravel migrations are authoritative.

### Shared/Canary schema

Oteryn Platform does not silently migrate Canary-owned tables.

A required shared schema change must:

- be documented in `docs/contracts/**`;
- identify the owning repository;
- define compatibility order;
- define rollback/backward compatibility;
- coordinate both repositories when atomic behavior is required.

Character Bazaar v1 requires no Canary schema migration. It uses the verified existing `players.account_id`, `accounts.id` and `cluster_sessions` schema at the pinned evidence revision.

## Identity data special rule

Credentials and game-login compatibility require explicit ownership discovery before implementation.

Questions that must be answered:

- Which table/system is authoritative for account credentials?
- Which hashing formats are accepted by login-server/Canary?
- Can the platform migrate hashes transparently?
- Which component creates game sessions/tokens?
- Which component revokes them?
- What happens to active sessions after password/MFA/account-state changes?

Until answered, agents must not implement a speculative credential migration.

## Character data special rule

Before web character creation/deletion/rename or ownership transfer, verify:

- required columns/defaults;
- name uniqueness and normalization rules;
- vocation/sex/town/world rules;
- starter state creation requirements;
- deletion semantics;
- online-character restrictions;
- foreign keys and dependent rows;
- Canary caches/runtime assumptions.

### Character Bazaar transfer boundary

Canary remains the semantic owner of the character and all gameplay-dependent rows keyed by `player_id`. The Platform may change only `players.account_id` through the dedicated transfer contract.

The transfer operation:

- resolves source and target account IDs server-side;
- locks source/target accounts and the player deterministically;
- requires `deletion = 0`;
- rejects any `cluster_sessions` row for the character;
- enforces the normal target account quota except for the dedicated escrow account;
- is idempotent when the expected target already owns the character;
- never changes name, skills, inventory, guild, house, market or session state.

A listing is not public merely because the first transfer succeeds. Platform-owned saga state waits for the configured quiescence interval and reconfirms escrow ownership plus absence of a cluster session before activation.

## Character Bazaar and wallet data

Primary Platform-owned tables:

- `character_auctions` — seller/winner Identity references, immutable public-safe character snapshot, auction terms, state machine and recovery code;
- `character_auction_bids` — immutable request identity, bidder, amount and bounded status lifecycle;
- `character_auction_watches` — user preference relation;
- `wallet_accounts` — transactionally maintained available/reserved projection;
- `wallet_ledger_entries` — append-oriented signed deltas and unique idempotency keys.

Rules:

- browser-supplied Canary account IDs never establish ownership;
- Oteryn Coins are not a Canary coin field, character bank balance or payment-provider balance;
- the ledger is the financial mutation trace; cached wallet balances are updated in the same transaction;
- available and reserved balances cannot become negative;
- the current leading bid alone remains reserved;
- outbid release, purchase settlement, seller proceeds and administrator adjustments use deterministic idempotency keys;
- administrator adjustments require confirmed MFA, exact permission and an audit event committed in the same Platform transaction;
- cross-database ownership and wallet settlement use a durable idempotent saga rather than claiming distributed ACID.

## Future financial data

Oteryn Coins and future payment balances are Platform-owned business data unless a later ADR says otherwise.

Use an immutable/append-oriented ledger as the source of financial history. A cached balance may exist, but mutation must remain transactional and auditable.

Do not reuse a generic mutable `premium_points`, Canary coin or tournament-coin field as the sole financial source of truth without a deliberate ADR and threat/concurrency analysis.

Payment providers, coin purchasing, refunds and chargebacks remain outside Character Bazaar v1 and require a separate payment threat model and reconciliation contract.

## Data classification

### Secret

Passwords, password hashes where exposure increases attack value, session tokens, reset tokens, MFA secrets, private keys, provider secrets.

### Sensitive personal/security data

Email addresses, IP/security history, account security events.

### Internal operational

Admin audit records, integration errors, deployment metadata, marketplace saga failure codes and wallet idempotency keys.

### Public game data

Character/guild/highscore data explicitly intended for public display, including the allowlisted immutable Character Bazaar snapshot. Canary account IDs, sessions, IPs and credentials are never public snapshot fields.

Classification affects logging, access, retention and export behavior.
