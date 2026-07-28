# Oteryn Portal Acceptance Coverage Matrix

## Result boundary

This matrix defines exhaustive acceptance against the **currently delivered, versioned portal surface contract**.

It does not define product completeness. A route contract can be fully green while required benchmark capabilities have no route, state or implementation. Benchmark product completeness is separately governed by `docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md` and `docs/testing/product-completeness-benchmark.json` under Issue #268.

It does not claim that Oteryn Platform is universally bug-free. It does not replace lower-layer deterministic tests, the separately authorized Platform-to-game login bridge, manual screen-reader verification or final production verification.

Evidence states remain:

- `PROVEN` — deterministic repository evidence at the stated layer;
- `STAGING_PROVEN` — directly exercised in the controlled production-like environment;
- `PRODUCTION_PROVEN` — directly exercised against the exact deployed production release;
- `UNKNOWN` — not directly proven for the stated boundary.

Coverage states in `scripts/acceptance/coverage/portal-coverage-manifest.json` are:

- `covered` — every required dimension is mapped to current evidence at the correct layer;
- `partial` — useful evidence exists, but exact required dimensions remain open;
- `planned` — the delivered surface is classified but its composed acceptance package remains to be implemented;
- `supporting_endpoint` — a browser-consumed resource/preview/redirect rather than a standalone page;
- `not_delivered` — outside the current delivered-surface contract and not represented as tested.

Product benchmark delivery states are different: `implemented`, `partial`, `missing`, `untested` and `not_applicable`. They must not be inferred from route-manifest status.

## Required CI contracts

| Contract | Command | Purpose | Current gate |
|---|---|---|---|
| Manifest integrity and live route classification | `npm --prefix scripts/acceptance run test:coverage-contract` | Fails for malformed records, duplicate routes, stale routes, missing evidence files/markers and unclassified named routes | Required by `Portal Acceptance Contract` |
| Product completeness benchmark integrity | `npm --prefix scripts/acceptance run test:product-completeness` | Fails for omitted Issue #268 capabilities, invalid classifications, unsupported evidence, missing evidence for implemented/partial claims or required gaps without focused issues | Included in the strict `Portal Acceptance Contract` |
| Complete account lifecycle | `npm --prefix scripts/acceptance run test:account-lifecycle` | Executes registration, login/logout, account overview, provisioning, password, MFA, sessions and character entry in real Chromium with zero retries | Required by `Portal Acceptance Contract` |
| Complete Character Bazaar lifecycle | `npx playwright test tests/marketplace-acceptance.spec.mjs` from `scripts/acceptance` plus focused PHP/MariaDB tests | Executes public EN/PL catalogue/detail, authenticated watch/bid/wallet dashboard, MFA/RBAC administrator wallet/recovery and lower-layer escrow/transfer/locking/idempotency contracts | Required by ordinary acceptance and portal coverage workflows for owned changes |
| Complete Downloads lifecycle | `npm --prefix scripts/acceptance run test:downloads` plus `test:downloads-portability` | Executes public/admin/localization/failure-recovery in Chromium and bounded public reads in Firefox/WebKit | Required by `Downloads Acceptance` for owned changes |
| Complete Events lifecycle | `npx playwright test --config=playwright.events.config.mjs` from `scripts/acceptance` | Executes public EN/PL calendar/detail states and exact MFA/RBAC administrator lifecycle on Chromium D/T/M plus public Firefox/WebKit with zero retries | Required by `Events Acceptance` |
| Complete Announcements lifecycle | `npx playwright test --config=playwright.announcements.config.mjs` from `scripts/acceptance` | Executes public active-window/localization states and exact MFA/RBAC administrator create, publish, translation, stale recovery, conflict and audit lifecycle on Chromium D/T/M plus public Firefox/WebKit with zero retries | Required by `Announcements Acceptance` |
| Complete Support/Legal content lifecycle | `npx playwright test --config=playwright.support-legal.config.mjs` from `scripts/acceptance` | Executes typed public CMS routes, EN/PL isolation, legal versions, approved links and exact MFA/RBAC administrator translation/audit lifecycle | Required by `Support Legal Acceptance`; does not claim tickets/moderation |
| Complete Editorial Media lifecycle | `npx playwright test --config=playwright.editorial-media.config.mjs` from `scripts/acceptance` | Executes exact MFA/RBAC, malformed and safe upload, private content, integrity display, reference lock, deletion and bounded audit on Chromium D/T/M | Required by `Editorial Media Acceptance` |
| Complete Wiki reconciliation lifecycle | `npx playwright test --config=playwright.wiki-reconciliation.config.mjs` from `scripts/acceptance` | Executes public route/search/error/recovery/localization and administrator validation, preview, conflict, publish/unpublish, revision restore, archive and audit | Required by `Wiki Reconciliation Acceptance` |
| Strict delivered-surface and benchmark-ledger closure | `npm --prefix scripts/acceptance run test:coverage-contract:strict` | Validates every delivered route and the complete Issue #268 capability ledger; it does not require missing benchmark products to be implemented in this audit PR | Required by `Portal Acceptance Contract` |
| Existing critical acceptance | Existing `critical` profile | Browser portability, responsive, resilience, accessibility and primary smoke | Preserved |
| Existing full acceptance | Existing `full` profile | Complete primary Chromium functional baseline, resilience, accessibility and visual collection | Preserved |

