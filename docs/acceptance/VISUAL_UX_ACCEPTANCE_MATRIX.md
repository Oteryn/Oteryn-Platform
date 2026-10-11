# Visual / UI / UX Acceptance Matrix

## Current result

**Visual / UX Acceptance — PASS for the currently delivered staging-verifiable launch scope.**

**Functional Acceptance — STAGING_PROVEN independently.**

**Public launch readiness — NO.** The final Production Go-Live Gate and production-only smoke remain pending against the exact deployed production SHA. No staging or production-like browser evidence in this matrix is promoted to `PRODUCTION_PROVEN`.

The original baseline assessment correctly classified the pre-remediation frontend as a technical MVP with launch-blocking visual and UX gaps. Merged PR #77 resolved those blockers for the delivered surfaces. Issue #81 / PR #86 adds the previously missing authenticated Account Overview and user-facing provisioning/binding status surface. The composed browser evidence now closes the requested Visual/UX launch gate without changing the independent production-verification gate.

## Evidence boundary

### Delivered-surface remediation evidence

Merged PR #77 established the post-remediation UI baseline and passed exact implementation-head CI, Agent Governance, Phase 7 production-like validation, Platform DB outage validation, full browser acceptance and the exploratory Visual/Accessibility collector. Its final collector produced 71 screenshots with zero measured status mismatch, document-level overflow, unlabeled-control, sampled low-contrast, focus-not-observed or raw-technical-message surfaces.

### Account Overview / provisioning-status evidence

Issue #81 implementation PR #86 application head after the final harness correction: `ce98fe22fb57eed0f2971128337304415ba3b59f`.

That implementation head passed:

- CI run `29827433313`;
- Agent Governance run `29827433334`;
- Phase 7 Production-Like Validation run `29827433316`;
- Platform DB Outage Validation run `29827433319`;
- PR browser smoke run `29827433308`.

Full production-like browser and Visual/Accessibility evidence was collected by validation-only PR #87 on SHA `81c39f8bde3045eb3240dbe300b6b5c54c7c7cd7`, run `29827467074`, job `88624016224`, artifact `acceptance-e2e-29827467074-1` (`8494039432`), digest `sha256:0bc455cdc65b8fbafa9796b263627246485269a563d3d703f2303265f371b72b`.

The validation-only branch was created directly from the PR #86 application head before a harness-only intended-redirect assertion correction. The same correction was then mirrored byte-identically on both branches; the validation branch's only intentional non-product difference is forcing `ACCEPTANCE_PROFILE=full` in `.github/workflows/acceptance-validation.yml`, because the available connector did not expose `workflow_dispatch`. No application/runtime implementation differs for the full evidence run.

Full evidence result:

| Check | Result |
|---|---:|
| Playwright tests | 13 passed, 0 failed |
| Dedicated Account Overview screenshots | 11 |
| Existing full Visual/Accessibility screenshots | 71 |
| HTTP status mismatches | 0 |
| Document-level horizontal-overflow surfaces | 0 |
| Unlabeled-control surfaces | 0 |
| Sampled low-contrast core surfaces | 0 |
| Interactive surfaces without observed focus | 0 |
| Raw technical-message surfaces | 0 |
| Browser-error surfaces | 6 expected resource messages for intentional 403/404/503 responses; 0 page errors |

The Account Overview acceptance matrix directly exercised desktop `1440x1000` and mobile `390x844` states for `ready`, `pending`, `recoverable`, `conflict` and `missing`, plus a successful recoverable retry. It verified that synthetic legacy engine account IDs and provisioning names are not present in rendered page text.

The 71-screen exploratory collector also covered desktop/mobile and materially distinct tablet surfaces at `768x1024`. Browser authentication used loopback HTTP with `SESSION_SECURE_COOKIE=false` only because `php artisan serve` does not terminate TLS; secure-cookie production behavior remains covered by the independent Phase 7 staging evidence.

## Delivered surface inventory

### Public

- Home with embedded character search.
- News list and detail.
- Online characters.
- Highscores.
- Servers/runtime status.
- Character detail.
- Guild detail.
- Managed public page.

### Identity and account

- Registration and login.
- Password recovery request and reset.
- Authenticated password change.
- MFA challenge, settings/enrollment, confirmation and recovery codes.
- Authenticated Account Overview.
- Provisioning/binding status: ready, pending/in-progress, recoverable dependency interruption, hard conflict/support-required and missing-state/support-required.
- Contract-authorized retry for recoverable pending provisioning.
- Character creation when a ready binding authorizes it.

### Administrator

- Admin dashboard.
- News/CMS list and create/edit form.
- Managed-page list and create/edit form.
- Role management.
- Audit log.

### Error, empty and dependency states

- Validation and authentication errors.
- News empty state.
- Public-game-data empty branches.
- Runtime dependency unavailable/unknown state.
- Controlled dependency failure `503`.
- Authorization denied `403`.
- Not found `404`.
- Administrator mutation success/error states.
- Account provisioning safe non-ready and failure states.

