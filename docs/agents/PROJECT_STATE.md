# Oteryn Platform Project State

This file is a compact project-state entry point. Live protected Git refs, active task records, PRs, Issues and exact-SHA evidence are authoritative when newer. It is **not** a programme scheduler and must not be used to bypass a programme's current live selector.

## Last architecture-state update

2026-08-14

## Live routing override

For `OTERYN_PORTAL_COMPLETION`, `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md` is the selection authority. On every invocation it classifies canonical entries from live evidence as `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY` and selects only the first unowned `READY` candidate. Historical backlog text below is product-state context only; closed Issues, old PR ownership and prior “next work” recommendations are never current routing authority.

At the selector-reconciliation base `166561fe066b12310fb534172542e60b51484c46`, the long-lived active task records are the blocked public-domain repair and blocked native-auth production verification records; the portal selector reconciliation itself is owned by Issue #1057 / PR #1058 until its terminal closeout. Any newer live state supersedes this snapshot.

## Engineering phase state

- **Phase 0 — Architecture and agent bootstrap: COMPLETE**
- **Phase 1 — Laravel application bootstrap: COMPLETE**
- **Phase 2 — Canary/login authentication discovery for current implementation boundaries: COMPLETE**
- **Phase 3 — Identity foundation: COMPLETE**
- **Phase 4 — Public website and read-only game data: COMPLETE for the delivered route contract**
- **Phase 5 — Account and character management: COMPLETE for the delivered route contract**
- **Phase 6 — CMS, Admin, RBAC and Audit: COMPLETE for the delivered route contract**
- **Phase 7 — Production hardening and operations: COMPLETE as an engineering milestone**

These phase statements do not claim benchmark product completeness. Issue #268 and its merged PR #315 reconciliation track backend, reachable frontend and browser-evidence capabilities that can be absent from an otherwise green delivered-surface contract.

## Operational release state

- **Production Readiness: STAGING_PROVEN for documented boundaries**
- **Delivered Portal Route Contract: COMPLETE AND MACHINE ENFORCED**
- **Benchmark Product Completeness: NOT COMPLETE; REQUIRED CHARACTER GAPS #317/#319 REMAIN, AND #321/#322 ARE MANDATORY BEFORE COMMERCE**
- **Backend–Frontend Promotion Enforcement: COMPLETE IN #340 / PR #341**
- **Viewport/Browser Evidence Linkage: COMPLETE IN #347 / PR #349**
- **Public Game-Data Stress/500 Slice: COMPLETE IN #350 / PR #351**
- **Exhaustive Backend–Frontend Evidence: NOT COMPLETE; PARENT #326 REMAINS OPEN**
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

The ready Platform-to-Canary binding remains immutable. Self-service import, unlink, rebind or transfer is intentionally not applicable without a separately reviewed operation contract. Email-code MFA is intentionally not adopted because email is the recovery channel. Optional account badge/loyalty/status presentation is tracked by #325; premium status depends on #322.

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

PR #351 completed Issue #350 as merge `923933222050999fec368bc2db1be6e546f13c12`. Its zero-retry Chromium desktop/tablet/mobile scenario proves long externally sourced values, more than one 50-row result page, a genuine non-debug Laravel `500` without sensitive disclosure and deterministic recovery for the public game-data surface. This is one bounded #326 state slice, not universal state completeness.

PR #308 completed owner-editable Platform comments, character-level privacy and optional main-character selection. Still not benchmark-complete: deletion/restore remains #317, rename remains #319, controlled world/channel transfer remains #320, and selected achievements depend on #301/#323. Customer commerce remains #321/#322 under parent #278. Structured authoritative spell/NPC/quest/achievement catalogues remain #301, while optional map/hunt/discovery decisions remain #302.

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
- desktop/tablet/mobile Chromium acceptance, accessibility and real-MariaDB concurrency evidence.

The wallet is not a payment system. Customer coin purchase, premium/VIP, products, signed provider events, refunds and chargebacks are split into #321 and #322 under parent #278. Canary tournament coins are not used.

## Delivered-surface acceptance contract

The route/state ledger is `scripts/acceptance/coverage/portal-coverage-manifest.json` plus sorted fragments under `scripts/acceptance/coverage/surfaces/`.

The strict contract proves:

- every delivered named route is classified exactly once or explicitly excluded as a framework/support endpoint;
- owned surfaces declare roles, states, viewports, browsers and evidence layers;
- evidence files and stable markers exist;
- strict closure fails for delivered surfaces left `partial` or `planned`.

The account-security fragment adds guest/authenticated EN/PL email, session, privacy, recovery-key and termination states. The Character Bazaar fragment adds public, authenticated and administrator marketplace surfaces. The community-data and character-profile fragments add highscore, profile, owner-preference, main-character race, deaths, guild, localization, dependency-failure/recovery and responsive states.