## Complete account lifecycle matrix

| Capability | Required states / abuse boundaries | Primary proof | Browser evidence |
|---|---|---|---|
| Registration | initial, required validation, invalid email/password confirmation, duplicate email, success | Feature/security + database uniqueness | `account-lifecycle-acceptance.spec.mjs`, `player-journey-acceptance.spec.mjs` |
| Login | invalid credentials, success, guest/auth redirects, session rotation where observable | Feature/security | account lifecycle + existing security portability evidence |
| Logout | authenticated POST, protected route denied afterward | Feature/CSRF + browser | account lifecycle + player journey |
| Account Overview | guest denial; ready, pending, recoverable, conflict and missing binding states | Feature + read-model/integration | `account-overview-acceptance.spec.mjs` |
| Provisioning retry | retry only for persisted recoverable intent, success, no raw Canary ID/name exposure | Database/integration + feature | Account Overview retry scenario |
| Password recovery | real test SMTP, reset success, old-session invalidation, old password denial, token replay/expiry denial | Feature/security + SMTP | `password-recovery-acceptance.spec.mjs` |
| Password change | current password, success, old password denial, all existing sessions revoked | Feature/security | `password-change-acceptance.spec.mjs` |
| MFA enrollment | QR-first provisioning, manual fallback boundary, current-password confirmation, recovery codes | Unit + feature/security | player journey and MFA lifecycle |
| MFA challenge | valid TOTP, invalid code, replayed TOTP, single-use recovery code | Feature/security | `mfa-security-acceptance.spec.mjs` |
| MFA disable | fresh credential/code required, disable, sign out everywhere | Feature/security | MFA lifecycle |
| Character creation | ready binding only, validation, reserved/duplicate, quota, foreign account injection ignored, idempotent visible result | Contract + database/integration + feature | player journey + character boundaries |
| Public character visibility | newly created character shown through public read route | Contract + browser | player journey |
| Returning player | login, MFA challenge, account/character action remains reachable | Browser/system | player journey |
| Session invalidation | reset/change/MFA disable invalidates other browser contexts | Feature/security + browser | password recovery/change and MFA lifecycle |

The delivered account profile is primary-Chromium only because it contains reset links, MFA secrets, recovery codes and authenticated session material. Cross-browser expansion remains bounded to representative non-secret or safely isolated flows under ADR 0008. Email change, targeted session inventory/revocation, termination and exceptional binding management are product gaps in #276 rather than hidden exclusions from this matrix.

## Character Bazaar lifecycle matrix

