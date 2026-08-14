# Oteryn Platform Module Catalog

This catalog defines module responsibilities and dependency boundaries.

## Status and evidence model

Module status describes **repository implementation availability only**:

- `PLANNED` — an ownership boundary is accepted, but no implementation capability is proven on `main`.
- `PLANNED-LATER` — an accepted boundary is intentionally outside the current delivery horizon or activation programme.
- `DISCOVERY` — contract/research work is required before a concrete capability can be implemented safely.
- `IMPLEMENTING` — source implementation exists only in active, unmerged work.
- `AVAILABLE` — at least one explicitly documented capability is merged and validated on `main`; this does not imply every operation in the module exists.

Do not infer capability completeness, staging proof, production proof or activation authority from this status. Those are separate evidence dimensions:

- **capability completeness** requires an accepted expected inventory and exact gap closure;
- **staging/production evidence** requires an exact environment/deployment identity;
- **activation authority** requires the applicable product, security, legal and operational gates.

The frozen PR #453 production-completion baseline and later exact merged PRs provide evidence inputs. Open Issues #365, #488, #489 and #490 retain current completeness, failure-path and environment gaps. An open gap inside a module does not mean the bounded module is absent.

| Module | Status | Owns | Must not own |
|---|---|---|---|
| Identity | AVAILABLE | Platform web authentication policy, credentials lifecycle, registered sessions, MFA, confirmed email changes, privacy, recovery and Platform termination | Payments, game runtime, arbitrary Canary account or character mutations |
| Accounts | AVAILABLE | Greenfield account provisioning/binding and future explicitly contracted account-level operations | Canary password verification logic, undocumented shared writes, game runtime |
| Characters | AVAILABLE | Contract-approved web-triggered character operations; currently create plus Character Bazaar ownership transfer | Direct undocumented Canary writes; uncontracted rename/delete |
| CharacterProfiles | AVAILABLE | Platform-stored character presentation/privacy preferences: public comment, per-character field visibility and optional main-character selection, with server-side owner verification and projection into PublicGameData | Authoritative character identity/current ownership, gameplay state, Canary/Oteryn-v2 mutation authority, or claiming canonical CharacterId migration completion |
| PublicPortal | AVAILABLE | Homepage/shared-shell composition, SEO/discoverability, federated public content-search orchestration and authorized view models, including future Today/command-centre and World Hub composition over bounded source modules | Authentication policy, raw Canary queries, arbitrary CMS persistence, source-module publication/search rules, module business rules, runtime/game-data truth, or routing/admission authority |
| Announcements | AVAILABLE | Typed scheduled public announcements and permission-scoped publication lifecycle | Authoritative runtime world/service state or unrestricted site settings |
| Events | AVAILABLE | Localized editorial event schedule/detail and audited administration lifecycle | Runtime raid/service truth unless consumed from an explicit LiveOps contract |
| Downloads | AVAILABLE | Approved client release/artifact metadata, platform variants, checksums and publication lifecycle | Arbitrary executable upload/proxy, release signing authority or client updater runtime without a contract |
| PublicGameData | AVAILABLE | Read models/queries for characters, guilds, highscores, online/status | Privileged mutations |
| LiveOps | PLANNED | Authoritative time-sensitive world/service status, maintenance, server save, raid schedules, runtime events/boost freshness and service history | Free-form editorial content, gameplay mutation or fabricated offline/zero state |
| CMS | AVAILABLE | Public content reads and permission-scoped Platform content management | Identity policy, game state, unreviewed rich/upload surfaces |
| Support | AVAILABLE | Platform tickets, reports, moderation decisions, enforcement orchestration/records, appeals, notifications, retention and privacy-safe user/moderator presentation | Authoritative game sanction state/effect, direct game-ban mutation, file attachments, disclosure of reporter identity or private moderator notes |
| EditorialMedia | AVAILABLE | Private normalized raster-image objects, integrity metadata, bounded consumer references and administrator lifecycle | Generic public file hosting, executable uploads, arbitrary documents, consumer-specific publication rules |
| Wiki | AVAILABLE | Localized Wiki public reads/search, categories, lifecycle, trusted administration, revisions and approved media references | Generic CMS pages, arbitrary HTML, player editing, claims of complete authoritative game content |
| GameCatalog | AVAILABLE | Versioned deterministic game-catalogue snapshots/projections, exact authority provenance and, where authoritative, stable typed server-specific system definitions with version/profile/ruleset/season applicability | Executable native game-content authority, inventing missing content, editorial strategy, current runtime/rotation truth, silent producer assumptions, or production activation without a gate |
| PlayerCompanion | AVAILABLE | Versioned calculators, build plans, hunt guidance, session analysis, progression goals, owner-private tracking/routines/change signals, shareable plans and explained recommendations | Canonical game/source facts, raw Canary access, notification transport, public follower graphs, game mutation, payment settlement or hidden balance policy |
| Admin | AVAILABLE | Admin UI, explicit RBAC/policies, privileged Platform use cases | Bypassing domain/application invariants or granting implicit wildcard authority |
| Audit | AVAILABLE | Security/admin audit primitives, privileged-action audit and bounded admin audit visibility | Secrets, raw credentials, business-rule authorization decisions |
| Integration | AVAILABLE | Implemented Canary read/write adapters, schema translation and contract enforcement; the bounded Identity -> Game Gateway -> private Platform login-context -> Game Session v1 bridge is merged, while native v2 and production cutover remain separate gated work | Product policy that belongs in domain modules, direct credential policy duplication or production-activation claims without exact evidence |
| Wallet | AVAILABLE | Oteryn Coins available/reserved projection and append-oriented idempotent ledger | Canary coins, payment-provider settlement, arbitrary balance edits |
| Marketplace | AVAILABLE | Character Bazaar listings, escrow saga, bids, watches, settlement, history and recovery policy | Canary gameplay state, generic character mutation, payment-provider commerce |
| Notifications | AVAILABLE | Formatting/transport and deterministic delivery state for already-authorized notifications, including existing account-security/support mail | Domain subscription/tracking ownership, source-change/threshold decisions, core auth decisions, token validation, domain rollback on mail failure or payment settlement |
| OperationsObservability | AVAILABLE | Repository/runtime health, release identity, structured observability, queue/mail/cache/session operational contracts, backup/restore/rollback expectations | Business policy, production-proof claims without exact environment evidence |
| PublicEdge | AVAILABLE | Repository-owned DNS/TLS/redirect/HSTS/WAF/tunnel/origin/private-ingress contracts and verification tooling | Application authorization, implicit live-state truth, production mutation without authority |
| QualityE2E | AVAILABLE | Capability/route ledgers, deterministic validators, exact-head browser/integration evidence and delivery-state classification | Product policy, weakening acceptance to obtain green CI, treating staging evidence as production proof |
| PlatformAPI | PLANNED | Stable first-party API endpoints and API-specific auth/limits | Duplicating business logic from modules or misclassifying bounded internal/game-auth endpoints as a general API |
| ProductsEntitlements | PLANNED | Product catalogue, premium/VIP/package/voucher entitlements, fulfilment, expiry and revocation | Provider payment settlement, Wallet gameplay policy, undocumented Canary premium mutations |
| LegalCommerce | PLANNED | Commerce-specific consumer presentation, payment-data privacy, retention, refund/complaint and invoice/tax decision boundaries | Generic CMS publishing, payment settlement, silent legal assumptions |
| Payments | PLANNED-LATER | Provider adapters, payments, signed webhooks, reconciliation, refunds/chargebacks and regulated commerce when approved | Identity core, basic account/login dependencies, Oteryn Coins gameplay marketplace policy, product fulfilment |

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