PR #315 established an additional product-level rule: a user-facing capability is `IMPLEMENTED` only when backend/domain behavior, a reachable frontend connected to the real route and applicable zero-retry browser evidence are all present. Backend-only delivery remains `PARTIAL`; frontend files without reliable integrated evidence remain `UNTESTED`.

PR #341 completed Issue #340 as merge `90035fa764f4477ebcffd9410075dc342972be42`. The fail-closed cross-ledger gate now gives every canonical product capability explicit backend, frontend and integration status; integrated claims reference exact covered portal surfaces with Playwright markers; non-UI exceptions require rationale; deterministic negative fixtures reject backend-only promotion and inconsistent evidence. Exact final head `4c29c21f448d3f17b169450a7a2667b9b2ca327a` passed all nine authoritative workflows, including strict Portal Acceptance, complete zero-retry account lifecycle and full Visual UX.

PR #349 completed Issue #347 as merge `30b9c4767b137cde3035a5529410c7d9add2d5ba`. Exactly 27 dimension records map all delivered surfaces to 13 executable profile groups; all 23 critical rendered surfaces require blocking zero-retry Chromium desktop/tablet/mobile evidence; Firefox/WebKit is either executable or explicitly excluded with a bounded risk rationale. Eight negative fixtures reject missing dimensions, projects, markers, browser IDs, rationale, orphan records and non-blocking critical evidence. Exact final head `f1524e767107afbf542d0d82e29a0f89eada1fc5` passed all nine authoritative workflows, including strict Portal Acceptance, complete zero-retry Account Lifecycle and full Visual UX.

The dimension work also corrected two truthful frontend-evidence problems: tablet/mobile localization now asserts the visible responsive language/navigation links instead of hidden desktop elements, and Marketplace no longer claims generic bounded portability without an executable Firefox/WebKit profile.

Broad responsive, portability, resilience and accessibility evidence exists, but it is not a universal every-route/every-state/browser Cartesian matrix. #326 owns the remaining exhaustive visual/state evidence after #340, #347 and the completed public game-data #350 slice.

## Product-completeness benchmark

The completed Issue #268 audit and merged PR #315 reconciliation are tracked by:

- `docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md`;
- `docs/testing/product-completeness-benchmark.json`;
- `docs/testing/product-backend-frontend-completeness.json`;
- `docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md`;
- `docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md`;
- `docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29_VALIDATION.md`;
- `scripts/acceptance/coverage/validate-product-completeness.mjs`;
- `scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs`;
- `scripts/acceptance/coverage/portal-evidence-dimensions.json`;
- `scripts/acceptance/coverage/validate-dimension-evidence.mjs`.

The current canonical ledger classifies 43 Tibia/RubinOT/OTS benchmark capabilities:

- 23 implemented;
- 3 partial;
- 14 missing;
- 3 not applicable;
- 22 required, 13 planned, 5 optional/differentiator and 3 not applicable.

The 2026-07-29 audit projects that two-axis ledger into the required vocabulary as 23 `IMPLEMENTED`, 3 `PARTIAL`, 2 `MISSING_REQUIRED`, 4 `MISSING_OPTIONAL`, 8 `PLANNED`, 3 `NOT_APPLICABLE`, 0 `UNTESTED` and 0 runtime `BROKEN` canonical capabilities. The expanded audit records additional evidence and documentation findings separately.

Completed focused slices:

- #276 — Platform-owned account security and lifecycle, merged in PR #283;
- #279 — Platform-owned support and moderation lifecycle, merged in PR #293;
- #280 — read-only community statistics and guild discovery with privacy-aware profiles, merged in PR #298;
- #281 — first versioned item/weapon/creature/loot Game Catalog scope delivered by PR #272 and evidence ownership closed by PR #303;
- #307 — Platform-owned character comments, per-character privacy and optional main-character selection in PR #308;
- #268 reconciliation — detailed product and frontend audit merged in PR #315 as `94b3457f4bb5b9aa73639a698c70ebb233940288` after all eight required workflows passed on exact head `92935a76e559d8716773ebec5d1a04264051cfa1`;
- #340 — backend/frontend/integration promotion enforcement merged in PR #341 as `90035fa764f4477ebcffd9410075dc342972be42` after all nine authoritative final-head workflows passed;
- #347 — exact viewport/browser evidence linkage merged in PR #349 as `30b9c4767b137cde3035a5529410c7d9add2d5ba` after all nine authoritative final-head workflows passed;
- #350 — public game-data long-value, large-result, real-500 and recovery evidence merged in PR #351 as `923933222050999fec368bc2db1be6e546f13c12`.

Open focused backlog:

