# Oteryn Platform Module Catalog

This catalog defines module responsibilities and dependency boundaries.

## Status legend

- `PLANNED` — architecture decision only; no implementation proven.
- `DISCOVERY` — contract/research work required before a concrete capability can be implemented.
- `IMPLEMENTING` — active source implementation exists in an active task.
- `AVAILABLE` — at least one explicitly documented capability is implemented and validated on `main`; this does not imply every conceivable operation in the module exists.

| Module | Status | Owns | Must not own |
|---|---|---|---|
| Identity | AVAILABLE | Platform web authentication policy, credentials lifecycle, registered sessions, MFA, confirmed email changes, privacy, recovery and Platform termination | Payments, game runtime, arbitrary Canary account or character mutations |
| Accounts | AVAILABLE | Greenfield account provisioning/binding and future explicitly contracted account-level operations | Canary password verification logic, undocumented shared writes, game runtime |
| Characters | AVAILABLE | Contract-approved web-triggered character operations; currently create plus Character Bazaar ownership transfer | Direct undocumented Canary writes; uncontracted rename/delete |
| PublicGameData | AVAILABLE | Read models/queries for characters, guilds, highscores, online/status | Privileged mutations |
| CMS | AVAILABLE | Public content reads and permission-scoped Platform content management | Identity policy, game state, rich/upload surfaces without explicit security controls |
| Support | AVAILABLE | Platform tickets, reports, enforcement records, notifications, retention and privacy-safe user/moderator presentation | Canary ban mutation, file attachments, disclosure of reporter identity or private moderator notes |
| EditorialMedia | IMPLEMENTING | Private normalized raster-image objects, integrity metadata, bounded consumer references and administrator lifecycle | Public/executable uploads, arbitrary documents, consumer-specific publication rules |
| Wiki | IMPLEMENTING | Localized Wiki articles, categories, lifecycle, optimistic locking and append-only revisions | Generic CMS pages, public activation before release criteria, arbitrary HTML, media/search without separate reviewed slices |
| Admin | AVAILABLE | Admin UI, explicit RBAC/policies, privileged Platform use cases | Bypassing domain/application invariants or granting implicit wildcard authority |
| Audit | AVAILABLE | Security/admin audit primitives, privileged-action audit and bounded admin audit visibility | Secrets, raw credentials, business-rule authorization decisions |
| Integration | AVAILABLE | Implemented Canary read/write adapters, schema translation, contract enforcement; future login bridge remains separate | Product policy that belongs in domain modules |
| Wallet | IMPLEMENTING | Oteryn Coins available/reserved projection and append-oriented idempotent ledger | Canary coins, payment-provider settlement, arbitrary balance edits |
| Marketplace | IMPLEMENTING | Character Bazaar listings, escrow saga, bids, watches, settlement, history and recovery policy | Canary gameplay state, generic character mutation, payment-provider commerce |
| Notifications | AVAILABLE | Password recovery, localized account-security email and deterministic support/moderation delivery state | Core auth decisions, token validation, domain rollback on mail failure, payment settlement |
| PlatformAPI | PLANNED | Stable first-party API endpoints and API-specific auth/limits | Duplicating business logic from modules |
| Payments | PLANNED-LATER | Provider adapters, payments, webhook handling and regulated commerce when approved | Identity core, direct dependency from basic account creation/login, Oteryn Coins gameplay marketplace policy |

## Identity

### Responsibilities

- login/logout;
- Platform credential hashing and lifecycle;
- registered web-session creation, rotation, inventory and owner-scoped revocation;
- password reset/change;
- confirmed primary-email change with old-address cancellation/recovery and cooldown;
- MFA/TOTP and recovery codes;
- account privacy/status controls;
- verifier-only high-assurance recovery key lifecycle;
- bounded Platform termination request, cancellation and finalization;
- English and Polish account-security presentation and notification links;
- authentication and sensitive-mutation rate limiting;
- security-sensitive Identity audit events.

### Current available boundary