## PublicPortal

### Responsibilities

- production homepage composition;
- shared public header, navigation, footer and route discoverability;
- public view models that combine already-authorized data from bounded modules;
- federated public content-search orchestration, normalized result envelope and deterministic cross-source grouping/interleaving over bounded source-module public queries;
- future `Today` / command-centre composition over editorial, LiveOps, PublicGameData and authenticated PlayerCompanion signals;
- future World Hub composition over public world identity/presentation, PublicGameData, LiveOps and optional approved analytics;
- sitemap/robots and public SEO orchestration;
- explicit empty, stale, unavailable, partial and not-found presentation.

### Current available boundary

The merged public shell provides the production homepage, public navigation and SEO endpoints. The homepage composes published editorial content and bounded public game/runtime projections without making Blade templates a raw data-access layer.

`Today`/command-centre and World Hub are accepted future composition directions under ADR 0032; federated public content search is an accepted future application capability under ADR 0033. None is claimed implemented merely because `PublicPortal` is `AVAILABLE`.

Current compatibility debt is explicit: `Announcements` and `Events` homepage provider/view-model paths still import `App\PublicPortal\PublicContentState`. That existing reverse edge predates ADR 0033 and is not the accepted dependency direction for federated-search provider onboarding.

### Invariants

- PublicPortal is presentation/orchestration, not a generic source-of-truth domain module;
- controllers and templates do not create raw Canary or cross-module persistence/model access paths;
- source modules retain public/search eligibility, publication, localization, source-local relevance and canonical source identity/URL semantics;
- the target federated-search provider edge is `PublicPortal -> source-module application query`; before Announcements or Events are onboarded as federated-search providers, their existing `PublicContentState` reverse imports must be removed or replaced by source-owned response/availability types that PublicPortal maps into its composition state;
- source modules must not depend on PublicPortal search contracts, search result types or presentation/view types;
- raw relevance scores from heterogeneous providers are not assumed globally comparable; cross-source grouping/interleaving policy is deterministic and versionable;
- PublicGameData exact-name character search remains a distinct privacy/enumeration product and is not silently broadened into fuzzy federated people search;
- a dedicated search index, if later adopted, is rebuildable derived state and never source truth;
- private PlayerCompanion, Support, Admin, Audit, Identity or Accounts records do not enter public federation without a separate explicit public contract;
- a failed dependency does not fabricate `0 online`, `offline`, no news, no event, no search results, no change or completed state;
- composed values preserve source applicability, freshness, confidence, publication and privacy semantics;
- authenticated PlayerCompanion signals remain owner-private and are omitted for unauthorized/guest contexts;
- Today/World Hub never become world-routing, runtime-readiness, admission or gameplay authority;
- links and sitemap entries activate only for real published/enabled routes;
- authentication and account policy remain owned by Identity/Accounts.

