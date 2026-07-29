# Oteryn Platform Project State

This file is the compact authoritative entry point for “where are we now?”. Live Git, PR, issue, task and exact-SHA evidence remain authoritative when they are newer.

## Last architecture-state update

2026-07-29

## Engineering phase state

- **Phase 0 — Architecture and agent bootstrap: COMPLETE**
- **Phase 1 — Laravel application bootstrap: COMPLETE**
- **Phase 2 — Canary/login authentication discovery for current implementation boundaries: COMPLETE**
- **Phase 3 — Identity foundation: COMPLETE**
- **Phase 4 — Public website and read-only game data: COMPLETE for the delivered route contract**
- **Phase 5 — Account and character management: COMPLETE for the delivered route contract**
- **Phase 6 — CMS, Admin, RBAC and Audit: COMPLETE for the delivered route contract**
- **Phase 7 — Production hardening and operations: COMPLETE as an engineering milestone**

These phase statements do not claim benchmark product completeness. The completed Issue #268 audit tracks capabilities that can be absent from an otherwise green delivered-surface contract.

## Operational release state

- **Production Readiness: STAGING_PROVEN for documented boundaries**
- **Delivered Portal Route Contract: COMPLETE AND MACHINE ENFORCED**
- **Benchmark Product Completeness: NOT COMPLETE; REQUIRED CHARACTER GAP #277 REMAINS OPEN AND #278 IS MANDATORY BEFORE COMMERCE**
- **Production Go-Live Gate: PENDING PRODUCTION VERIFICATION**
- **Production Verification: REQUIRED BEFORE GO-LIVE**

Repository, isolated acceptance, Synology preflight and staging-like evidence never substitute for direct verification of the exact deployed production release.

## Current architecture

Oteryn Platform is a Laravel 13 / PHP 8.5 modular monolith with Platform-owned Identity and application persistence.

Supported game accounts remain greenfield and use the immutable current binding model:

`1 Platform Identity <-> 1 Canary accounts.id`

Existing Canary accounts are not imported or claimed. The browser does not communicate directly with Canary or Freqtrade-like private runtimes. Shared state changes use explicit operation-specific contracts and credentials.

## Delivered product surfaces

### Identity and accounts

- registration, login and logout;
- password reset/change with expiring single-use tokens and session revocation;
- TOTP MFA, replay protection and recovery codes;
- account overview with pending, ready, recoverable, conflict and missing provisioning states;
- bounded provisioning retry;
- confirmed primary-email change with new-address confirmation, old-address recovery and cooldown;
- registered active-session inventory with targeted, current and all-other revocation;
- private-by-default account association and status controls;
- verifier-only high-assurance recovery key generation, rotation, revocation, single use and replay denial;
- bounded Platform account termination with grace, cancellation and idempotent finalization that preserves Canary-owned data;
- English and Polish account-security UI, validation, token errors and notification links;
- character creation for a ready immutable binding.

PR #283 merged the complete approved account-security lifecycle as `28faad47f95df10d1a9b437a16a1be91556671c6` after all 12 exact-final-head workflows passed.

The ready Platform-to-Canary binding remains immutable. Self-service import, unlink, rebind or transfer is intentionally not applicable without a separately reviewed operation contract. Email-code MFA is intentionally not adopted because email is the recovery channel. Optional account badge/loyalty/status presentation remains absent.

### Public portal and game data

- localized home, navigation, SEO, news and managed pages;
- character name search and privacy-aware server-backed character profiles;
- authenticated Platform-owned character comments, per-character visibility controls and optional main-character selection;
- fixed-allowlist highscore categories with supported vocation filtering, truthful global scope and bounded pagination;
- latest deaths and bounded player-kill statistics;
- read-only guild directory, search, detail, ranks and members;
- public house summary and private-by-default account association/status disclosure;
- online players and configured server/channel status;
- explicit validation, empty, not-found, unavailable, restoration and responsive states.

PR #298 merged the approved read-only community-data boundary as `7533b12b1e1c6d266c6bf5a8800e584fad23a01e` after all 11 exact-final-head workflows passed. Canary mutation, guild administration, selectable achievements, world-transfer history, polls and public enforcement publication remain excluded until authoritative ownership and privacy contracts exist.

