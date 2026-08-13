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
- platform-specific user preferences, including character presentation/privacy preferences;
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

For the native Oteryn-v2 target, ADR 0030 and the Oteryn-v2 Character Authority contract define a different steady-state boundary: Platform consumes authorized character projections and versioned game-owned commands rather than becoming a direct writer of native game character tables.

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

Authenticated native Character Portfolio reads must follow ADR 0030: `Accounts` composes an owner-scoped application result from canonical `AccountId`, an authorized game-owned character projection and Platform-owned presentation/privacy state. A Platform cache/read model is non-authoritative and never establishes current character ownership or mutation authority.

Rules:

- use explicit selected columns rather than depending on arbitrary whole-row shapes when practical;
- handle missing/deprecated fields deliberately;
- document freshness/cache expectations;
- avoid N+1/mass-query patterns;
- do not accidentally turn read models into mutation-capable domain models;
- distinguish authoritative empty, stale, unavailable, ambiguous and incompatible projection states where the source contract supports them;
- never convert dependency failure into an empty owner portfolio.

## Database credentials

Target production direction:

- platform migration owner: allowed only for platform-owned schema and approved migrations;
- application runtime credential: least privileges required by the platform;
- read-only game-data credential where architecture permits separation;
- operation-specific Canary write credentials for provisioning, character creation and Character Bazaar transfer;
- Canary uses its own credential;
- no shared root/admin database credential in application configuration.

The Character Bazaar transfer principal is restricted to approved column-level reads on `accounts`, `players` and `cluster_sessions`, plus `UPDATE (account_id)` on `players`. It cannot read credentials/coins, update other player fields or write sessions.

Native Oteryn-v2 Character Authority integration must not inherit these Canary SQL grants as its steady-state mutation model. Native write authority is expressed through the game-owned command boundary accepted by ADR 0030 and the Oteryn-v2 Character Authority contract.

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

ADR 0030 does not authorize a CharacterId migration. A later Platform-owned preference migration must be additive: canonical `CharacterId` is added before legacy `canary_player_id` removal, mapping must come from authoritative migration/projection evidence, unresolved mappings fail closed, rollback remains possible while compatibility references are retained and legacy removal occurs only after all consumers and rollback gates are proven.

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

For native Oteryn-v2 integration, canonical cross-boundary account identity is the Platform-issued `AccountId` defined by ADR 0028. Current `canary_account_id` remains compatibility state and must not be promoted to native account identity.

Account-security operations may revoke Platform web sessions and the separately contracted game-authorization generation. They do not delete, unlink, rebind or transfer the bound Canary account or any Canary-owned character data. Platform termination disables and anonymizes Platform login while preserving the immutable binding for audit and safety.

Any new cross-system credential, game-session or account-binding mutation still requires an explicit contract defining ownership, compatibility, rollout and rollback. The delivered lifecycle does not prove that native Canary/external login-server authentication has been replaced by Platform authorization.

## Character data special rule

Before web character creation/deletion/rename or ownership transfer on the current Canary compatibility path, verify:

- required columns/defaults;
- name uniqueness and normalization rules;
- vocation/sex/town/world rules;
- starter state creation requirements;
- deletion semantics;
- online-character restrictions;
- foreign keys and dependent rows;
- Canary caches/runtime assumptions.

### Native Oteryn-v2 Character Authority boundary

ADR 0030 accepts the native steady-state split:

- Oteryn Platform Identity owns/issues canonical `AccountId`;
- Oteryn-v2 Character Authority owns/issues canonical `CharacterId`;
- Character Authority owns authoritative current `AccountId <-> CharacterId` ownership, character lifecycle, final game-domain capability decisions and native mutation results;
- `Accounts` owns authenticated Character Portfolio composition for Account Center but consumes an authorized game-owned projection and remains non-authoritative for ownership;
- `Characters` owns Platform-side command orchestration only; native create, rename, delete/restore/finalize, world transfer and account/Bazaar transfer execute through versioned game-owned command boundaries;
- a Platform cache, preference row, prior portfolio response or browser-supplied identifier never authorizes a native character mutation;
- native command handling revalidates current authoritative ownership and preconditions;
- rename, legal world transfer and legal account transfer preserve `CharacterId`; terminal deletion never permits CharacterId reuse.

Platform must not infer native game-owned capability from raw row counts. In particular, the current Canary-compatible ten-character rule in Account Center remains implementation/compatibility evidence, not the native source of truth. Platform-owned authentication, MFA, legal/business or entitlement gates may be combined with game-owned capability results fail-closed, but a Platform entitlement that changes a game-domain limit must cross an explicit versioned contract rather than bypass Character Authority locally.

Exact transport, command envelope, projection TTL, capability-code vocabulary, entitlement exchange and migration implementation remain separately gated.

### Character Bazaar transfer boundary

For the delivered Canary compatibility path, Canary remains the semantic owner of the character and all gameplay-dependent rows keyed by `player_id`. The Platform may change only `players.account_id` through the dedicated transfer contract.

The transfer operation:

