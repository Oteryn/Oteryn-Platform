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
- Platform Identity credentials and password-reset state;
- Platform registered web-session records and revocation state;
- primary-email change requests, cooldown and old-address recovery state;
- account privacy flags and Platform termination state;
- recovery-key verifier, generation, use and revocation state;
- platform-specific user preferences;
- platform notification metadata;
- support tickets/messages, player/content/guild reports, Platform enforcement records and support notification delivery state;
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

Platform Identity is the authority for supported web-user credentials and the Platform-owned account-security lifecycle. The following are Platform-owned and migrated only by Oteryn Platform:

- password hash and password-reset token lifecycle;
- MFA secret/recovery-code state;
- registered web-session inventory, generation and revocation timestamps;
- confirmed primary-email change requests, cooldown and old-address recovery windows;
- account privacy preferences;
- verifier-only high-assurance recovery key state;
- Platform termination request, grace period, cancellation and anonymization state;
- bounded Identity security-event metadata.

The authenticated Identity and its ready server-resolved binding establish the user-scoped Canary account relationship. Browser-supplied Canary account IDs, web-session IDs or email-change identifiers never establish ownership.

Account-security operations may revoke Platform web sessions and the separately contracted game-authorization generation. They do not delete, unlink, rebind or transfer the bound Canary account or any Canary-owned character data. Platform termination disables and anonymizes Platform login while preserving the immutable binding for audit and safety.

Any new cross-system credential, game-session or account-binding mutation still requires an explicit contract defining ownership, compatibility, rollout and rollback. The delivered lifecycle does not prove that native Canary/external login-server authentication has been replaced by Platform authorization.

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

## Support and moderation data

Platform owns the additive support schema and lifecycle:

- `support_tickets` and `support_ticket_messages`;
- `player_reports`;
- `enforcement_records`;
- `support_notification_deliveries`.

Public ULIDs are routing references, not ownership proof. Identity relations and moderator permissions are resolved server-side. User reads exclude internal ticket notes, reporter identity outside the owner view, moderator notes and administrator audit metadata.

Platform enforcement records are communication and workflow records only. They do not mutate or supersede Canary-owned bans, account status or game runtime enforcement. Any future synchronization requires an explicit cross-repository contract, rollout order, least-privilege credential and rollback plan.

Retention may delete old closed ticket/report records and anonymize expired enforcement reasons through the supported command. It never deletes Canary-owned data and must preserve the configured audit/privacy boundary.

## Future financial data

Oteryn Coins and future payment balances are Platform-owned business data unless a later ADR says otherwise.

Use an immutable/append-oriented ledger as the source of financial history. A cached balance may exist, but mutation must remain transactional and auditable.

Do not reuse a generic mutable `premium_points`, Canary coin or tournament-coin field as the sole financial source of truth without a deliberate ADR and threat/concurrency analysis.

Payment providers, coin purchasing, refunds and chargebacks remain outside Character Bazaar v1 and require a separate payment threat model and reconciliation contract.

## Data classification

### Secret

Passwords, password hashes where exposure increases attack value, session tokens, reset tokens, email-change tokens, MFA secrets, raw recovery keys, private keys, provider secrets.

### Sensitive personal/security data

Email addresses, protected source-address fingerprints, registered session metadata, account security events, recovery-key verifier, termination history, ticket/report/appeal content and enforcement history.

### Internal operational

Admin audit records, integration errors, deployment metadata, marketplace saga failure codes and wallet idempotency keys.

### Public game data

Character/guild/highscore data explicitly intended for public display, including the allowlisted immutable Character Bazaar snapshot. Canary account IDs, sessions, IPs and credentials are never public snapshot fields.

Classification affects logging, access, retention and export behavior.

## Public community read ownership

Canary owns character levels, vocation, experience, magic/skill values, comments, boss points, guild membership/ranks, houses, deaths and runtime leases. Oteryn Platform reads the approved subset through the dedicated direct-table `SELECT` principal and does not copy these values into a competing mutable source of truth.

Platform Identity owns `public_account_association` and `public_status_visible`. A ready server-resolved Identity-to-Canary binding is required before related characters or status timestamps may be disclosed. Browser-supplied account, Identity or player identifiers are never ownership evidence.

The current character model is global across channels. Per-channel highscore ownership, world-transfer history and selectable achievements have no authoritative current source and remain explicitly unavailable rather than inferred. Canary continues to own guild mutations; Platform delivers directory/search/detail only.


## Character profile preference ownership

Platform owns `character_profile_preferences`: the owner-authored public comment, per-character visibility flags and optional main-character selection. Canary remains the source of current character identity, account ownership and gameplay/profile facts. Every management write re-resolves the ready binding and reads the active Canary character before changing Platform state; stored player IDs never become ownership proof.

Account-level association and status flags are disclosure upper bounds. Per-character preferences may only narrow them. Platform comments are bounded plain text rendered escaped and do not update `players.comment`. Main-character replacement locks the Platform Identity row, writes atomically and is proven under concurrent real-MariaDB processes. This boundary authorizes no Canary rename, deletion, restore, transfer, achievement or generic player update.