The Platform web Identity authority provides registration, framework-hashed credentials, login/logout, revocable registered web sessions, password recovery/change, rate limiting, security-event recording and opt-in web MFA.

The account-security lifecycle additionally provides:

- new-address confirmation and previous-address cancellation/recovery for primary-email changes;
- single-use token, cooldown and global web/game authorization revocation policy;
- active browser-session inventory with current, targeted and all-other revocation;
- private-by-default account-association and public-status controls;
- one active recovery key displayed once and stored only as a keyed verifier;
- recovery-key rotation, revocation, use and replay denial, including password/MFA reset;
- Platform termination grace period, cancellation and idempotent due finalization;
- scoped session-backed `en`/`pl` localization for protected and token account-security routes.

Phase 5 makes Platform Identity the ownership authority for supported greenfield game accounts, but it does not mean native Canary/external login-server game authentication has already been replaced by Platform authorization.

### Invariants

- one authoritative Platform Identity policy for supported product users;
- user credentials and raw recovery keys are never stored reversibly;
- email-change, recovery-key and termination operations revoke the exact Platform web/game authorization state defined by policy;
- browser-supplied session, Canary account or token-adjacent identifiers never establish ownership;
- stale or revoked registered sessions are invalidated before protected controller execution;
- account-security audit metadata excludes raw passwords, session IDs, email/reset tokens, MFA secrets, recovery keys and complete source addresses;
- privileged/Admin routes combine authentication, explicit authorization and `mfa.confirmed`;
- MFA never grants authorization by itself;
- email-code MFA is intentionally not adopted while email remains the recovery channel;
- the ready Platform-to-Canary binding remains immutable without a separate operation contract;
- Platform termination does not delete, unlink, rebind or transfer Canary-owned accounts or characters;
- game-login compatibility/migration remains contract-driven.

## Accounts

### Responsibilities

- durable mapping from authenticated Platform Identity to supported Canary game account;
- greenfield Canary account provisioning;
- account state/preferences and future lifecycle operations only when explicitly contracted.

### Current available boundary

Phase 5 implements:

- immutable `1 Platform Identity <-> 1 Canary accounts.id` greenfield ownership model;
- durable pending/ready/conflict provisioning/binding state;
- dedicated least-privilege `canary_provisioning` adapter;
- forward-recoverable account-create saga;
- non-user sink credential compatibility representation;
- fail-closed effective-grant verification and real MariaDB integration coverage.

The Identity account-security lifecycle may disable and anonymize Platform login while preserving this binding. It does not mutate the bound Canary account.

Existing Canary account claim/import, Canary account deletion, unlink/rebind/transfer and broader Canary account profile mutations are not implied by `AVAILABLE` and require separate contracts.

### Invariants

- every account mutation requires authenticated/authorized Platform context;
- browser-supplied account IDs are never ownership proof;
- ready immutable binding is the trusted source for user-scoped Canary authorization;
- generic `canary` remains read-only;
- account write capability is operation-specific and least-privileged;
- Platform does not duplicate Canary reusable-password verification.

## Characters

### Responsibilities

- web-triggered character lifecycle and ownership operations explicitly approved by product/Canary contracts.

### Current available boundary

Phase 5 implements character creation:

- authorization through the authenticated Identity's ready immutable Canary account binding;
- ADR 0005 canonical-name, vocation/sex, starter-state and quota policy;
- dedicated least-privilege `canary_character_create` adapter;
- account-row locking, same-name idempotent recovery, maximum-10-active-character enforcement and unique-name race handling;
- real MariaDB privilege/concurrency coverage.

Character Bazaar adds one separate ownership-transfer operation under `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`:

- seller, escrow and winner account IDs are resolved server-side;
- only `players.account_id` may be updated by `canary_character_transfer`;
- source/target account rows and the player row are locked deterministically;
- deleted characters and every existing `cluster_sessions` row fail closed;
- normal target character quota is enforced while the non-login escrow account is explicitly exempt;
- already-target-owned state is idempotent success;
- public activation waits for a second ownership/session check after a bounded quiescence period.