Still not benchmark-complete: Issue #307 delivers owner-editable Platform comments, character-level privacy and optional main-character selection, while selected achievements, deletion/restore, rename and controlled transfer remain #277. Customer commerce remains #278. Structured authoritative spell/NPC/quest/achievement catalogues remain #301, while optional map/hunt/discovery decisions remain #302.

### CMS and community publishing

- News, Managed Pages, Downloads, Events and Announcements public/admin/localization lifecycles;
- typed Support/Legal content administration;
- Editorial Media private storage, integrity validation and reference protection;
- first-party Wiki public search/category/article flows plus editor/reviewer/publisher, revisions, signed preview and media integration;
- reviewed bilingual launch content.

Authenticated owner-scoped tickets, bounded reports, exact-MFA/RBAC moderation, Platform enforcement/appeals, notifications and retention are delivered through PR #293. PR #293 squash-merged as `02aa4ab8180c0e9cecb0d42bc1f8f5af6db640a1` after all exact-final-head workflows passed. Canary ban mutation and attachments remain excluded. The Wiki is editorially complete for delivered articles. PR #272 delivered the first authoritative versioned item/weapon/creature/loot Game Catalog scope. Structured spells/NPCs/quests/achievements remain #301 and optional map/hunt/discovery decisions remain #302.

### Support and moderation

- authenticated owner-scoped ticket creation, detail, reply and explicit close/reopen lifecycle;
- bounded player/content/guild reports with pending limits, idempotency, history and public-safe outcomes;
- exact-permission confirmed-MFA moderator ticket/report/enforcement queues;
- Platform-owned warnings/restrictions/suspensions with acknowledgement and appeal states;
- deterministic notification delivery status, bounded audit metadata and configurable retention;
- English/Polish desktop/tablet/mobile acceptance;
- no Canary ban mutation and no support attachments.

### Character Bazaar and Wallet

PR #270 merged the complete first Character Bazaar as `0f19656e0875d0a10b22002ac0e096deb20e94d8`.

Delivered boundaries:

- public localized catalogue, filters, immutable snapshots and bounded bid history;
- authenticated watchlist, listing, bidding, buy-now, cancellation and history;
- Platform-owned Oteryn Coins wallet with append-oriented ledger and available/reserved balances;
- transactional bid reservation and deterministic outbid release;
- dedicated least-privilege Canary character-transfer connection;
- non-login escrow account, session/offline/quota checks, deterministic locking and idempotency;
- recoverable cross-database listing/cancellation/settlement saga;
- MFA/permission/audit-protected administrator wallet adjustment and recovery queue;
- desktop/tablet/mobile, accessibility, real-MariaDB concurrency and full browser acceptance.

The wallet is not a payment system. Customer coin purchase, premium/VIP, products, webhooks, refunds and chargebacks remain #278. Canary tournament coins are not used.

## Delivered-surface acceptance contract

The route/state ledger is `scripts/acceptance/coverage/portal-coverage-manifest.json` plus sorted fragments under `scripts/acceptance/coverage/surfaces/`.

The strict contract proves:

- every delivered named route is classified exactly once or explicitly excluded as a framework/support endpoint;
- owned surfaces declare roles, states, viewports, browsers and evidence layers;
- evidence files and stable markers exist;
- strict closure fails for delivered surfaces left `partial` or `planned`.

The account-security fragment adds guest/authenticated EN/PL email, session, privacy, recovery-key and termination states. The Character Bazaar fragment adds public, authenticated and administrator marketplace surfaces. The community-data and character-profile fragments add highscore, profile, owner-preference, main-character race, deaths, guild, localization, dependency-failure/recovery and responsive states.

## Product-completeness benchmark

The completed Issue #268 audit is tracked by:

- `docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md`;
- `docs/testing/product-completeness-benchmark.json`;
- `scripts/acceptance/coverage/validate-product-completeness.mjs`.

The current ledger classifies 43 Tibia/RubinOT/OTS benchmark capabilities:

- 23 implemented;
- 3 partial;
- 14 missing;
- 3 not applicable;
- 22 required, 13 planned, 5 optional/differentiator and 3 not applicable.

Completed focused slices:

