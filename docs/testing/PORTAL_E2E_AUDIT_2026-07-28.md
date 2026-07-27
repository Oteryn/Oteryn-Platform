# Oteryn Portal E2E Audit — 2026-07-28

## Status

`IN_PROGRESS`

This is a live, append-only-for-findings audit record for the exact task head on `test/OTERYN-20260728-portal-e2e-audit`. It records failures and missing capabilities without promoting repository or staging evidence to `PRODUCTION_PROVEN`.

## Audit target

- Base repository SHA: `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b`
- Base changes: merged PR #262 staging refresh and PR #264 container-namespace verification fix
- Delivered-surface baseline: PR #260 / Issue #240 closure
- Audit branch: `test/OTERYN-20260728-portal-e2e-audit`
- Final tested SHA: `PENDING`
- Audit PR: #265

## Required execution matrix

| Area | Required evidence | State |
|---|---|---|
| Primary functional browser baseline | Existing `full` profile, Chromium, zero retries | Pending |
| Critical risk matrix | Smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, resilience and keyboard/focus accessibility, zero retries | Pending |
| Identity/account lifecycle | Registration, login/logout, account overview, provisioning, password recovery/change, MFA, sessions and character creation | Pending |
| Delivered route contract | Strict zero-gap route/evidence ledger against the exact Laravel route table | Pending |
| Downloads | Public/admin/localization/failure recovery and bounded Firefox/WebKit | Pending |
| Events | Public/admin/localization/conflict lifecycle across declared browser/viewports | Pending |
| Announcements | Public/admin/localization/stale/conflict/audit lifecycle | Pending |
| Support and legal | Typed routes, legal versions, localization, RBAC/MFA and audit | Pending |
| Editorial media | Upload validation, private content, integrity, reference lock, deletion and audit | Pending |
| Wiki | Public/search/errors/recovery/localization and complete admin editorial lifecycle | Pending |
| Stability | Three fresh isolated zero-retry critical iterations | Pending |
| Soak | Bounded read-only public surface calibration | Pending |
| Repository CI | Composer validation/audit, formatting, PHPStan and full automated tests | Pending |

## Findings ledger

### E2E-AUD-001 — Reusable full profile is overridden for pull-request callers

- Classification: test harness defect
- Severity: medium
- Status: confirmed; audit workaround being implemented
- Evidence: `.github/workflows/acceptance-validation.yml` sets `ACCEPTANCE_PROFILE` from `github.event_name == 'pull_request'` before `inputs.profile`.
- Impact: a reusable workflow called from a pull-request workflow cannot request `full`; it silently executes `critical` instead. This can create a false belief that the full primary Chromium baseline was executed.
- Disposition: the audit orchestration invokes the reusable workflow from a push event and records the exact selected profile from job logs. Ordinary pull-request behavior remains unchanged.

### E2E-AUD-002 — Project-state documentation is behind the delivered portal state

- Classification: documentation drift
- Severity: low
- Status: confirmed; not a runtime defect
- Evidence: `docs/agents/PROJECT_STATE.md` is dated 2026-07-21 and describes continuous E2E closure only through PR #111, while `docs/agents/ACTIVE_WORK.md` records portal closure through PR #260 and current main includes PR #264.
- Impact: an agent reading only the required core startup file can underestimate current delivered coverage and repeat already completed work.
- Disposition: recorded here; any correction must remain a narrow documentation update and must not alter runtime acceptance claims.

### E2E-AUD-003 — Staging release identity text is stale after the final refresh

- Classification: documentation drift
- Severity: medium
- Status: confirmed
- Evidence: `docs/agents/ACTIVE_WORK.md` still names `415aa3febd04c8d9c61082d4a7451352bf084013` as the previously verified staging release, while PR #262 and PR #264 established later staging refresh and verification evidence.
- Impact: operators or agents may use an obsolete staging release identity when preparing evidence or comparing deployed state.
- Disposition: record the exact PR #262/#264 deployment evidence before changing the authoritative release identity text.

### E2E-AUD-004 — Audit target changed while the first branch was being prepared

- Classification: process/evidence invalidation
- Severity: informational
- Status: handled
- Evidence: main advanced from `ccd45fdce3176bd1da97a264bbbaf19a68c1397b` to `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b` through PR #264 before PR #265 was opened.
- Impact: any run on the superseded task head cannot be used as final exact-head evidence.
- Disposition: branch was force-synchronized to current main before recreating the audit workflow; superseded runs are excluded.

## Known missing capabilities and explicit nonclaims

These are not counted as newly discovered regressions, but they remain material gaps:

| ID | Missing or unproven capability | Current owner/boundary |
|---|---|---|
| GAP-001 | Audited administrator homepage-template selector is not delivered | Open Issue #244 |
| GAP-002 | Production Go-Live verification is not executed | Open Issue #91; explicit production authorization and access required |
| GAP-003 | Authoritative Platform-originated game-login bridge is not implemented/proven | Separate cross-repository authorization and contract required |
| GAP-004 | Full manual screen-reader compatibility is not proven by automated keyboard/focus checks | Manual assistive-technology validation |
| GAP-005 | Deferred account deletion, unlink/rebind/transfer, character rename/deletion and payments are not delivered | Explicitly outside the current delivered-surface contract |

## Runtime failures

No runtime result is recorded until an exact-head workflow finishes. Every failure will be added here with workflow, run ID, job, step, exact error, classification and resolution.

## Final evidence

Pending exact-head runs.