## Announcements

### Responsibilities

- typed time-bounded public notices;
- localized title/body and severity;
- draft/publication scheduling;
- trusted internal or allowlisted external action links;
- exact-permission administration and audit.

### Current available boundary

The merged announcements module provides public active-announcement projection and permission-scoped administration. It is editorial communication, not proof of runtime world or service status.

### Invariants

- only active published records are public;
- time boundaries are deterministic;
- links are validated and cannot become open redirects or active-content URLs;
- runtime maintenance/offline truth comes from LiveOps when that boundary exists;
- privileged mutations require authentication, confirmed MFA, exact permission and audit.

## Events

### Responsibilities

- localized editorial event records and slugs;
- draft, scheduled, active, completed and cancelled lifecycle;
- time-zone normalization and deterministic public visibility;
- optional approved links, news/media associations and homepage previews;
- exact-permission administration and audit.

### Current available boundary

The merged module exposes public event list/detail routes and separate administration/publishing permission gates.

### Invariants

- unpublished and cancelled state follows explicit visibility policy;
- event schedules are not inferred from free-form news;
- runtime raids, world events and service windows require a LiveOps source when presented as authoritative current state;
- consumer-specific EditorialMedia integration remains reviewed and bounded;
- time-zone behavior is tested at boundaries.

## Downloads

### Responsibilities

- supported client release records and channels;
- platform/architecture artifact metadata;
- immutable approved artifact references;
- version, size, checksum and release-note presentation;
- publication and withdrawal lifecycle;
- future minimum-version, mandatory-update and provenance/signature policy.

### Current available boundary

The merged Download Center provides a public platform-filtered route and permission-scoped create/edit/publish administration for approved release metadata.

### Invariants