Character deletion/soft deletion and rename are not implemented or authorized. They require separate operation contracts and do not inherit create or transfer privileges.

### Invariants

- character ownership is resolved server-side from the ready binding or the durable locked marketplace auction;
- client-controlled account IDs cannot establish ownership;
- names and vocation/sex choices follow verified product policy;
- concurrency-sensitive writes are transactional;
- no raw undocumented shared-table mutation;
- each new mutation operation gets its own contract and least-privilege boundary;
- the Character Bazaar transfer never changes character name, skills, inventory, guild, house, market or session state.

## PublicGameData

### Responsibilities

- character profiles;
- guild pages;
- highscores;
- online list;
- server/channel status;
- public search.

### Current available boundary

Implemented Phase 4 read-only surfaces use explicit field allowlists, bounded pagination and the database-enforced `canary` / `oteryn_readonly` SQL boundary. Runtime availability uses the separate read-only `canary_runtime` Redis boundary with TTL freshness and fail-closed semantics.

Character Bazaar listing creation uses its own operation-specific transfer connection to capture an immutable public-safe allowlisted character snapshot before escrow. Public catalogue/detail reads use the Platform snapshot rather than live mutable Canary data.

### Invariants

- no privileged mutations through public read services;
- private account/session/security fields are never public output;
- dependency failure is explicit, not fabricated empty/offline state;
- freshness boundaries are not extended by unbounded caching;
- Character Bazaar snapshots exclude Canary account IDs, IPs, sessions, credentials and arbitrary blobs.

## CMS

### Responsibilities

- news/articles;
- managed pages;
- publication state;
- media references only if a future explicit upload-security task introduces them.

### Current available boundary

Phase 4 provides Platform-owned published-only public news display with deterministic pagination and escaped plain-text rendering.

Phase 6 adds:

- news create/update behind `cms.news.manage`;
- Platform-owned managed-page persistence;
- published-only managed-page public reads;
- managed-page create/update behind `cms.pages.manage`;
- authenticated confirmed-MFA administrator context for every privileged CMS route;
- audit append in the same Platform transaction as CMS state mutation where practical;
- plain-text authoring and escaped public output only.

Rich HTML, media uploads and arbitrary plugin/code upload remain out of scope and are not implied by the current CMS module.

### Security

- output escaped by default;
- rich text, if introduced, requires maintained allowlist sanitization;
- uploads require explicit MIME/content/size/storage controls;
- privileged mutation requires explicit Admin/RBAC authorization, confirmed MFA and audit.

## EditorialMedia

### Responsibilities

- secure ingestion and normalization of approved editorial raster images;
- private immutable object naming and integrity metadata;
- bounded alternative text and administrator lifecycle;
- reusable reference tracking for explicitly known editorial consumers.

### Current implementing boundary

The safe editorial media task provides:

- Platform-owned media and reference records;
- JPEG, PNG and WebP only with extension, fileinfo MIME and image-header agreement;
- configured byte, dimension and decoded-pixel limits before full decode;
- GD decode and same-format re-encode, removing source metadata and appended payloads;
- SHA-256, immutable random private storage names and private administrator serving;
- thumbnails only for images above the administrator-preview boundary;
- exact `media.manage` authorization behind authentication and confirmed MFA;
- bounded upload/deletion audit metadata;
- known `cms`, `events` and `wiki` reference slots with application locking and database-restricted deletion.

No public media route or Wiki, Events or CMS consumer integration is activated by this slice.

### Invariants

- original untrusted upload bytes are never retained after successful normalization;
- SVG, active content, executable files, archives and arbitrary documents are never accepted;
- storage and codec failures fail closed;
- objects remain outside the public storage symlink;
- media storage names are server-generated, random and immutable;
- referenced media cannot be deleted through either the application path or direct database deletion;
- future consumers own their publication and authorization policy and must use the bounded reference manager.

## Wiki

### Responsibilities