| Capability | Required states / abuse boundaries | Primary proof | Browser evidence |
|---|---|---|---|
| Public catalogue/detail | active, filtered, empty, detail, bid-history empty, terminal history, not-found, EN/PL | Feature/query + manifest/SEO | `marketplace-acceptance.spec.mjs` public scenario |
| Listing eligibility | authenticated ready binding, exact server-resolved ownership, active/offline character, validation, duplicate/idempotent request, unavailable dependency | Feature + transfer contract | listing form/account surfaces; lower-layer fake/real adapter tests |
| Escrow activation | deterministic account/player locking, no cluster session, transfer to non-login escrow, delayed quiescence, second owner/session check | Real MariaDB integration + saga feature tests | user-visible pending/active states |
| Direct bidding | no self-bid, current minimum/increment, auction lock, leading reservation, outbid release, request idempotency, insufficient funds | Transactional feature/database tests | authenticated watch/bid/wallet dashboard scenario |
| Fixed-price purchase/settlement | winner binding, target quota, escrow-to-winner transfer, reserved debit, seller proceeds, commission, retry after partial step | Transfer contract + wallet/saga tests | detail purchase and final account/history state |
| Cancellation/expiry | seller-only, no existing bids, escrow return, idempotent already-returned state, no-bid expiry | Feature/saga + transfer tests | account seller actions/history |
| Administrator wallet/recovery | guest/no-MFA/no-permission denial, signed adjustment validation, atomic ledger/audit, bounded ownership recovery | Feature/security/audit + operations runbook | administrator acceptance scenario |
| Responsive/accessibility | desktop/tablet/mobile no overflow, semantic headings/labels, keyboard-reachable controls, visible focus | CSS + Playwright smoke | public, account and administrator marketplace scenarios |

Character Bazaar concurrency and cross-database failure safety are proved at the smallest deterministic layer. Browser tests do not claim to reproduce database races; they prove composed navigation, authorization, visible state and recovery affordances. Bazaar ownership transfer does not imply that general character rename/delete/restore/world-transfer products exist.

## Portal surface status

| Surface group | Current coverage | Required dimensions | Product-boundary note |
|---|---|---|---|
| Identity registration/login/logout | `covered` | guest/authenticated; validation, duplicate, invalid credentials, redirect/logout; D/T/M; bounded portability | Broader account lifecycle remains #276 |
| Password lifecycle | `covered` | recovery/change, SMTP, replay/expiry, multi-session revocation; D/M | High-assurance recovery artifact remains planned |
| MFA lifecycle | `covered` | enroll/confirm/challenge/replay/recovery/disable; D/M; bounded portability | TOTP is complete; email-code method decision remains #276 |
| Account Overview and Canary provisioning | `covered` | ready/pending/recoverable/conflict/missing; retry; no internal identifiers; D/M | No import/claim/unlink/rebind |
| Character creation and public visibility | `covered` | authorization, validation, ownership injection, quota/idempotency, public read; D/M | Character owner-management remains #277 |
| Character Bazaar public/account/admin | `covered` | public catalogue/detail/filter/empty/EN/PL; watch/list/bid/wallet/history; MFA/RBAC adjustment/recovery; escrow/session/quota/idempotency/concurrency; D/T/M | Coin purchasing and commerce remain #278 |
| Homepage, navigation and SEO | `covered` | available/empty/stale/unavailable, EN/PL, sitemap/robots; D/T/M; bounded engines | Template selector remains #244 |
| News and managed public pages | `covered` | published/hidden/empty/not-found/long/localized; D/M | None in delivered contract |
| Public game data | `covered` for current read model | search/detail/index, pagination/empty/not-found, Redis/Canary failure and recovery; D/T/M | Benchmark completeness remains #280 |
| Core Admin, RBAC, CMS and Audit | `covered` | guest/no-MFA/no-permission/exact roles, mutations, publish/hide, audit and final-admin protection; D/T/M | No moderation/ticket administration |
| Public/admin localization core | `covered` | EN/PL, missing/incomplete/draft/published/stale, route-preserving switch | Module-specific gaps are tracked separately |
| Downloads | `covered` | public/admin/localization/URL-policy/failure-recovery/audit; D/T/M; bounded engines | None in delivered contract |
| Events | `covered` | public lifecycle, localization, RBAC, conflict and audit; D/T/M; bounded engines | Server-backed annual-event catalogue remains #281 |
| Announcements | `covered` | time windows, escaping, localization, RBAC, conflict and audit; D/T/M; bounded engines | None in delivered contract |
| Support and legal content | `covered` | typed CMS routes, publication, legal version, approved links, translation and audit | Operational support/moderation is missing in #279 |
| Public Wiki | `covered` | article/category/search/error/recovery/EN/PL; D/T/M; bounded engines | Structured server-backed catalogues remain #281 |
| Wiki administration | `covered` | draft/review/publish/archive/revision/conflict/preview/audit; D/T/M; bounded engines | Data ingestion/provenance remains #281 |
| Editorial Media administration | `covered` | upload/integrity/private bytes/reference lock/delete/denial/audit; D/T/M | None in delivered contract |
| Media/preview supporting endpoints | `supporting_endpoint` | publication/reference/signature/integrity/authorization | Existing feature/storage and consumer-browser evidence |