- #277 parent — character management/public-profile completion;
- #317 — character deletion, grace and restore;
- #319 — conflict-safe rename lifecycle;
- #320 — controlled world/channel transfer product decision and service;
- #323 — authoritative achievement selection, dependent on #301;
- #278 parent — commerce;
- #321 — provider-neutral payment security foundation;
- #322 — products, entitlements, vouchers and histories;
- #301 — authoritative spell/NPC/quest/achievement catalogue expansion;
- #302 — optional maps, hunt tools and server-specific discovery planning;
- #325 — optional loyalty/badge/status presentation;
- #326 — remaining exhaustive delivered-screen/browser/visual/state matrix.

These identifiers are historical/product-gap context, not a current execution queue. A green route or API contract must not be described as product complete while required benchmark or frontend gaps remain.

## Production hardening and evidence

The repository has controlled evidence for clean migrations, rollback/redeploy, least-privilege database principals, Redis ACL behavior, test SMTP, security headers/cookies, request correlation, backup/restore smoke, dependency outage/recovery and browser portability/responsive/accessibility profiles.

Scheduled E2E calibration is now backed by completed exact-SHA runtime evidence. Public-soak run `29987560312` on `8006534108d835474dadd208b0ec934e4a12528b` passed 1,764 read-only requests over 303 measured seconds with unchanged Redis key count. Stability-repeat run `30243589211` on `37eb31d60aa8a47914745cd326aff6b313851dd0` passed all three distinct zero-retry critical iterations. Later run `30790638508` retained a responsive-mobile failure signal that was classified from its artifact as a transient Wiki success-flash harness race after the durable state had already reached `In Review`; current-main durable-state assertions and PR #495 deep validation provide the remediation evidence. These samples remain controlled calibration/stability evidence and introduce no blocking performance or flakiness threshold.

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

Repository and exact-revision E2E work has hardened the native-auth direction, but production activation remains separately gated. Any new external repository access or write requires separate current owner authorization and a coordinated contract/rollout; this state file does not grant it.

## Current active task

Do not infer a single global owner from this section. Current ownership is the union of live `docs/agents/tasks/active/**`, branches/PRs and programme leases. At selector-reconciliation base `166561fe066b12310fb534172542e60b51484c46`, the two pre-existing active task records are blocked and non-overlapping; Issue #1057 / PR #1058 owns the bounded portal selector reconciliation. The concurrent PublicPortal Today architecture is separately owned by Issue #1049 / PR #1055 and does not overlap #1058's claimed paths.

## Recommended sequence

For portal completion, do not execute this numbered history as a queue. Run `OTERYN_PORTAL_COMPLETION.md` from live protected `main` and select the first exact candidate classified `READY` after persisting exact reasons for earlier `TERMINAL`, `OWNED`, `BLOCKED` and `DECISION_REQUIRED` entries. In particular, closed remediation Issues #948/#944/#941 are historical terminal evidence, LiveOps architecture #1046 is terminal without implying runtime implementation, `WorldStatus + configured Maintenance` requires exact authoritative runtime-source evidence, `ServerSave` requires its own proven source, and Client Distribution Issue #1039 is explicitly reachable by the canonical order. Production, external-repository and owner-funded-service gates remain separate.

## Community data delivery

PR #298 completed Issue #280's approved read-only boundary: categorized/vocation highscores, privacy-aware rich profiles, latest deaths/kill statistics, guild directory search/detail, direct-table grant verification and EN/PL zero-retry desktop/tablet/mobile acceptance. Exact final head `45efd2a8f0162df22313e141e973c6a8c3ffb5d1` passed all 11 required workflows before squash merge `7533b12b1e1c6d266c6bf5a8800e584fad23a01e`. Canary mutation, guild administration, transfer history, polls and public enforcement publication remain explicitly excluded, and no production-verification claim was made.

PR #351 later added the bounded Issue #350 stress/error evidence without changing the delivered product boundary: 76 acceptance-only active characters proved deterministic page two, long values rendered without document-level overflow, a genuine non-debug 500 exposed no internal details, and restoration returned the same route to 200. Parent #326 remains open for other surfaces and permutations.

## Game Catalog first-scope closeout

PR #303 completed Issue #281's accepted first scope by reconciling the versioned item/weapon/creature/loot delivery from PR #272 with the 43-capability benchmark. Exact final head `7c6bd2b46f3c29d5a2bd4862d59614fcaec423bc` passed all eight required workflows before squash merge `e1df0608eb6a8321f47fe51da65233a613a27b25`. Deferred spells/NPCs/quests/achievements remain #301, optional map/hunt/discovery decisions remain #302, and no runtime, Canary, producer, activation or production change occurred.

## Character profile preferences delivery

PR #308 completed the Platform-owned Issue #307 slice as merge `86847d0068e470274b6c3ee5523fe41cbb9663af`. Exact final head `3797a094cfa522f5147d624786f49fee5027c77b` passed all 11 required workflows, including real-MariaDB main-character concurrency and zero-retry EN/PL desktop/tablet/mobile browser acceptance. Canary remained read-only; rename, deletion, restore, transfer and selected achievements remain outside this contract under parent #277, and no production claim was made.
