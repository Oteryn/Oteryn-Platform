# Portal backend/frontend audit — Phase 1 inventory

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Environment classification: `REPO_MAIN` for source inspection; current-target runtime and deployment remain `UNKNOWN`.

## Result

The canonical repository inventory contains **27 portal surfaces** with **228 named-route assignments**. It is assembled from the base portal coverage manifest and six sorted module fragments, then reconciled against the backend/frontend capability ledger, dimension evidence contract and route/view/navigation closure policy.

This phase proves the static source and policy inventory. It does **not** prove the exact target runtime route table, current-target browser execution, staging deployment or production deployment because the current environment cannot run the checkout, Laravel or Playwright.

## Canonical inventory

| Surface | Owner | Named routes | Kind | Manifest claim | Portability contract | Audit-target deployment |
|---|---:|---:|---|---|---|---|
| `identity.registration-login-session` | Identity | 5 | rendered | covered | covered | UNKNOWN |
| `identity.password-lifecycle` | Identity | 6 | rendered | covered | excluded | UNKNOWN |
| `identity.mfa-lifecycle` | Identity | 6 | rendered | covered | covered | UNKNOWN |
| `account.overview-provisioning` | Accounts | 2 | rendered | covered | covered | UNKNOWN |
| `account.character-creation-and-visibility` | Accounts / Characters | 2 | rendered | covered | excluded | UNKNOWN |
| `public.home-and-seo` | PublicPortal | 4 | rendered + resources | covered | covered | UNKNOWN |
| `public.news-and-managed-pages` | CMS | 6 | rendered | covered | covered | UNKNOWN |
| `public.game-data` | PublicGameData | 14 | rendered | covered | covered | UNKNOWN |
| `admin.core-rbac-cms-audit` | Admin / CMS / RBAC / Audit | 15 | rendered | covered | covered | UNKNOWN |
| `public.localization-core` | Localization | 4 | rendered | covered | covered | UNKNOWN |
| `downloads.public-admin-localization` | Downloads | 10 | rendered | covered | covered | UNKNOWN |
| `events.public-admin` | Events | 10 | rendered | covered | covered | UNKNOWN |
| `announcements.admin-localization-home-composition` | Announcements | 7 | rendered | covered | covered | UNKNOWN |
| `support-legal.public-admin-localization` | Support / CMS | 21 | rendered | covered | covered | UNKNOWN |
| `wiki.public` | Wiki | 8 | rendered | covered | covered | UNKNOWN |
| `wiki.admin-editorial-lifecycle` | Wiki | 19 | rendered | covered | covered | UNKNOWN |
| `editorial-media.admin` | EditorialMedia | 3 | rendered | covered | excluded | UNKNOWN |
| `browser-supporting-media-preview-endpoints` | Wiki / EditorialMedia | 7 | supporting endpoint | supporting_endpoint | not applicable | UNKNOWN |
| `identity.account-security-lifecycle` | Identity | 15 | rendered | covered | excluded | UNKNOWN |
| `support.moderation-lifecycle` | Support / Moderation | 28 | rendered | covered | excluded | UNKNOWN |
| `public.community-deaths-and-policy` | PublicGameData | 2 | rendered | covered | excluded | UNKNOWN |
| `marketplace.public-catalogue-and-detail` | Marketplace | 4 | rendered conditional | covered | excluded | UNKNOWN |
| `marketplace.authenticated-auction-lifecycle` | Marketplace / Wallet / CanaryIntegration | 8 | rendered conditional | covered | excluded | UNKNOWN |
| `marketplace.admin-wallet-and-recovery` | Marketplace / Wallet / Admin / Audit | 3 | rendered conditional | covered | excluded | UNKNOWN |
| `game-catalog.public-items-creatures-and-loot` | GameCatalog | 10 | rendered | covered | covered | UNKNOWN |
| `game-catalog.administrator-inspection` | GameCatalog / Admin / Security | 7 | rendered | covered | excluded | UNKNOWN |
| `identity.character-profile-preferences` | CharacterProfiles | 2 | rendered | covered | excluded | UNKNOWN |

## Reconciliation

### Route sources