## Visual quality taxonomy

The matrix uses:

- `PRODUCTION_READY` — visually and interactively suitable for the current launch scope, without implying production deployment proof.
- `FUNCTIONAL_BUT_NEEDS_POLISH` — usable and non-blocking, with optional refinement opportunities.
- `UX_BLOCKER` — launch-blocking interaction or navigation defect.
- `VISUAL_BLOCKER` — launch-blocking presentation defect.

No current delivered surface is classified `UX_BLOCKER` or `VISUAL_BLOCKER` by the composed evidence.

## Per-surface classification

| Surface | Required viewport/state evidence | Current result | Classification |
|---|---|---|---|
| Home | D/M + public navigation | Coherent shell, auth/account discovery, responsive | `PRODUCTION_READY` |
| Character search interaction | Home/search + detail/not-found | Usable embedded interaction; dedicated results page remains optional | `FUNCTIONAL_BUT_NEEDS_POLISH` |
| News list | D/M + empty | Responsive cards, pagination/empty state usable | `PRODUCTION_READY` |
| News detail | D/M + long content | Readable and resilient; breadcrumb remains optional polish | `FUNCTIONAL_BUT_NEEDS_POLISH` |
| Online | D/M + controlled 503 | Responsive; safe product-owned dependency failure | `PRODUCTION_READY` |
| Highscores | D/T/M | Table contained without document overflow | `PRODUCTION_READY` |
| Servers | D/T/M + runtime dependency failure | Long content bounded; safe text status | `PRODUCTION_READY` |
| Character detail | D/M + 404 | Responsive and recoverable; richer search-again affordance optional | `FUNCTIONAL_BUT_NEEDS_POLISH` |
| Guild detail | D/T/M | Member data contained responsively | `PRODUCTION_READY` |
| Managed public page | D/M | Long content resilient; global discovery remains content/navigation policy | `FUNCTIONAL_BUT_NEEDS_POLISH` |
| Login | D/M + validation | Unified identity shell and recovery navigation | `PRODUCTION_READY` |
| Registration | D/M + validation | Unified shell and clear next account entry | `PRODUCTION_READY` |
| Password recovery | D/M | Discoverable, labeled and responsive | `PRODUCTION_READY` |
| Password reset | D/M | Unified shell and safe validation | `PRODUCTION_READY` |
| Password change | D/M | Coherent authenticated account navigation | `PRODUCTION_READY` |
| MFA challenge | D/M | Coherent auth shell and safe challenge UX | `PRODUCTION_READY` |
| MFA settings — not enabled | D | Coherent account shell | `PRODUCTION_READY` |
| MFA enrollment | D/M | Provisioning data bounded; no document overflow | `PRODUCTION_READY` |
| MFA recovery codes | D | Clear critical-material presentation | `PRODUCTION_READY` |
| MFA settings — confirmed | D/M | Clear destructive hierarchy and account navigation | `PRODUCTION_READY` |
| Account Overview — ready | D/M | Ready status, character action, security/password orientation | `PRODUCTION_READY` |
| Account Overview — pending | D/M | Explicit in-progress state; no unauthorized retry/character action | `PRODUCTION_READY` |
| Account Overview — recoverable | D/M + retry success | Safe retry of persisted provisioning intent only | `PRODUCTION_READY` |
| Account Overview — conflict | D/M | Fail-closed support guidance; no retry/rebind/replacement action | `PRODUCTION_READY` |
| Account Overview — missing binding | D/M | Safe non-actionable support state; no inferred legacy engine ownership | `PRODUCTION_READY` |
| Character creation | D/M | User-facing labels and coherent account context | `PRODUCTION_READY` |
| Admin dashboard | D/M | Oriented admin shell with account/logout path | `PRODUCTION_READY` |
| CMS news list/form | D/M | Responsive table/form and action hierarchy | `PRODUCTION_READY` |
| Managed pages list/form | D/M | Responsive table/form and explicit empty/action states | `PRODUCTION_READY` |
| Role management | D/T/M | Contained responsive data and destructive hierarchy | `PRODUCTION_READY` |
| Audit log | D/T/M | Long metadata contained without document overflow | `PRODUCTION_READY` |
| Authorization denied 403 | D/M | Product-owned CSP-compatible recovery view | `PRODUCTION_READY` |
| Not found 404 | D/M | Product-owned recovery view | `PRODUCTION_READY` |
| Controlled dependency failure 503 | D/M | Safe product-owned recovery view without technical leakage | `PRODUCTION_READY` |
| News empty state | D/M | Clear safe empty state | `PRODUCTION_READY` |
| Servers runtime dependency failure | D/M | Explicit safe state, responsive long content | `PRODUCTION_READY` |

## Account Overview and provisioning-status safety matrix