- language-independent article and category identity;
- localized article and category content with per-locale slugs;
- deterministic editorial lifecycle;
- optimistic locking for concurrent edits;
- append-only localized revisions and restore-as-new-revision behavior;
- later public reads, safe Markdown rendering, search, redirects and media through separately reviewed slices.

### Current implementing boundary

The Wiki foundation task provides:

- Platform-owned additive migrations for articles, translations, categories, article-category relations and revisions;
- exact supported locales `en` and `pl`;
- unique localized article and category slugs;
- draft, review, published and archived lifecycle rules;
- explicit stale-edit failure through monotonic lock versions;
- content revisions appended on create, update and restore;
- restore by creating a new revision that references the selected source revision;
- publication only when complete English and Polish translations exist;
- exact reserved Wiki permissions with no wildcard and no automatic role grants;
- bounded administrator audit metadata without complete article bodies;
- restricted Markdown source persistence with no raw HTML;
- focused domain, migration, database and authorization tests.

No public Wiki route, navigation contribution, renderer, search service, media upload, comments or player editing is activated by this foundation.

### Invariants

- Wiki persistence is Platform-owned and does not modify Canary/login-server data;
- missing or unsupported locale fails explicitly;
- localized slugs are unique by `(locale, slug)`;
- stale updates never silently overwrite newer content;
- revisions are append-only through supported application/model paths;
- privileged application operations use one exact Wiki permission;
- future HTTP administration must combine `auth`, `mfa.confirmed` and that exact permission;
- article bodies and category descriptions are excluded from audit metadata;
- public activation remains a separately reviewed later slice.

## Support and Moderation

### Responsibilities

- authenticated owner-scoped support tickets and public messages;
- bounded player, content and guild reports;
- exact-permission moderator queues and private notes;
- Platform-owned account-visible enforcement, acknowledgement and appeal state;
- deterministic notification delivery records;
- configurable retention, pruning/anonymization and privacy-safe audit metadata.

### Current available boundary

The delivered lifecycle provides server-generated public identifiers, idempotent request keys, owner-scoped reads, optimistic locking, explicit report transitions, exact `support.tickets.manage`, `support.reports.manage` and `support.enforcement.manage` permissions behind confirmed MFA, EN/PL desktop/tablet/mobile UI and isolated mail-delivery failure state.

Support attachments are disabled. Canary remains authoritative for native game bans and account status; no support action writes Canary data.

### Invariants

- browser-supplied Identity, reporter, owner or target identifiers never establish authorization;
- user views never expose reporter identity, another user's records, moderator-private notes or internal audit data;
- privileged mutations require authentication, confirmed MFA, one exact permission, row locking/version checks and bounded audit metadata;
- notification failure never rolls back a committed support transition;
- raw ticket bodies, report evidence, appeal bodies and moderator notes do not enter audit metadata;
- pruning/anonymization follows configured Platform retention and never deletes Canary-owned data;
- file attachments and Canary ban mutation require separate reviewed contracts.

## Admin

### Responsibilities

- administration UI;
- RBAC/policies;
- security-sensitive Platform actions approved by product policy;
- CMS administration;
- operational visibility safe for the assigned role.

### Current available boundary

Phase 6 merged through PRs #44 and #45 and provides:

- durable explicit role, permission, role-permission and Identity-role assignment persistence;
- no administrator assignment by default;
- explicit current permissions with no wildcard authorization shortcut;
- reusable deny-by-default `admin.permission` middleware;
- mandatory composition of `auth`, `mfa.confirmed` and an exact permission on privileged routes;
- one-time console-only first-admin bootstrap requiring confirmed MFA and no prior administrator assignment;
- explicit `content_editor`, `security_admin` and `platform_admin` role bundles governed by ADR 0006;
- audited transactional role assignment/removal behind `admin.roles.manage`;
- supported-path protection against removing the final `platform_admin`;
- permission-scoped CMS administration;
- permission-scoped bounded audit visibility;
- Character Bazaar wallet inspection/adjustment and bounded saga recovery behind exact `marketplace.manage`;
- optional Cloudflare Access deployment guidance as defense in depth.

### Invariants