- no arbitrary executable upload, proxy or user-supplied artifact URL;
- allowed artifact hosts and immutable references are validated;
- public output shows supplied version/platform/size/checksum facts without claiming a publisher signature that is not proven;
- a checksum provides integrity only after the expected value is trusted and does not by itself establish origin;
- release signing, updater manifests and automatic client updates require separate contracts/threat modelling.

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
- Character Bazaar snapshots exclude Canary account IDs, IPs, sessions, credentials and arbitrary blobs;
- exact-name character lookup retains its existing privacy/enumeration semantics and is not implicitly widened by PublicPortal federated content search.

## LiveOps — planned

### Responsibilities

- authoritative current service/world status and freshness;
- maintenance and server-save schedules;
- service-status and maintenance history;
- raid/boss/runtime-event schedules when an authoritative producer exists;
- boosted creature/boss or other rotating runtime projections when their exact source and semantics are contracted;
- current schedule/rotation/season-runtime state for typed server-specific systems when an authoritative producer exists;
- explicit unavailable/stale behavior and operational ownership.

### Invariants

- CMS, Announcements, Events and Wiki may explain or present state but cannot originate authoritative runtime truth;
- stable deterministic system definitions belong to GameCatalog; LiveOps owns only current operational/schedule/runtime state;
- zero players, offline, maintenance, stale and unavailable are distinct states;
- every projection declares producer, observation time, TTL and failover behavior;
- no gameplay mutation is implied;
- multi-world/season identity is explicit;
- production claims require exact environment evidence.

## CMS

### Responsibilities

- news/articles;
- managed pages;
- publication state;
- consumer-specific media references only through an explicitly reviewed EditorialMedia integration.

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

Rich HTML, a generic CMS media consumer and arbitrary plugin/code upload remain out of scope and are not implied by the current CMS module.

### Security

- output escaped by default;
- rich text, if introduced, requires maintained allowlist sanitization;
- media use must go through the private normalized EditorialMedia boundary and consumer-specific publication rules;
- privileged mutation requires explicit Admin/RBAC authorization, confirmed MFA and audit.

## EditorialMedia

### Responsibilities

- secure ingestion and normalization of approved editorial raster images;
- private immutable object naming and integrity metadata;
- bounded alternative text and administrator lifecycle;
- reusable reference tracking for explicitly known editorial consumers.

### Current available boundary

The merged safe editorial media library provides:

- Platform-owned media and reference records;
- JPEG, PNG and WebP only with extension, fileinfo MIME and image-header agreement;
- configured byte, dimension and decoded-pixel limits before full decode;
- GD decode and same-format re-encode, removing source metadata and appended payloads;
- SHA-256, immutable random private storage names and private administrator serving;
- thumbnails only for images above the administrator-preview boundary;
- exact `media.manage` authorization behind authentication and confirmed MFA;
- bounded upload/deletion audit metadata;
- known `cms`, `events` and `wiki` reference slots with application locking and database-restricted deletion.

The merged Wiki consumer permits exact authorized editors to discover and insert approved media, synchronizes translation-scoped references transactionally and exposes public bytes only through an effective published translation that still references the object. Signed administrator previews remain scoped and short-lived.

There is no generic public media-file service. CMS and Events still require their own reviewed consumer integration before using EditorialMedia. Issue #488 retains expected-content, failure/recovery and portability gaps without invalidating the available library/consumer boundary.

### Invariants

- original untrusted upload bytes are never retained after successful normalization;
- SVG, active content, executable files, archives and arbitrary documents are never accepted;
- storage and codec failures fail closed;
- objects remain outside the public storage symlink;
- media storage names are server-generated, random and immutable;
- referenced media cannot be deleted through either the application path or direct database deletion;
- consumers own publication and authorization policy and must use the bounded reference manager.

## Wiki

### Responsibilities

- language-independent article and category identity;
- localized article and category content with per-locale slugs;
- deterministic editorial lifecycle;
- optimistic locking for concurrent edits;
- append-only localized revisions and restore-as-new-revision behavior;
- public published-only reads, restricted Markdown rendering, category navigation and locale-isolated search;
- trusted administration and approved EditorialMedia references;
- explanatory/editorial guides and player-facing strategy for server-specific systems without impersonating deterministic or current runtime truth.

### Current available boundary

The merged Wiki boundary provides:

