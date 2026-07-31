# Oteryn Platform portal backend/frontend audit

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Parent: Issue `#326`  
Related evidence/defect: Issue `#365`  
Frozen audit target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`

## 1. Executive summary

The current repository has a broad, internally consistent portal implementation and a mature machine-enforced acceptance architecture:

- 27 canonical surface groups;
- 228 classified route assignments;
- 126 rendered routes;
- 95 bound views;
- zero orphan views in the recovered strict route/view/navigation run;
- 43 canonical product capabilities, including 23 integrated, 3 partial, 14 missing and 3 not applicable;
- no statically identified user-facing backend-only or frontend-only capability promoted to implemented;
- complete media applicability closure for all 27 surfaces;
- broad zero-retry browser evidence across Chromium, Firefox, WebKit and three required viewports.

The audit does **not** establish exhaustive every-screen/state visual completeness or production correctness.

Confirmed findings:

- one historical `HIGH`: Wiki administrator thumbnail requests returned HTTP 500 on two exact historical CI heads;
- four `MEDIUM` findings: content-scale closure omits nine canonical surfaces, the global error matrix omits HTTP 503, accessibility evidence is not fail-closed per delivered surface, and historical mobile Wiki publication lost transient success feedback after durable success;
- one `LOW` governance conflict: `ACTIVE_WORK.md` conflicts with live PR/task state.

The current audit target was not directly executed for the focused Issue #365 reproduction. Its Wiki state remains `UNKNOWN`. No production claim is made.

## 2. Audit identity and environments

| Classification | Exact identity | Result |
|---|---|---|
| `REPO_MAIN` | `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` | frozen source audit target |
| `CI_PROVEN` | `fdb45a4325949d3ab1c4860e3a4527553f11c789` | strict contract and critical browser artifacts passed |
| `DERIVED` runtime equivalence | direct CI source → audit target | only docs and byte-identical Marketplace config differ |
| `STAGING_PROVEN` | `717977f252b09b9b2e979f8110b7f48b88682223` | exact staging control evidence, not audit-target deployment |
| `PRODUCTION_PROVEN` | none | not established |
| `UNKNOWN` | audit-target staging/production and current-target Wiki reproduction | insufficient direct evidence |

The audit target remained frozen even when `main` advanced. Later commits or open PRs are deltas, not part of `REPO_MAIN` for this report.

## 3. Scope and exclusions

Included:

- public portal, content modules and public game data;
- Identity and Account Center;
- character owner surfaces;
- Administration, RBAC, CMS, moderation, Wiki, Editorial Media, Marketplace and Game Catalog;
- backend/frontend/integration reconciliation;
- route/view/navigation inventory;
- browser, viewport, error, media, content-scale, resilience and accessibility evidence;
- staging and production evidence boundaries;
- historical and current-target classification of Issue #365.

Explicitly excluded:

- implementation or repair of findings;
- Canary mutation or other repository writes;
- payment-provider implementation;
- optional or planned product capabilities as automatic defects;
- merge, deployment or production operation;
- promotion of open-PR code into `REPO_MAIN`;
- production claims from repository, CI or staging evidence.

## 4. Canonical surface inventory

The inventory contains 27 groups and 228 named-route assignments.

| Surface | Routes | Type | Repository classification |
|---|---:|---|---|
| `identity.registration-login-session` | 5 | rendered | covered |
| `identity.password-lifecycle` | 6 | rendered | covered |
| `identity.mfa-lifecycle` | 6 | rendered | covered |
| `account.overview-provisioning` | 2 | rendered | covered |
| `account.character-creation-and-visibility` | 2 | rendered | covered |
| `public.home-and-seo` | 4 | rendered/resources | covered |
| `public.news-and-managed-pages` | 6 | rendered | covered |
| `public.game-data` | 14 | rendered | covered |
| `admin.core-rbac-cms-audit` | 15 | rendered | covered |
| `public.localization-core` | 4 | rendered | covered |
| `downloads.public-admin-localization` | 10 | rendered | covered |
| `events.public-admin` | 10 | rendered | covered |
| `announcements.admin-localization-home-composition` | 7 | rendered | covered |
| `support-legal.public-admin-localization` | 21 | rendered | covered |
| `wiki.public` | 8 | rendered | covered |
| `wiki.admin-editorial-lifecycle` | 19 | rendered | covered |
| `editorial-media.admin` | 3 | rendered | covered |
| `browser-supporting-media-preview-endpoints` | 7 | supporting endpoints | supporting_endpoint |
| `identity.account-security-lifecycle` | 15 | rendered | covered |
| `support.moderation-lifecycle` | 28 | rendered | covered |
| `public.community-deaths-and-policy` | 2 | rendered | covered |
| `marketplace.public-catalogue-and-detail` | 4 | rendered/conditional | covered |
| `marketplace.authenticated-auction-lifecycle` | 8 | rendered/conditional | covered |
| `marketplace.admin-wallet-and-recovery` | 3 | rendered/conditional | covered |
| `game-catalog.public-items-creatures-and-loot` | 10 | rendered | covered |
| `game-catalog.administrator-inspection` | 7 | rendered | covered |
| `identity.character-profile-preferences` | 2 | rendered | covered |

Marketplace route registration depends on `MARKETPLACE_ENABLED`. Repository presence and acceptance tests do not by themselves prove reachability on any deployment.

Machine/API game-auth ticket issuance, redemption and login-context endpoints are intrinsic service contracts and do not require standalone portal screens.

## 5. Backend / Frontend / Integration / States / Browser / Deployment matrix

The full machine-readable matrices are:

- `phase-1-surface-inventory.json`;
- `phase-2-capability-reconciliation.json`;
- `phase-3-5-state-browser-evidence.json`.

Summary by product domain:

| Domain | Implemented | Partial | Missing | N/A | Backend/frontend mismatch |
|---|---:|---:|---:|---:|---:|
| Account | 7 | 0 | 1 | 1 | 0 |
| Character | 5 | 0 | 4 | 0 | 0 |
| Commerce | 0 | 2 | 4 | 0 | 0 |
| Support | 4 | 0 | 0 | 0 | 0 |
| Public | 6 | 0 | 0 | 2 | 0 |
| Knowledge | 1 | 1 | 5 | 0 | 0 |

For all 27 surfaces:

- source and manifest classification are `PROVEN`;
- direct strict runtime contract is `CI_PROVEN` on the exact CI source;
- equivalence to the frozen target is `DERIVED`;
- audit-target deployment is `UNKNOWN`;
- production is `UNKNOWN`.

## 6. Findings ordered by severity

### HIGH

#### OTERYN-AUDIT-P35-006 — Historical Wiki thumbnail requests returned HTTP 500

```yaml
id: OTERYN-AUDIT-P35-006
title: Historical Wiki editorial thumbnail requests returned HTTP 500
fact_state: PROVEN
severity: HIGH
confidence: HIGH
environment: CI_PROVEN
surface: wiki.admin-editorial-lifecycle and browser-supporting-media-preview-endpoints
capability: approved-media thumbnail rendering
exact_sha: 35f39b48233b186502cbdcc05aec7ffc40e78fc7 and fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb
backend_status: historical thumbnail endpoints returned HTTP 500
frontend_status: affected previews were unavailable
integration_status: multiple failures occurred during the historical Wiki page loads
state_coverage: historical reproduction proven; audit-target state UNKNOWN
evidence: Issue #365; runs 30562698853 and 30578806660
impact: a core administrator media workflow experienced repeated server errors
recommendation: reproduce on the frozen target and inspect Laravel/server logs independently from flash/session behavior
suggested_followup_task: existing Issue #365
overlaps: [Issue #365]
```

### MEDIUM

#### OTERYN-AUDIT-P35-001 — Content-scale closure omits nine canonical surfaces

The strict content-scale validator loads only the 18 base-manifest surfaces, while the canonical inventory contains 27. Nine fragment surfaces can remain unclassified for long values, large collections, pagination and wrapping while the strict command passes.

Recommendation: one #326 acceptance-evidence remediation task should load the canonical manifest plus all sorted fragments and require exactly one applicability/evidence record per surface.

#### OTERYN-AUDIT-P35-002 — Dedicated global error matrix omits HTTP 503

A localized 503 view and bounded public dependency-failure scenario exist. However, HTTP 503 is absent from the dedicated EN/PL desktop/tablet/mobile matrix that verifies noindex, recovery, overflow and sensitive-disclosure properties.

Recommendation: add 503 to the existing deterministic global error contract without test-only routes.

#### OTERYN-AUDIT-P35-003 — Accessibility evidence is not fail-closed per surface

Nine representative zero-retry accessibility scenarios pass. No complete one-record-per-rendered-surface applicability/evidence ledger was found, and reduced-motion applicability remains unknown.

Recommendation: add per-surface applicability records with explicit not-applicable justification where valid.

#### OTERYN-AUDIT-P35-005 — Historical mobile Wiki publication lost transient success feedback

On two exact CI heads, the publish POST succeeded and the redirected form showed `Published`, version 3 and `Unpublish to draft`, but the expected accessible `role=status` publication message was absent.

Recommendation: reproduce on the frozen target with request/session logs. Do not infer a cause from the historical symptom.

### LOW

#### OTERYN-AUDIT-P1-001 — Active-work index conflicts with live state

`ACTIVE_WORK.md` reported no active tasks while live open PRs contained active task records and owned paths. Live PR/task state was used as authoritative.

### INFO / UNKNOWN

- `home-preview` is an intentional unreachable noindex design reference, not a delivered screen.
- Marketplace reachability is environment-gated.
- No backend-only or frontend-only implemented promotion was found among 43 canonical capabilities.
- Character deletion/restore, rename, world transfer and achievements are truthfully classified as missing.
- Bazaar wallet/history does not establish customer payment commerce.
- PR #338 NPC/shop consumer and PR #328 rename contract remain `OPEN_PR_ONLY`.
- Critical browser evidence is broad but explicitly not full visual acceptance.
- Current audit-target Issue #365 reproduction is `UNKNOWN` / `NOT_RUN`.

## 7. Evidence-state summary

### PROVEN

- frozen repository source inventory;
- 27 surface and 43 capability ledger structures;
- direct strict CI closure on exact source `fdb45a...`;
- broad zero-retry critical browser evidence on exact source `fdb45a...`;
- exact staging source and control evidence for `717977...`;
- historical Wiki flash loss and thumbnail 500 evidence;
- the listed evidence-contract gaps.

### DERIVED

- runtime-code equivalence from direct CI source `fdb45a...` to frozen audit target, based on changed-file comparison and byte-identical Marketplace configuration;
- risk that content-scale/accessibility omissions may allow a future unclassified surface to pass.

### UNKNOWN

- exact deployed staging state of the audit target;
- exact production release and availability;
- audit-target reproduction of either Issue #365 symptom;
- local installed PHP, Composer, Node and npm versions in this connector-only session;
- reduced-motion applicability per surface;
- full every-screen visual result.

### CONFLICT

- `ACTIVE_WORK.md` versus live open PR/task state.

## 8. Backend-only, frontend-only, unreachable and open-PR-only inventory

### Backend-only user-facing implemented capabilities

None found in the canonical 43-capability ledger.

### Frontend-only implemented capabilities

None found in the canonical 43-capability ledger.

### Supporting non-UI endpoints

- game-auth ticket issuance;
- internal ticket redemption;
- internal login-context read;
- browser media/preview resources.

Each has an intrinsic supporting/API contract rather than a standalone screen.

### Unreachable or dormant views

- `home-preview`: intentional noindex design reference, excluded from delivered routing;
- `game-auth.oauth.authorize`: protocol/framework view exclusion.

The recovered strict validator found zero unclassified orphan views beyond the explicit exclusions.

### Open-PR-only capabilities

- PR #338: inactive Game Catalog schema 1.3 NPC/shop consumer and administrator diagnostics; no public NPC/shop frontend;
- PR #328: character rename architecture contract; no rename implementation;
- PR #335: Synology boot recovery operation, not a delivered portal feature;
- PR #116: scheduled E2E evidence task, not product implementation.

## 9. Tests, workflows and artifacts

### Direct portal contract evidence

- Portal Acceptance Contract run `30633216358`;
- strict job `91164376176`;
- artifact `8794204786`;
- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- result: PASS.

### Direct browser evidence

- Acceptance E2E run `30633216753`;
- job `91164367653`;
- artifact `8794373786`;
- exact source `fdb45a4325949d3ab1c4860e3a4527553f11c789`;
- smoke `7/7`, portability `36/36`, responsive `42/42`, resilience `2/2`, accessibility `9/9`;
- retries: zero;
- full and visual profiles: skipped.

### Audit PR validation

On audit PR head `220ae3d231d4269bf80fc51409f5b3b95a7975be`:

- Phase 7 Production-Like Validation `30648697391`: PASS;
- Agent Governance `30648697395`: PASS;
- Game Auth Ticket Concurrency `30648697408`: PASS;
- Edge Security Emulation `30648697440`: PASS;
- Platform DB Outage Validation `30648697392`: PASS;
- CI `30648697401`: PASS.

### Historical Wiki evidence

- run `30562698853`, job `90939481510`, artifact `8767657461`;
- run `30578806660`, job `90993603962`, artifact `8773887288`.

## 10. Staging and production

### Staging

The latest directly proven staging source is `717977f252b09b9b2e979f8110b7f48b88682223`, with control run `30633745660`, job `91166065335` and sanitized artifact `8794683627`.

This exact staging evidence includes Marketplace enabled and required staging probes, but it does not establish deployment of the frozen audit target. The audit target's staging classification remains `UNKNOWN`.

### Production

No production operation was performed. No direct exact deployed release evidence satisfying Issue #91 was found. Production is `UNKNOWN`, not failed.

## 11. Audit limitations

- No local checkout was available in this session because GitHub DNS resolution failed.
- Preserved workflow artifacts were used instead of rerunning Laravel and Playwright locally.
- The focused current-target Wiki scenario and application/server log capture were not executed.
- Direct strict/browser CI is on a runtime-equivalent predecessor rather than the frozen target itself.
- Full primary acceptance and exploratory visual profiles were explicitly skipped in the recovered critical artifact.
- No production access or execution was authorized.
- Fresh independent validator-session evidence is not yet available.

## 12. Recommended further tasks

Use the minimum safe set:

1. **One acceptance-evidence remediation task under Issue #326**
   - ownership: acceptance coverage manifests/validators/tests only;
   - scope: canonical fragment-aware content-scale closure, HTTP 503 global error evidence, fail-closed per-surface accessibility applicability/evidence;
   - dependency: frozen 27-surface inventory;
   - order: after audit validation confirms the findings.

2. **Continue existing Issue #365 without creating a duplicate task**
   - ownership: Wiki publication/session and Editorial Media thumbnail paths after cause is proven;
   - scope: focused current-target reproduction, sanitized request/application/server logs, independent cause classification;
   - order: reproduce before repair;
   - do not combine flash and thumbnail repair unless one shared cause is proven.

No new tasks are recommended for truthfully classified intentional product gaps; retain their existing issue ownership. Governance index repair is optional and low priority.

## 13. Independent validation

Status: `PENDING`.

A fresh checkout-capable validator session must:

- verify frozen target `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`;
- execute exact-target route/ledger/state/browser commands;
- reproduce or classify Issue #365 on that target;
- attempt to disprove the HIGH and MEDIUM findings;
- confirm severity, deduplication and environment separation;
- persist a separate validation artifact on the same task and branch.

Until that artifact exists, the audit is evidence-complete for the available connector/CI sources but not final-complete against the task's independent-validation acceptance criterion.
