# Oteryn Platform Frontend Completeness Addendum — 2026-07-30

## Purpose

This addendum makes frontend delivery a mandatory part of the Issue #268 product-completeness audit. A working controller, service, endpoint, migration or background operation is not sufficient evidence that a user-facing capability is implemented.

Repository: `blakinio/Oteryn-Platform`

Parent audit: `docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md`

Audited `main`: `f90bb8075b300569b7d493c84f0080e6b3295c35`

Audit pull request: #315

Machine-enforcement issue: #340 under parent #326

## Mandatory integrated-delivery rule

A user-facing capability may be classified `IMPLEMENTED` only when all of the following are proven:

1. **Backend implementation** — the route, controller/request validation, domain/service behavior, persistence or authoritative read adapter and authorization rules exist.
2. **Frontend implementation** — a real rendered screen or interaction is connected to the backend route. A dormant component, unused view, mock, design file or route without reachable navigation does not qualify.
3. **State completeness** — the frontend represents applicable success, validation, unauthorized, forbidden, conflict, empty, unavailable, not-found and recovery states without exposing internal details.
4. **Browser evidence** — the integrated user flow has zero-retry browser evidence at the required viewport and locale boundary.
5. **Accessibility and responsive behavior** — keyboard/focus and responsive behavior are covered according to the route-risk profile.

Classification consequences:

- backend complete but no connected frontend: `PARTIAL` for a required user-facing capability;
- frontend files appear to exist but no reliable integrated/browser evidence exists: `UNTESTED`;
- neither backend nor frontend exists and the capability is required: `MISSING_REQUIRED`;
- approved future capability with neither complete backend nor frontend: `PLANNED`;
- API, webhook, worker or supporting resource endpoint may omit a standalone screen only when that is intrinsic to the capability and explicitly justified.

No capability may be promoted to `IMPLEMENTED` solely because unit, feature or API tests are green.

## Evidence model

The audit must record these columns for every user-facing capability:

| Evidence column | Required proof |
|---|---|
| Backend | route/controller/domain/persistence or authoritative read path |
| Frontend | reachable rendered view and user interaction |
| Integration | frontend invokes the real route and presents the committed result |
| States | applicable negative, empty, error and recovery states |
| Browser | exact-SHA zero-retry evidence with declared viewport/browser/locale |
| Final classification | lowest truthful status across backend, frontend and evidence |

The authoritative frontend inventory is built from actual named routes and rendered views, then reconciled with `scripts/acceptance/coverage/portal-coverage-manifest.json` and fragments under `scripts/acceptance/coverage/surfaces/`. Menu entries and documentation claims are not implementation proof.

## Machine-enforced cross-ledger slice

Issue #340 introduces the first fail-closed implementation of this rule:

- `docs/testing/product-backend-frontend-completeness.json` contains exactly one backend/frontend/integration record for every canonical product capability;
- `scripts/acceptance/coverage/validate-backend-frontend-completeness.mjs` cross-checks those records against `docs/testing/product-completeness-benchmark.json` and the actual portal surface manifest/fragments;
- user-facing product capabilities marked `implemented` must have backend, frontend and integration statuses all `implemented`;
- integrated records must reference existing `covered` surfaces with stable Playwright evidence markers;
- machine-to-machine or background capabilities without a standalone screen require a bounded `exception_reason`;
- `scripts/acceptance/coverage/test-backend-frontend-completeness.mjs` proves deterministic rejection of backend-only promotion, unknown surfaces, missing exception rationale, cross-ledger status contradiction and missing records;
- strict Portal Acceptance runs the product, route and backend/frontend validators on the exact PR SHA.

This enforcement closes only the cross-ledger promotion risk. It does not close the remaining every-rendered-screen, every-state, every-browser or production matrix owned by parent #326.

## Current backend–frontend reconciliation

The following matrix records the current integrated boundary at the audited revision. `Integrated` means both backend and reachable frontend are present for the stated delivered scope. It does not imply exhaustive every-state visual proof; remaining exhaustive evidence is owned by #326.

