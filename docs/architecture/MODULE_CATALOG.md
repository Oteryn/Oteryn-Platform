# Oteryn Platform Module Catalog

This catalog defines module responsibilities and dependency boundaries.

## Status legend

- `PLANNED` — architecture decision only; no implementation proven.
- `DISCOVERY` — contract/research work required before a concrete capability can be implemented.
- `IMPLEMENTING` — active source implementation exists in an active task.
- `AVAILABLE` — at least one explicitly documented capability is implemented and validated on `main`; this does not imply every conceivable operation in the module exists.

| Module | Status | Owns | Must not own |
|---|---|---|---|
| Identity | AVAILABLE | Platform web authentication policy, credentials lifecycle, sessions, MFA, recovery | Payments, game runtime, arbitrary character mutations |
| Accounts | AVAILABLE | Greenfield account provisioning/binding and future explicitly contracted account-level operations | Canary password verification logic, undocumented shared writes, game runtime |
| Characters | AVAILABLE | Contract-approved web-triggered character operations; currently create | Direct undocumented Canary writes; uncontracted rename/delete |
| PublicGameData | AVAILABLE | Read models/queries for characters, guilds, highscores, online/status | Privileged mutations |
| CMS | AVAILABLE | Public content reads and permission-scoped Platform content management | Identity policy, game state, rich/upload surfaces without explicit security controls |
| Wiki | IMPLEMENTING | Localized Wiki articles, categories, lifecycle, optimistic locking and append-only revisions | Generic CMS pages, public activation before release criteria, arbitrary HTML, media/search without separate reviewed slices |
| Admin | AVAILABLE | Admin UI, explicit RBAC/policies, privileged Platform use cases | Bypassing domain/application invariants or granting implicit wildcard authority |
| Audit | AVAILABLE | Security/admin audit primitives, privileged-action audit and bounded admin audit visibility | Secrets, raw credentials, business-rule authorization decisions |
| Integration | AVAILABLE | Implemented Canary read/write adapters, schema translation, contract enforcement; future login bridge remains separate | Product policy that belongs in domain modules |
| Notifications | PLANNED | Email and asynchronous user notifications | Core auth decisions, payment settlement |
| PlatformAPI | PLANNED | Stable first-party API endpoints and API-specific auth/limits | Duplicating business logic from modules |
| Payments | PLANNED-LATER | Provider adapters, payments, webhook handling, ledger/coins/shop when approved | Identity core, direct dependency from basic account creation/login |

## Identity

### Responsibilities

- login/logout;
- Platform credential hashing and lifecycle;
- session creation, rotation and revocation;
- password reset/change;
- email verification if later enabled by product policy;
- MFA/TOTP and recovery codes;
- authentication rate limiting;
- security-sensitive Identity audit events.

### Current available boundary

The Platform web Identity authority provides registration, framework-hashed credentials, login/logout, revocable web sessions, password recovery/change, rate limiting, security-event recording and opt-in web MFA.

Phase 5 makes Platform Identity the ownership authority for supported greenfield game accounts, but it does not mean native Canary/external login-server game authentication has already been replaced by Platform authorization.

### Invariants

- one authoritative Platform Identity policy for supported product users;
- user credentials never stored reversibly;
- security-sensitive changes may revoke Platform web sessions;
- privileged/Admin routes combine authentication, explicit authorization and `mfa.confirmed`;
- MFA never grants authorization by itself;
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

Existing Canary account claim/import, account deletion, unlink/rebind/transfer and broader account profile mutations are not implied by `AVAILABLE` and require separate contracts.

### Invariants

- every account mutation requires authenticated/authorized Platform context;
- browser-supplied account IDs are never ownership proof;
- ready immutable binding is the trusted source for user-scoped Canary authorization;
- generic `canary` remains read-only;
- account write capability is operation-specific and least-privileged;
- Platform does not duplicate Canary reusable-password verification.

