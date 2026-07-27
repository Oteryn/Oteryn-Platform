# Oteryn Portal Acceptance Coverage Matrix

## Result boundary

This matrix defines exhaustive acceptance against the **currently delivered, versioned portal surface contract**.

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
- `not_delivered` — outside the current product and not represented as tested.

## Required CI contracts

| Contract | Command | Purpose | Current gate |
|---|---|---|---|
| Manifest integrity and live route classification | `npm --prefix scripts/acceptance run test:coverage-contract` | Fails for malformed records, duplicate routes, stale routes, missing evidence files/markers and unclassified named routes | Required by `Portal Acceptance Contract` |
| Complete account lifecycle | `npm --prefix scripts/acceptance run test:account-lifecycle` | Executes registration, login/logout, account overview, provisioning, password, MFA, sessions and character entry in real Chromium with zero retries | Required by `Portal Acceptance Contract` |
| Complete Downloads lifecycle | `npm --prefix scripts/acceptance run test:downloads` plus `test:downloads-portability` | Executes public/admin/localization/failure-recovery in Chromium and bounded public reads in Firefox/WebKit | Required by `Downloads Acceptance` for owned changes |
| Complete Events lifecycle | `npx playwright test --config=playwright.events.config.mjs` from `scripts/acceptance` | Executes public EN/PL calendar/detail states and exact MFA/RBAC administrator lifecycle on Chromium D/T/M plus public Firefox/WebKit with zero retries | Required by `Events Acceptance` |
| Complete Announcements lifecycle | `npx playwright test --config=playwright.announcements.config.mjs` from `scripts/acceptance` | Executes public active-window/localization states and exact MFA/RBAC administrator create, publish, translation, stale recovery, conflict and audit lifecycle on Chromium D/T/M plus public Firefox/WebKit with zero retries | Required by `Announcements Acceptance` |
| Complete Support/Legal lifecycle | `npx playwright test --config=playwright.support-legal.config.mjs` from `scripts/acceptance` | Executes all typed public routes, EN/PL isolation, legal versions, approved links and exact MFA/RBAC administrator translation/audit lifecycle with zero retries | Required by `Support Legal Acceptance` |
| Complete Editorial Media lifecycle | `npx playwright test --config=playwright.editorial-media.config.mjs` from `scripts/acceptance` | Executes exact MFA/RBAC, malformed and safe upload, private content, integrity display, reference lock, deletion and bounded audit on Chromium D/T/M with zero retries | Required by `Editorial Media Acceptance` |
| Complete Wiki reconciliation lifecycle | `npx playwright test --config=playwright.wiki-reconciliation.config.mjs` from `scripts/acceptance` | Executes public route/search/error/recovery/localization and administrator validation, preview, conflict, publish/unpublish, revision restore, archive and audit on the required browser/viewport matrix with zero retries | Required by `Wiki Reconciliation Acceptance` |
| Strict delivered-surface closure | `npm --prefix scripts/acceptance run test:coverage-contract:strict` | Fails while any delivered required surface is `partial` or `planned` | Required by `Portal Acceptance Contract` |
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

The account profile is primary-Chromium only because it contains reset links, MFA secrets, recovery codes and authenticated session material. Cross-browser expansion remains bounded to representative non-secret or safely isolated flows under ADR 0008.

## Portal surface status

| Surface group | Current coverage | Required dimensions | Remaining exact work |
|---|---|---|---|
| Identity registration/login/logout | `covered` | guest/authenticated; validation, duplicate, invalid credentials, redirect/logout; D/T/M; bounded portability | None in current contract |
| Password lifecycle | `covered` | recovery/change, SMTP, replay/expiry, multi-session revocation; D/M | None in current contract |
| MFA lifecycle | `covered` | enroll/confirm/challenge/replay/recovery/disable; D/M; bounded portability | Manual screen-reader verification remains outside automated proof |
| Account Overview and Canary provisioning | `covered` | ready/pending/recoverable/conflict/missing; retry; no internal identifiers; D/M | None in current contract |
| Character creation and public visibility | `covered` | authorization, validation, ownership injection, quota/idempotency, public read; D/M | Authoritative game login remains separate |
| Homepage, navigation and SEO | `covered` | available/empty/stale/unavailable, EN/PL, sitemap/robots; D/T/M; Chromium/Firefox/WebKit bounded | None in current contract |
| News and managed public pages | `covered` | published/hidden/empty/not-found/long/localized; D/M | None in current contract |
| Public game data | `covered` | search/detail/index, pagination/empty/not-found, Redis/Canary failure and recovery; D/T/M | None in current contract |
| Core Admin, RBAC, CMS and Audit | `covered` | guest/no-MFA/no-permission/exact roles, mutations, publish/hide, audit and final-admin protection; D/T/M | None in current contract |
| Public/admin localization core | `covered` | EN/PL, missing/incomplete/draft/published/stale, route-preserving switch | Module-specific translation gaps remain below |
| Downloads | `covered` | public empty/current/platform filter; admin create/publish; URL-policy failure/recovery; EN/PL; guest/no-MFA/no-permission; audit; D/T/M; bounded Firefox/WebKit | None in current contract |
| Events | `covered` | public empty/calendar/detail; active/upcoming/archived/cancelled; EN/PL isolation; guest/no-MFA/no-permission/manage/publish roles; validation/draft/publish/edit-to-draft/conflict/audit; Chromium D/T/M and public Firefox/WebKit | None in current contract |
| Announcements | `covered` | active/empty/future/expired/draft visibility; escaped text; EN/PL; exact MFA/RBAC; validation/create/publish/source edit/stale translation recovery/409/audit; Chromium D/T/M and public Firefox/WebKit | None in current contract |
| Support and legal | `covered` | every typed route, published/missing/unpublished, legal version, approved links, admin and PL translation; D/T/M; bounded public Firefox/WebKit | None in current contract |
| Public Wiki | `covered` | home/category/article/search/error/unavailable/recovery/EN/PL; D/T/M; Chromium/Firefox/WebKit | None in current contract |
| Wiki administration | `covered` | draft/review/publish/unpublish/archive/revision restore/conflict/permissions/preview/audit; D/T/M; Chromium/Firefox/WebKit | None in current contract |
| Editorial Media administration | `covered` | upload validation/integrity, private library/content/thumbnail, delete/reference lock, denial and audit; D/T/M | None in current contract |
| Media/preview supporting endpoints | `supporting_endpoint` | publication/reference/signature/integrity/authorization | Existing feature/storage and consumer-browser evidence |

The machine-readable manifest is authoritative if this summary and the manifest conflict.

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

## Strict closure sequence

1. Keep classification validation required immediately.
2. Execute the account-lifecycle profile on every affected pull request and current main.
3. Close one `planned` or `partial` surface package per bounded PR.
4. Update its manifest state only after exact-head lower-layer and browser evidence passes.
5. Run the ordinary required repository workflows and the portal coverage contract.
6. When every delivered required surface is `covered` or a reviewed `supporting_endpoint`, enable strict validation as a release gate.
7. Preserve issue #91 as the final production-only verification boundary.

## Explicit nonclaims

The matrix does not claim:

- a mathematical guarantee that no defect exists;
- production correctness from repository or staging evidence;
- full screen-reader compatibility from automated checks;
- the authoritative Platform-to-game login path;
- imported/claimed legacy Canary accounts;
- account deletion, unlink/rebind/transfer;
- character rename/deletion;
- payments, webshop or other deferred commerce.