| Product surface | Backend | Reachable frontend | Integrated browser evidence | Current result |
|---|---|---|---|---|
| Registration, login and logout | present | present | zero-retry account lifecycle | `IMPLEMENTED` |
| Password recovery and password change | present | present | token, validation and session-revocation flows | `IMPLEMENTED` |
| TOTP MFA enrollment and challenge | present | present | enrollment, challenge, recovery and replay states | `IMPLEMENTED` |
| Account overview and provisioning | present | present | pending, ready, recoverable, conflict and missing states | `IMPLEMENTED` |
| Email change, sessions, privacy, recovery key and account termination | present | present | EN/PL account-security browser lifecycle | `IMPLEMENTED` |
| Character creation | present | present | validation, quota, authorization and idempotent outcome | `IMPLEMENTED` |
| Character profile preferences | present | present | owner update and public profile desktop/tablet/mobile evidence | `IMPLEMENTED` |
| Home, navigation, SEO, news and managed pages | present | present | public and localization acceptance | `IMPLEMENTED` |
| Highscores, character search/detail, guilds, online, servers, deaths and kills | present | present | community-data and dependency recovery acceptance | `IMPLEMENTED` |
| Downloads | present | present | public/admin/localization and portability acceptance | `IMPLEMENTED` |
| Events | present | present | public/admin desktop/tablet/mobile plus bounded portability | `IMPLEMENTED` |
| Announcements | present | present | public/admin/localization and responsive acceptance | `IMPLEMENTED` |
| Support and legal publishing | present | present | public/admin/EN/PL acceptance | `IMPLEMENTED` |
| Tickets, reports, moderation, enforcement and appeals | present | present | user/moderator/notification/privacy responsive lifecycle | `IMPLEMENTED` |
| Wiki public and editorial administration | present | present | public/admin/search/revision/media browser lifecycle | `IMPLEMENTED` |
| First Game Catalog scope: items, weapons, creatures and loot | present | present | public/admin/localization/responsive acceptance | `IMPLEMENTED` for the first scope |
| Character Bazaar public, account and administrator flows | present | present | catalogue/listing/bidding/settlement/recovery acceptance | `IMPLEMENTED` |
| Admin dashboard, RBAC, CMS and audit | present | present | guest/MFA/permission/CMS/audit browser acceptance | `IMPLEMENTED` for delivered operations |
| Exhaustive error, long-data, media-failure and every-browser matrix | broad backend handling | broad frontend handling | not universal for every rendered route/state combination | `PARTIAL`; #326 |

## Confirmed capabilities without complete frontend

The following capabilities must not be described as implemented because the complete user-facing lifecycle is absent:

| Capability | Backend status | Frontend status | Classification | Owner |
|---|---|---|---|---|
| Character deletion, grace and restore | absent operation contract | no owner lifecycle UI | `MISSING_REQUIRED` | #317 |
| Character rename, reservation, cooldown and history | absent operation contract | no owner lifecycle UI | `MISSING_REQUIRED` | #319 |
| Controlled world/channel transfer | product/authority model unresolved | no eligibility, destination or history UI | `PLANNED` | #320 |
| Featured earned achievements | authoritative catalogue/source absent | no owner selection or public section | `PLANNED` | #301, #323 |
| Customer payments | provider/security lifecycle absent | no real checkout or payment-history UI | `PLANNED` | #321 |
| Products, VIP/Premium, coin packages, vouchers and services | entitlement lifecycle absent | no catalogue, redemption or service-history UI | `PLANNED` | #322 |
| Structured spells, NPCs, quests and achievements | authoritative data contract incomplete | no complete catalogue/detail surfaces | `PLANNED` | #301 |
| Maps, Huntfinder-style tools and optional discovery systems | not adopted/contracted | no delivered screens | `MISSING_OPTIONAL` / `PLANNED` | #302 |
| Loyalty and account badges | authoritative model absent | no account/public presentation | `MISSING_OPTIONAL` / `PLANNED` | #325 |

## Frontend verification requirements for future issues

Every new user-facing implementation issue must include:

- exact route and navigation entry ownership;
- server-rendered or client-rendered screen ownership;
- request validation and safe error mapping;
- loading/pending state where asynchronous behavior exists;
- empty, forbidden, conflict, unavailable, not-found and retry/recovery states where applicable;
- EN/PL behavior;
- desktop, tablet and mobile acceptance;
- keyboard navigation and visible focus;
- zero-retry critical browser tests;
- explicit proof that the screen is connected to the production code path rather than a test-only mock;
- audit verification that no backend-only capability was incorrectly promoted.

## Remaining frontend evidence gap

Issue #326 remains required because current browser evidence is broad but does not yet prove the complete Cartesian matrix of every rendered screen against every requested viewport, engine, long-data, 500, missing-image and broken-image state.

This gap does not invalidate the integrated flows already proven. It prevents claiming exhaustive frontend completeness and requires any unproven surface/state to remain `PARTIAL` or `UNTESTED` rather than being inferred from backend coverage.

## Claim boundary

- `CONTRACT_TESTED`: only when the declared backend and frontend route boundary is integrated and tested.
- `PRODUCT_COMPLETE`: false while required backend or frontend lifecycles are missing.
- `STAGING_PROVEN`: only for exact deployed/tested boundaries explicitly recorded elsewhere.
- `PRODUCTION_PROVEN`: false without direct exact-release production verification under #91.