- Platform-owned articles, translations, categories, article-category relations and revisions;
- exact supported locales `en` and `pl` with unique localized slugs;
- draft, review, published and archived lifecycle rules;
- explicit stale-edit failure through monotonic lock versions;
- content revisions appended on create, update and restore;
- publication only when complete English and Polish translations exist;
- exact reserved Wiki permissions with no wildcard and no automatic role grants;
- restricted Markdown with raw HTML disabled;
- public Wiki home, category, article and locale-isolated search routes;
- canonical/hreflang, breadcrumb, related-content and bounded empty/not-found/unavailable presentation;
- trusted `auth` + `mfa.confirmed` administration for articles, categories, previews, lifecycle and revision restore;
- translation-scoped approved EditorialMedia insertion and public delivery authorization.

Player editing, comments, arbitrary HTML and a complete authoritative game-content inventory are not implied. Issue #365 retains a focused flash/thumbnail investigation; Issue #488 retains content-completeness and explicit failure/recovery/portability gaps.

### Invariants

- Wiki persistence is Platform-owned and does not modify Canary/login-server data;
- missing or unsupported locale fails explicitly;
- localized slugs are unique by `(locale, slug)`;
- stale updates never silently overwrite newer content;
- revisions are append-only through supported application/model paths;
- privileged operations combine `auth`, `mfa.confirmed` and exact Wiki permission;
- article bodies and category descriptions are excluded from audit metadata;
- public output is published-only, locale-scoped and safely rendered;
- Wiki prose cannot become authoritative deterministic GameCatalog configuration or current LiveOps state;
- media access requires an effective published translation reference and integrity verification.

## Game Catalog

ADR 0034 and `OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md` define the native target: the game domain owns canonical native content identity, executable definitions/applicability and authority revisions; `GameCatalog` owns validated immutable imported persistence, profiles, projections, presentation and transactional activation/rollback. Current Canary schemas/importers remain Legacy Canary Compatibility and do not define the native domain model. One profile cannot blend native, Canary and editorial fields into an apparently co-authoritative record.

### Responsibilities

- validate versioned deterministic Canary catalogue snapshots against pinned byte-identical schemas;
- import immutable snapshots transactionally and inactive by default;
- preserve release, completeness, availability and provenance facts without inventing missing evidence;
- activate and roll back only compatible snapshots with a concrete verified-content boundary;
- project public item, creature and loot visibility fail closed;
- own stable typed definitions for adopted server-specific systems when an authoritative structured source exists;
- preserve game-profile/ruleset/world-or-global/season/version/effective applicability and deterministic system parameters/entity relations where authoritative.

### Current available boundary

Schema `1.0.0` remains supported for retained imports and rollback. Schema `1.1.0` additionally represents an unknown `verified_content_through_release` as null. Schema `1.2.0` preserves exact Canary `canary_dynamic_threshold_v1` loot thresholds and declared roll maxima without presenting contextual thresholds as static probabilities. Unknown-boundary snapshots may be imported for review but cannot be activated or exposed publicly.

Open draft PR #338 contains an inactive schema `1.3.0` NPC-shop consumer and remains outside the current `main` boundary until its separate producer/compatibility gate is terminal. Issue #489 retains authoritative expected-inventory, capability, failure/recovery and portability gaps.

ADR 0032 assigns future typed server-specific system definitions to this module but does not claim that any such schema/importer is already implemented.

### Invariants

- schema bytes and hashes are pinned per supported version;
- null verified-content means unknown, never complete;
- unsupported schema versions, malformed snapshots and schema/hash mismatches fail closed;
- legacy rational loot probabilities and contextual runtime thresholds remain distinct persisted models;
- threshold values are never clamped or rendered as percentages;
- a server-specific system without an authoritative structured source remains unknown/editorial rather than fabricated deterministic truth;
- GameCatalog owns stable deterministic definition, not Wiki strategy prose or LiveOps current schedule/rotation/runtime state;
- import never activates automatically;
- activation failure preserves the prior active snapshot and projections;
- production import or activation requires a separate environment-gated task.

## PlayerCompanion — available

### Responsibilities