- deny by default;
- no implicit "admin can do everything" shortcut;
- `platform_admin` is an explicit current permission bundle, not a wildcard for future permissions; new migration-owned permissions are assigned explicitly;
- privileged state changes are audited where delivered by Phase 6 or the owning module;
- no arbitrary PHP/code/plugin execution feature;
- admin web access combines explicit authorization with confirmed MFA and may additionally use Cloudflare Access in production.

## Audit

### Responsibilities

- append-oriented security events;
- administrator action audit;
- authentication anomalies and important account security events;
- actor/target references without secrets.

### Current available boundary

Identity security events remain append-oriented security primitives. The account-security lifecycle adds bounded events for email-change request/confirmation/cancellation/recovery, session revocation, privacy changes, recovery-key generation/revocation/use and termination request/cancellation/finalization.

Phase 6 adds:

- dedicated append-oriented administrator audit storage;
- audit events for first-admin bootstrap, role assignment/removal and privileged CMS create/update operations;
- minimal actor/action/target/non-secret metadata records;
- bounded 50-row-per-page administrator audit visibility behind `audit.view`, authentication and confirmed MFA.

The Wiki foundation reuses the same recorder for bounded article lifecycle, content/revision and category events. Complete article bodies, complete category descriptions and change-note text are excluded from audit metadata.

Character Bazaar records administrator wallet adjustment and recovery actions. Wallet mutation and its administrator audit row commit in the same Platform transaction; unrestricted reason text and character snapshot bodies are excluded from audit metadata.

Audit storage is not a replacement for infrastructure/application logs and must never contain raw credentials, session/reset/email tokens, complete registered-session identifiers, MFA secrets, raw recovery keys or complete source addresses.

## Integration

### Responsibilities

- explicit interfaces to Canary/login-server/shared schema;
- mapping/translation between Platform domain models and external schema;
- compatibility assertions;
- operation-specific least-privilege database adapters;
- integration tests/fixtures based on verified contracts.

### Current available boundary

Implemented adapters include:

- read-only Canary SQL game-data access;
- read-only Canary runtime Redis access;
- greenfield account provisioning through `canary_provisioning`;
- greenfield character creation through `canary_character_create`;
- Character Bazaar snapshot/ownership/transfer through `canary_character_transfer`, restricted to approved reads and `UPDATE (account_id)` only.

The authoritative Platform game-login bridge remains a separate cross-repository integration task and is not implied by the module being `AVAILABLE`.

### Invariants

- external schema assumptions documented in `docs/contracts/**`;
- no hidden shared-table usage outside agreed read/operation boundaries;
- generic Canary access remains deny-by-default for mutations;
- every write adapter has an effective-grant verifier and reviewed provisioning template;
- breaking changes require contract updates and cross-repository coordination;
- external repository changes require explicit authorization.

## Wallet

### Responsibilities

- one Oteryn Coins wallet projection per Platform Identity;
- available and reserved balance invariants;
- append-oriented signed-delta financial history;
- deterministic idempotency for every reservation, release, settlement and administrator adjustment.

### Current implementing boundary

Character Bazaar v1 provides:

- `wallet_accounts` transactionally maintained available/reserved balances;
- `wallet_ledger_entries` as the immutable mutation trace;
- race-safe wallet creation and row locking;
- leading-bid reservation and exactly-once outbid release;
- exactly-once buyer reserved debit and seller proceeds/commission settlement;
- signed administrator adjustment with bounded reason hash and same-transaction audit.

Initial funding is administrator-controlled. Payment-provider purchase, refund and chargeback handling is not implemented.

### Invariants

- available and reserved balances never become negative;
- ledger idempotency keys are unique and checked inside the locked transaction;
- no routine direct update of balances outside `WalletMutator`;
- Oteryn Coins are not Canary account coins, transferable coins, tournament coins or character bank gold;
- payment-provider records, if later approved, remain a separate Payments responsibility.

## Marketplace

### Responsibilities