- resolves source and target account IDs server-side;
- locks source/target accounts and the player deterministically;
- requires `deletion = 0`;
- rejects any `cluster_sessions` row for the character;
- enforces the normal target account quota except for the dedicated escrow account;
- is idempotent when the expected target already owns the character;
- never changes name, skills, inventory, guild, house, market or session state.

A listing is not public merely because the first transfer succeeds. Platform-owned saga state waits for the configured quiescence interval and reconfirms escrow ownership plus absence of a cluster session before activation.

For the native target, Character Bazaar remains a Platform-owned commercial saga while final character ownership mutation is performed atomically by Character Authority through the versioned game-owned command boundary and reconciled from an idempotent game-owned result/receipt. The existing Canary direct-write transfer remains compatibility-only until a separately authorized cutover.

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

Platform enforcement records are communication, decision and orchestration records only. They do not mutate or supersede game-owned sanctions, Canary-owned bans/account status or game runtime enforcement, and a dispatched/pending Platform operation is not proof of game effect.

For the native target, `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md` defines the accepted semantic boundary: Platform owns the authorized decision, appeal/communication workflow and stable operation ledger; Oteryn-v2 owns authoritative target state, sanction applicability, mutation, expiry, active-session/runtime enforcement and results. Retries/reconciliation preserve one operation identity and monotonic decision revisions prevent stale apply/replace/revoke/expire commands from weakening newer state. Exact transport, persistence and activation remain deferred.

Current Canary bans/account status remain Legacy Canary Compatibility authority. Any compatibility synchronization or native activation requires an explicit rollout order, least-privilege credential, mixed-version evidence and rollback plan; native and Canary sanctions must not be silently dual-written as co-authoritative state.

Retention may delete old closed ticket/report records and anonymize expired enforcement reasons through the supported command. It never deletes Canary-owned data and must preserve the configured audit/privacy boundary.

## Game Catalog content data

ADR 0034 separates source authority from catalogue persistence. Oteryn-v2 owns native canonical content identities, executable gameplay definitions/relations, ruleset applicability, authority revisions/epochs and authoritative removal semantics. Platform `GameCatalog` owns immutable imported snapshots, validation findings, profiles, activation/rollback, rebuildable projections, presentation metadata and administration; those rows remain evidence from their declared source and do not transfer gameplay authority.

Current `oteryn.game-catalog` Canary snapshots remain Legacy Canary Compatibility data. Canary identifiers and final-registry assumptions stay namespaced to that authority profile and cannot become native canonical identity. One active profile has one declared gameplay-content authority: Platform must not merge native, Canary and editorial fields into a co-authoritative row or silently fall back to Canary when native evidence is stale/unavailable.

Native intake uses a separately versioned immutable producer contract with explicit authority epoch/revision, capability/completeness evidence, stable typed identity, deterministic digest and tombstones. Imports are inactive by default; activation/rollback are exact-snapshot, transactional and audited. Direct native game-table access is not an accepted steady-state integration.

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

Canary owns character levels, vocation, experience, magic/skill values, comments, boss points, guild membership/ranks, houses, deaths and runtime leases for the current compatibility model. Oteryn Platform reads the approved subset through the dedicated direct-table `SELECT` principal and does not copy these values into a competing mutable source of truth.

Platform Identity owns `public_account_association` and `public_status_visible`. A ready server-resolved Identity-to-Canary binding is required before related characters or status timestamps may be disclosed. Browser-supplied account, Identity or player identifiers are never ownership evidence.

The current character model is global across channels. Per-channel highscore ownership, world-transfer history and selectable achievements have no authoritative current source and remain explicitly unavailable rather than inferred. Canary continues to own guild mutations; Platform delivers directory/search/detail only.

For the native target, public/general projections remain a `PublicGameData` concern while authenticated owner portfolio composition belongs to `Accounts` under ADR 0030. A public projection must not be reused as proof of owner-private character authority.

## Character profile preference ownership

Platform owns `character_profile_preferences`: the owner-authored public comment, per-character visibility flags and optional main-character selection.

Current Canary compatibility behavior uses numeric `canary_player_id`. Canary remains the source of current character identity, account ownership and gameplay/profile facts for that delivered path. Every management write re-resolves the ready binding and reads the active Canary character before changing Platform state; stored player IDs never become ownership proof.

The accepted native target under ADR 0030 uses canonical `CharacterId` for the game-character reference while Platform-local `identity_id` may remain an internal persistence surrogate. The preference record remains Platform-owned presentation/privacy state only. It never proves current game ownership, lifecycle eligibility or mutation authority, and `is_main_character` is presentation preference rather than game state.

Migration from `canary_player_id` must be additive and mapping-driven. Canonical `CharacterId` is added before legacy removal; mapping comes from authoritative migration/projection evidence; missing/conflicting mapping fails closed; rollback remains possible while compatibility state is retained; legacy removal is a later gated task.

Account-level association and status flags are disclosure upper bounds. Per-character preferences may only narrow them. Platform comments are bounded plain text rendered escaped and do not update `players.comment`. Main-character replacement locks the Platform Identity row, writes atomically and is proven under concurrent real-MariaDB processes. This boundary authorizes no Canary/native rename, deletion, restore, transfer, achievement or generic player update.