- deterministic calculators and formula executions;
- saved character/build/equipment plans;
- hunt discovery and explained ranking orchestration;
- private session-log analysis and loot-split calculations;
- quest/access/bestiary/bosstiary/goal tracking;
- owner-private tracked-entity/routine/subscription preferences and bounded derived change/progress signals;
- bounded shareable plans and sanitized summaries;
- clearly classified simulations and recommendations;
- ruleset/catalog/world/season freshness presentation.

The focused architecture is `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`; ADR 0025 owns the broad durable module decision and ADR 0032 owns the tracking/subscription responsibility refinement.

### Current available boundary

Hunt Session Analyzer v1 is the first implemented PlayerCompanion capability. It provides authenticated owner-private normalized session analysis, deterministic XP/profit rates and advisory equal-split settlement suggestions, parser/formula versioning, explicit applicability metadata, private history/detail/delete flows and EN/PL presentation. Raw submitted session text is intentionally not persisted. Other PlayerCompanion responsibilities listed above remain planned and are not implied by the module-level `AVAILABLE` status.

### Dependencies

- `GameCatalog` supplies canonical versioned entities, relations, formulas, stable typed system definitions and snapshot identity;
- `Wiki` supplies guides and editorial explanation;
- `PublicGameData` supplies public or separately authorized character/world projections;
- future `GameAnalytics` supplies measured aggregates, sample/confidence and balance evidence;
- `LiveOps` supplies time-sensitive world/service/system state;
- `Notifications` transports already-authorized notifications after PlayerCompanion has produced a domain signal/notification intent;
- `PlatformAPI` may later expose the same application services to approved clients.

### Invariants

- output is explicitly `DETERMINISTIC`, `SIMULATION` or `RECOMMENDATION`; a derived tracked-source signal remains a state-change/threshold observation over its source rather than a fourth calculation-certainty class;
- formulas and persisted results are pinned to game profile, ruleset, catalog snapshot, formula version and applicable world/season/effective dates;
- tracked signals additionally preserve source observation/revision and the rule revision used to derive the signal;
- stale, deprecated, experimental and unavailable data is visible and fails closed where correctness requires it;
- missing/stale source evidence never becomes a false “no change”, `offline`, completed or reset signal;
- browser-provided account or character identifiers never establish ownership;
- saved builds, goals, tracking preferences and raw/normalized session data are private by default;
- source facts remain owned by PublicGameData/LiveOps/GameAnalytics/GameCatalog or another accepted producer merely because a user follows them;
- `Notifications` owns transport/delivery state only and does not decide what is tracked or whether a source changed/threshold crossed;
- refresh/evaluation cadence is source-aware and bounded rather than one unbounded poller per user;
- public follower graphs/social tracking are not implied;
- shared representations are allowlisted, bounded, versioned, non-identifying and revocable where linked to private persisted state;
- raw session logs, private plans and private tracking lists/notification destinations never enter ordinary logs or audit metadata;
- domain formulas are not independently duplicated across Blade, JavaScript, API and clients;
- recommendations expose basis, confidence, freshness, explanation and limitations;
- no game, payment or economy mutation is implied by a calculation.

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

Support attachments are disabled. Current Canary bans/account status remain Legacy Canary Compatibility authority and no delivered support action writes game data. For the native target, `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md` makes Support an authorized decision/orchestration consumer while Oteryn-v2 owns sanction applicability, game mutation, active-session/runtime effect and authoritative receipts; that accepted semantic boundary is not runtime implementation or activation proof.

### Invariants

- browser-supplied Identity, reporter, owner or target identifiers never establish authorization;
- user views never expose reporter identity, another user's records, moderator-private notes or internal audit data;
- privileged mutations require authentication, confirmed MFA, one exact permission, row locking/version checks and bounded audit metadata;
- notification failure never rolls back a committed support transition;
- raw ticket bodies, report evidence, appeal bodies and moderator notes do not enter audit metadata;
- pruning/anonymization follows configured Platform retention and never deletes Canary-owned data;
- native game-effect claims require an authoritative current game result; dispatch, Platform workflow state and notification are insufficient;
- retries and ambiguous outcomes preserve one stable operation identity, while newer decision revisions fence stale apply/revoke/expire commands;
- appeal submission alone never mutates game state; an authorized appeal outcome creates a newer bounded decision;
- file attachments and any compatibility-mode Canary ban mutation require separate reviewed contracts.

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
- `platform_admin` is an explicit current permission bundle, not a wildcard for future permissions;
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

