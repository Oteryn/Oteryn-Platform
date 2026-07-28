# Oteryn Platform Project State

This file is the compact authoritative entry point for “where are we now?”. Live Git, PR, issue, task and exact-SHA evidence remain authoritative when they are newer.

## Last architecture-state update

2026-07-28

## Engineering phase state

- **Phase 0 — Architecture and agent bootstrap: COMPLETE**
- **Phase 1 — Laravel application bootstrap: COMPLETE**
- **Phase 2 — Canary/login authentication discovery for current implementation boundaries: COMPLETE**
- **Phase 3 — Identity foundation: COMPLETE**
- **Phase 4 — Public website and read-only game data: COMPLETE for the delivered route contract**
- **Phase 5 — Account and character management: COMPLETE for the delivered route contract**
- **Phase 6 — CMS, Admin, RBAC and Audit: COMPLETE for the delivered route contract**
- **Phase 7 — Production hardening and operations: COMPLETE as an engineering milestone**

These phase statements do not claim benchmark product completeness. Issue #268 audits capabilities that can be absent from an otherwise green delivered-surface contract.

## Operational release state

- **Production Readiness: STAGING_PROVEN for documented boundaries**
- **Delivered Portal Route Contract: COMPLETE AND MACHINE ENFORCED**
- **Benchmark Product Completeness: NOT COMPLETE; AUDIT IN PROGRESS IN #268**
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

The ready Platform-to-Canary binding remains immutable. Self-service import, unlink, rebind or transfer is intentionally not applicable without a separately reviewed operation contract. Email-code MFA is intentionally not adopted because email is the recovery channel. Optional account badge/loyalty/status presentation remains absent.

### Public portal and game data

- localized home, navigation, SEO, news and managed pages;
- character name search and basic active-character detail;
- read-only guild detail and members;
- level highscores;
- online players and configured server/channel status;
- explicit empty, not-found, unavailable and restoration states.

Not yet benchmark-complete: rich character profiles, guild directory/administration, highscore categories/filters, deaths, transfer history and kill statistics. Trackers: #277 and #280.

### CMS and community publishing

- News, Managed Pages, Downloads, Events and Announcements public/admin/localization lifecycles;
- typed Support/Legal content administration;
- Editorial Media private storage, integrity validation and reference protection;
- first-party Wiki public search/category/article flows plus editor/reviewer/publisher, revisions, signed preview and media integration;
- reviewed bilingual launch content.

Static support content is not an authenticated ticket/report/moderation system. Tracker: #279. The Wiki is editorially complete for delivered articles but lacks authoritative server-backed creature/item/loot/gameplay catalogues. Tracker: #281.

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

The account-security fragment adds guest/authenticated EN/PL email, session, privacy, recovery-key and termination states. The Character Bazaar fragment adds public, authenticated and administrator marketplace surfaces.

## Product-completeness benchmark

Issue #268 is tracked by:

- `docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md`;
- `docs/testing/product-completeness-benchmark.json`;
- `scripts/acceptance/coverage/validate-product-completeness.mjs`.

The current ledger classifies 43 Tibia/RubinOT/OTS benchmark capabilities:

- 9 implemented;
- 8 partial;
- 25 missing;
- 1 not applicable;
- 22 required, 13 planned, 7 optional/differentiator and 1 not applicable.

Focused backlog:

- #276 — delivered Platform-owned account security and lifecycle;
- #277 — character management and public profiles;
- #278 — premium, coins and entitlement commerce;
- #279 — tickets, reports and enforcement history;
- #280 — community statistics and guild workflows;
- #281 — server-backed Wiki/gameplay catalogues.

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

`OTERYN-20260728-account-security-lifecycle` on draft PR #283, resolving Issue #276 for the approved Platform-owned boundary.

## Recommended sequence

1. Review and merge PR #283 only after exact-final-head CI, strict portal coverage and zero-retry browser acceptance pass.
2. Deliver remaining required benchmark gaps as bounded tasks: #277, #279 and #280.
3. Keep #278 disabled until a dedicated payment ADR, threat model and provider lifecycle are reviewed.
4. Build #281 from authoritative Oteryn server availability, never by copying third-party datasets or prose.
5. Resume #91 only after explicit production deployment/verification authorization and required production evidence access exist.