| Authoritative Platform state | User-facing state | Character creation | Retry | Safety result |
|---|---|---:|---:|---|
| binding `ready` | `Ready` | allowed | not shown | Uses ready Platform binding; no raw legacy engine ID/name shown |
| binding `pending`, no recoverable dependency code | `Setup in progress` | blocked | not shown | Fails closed until ready |
| binding `pending` + `dependency_unavailable` | `Setup interrupted` | blocked | allowed | Reuses existing idempotent provisioning action and persisted immutable intent |
| binding `conflict` | `Support required` | blocked | not shown | No self-service rebind/unlink/replacement account |
| missing/unknown binding state | `Support required` | blocked | not shown | Does not infer ownership from legacy engine data |

The Account Overview reads Platform-owned `identity_legacy_accounts` state only. It does not expose raw legacy engine account IDs, provisioning names, credentials, tokens, hashes or internal exception details. Hard conflict and unknown/missing states remain fail-closed.

## Cross-cutting UX assessment

### Location and navigation — PASS

Public, identity/account and administrator contexts provide coherent orientation and appropriate return/account/logout paths. Authenticated users can enter Account Overview from the public shell and navigate to security and password actions. Character creation is surfaced from the ready provisioning state rather than as an unconditional action for non-ready accounts.

### Primary, secondary and destructive actions — PASS

Shared styling distinguishes primary, secondary and destructive actions. Provisioning retry is shown only for the recoverable state; conflict and missing states expose guidance rather than unsafe mutation controls.

### Forms and validation — PASS

Observed controls are labeled, validation is visible, and the current collector reports zero unlabeled-control surfaces. Identity/account/admin forms participate in the shared presentation system and required mobile widths do not produce document-level overflow.

### Tables and long content — PASS

Required public/admin table-heavy tablet/mobile surfaces use contained responsive behavior. The current collector reports zero document-level overflow. Long names, emails, runtime messages, audit metadata and MFA provisioning content remain bounded.

### Empty and failure states — PASS

Product-owned empty and representative 403/404/503 failure states provide safe user-facing messaging and recovery navigation. Account provisioning includes explicit pending, recoverable, conflict and missing-state presentation. No raw framework/database exception text was detected.

### Confirmation after operations — PASS

Existing account/admin mutations retain status/error feedback. Recoverable provisioning retry redirects back to Account Overview with completion status on success and safe preserved-request guidance on dependency failure.

### Dead ends — PASS for launch-blocking criteria

The composed navigation and error-recovery paths remove the previously identified launch-blocking dead ends. Optional contextual refinements such as richer character-search-again or news breadcrumbs remain non-blocking polish.

## Accessibility assessment

- Semantic headings and landmarks: **PASS** for observed delivered surfaces.
- Form labels: **PASS**; zero unlabeled-control surfaces in the current collector.
- Keyboard navigation and visible focus: **PASS**; zero interactive focus-not-observed surfaces.
- Color contrast: **PASS** for sampled core styles; zero sampled low-contrast surfaces.
- Link/button distinction: **PASS** for launch criteria.
- Table semantics: **PASS** semantically and responsively for required evidence.
- ARIA/status communication: **PASS** for observed states; critical state is communicated in text and not color alone.

The Account Overview screenshots intentionally capture a focused `Skip to content` link because the accessibility smoke test exercises keyboard focus. The link is not a permanent overlay in the unfocused page state.

## Design-system assessment

The post-PR #77 frontend uses the documented Oteryn visual foundation across public, identity/account, administrator and error surfaces: shared palette/tokens, typography, spacing, cards, buttons, inputs/selects, alerts, badges/statuses, navigation, pagination patterns and responsive breakpoints. Issue #81 reuses these patterns rather than introducing a separate account-only visual system.

## Screenshot evidence index

The full issue #81 artifact contains:

- dedicated Account Overview evidence: `account-overview-ready-*`, `account-overview-pending-*`, `account-overview-recoverable-*`, `account-overview-conflict-*`, `account-overview-missing-*`, and `account-overview-retry-success-desktop.png`;
- the complete 71-screen exploratory Visual/Accessibility screenshot set and contact sheet covering public, identity/MFA, character, admin/CMS/RBAC/audit, error, empty and dependency-failure states;
- machine-readable `evidence.json`, `junit.xml` and `visual/visual-acceptance-results.json`.

## Remaining visual / UX blockers

**None for the currently delivered staging-verifiable launch scope.**

Optional non-blocking polish remains possible on embedded character-search recovery, news breadcrumbs and managed-page discovery, but the composed evidence does not classify these as launch blockers.

## Acceptance gate

**Visual / UX Acceptance: PASS.**

This PASS is independent of Functional Acceptance and does not imply production deployment proof. The project must still complete the independent Production Go-Live Gate and production smoke against the exact deployed SHA. Any separately authorized Platform game-login bridge required by final launch scope also remains outside this visual/UX gate.