Phase 6 adds dedicated append-oriented administrator audit storage, bounded audit visibility and events for first-admin bootstrap, role assignment/removal and privileged CMS operations. Wiki and Marketplace reuse the recorder for bounded lifecycle, wallet adjustment and recovery actions.

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

### Current available boundary

Character Bazaar v1 provides:

- `wallet_accounts` transactionally maintained available/reserved balances;
- `wallet_ledger_entries` as the immutable mutation trace;
- race-safe wallet creation and row locking;
- leading-bid reservation and exactly-once outbid release;
- exactly-once buyer reserved debit and seller proceeds/commission settlement;
- signed administrator adjustment with bounded reason hash and same-transaction audit.

Initial funding is administrator-controlled. Payment-provider purchase, refund and chargeback handling is not implemented. PR #270 proves repository and isolated acceptance; PR #368 adds staging-only enablement evidence. Neither proves production activation.

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

### Current available boundary

The merged Character Bazaar provides:

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

PR #270 proves repository and isolated acceptance. PR #368 proves a bounded staging enablement/control package. Issue #489 retains marketplace catalogue state and broader commerce/product gaps; no provider-payment or production-complete claim follows.

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

### Responsibilities

- format and transport already-authorized notifications through approved channels;
- persist deterministic delivery attempt/status state where the owning use case requires it;
- preserve locale/presentation metadata supplied by the owning domain without taking over that domain's authorization or business rules.

### Current available boundary

Implemented email use cases include password reset, confirmed email-change communications, English and Polish account-security messages and deterministic support/moderation delivery records.

Mail delivery should be asynchronous once queue infrastructure exists. Security tokens and locale selection remain owned by Identity use cases; Notifications formats and transports messages but does not validate tokens, authorize account changes or own Marketplace state.

Future marketplace outbid/completion notifications require an explicitly reviewed asynchronous delivery slice. Future PlayerCompanion tracking notifications likewise require PlayerCompanion to produce the authorized domain signal/notification intent first; Notifications does not own what is tracked, source-change detection or threshold rules.

### Invariants

- notification transport/delivery failure does not rewrite committed owning-domain state unless an explicit owning-domain saga contract says otherwise;
- Notifications does not become tracking/subscription authority merely because it delivers an alert;
- source facts, comparison rules and threshold decisions remain with their accepted owning modules;
- destinations and private tracking lists are not exposed through ordinary logs, metrics labels or public output.

## Operations and Observability

### Responsibilities

- release/build identity and environment applicability;
- health/readiness and dependency-failure visibility;
- structured logs, correlation, metrics and alert ownership;
- queue, mail, cache/session and scheduler operational contracts;
- backup, restore, rollback and disaster-recovery expectations;
- runbooks and sanitized evidence for approved environments.

### Current available boundary

Repository capabilities include health/readiness behavior, correlation/security/admin logging, production-like and dependency-outage validation, exact release/staging evidence contracts and operational runbooks. This is a partial repository/staging boundary, not proof of complete production topology, on-call readiness, restore rehearsal or private-production reachability.

Issue #490 owns the missing explicit applicability and exact environment evidence. Operations must not infer `PRODUCTION_PROVEN` from a passing repository workflow.

### Invariants

- evidence names the exact commit, artifact/deployment identity and environment;
- dependency failure remains explicit and observable;
- secrets and private data do not enter logs, artifacts or task records;
- restore/rollback claims require an exercised, bounded path;
- operational controls do not bypass module authorization or data ownership.

## Public Edge

### Responsibilities

- expected public DNS and hostname contract;
- TLS, redirects, HSTS, WAF/bot/rate-limit and administrative-access boundary;
- tunnel/origin reachability and private-ingress expectation;
- sanitized exact environment verification and rollback boundaries.

### Current available boundary

The repository contains Cloudflare/public-edge contracts, verification tooling, deployment markers and bounded repair/audit records. Exact live state has also produced blocked/direct-failure evidence. Therefore the repository ownership boundary is available while public-production correctness remains separately unproven or blocked until exact authorized evidence passes.