## Characters

### Responsibilities

- web-triggered character lifecycle operations explicitly approved by product/Canary contracts.

### Current available boundary

Phase 5 implements **character creation only**:

- authorization through the authenticated Identity's ready immutable Canary account binding;
- ADR 0005 canonical-name, vocation/sex, starter-state and quota policy;
- dedicated least-privilege `canary_character_create` adapter;
- account-row locking, same-name idempotent recovery, maximum-10-active-character enforcement and unique-name race handling;
- real MariaDB privilege/concurrency coverage.

Character deletion/soft deletion and rename are not implemented or authorized. They require separate operation contracts and do not inherit create privileges.

### Invariants

- character ownership is resolved server-side from the ready binding;
- client-controlled account IDs cannot establish ownership;
- names and vocation/sex choices follow verified product policy;
- concurrency-sensitive writes are transactional;
- no raw undocumented shared-table mutation;
- each new mutation operation gets its own contract and least-privilege boundary.

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

### Invariants

- no privileged mutations;
- private account/session/security fields are never public output;
- dependency failure is explicit, not fabricated empty/offline state;
- freshness boundaries are not extended by unbounded caching.

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
- optional Cloudflare Access deployment guidance as defense in depth.

### Invariants

- deny by default;
- no implicit "admin can do everything" shortcut;
- `platform_admin` is an explicit current permission bundle, not a wildcard for future permissions;
- privileged state changes are audited where delivered by Phase 6;
- no arbitrary PHP/code/plugin execution feature;
- admin web access combines explicit authorization with confirmed MFA and may additionally use Cloudflare Access in production.

## Audit

### Responsibilities

- append-oriented security events;
- administrator action audit;
- authentication anomalies and important account security events;
- actor/target references without secrets.

### Current available boundary

Identity security events remain append-oriented security primitives.

Phase 6 adds:

- dedicated append-oriented administrator audit storage;
- audit events for first-admin bootstrap, role assignment/removal and privileged CMS create/update operations;
- minimal actor/action/target/non-secret metadata records;
- bounded 50-row-per-page administrator audit visibility behind `audit.view`, authentication and confirmed MFA.

The Wiki foundation reuses the same recorder for bounded article lifecycle, content/revision and category events. Complete article bodies, complete category descriptions and change-note text are excluded from audit metadata.

Audit storage is not a replacement for infrastructure/application logs and must never contain raw credentials, session/reset tokens or MFA secrets.

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
- greenfield character creation through `canary_character_create`.

The authoritative Platform game-login bridge remains a separate cross-repository integration task and is not implied by the module being `AVAILABLE`.

### Invariants

- external schema assumptions documented in `docs/contracts/**`;
- no hidden shared-table usage outside agreed read/operation boundaries;
- generic Canary access remains deny-by-default for mutations;
- breaking changes require contract updates and cross-repository coordination;
- external repository changes require explicit authorization.

## Notifications

Initial use cases:

- email verification if later enabled;
- password reset;
- security alerts.

Mail delivery should be asynchronous once queue infrastructure exists, while security tokens remain owned by Identity use cases.

## PlatformAPI

Expose API endpoints only for a concrete client/use case. API endpoints must reuse module services/policies and must not implement a second business-rule path.

## Payments — deferred

No payment implementation belongs in the current core platform scope.

Future requirements include provider abstraction, signed webhook verification, idempotency, immutable ledger, reconciliation, refunds/chargebacks and a dedicated threat model/security review.

## Adding or expanding a module

Before adding a new module or a new shared-write operation:

1. prove the responsibility cannot be owned cleanly by an existing boundary;
2. document responsibility/dependencies here when architectural state changes;
3. create/update an ADR for durable product/architecture choices when material;
4. establish task ownership and explicit contracts;
5. apply least privilege and required authorization/concurrency tests;
6. avoid cyclic dependencies and undocumented cross-repository coupling.