`routes/web.php` registers the core Identity, Account, CMS, Admin and public game-data routes and loads sorted `routes/modules/*.php`. `bootstrap/app.php` additionally loads API, internal and localization routes. The localization registrar converts default-locale public route names to `legacy.*` and registers canonical `/en` and `/pl` named routes.

The inspected module route sources cover Public Portal, Downloads, Events, Announcements, Support/Legal and Moderation, Wiki, Editorial Media, public community statistics, Marketplace, Game Catalog and character profile preferences.

### Supporting endpoint exceptions

The manifest classifies seven media/preview routes as one supporting endpoint group. Route/view policy additionally treats robots, sitemap and the private Wiki media-library JSON index as resource endpoints rather than standalone rendered screens.

The game-auth ticket issuance, redemption and login-context endpoints are unnamed API/internal service contracts. A standalone portal screen is not intrinsic to those endpoints.

### Direct-entry and dormant inventory

The strict route/view/navigation policy contains 30 bounded direct-entry routes, primarily token-entry screens and default-locale compatibility routes. It excludes exactly two views:

- `game-auth.oauth.authorize` — framework/protocol view;
- `home-preview` — intentional noindex design-preview reference, unreachable from delivered routing and pending separate retirement.

No other dormant or orphan view can be independently declared absent until the final route/view/navigation validator executes on the exact target.

### Conditional Marketplace boundary

Marketplace source, screens and browser contracts exist in `REPO_MAIN`, but route registration is conditional on `MARKETPLACE_ENABLED`. Testing and acceptance default it on; other runtime environments fail closed unless explicitly enabled. Therefore repository and isolated browser evidence do not prove Marketplace reachability on an arbitrary deployment.

## Browser dimension inventory

The dimension contract contains 27 records and 13 executable profile groups. It distinguishes:

- exact Chromium desktop `1440x1000`, tablet `820x1180` and mobile `390x844` mappings;
- bounded Firefox/WebKit coverage;
- bounded risk-based portability exclusions;
- supporting resource endpoints.

The critical-viewport validator requires all three Chromium viewports for every rendered surface marked critical. Missing explicit mappings may be derived only from a blocking Chromium profile that selects the same evidence file and includes exact stable markers. This explains identity records where tablet evidence is derived rather than repeated explicitly.

This is a mapping contract, not a fresh browser run on the audit target.

## Capability reconciliation

The machine backend/frontend ledger contains explicit integrated, partial, missing and not-applicable statuses. No user-facing capability inspected in that ledger is marked backend `implemented` while frontend or integration is missing.

Explicit missing or partial product capabilities include character deletion/restore, rename, world transfer, achievements, customer payments, premium/VIP and broader structured knowledge content. These are not automatically portal defects when consistently classified and not presented as delivered.

## Phase 1 findings

### OTERYN-AUDIT-P1-001 — Active work index conflicts with live state

- fact_state: `CONFLICT`
- severity: `LOW`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: `ACTIVE_WORK.md` reports no active tasks while live open PRs contain active task records and owned paths.
- recommendation: continue treating live PRs and individual task records as authoritative; repair the index in a separate governance task only if desired.

### OTERYN-AUDIT-P1-002 — Exact-target runtime closure not executed

- fact_state: `UNKNOWN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `UNKNOWN`
- impact: the static 27-surface/228-route inventory cannot be promoted to runtime route/view/navigation proof.
- recommendation: execute the strict route, dimension, backend/frontend and route/view/navigation validators from a full checkout of the frozen target.

### OTERYN-AUDIT-P1-003 — Marketplace reachability is environment-gated

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: source and acceptance evidence alone do not prove that Marketplace routes exist on a deployment.
- recommendation: require exact deployed configuration evidence for staging or production classification.

### OTERYN-AUDIT-P1-004 — Intentional dormant design preview

- fact_state: `PROVEN`
- severity: `INFO`
- confidence: `HIGH`
- environment: `REPO_MAIN`
- impact: `home-preview` is retained but intentionally unreachable and excluded by policy.
- recommendation: keep it classified as an intentional non-delivered reference or retire it in a separate bounded task; do not count it as a delivered screen.

## Phase boundary

Phase 1 static discovery is complete. Dynamic runtime route closure, current-main Wiki Issue #365 reproduction and fresh browser evidence remain blocked in this connector-only environment and retain `UNKNOWN` / `NOT_RUN`.