Issue #490 owns the current applicability and environment-proof gap. Cloudflare remains defense in depth; Laravel cannot trust edge presence as application authorization.

### Invariants

- repository configuration is not treated as current provider state;
- live mutation requires explicit authorization and least-privilege credentials;
- origin bypass, TLS, redirect, HSTS and WAF claims require exact endpoint/environment evidence;
- provider secrets never enter Git, logs or artifacts;
- edge controls never replace server-side validation, authorization, CSRF or rate limiting.

## Quality and E2E

### Responsibilities

- versioned capability, route/surface/state and applicability ledgers;
- deterministic contract validators and change classification;
- focused, integration, browser, accessibility, responsive and resilience evidence;
- exact-head/exact-deployment evidence identity;
- distinction among existence, functional evidence, product completeness, staging proof and production proof.

### Current available boundary

The repository has machine-enforced route/capability ledgers, zero-retry browser profiles, module acceptance workflows, production-like/dependency-failure checks and exhaustive audit evidence. These prove bounded repository and staging journeys; they do not prove universal absence of defects, complete expected content or production correctness.

Issues #488, #489 and #490 retain missing expected inventories, failure/recovery, portability and non-UI applicability evidence. Quality/E2E records gaps but does not choose product policy or weaken acceptance to close them.

### Invariants

- worker narrative is never evidence;
- every result is tied to an exact head and, when applicable, exact deployment identity;
- mocked-only UI tests do not replace real integration/E2E;
- retries do not hide deterministic defects;
- `NOT_APPLICABLE` requires an explicit reason and owner/contract where material;
- staging or repository evidence never silently becomes `PRODUCTION_PROVEN`.

## Platform API — planned

Expose API endpoints only for a concrete client/use case. API endpoints must reuse module services/policies and must not implement a second business-rule path.

Bounded internal endpoints, game-auth tickets or operational probes are not a general public/first-party Platform API. Issue #490 retains the decision and evidence gap.

PlayerCompanion is a concrete future consumer candidate: the API may expose calculator metadata/execution, owner workspaces, compatible recommendations, owner tracking/routine preferences and share resolution only through the same application services, version/freshness semantics, authentication and rate limits as the web UI.

Federated public content search is another concrete future candidate under ADR 0033. If exposed, PlatformAPI adapts the same `PublicPortal` FederatedSearch application service and normalized result/failure semantics; it must not independently fan out to CMS/Announcements/Events/Wiki/GameCatalog or recreate cross-source grouping/ranking policy.

## Products and Entitlements — planned

This boundary will own product definitions and fulfilment state such as premium/VIP, packages, vouchers/codes, expiry, revocation and customer-visible entitlement history when accepted.

It must define idempotent fulfilment, reconciliation and rollback separately from provider payment settlement. Direct Canary premium mutations, product policy and activation require explicit contracts. Issue #489 retains the missing-capability decisions.

## Legal Commerce — planned

This boundary will own commerce-specific consumer presentation and decisions for payment-data privacy, retention, refund/complaint handling and invoice/tax responsibilities for the target jurisdictions.

Generic legal-page publishing remains CMS responsibility. LegalCommerce does not execute provider settlement or invent legal requirements without authoritative review. Real-money activation cannot proceed until its decisions and acceptance evidence are explicit.

## Payments — planned later

No payment-provider implementation belongs in the current core platform or Character Bazaar v1 scope.

Future requirements include provider abstraction, signed webhook verification, provider-event idempotency, immutable provider/financial ledgers, reconciliation, refunds/chargebacks, tax/legal review and explicit separation from Oteryn Coins gameplay policy.

Payments must not become a dependency of basic Identity/account creation/login, must not own product fulfilment and must not reuse Canary mutable coin fields as the provider settlement source of truth. Issue #489 retains the missing provider/commerce capability decisions.

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

## Character Profile Preferences

The CharacterProfiles module owns authenticated management of Platform-stored character comments, per-character public visibility and optional main-character selection. It verifies ownership from the immutable ready binding plus a fresh read-only Canary character lookup on every edit/update, records bounded audit events and projects effective privacy into PublicGameData. It does not mutate Canary or implement rename, delete, restore, transfer or achievements. Contract: `docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md`.