- #276 — Platform-owned account security and lifecycle, merged in PR #283;
- #279 — Platform-owned support and moderation lifecycle, merged in PR #293;
- #280 — read-only community statistics and guild discovery with privacy-aware profiles, merged in PR #298;
- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272 and evidence ownership closed by PR #303;
- #307 — Platform-owned character comments, per-character privacy and optional main-character selection in PR #308.

Open focused backlog:

- #277 — character management and public profiles;
- #278 — premium, coins and entitlement commerce;
- #301 — authoritative spell/NPC/quest/achievement catalogue expansion;
- #302 — optional maps, hunt tools and server-specific discovery planning.

A green route contract must not be described as product complete while required benchmark gaps remain.

## Production hardening and evidence

The repository has controlled evidence for clean migrations, rollback/redeploy, least-privilege database principals, Redis ACL behavior, test SMTP, security headers/cookies, request correlation, backup/restore smoke, dependency outage/recovery and browser portability/responsive/accessibility profiles.

The authoritative production gate remains `docs/operations/PRODUCTION_READINESS_CHECKLIST.md` and issue #91. Direct production facts remain unknown until verified, including:

- exact deployed Platform, Gateway and Canary identities;
- production DNS/edge/TLS/WAF/origin and private ingress;
- production database topology, effective grants, backup and dated restore evidence;
- production Redis, session/cache/queue and mail topology;
- logs, metrics, alerts and on-call ownership;
- actual deployment/migration/rollback mechanism;
- final mutation-authorized production smoke.

A deployment-targeted preflight previously failed closed before network or mutation because required production Environment metadata, controlled credentials, backup evidence identification and explicit mutation authorization were absent. Generic continuation does not authorize production action.

## Game-login boundary

Repository and exact-revision E2E work has hardened the native-auth direction, but production activation remains separately gated. Cross-repository writes to Canary or a login server require explicit user authorization and a coordinated contract/rollout.

## Current active task

None. PR #308 completed Issue #307 and its task is archived; parent #277 remains open for separately contracted mutation and achievement work.

## Recommended sequence

1. Continue parent #277 only through a new bounded task after selecting an explicitly authorized mutation or authoritative achievement scope.
2. Define explicit operation-specific contracts before implementing character rename, deletion, restoration or transfer.
3. Keep #278 disabled until a dedicated payment ADR, threat model and provider lifecycle are reviewed.
4. Start #301 only after an additive authoritative producer contract is approved; treat #302 as optional product discovery.
5. Resume #91 only after explicit production deployment/verification authorization and required production evidence access exist.

## Community data delivery

PR #298 completed Issue #280's approved read-only boundary: categorized/vocation highscores, privacy-aware rich profiles, latest deaths/kill statistics, guild directory search/detail, direct-table grant verification and EN/PL zero-retry desktop/tablet/mobile acceptance. Exact final head `45efd2a8f0162df22313e141e973c6a8c3ffb5d1` passed all 11 required workflows before squash merge `7533b12b1e1c6d266c6bf5a8800e584fad23a01e`. Canary mutation, guild administration, transfer history, polls and public enforcement publication remain explicitly excluded, and no production-verification claim was made.

## Game Catalog first-scope closeout

PR #303 completed Issue #281's accepted first scope by reconciling the versioned item/weapon/creature/loot delivery from PR #272 with the 43-capability benchmark. Exact final head `7c6bd2b46f3c29d5a2bd4862d59614fcaec423bc` passed all eight required workflows before squash merge `e1df0608eb6a8321f47fe51da65233a613a27b25`. Deferred spells/NPCs/quests/achievements remain #301, optional map/hunt/discovery decisions remain #302, and no runtime, Canary, producer, activation or production change occurred.


## Character profile preferences delivery

PR #308 completed the Platform-owned Issue #307 slice as merge `86847d0068e470274b6c3ee5523fe41cbb9663af`. Exact final head `3797a094cfa522f5147d624786f49fee5027c77b` passed all 11 required workflows, including real-MariaDB main-character concurrency and zero-retry EN/PL desktop/tablet/mobile browser acceptance. Canary remained read-only; rename, deletion, restore, transfer and selected achievements remain outside this contract under parent #277, and no production claim was made.