The machine-readable delivered-surface manifest is authoritative for route classification. The machine-readable product benchmark is authoritative for benchmark capability classification. Neither may be substituted for the other.

## Product completeness overlay

The Issue #268 benchmark currently classifies 43 capabilities:

- 3 implemented;
- 11 partial;
- 29 missing;
- 23 required, 13 planned and 7 optional/differentiator.

Focused backlog ownership:

- #276 — account security and lifecycle;
- #277 — character management and public profiles;
- #278 — premium, coins and entitlement commerce;
- #279 — tickets, reports and enforcement;
- #280 — community statistics and guild workflows;
- #281 — server-backed Wiki/gameplay catalogues.

Required partial/missing capabilities prevent a product-complete claim even while every currently delivered route remains `covered`.

## Required evidence dimensions

A delivered surface is not closed merely because its happy-path URL returns HTTP 200. Its record must identify the applicable dimensions:

1. **Route and navigation** — direct URL, route-generated navigation and no dead-end recovery.
2. **Roles** — guest, authenticated, pending MFA, confirmed MFA, denied permission and each exact privileged role that changes behavior.
3. **Functional states** — success, validation, empty, stale, unavailable, conflict, not-found, rate-limited and restored state where applicable.
4. **Viewport** — desktop, tablet and mobile only where layout/interaction risk requires them.
5. **Browser** — full primary Chromium and a bounded Firefox/WebKit subset justified by portability risk.
6. **Security** — CSRF, IDOR/ownership, privilege escalation, replay, session revocation, sanitization and technical-data leakage where applicable.
7. **Persistence and concurrency** — database/integration proof for transactions, locking, uniqueness, retry/idempotency and ambiguous commits.
8. **Accessibility** — labels, semantic headings/landmarks, keyboard traversal, activation and visible focus; manual assistive-technology evidence remains separate.
9. **Failure and recovery** — known pre-state, deterministic interruption, fail-closed result, data integrity, restoration and successful subsequent use.
10. **Evidence environment** — repository, controlled staging or actual production, tied to the exact tested SHA/version.

A benchmark capability additionally requires explicit relevance, delivery status, external benchmark source, Oteryn evidence for implemented/partial claims and a focused issue for every required open gap.

## Strict closure sequence

1. Keep route classification and product-ledger validation required immediately.
2. Execute the account-lifecycle profile on every affected pull request and current main.
3. Close one delivered `planned` or `partial` route-surface package per bounded PR.
4. Update route-manifest state only after exact-head lower-layer and browser evidence passes.
5. Update benchmark delivery status only after the full benchmark capability—not merely one route—is proven.
6. Run ordinary required repository workflows and the Portal Acceptance Contract.
7. Preserve #91 as the final production-only verification boundary.

## Explicit nonclaims

The matrix does not claim:

- a mathematical guarantee that no defect exists;
- production correctness from repository or staging evidence;
- benchmark product completeness while required gaps remain;
- full screen-reader compatibility from automated checks;
- the authoritative Platform-to-game login path;
- imported/claimed legacy Canary accounts;
- email change, active-session inventory, account termination or exceptional unlink/rebind;
- character profile editing/privacy, rename, deletion/restore or general world transfer;
- authenticated support tickets, player-report queues or enforcement histories;
- rich highscore categories, deaths, kill statistics or guild administration;
- payment-provider coin purchasing, webshop, premium or entitlement commerce;
- authoritative server-backed creature/item/loot catalogues, interactive maps or Huntfinder-like tooling.