- public Character Bazaar catalogue, filters, deterministic sorting and immutable auction detail;
- authenticated watchlist, bids, owned listings and history;
- listing eligibility and escrow activation policy;
- direct ascending bid and optional fixed-price purchase policy;
- cross-database cancellation/settlement saga and deterministic recovery;
- bounded administrator recovery interface and operations documentation.

### Current implementing boundary

The Character Bazaar task provides:

- Platform-owned auction, bid and watch persistence;
- immutable public-safe snapshot captured before escrow;
- non-login Canary escrow account deployment boundary;
- delayed activation after ownership/session quiescence confirmation;
- auction-row and wallet-row locking for direct bids;
- seller self-bid denial and minimum start/increment rules;
- fixed-price purchase through the same reservation and settlement invariants;
- seller cancellation only before any bid and automatic no-bid expiry return;
- idempotent escrow-to-winner transfer followed by Platform wallet settlement;
- `recovery_required` rather than fabricated completion after ambiguous failure;
- EN/PL public/account/admin surfaces with desktop/tablet/mobile accessibility coverage;
- explicit production activation flag and fail-closed deployment verifier.

### Invariants

- only an authenticated ready binding may list or bid;
- browser-supplied Canary account IDs never authorize ownership;
- active auctions are backed by an escrow-owned, active character with no cluster session;
- the current leading bid alone remains reserved;
- an auction reaches `completed` only after actual winner ownership and wallet ledger settlement are reconciled;
- public output never exposes account IDs, sessions, IPs, credentials or raw integration errors;
- disabling the production feature flag removes routes/navigation/sitemap exposure while preserving data and reconciliation capability;
- payment-provider commerce is not implied by Character Bazaar v1.

## Notifications

### Current available boundary

Implemented email use cases include:

- password reset;
- new-address confirmation for primary-email changes;
- old-address cancellation/recovery notice for primary-email changes;
- English and Polish account-security subjects, copy, actions and locale-preserving token links.

Mail delivery should be asynchronous once queue infrastructure exists. Security tokens and locale selection remain owned by Identity use cases; Notifications formats and transports messages but does not validate tokens, authorize account changes or own Marketplace state.

Future marketplace outbid/completion notifications require an explicitly reviewed asynchronous delivery slice.

## PlatformAPI

Expose API endpoints only for a concrete client/use case. API endpoints must reuse module services/policies and must not implement a second business-rule path.

## Payments — deferred

No payment-provider implementation belongs in the current core platform or Character Bazaar v1 scope.

Future requirements include provider abstraction, signed webhook verification, provider-event idempotency, immutable provider/financial ledgers, reconciliation, refunds/chargebacks, tax/legal review and explicit separation from Oteryn Coins gameplay policy.

Payments must not become a dependency of basic Identity/account creation/login, and must not reuse Canary mutable coin fields as the provider settlement source of truth.

## PublicGameData community completeness

### Complete community read boundary

ADR 0018 and `docs/contracts/PUBLIC_COMMUNITY_DATA_CONTRACT.md` extend the available read-only boundary with:

- allowlisted level, experience, magic and skill highscores with vocation filtering and explicit global scope;
- privacy-aware character profiles containing approved comment, skills, guild/rank, house, deaths and kill statistics;
- related-account characters and status timestamps only when Platform Identity privacy flags permit disclosure;
- latest-death pagination and guild directory search/detail with deterministic empty, not-found and unavailable states;
- direct-table `SELECT` grants for `houses` and `player_deaths`, with no Canary write capability.

Guild administration, world/channel transfer history, selectable achievements, polls and public enforcement publication are not owned by this boundary. They require separate authoritative product, ownership and security contracts.

### Community invariants

- browser input never establishes account association, status visibility or guild authority;
- highscore columns are selected only from a server-side allowlist;
- characters are ranked globally because the authoritative schema stores no per-channel character ownership;
- public output excludes Identity email/IDs, Canary account IDs, raw death participants, house coordinates, runtime leases and moderator data;
- dependency failure returns localized sanitized `503`, never fabricated empty data;
- collections use bounded limits, deterministic ordering and documented index expectations.
