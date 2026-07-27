# Oteryn Portal E2E Audit — 2026-07-28

## Status

`REMEDIATING_AND_REVALIDATING`

This is the durable findings record for the exact task head on `test/OTERYN-20260728-portal-e2e-audit`. It records failures and missing capabilities without promoting repository or staging evidence to `PRODUCTION_PROVEN`.

## Audit target

- Base repository SHA: `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b`
- Base changes: merged PR #262 staging refresh and PR #264 container-namespace verification fix
- Delivered-surface baseline: PR #260 / Issue #240 closure
- Audit branch: `test/OTERYN-20260728-portal-e2e-audit`
- Preliminary tested SHA: `bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab`
- Final tested SHA: `PENDING`
- Audit PR: #265

## Required execution matrix

| Area | Required evidence | Preliminary state | Final state |
|---|---|---|---|
| Primary functional browser baseline | Existing `full` profile, Chromium, zero retries | Failed; harness defects found | Pending rerun |
| Critical risk matrix | Smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, resilience and keyboard/focus accessibility, zero retries | Cancelled by workflow concurrency collision | Pending rerun |
| Identity/account lifecycle | Registration, login/logout, account overview, provisioning, password recovery/change, MFA, sessions and character creation | Passed in run `30310329648` | Pending final head |
| Delivered route contract | Strict zero-gap route/evidence ledger against the exact Laravel route table | Passed in run `30310329648` | Pending final head |
| Downloads | Public/admin/localization/failure recovery and bounded Firefox/WebKit | Passed in run `30310334330` | Pending final head |
| Events | Public/admin/localization/conflict lifecycle across declared browser/viewports | Passed in run `30310339727` | Pending final head |
| Announcements | Public/admin/localization/stale/conflict/audit lifecycle | Passed in run `30310345489` | Pending final head |
| Support and legal | Typed routes, legal versions, localization, RBAC/MFA and audit | Passed in run `30310350565` | Pending final head |
| Editorial media | Upload validation, private content, integrity, reference lock, deletion and audit | Failed on tablet image decode race in run `30310355821` | Pending rerun |
| Wiki | Public/search/errors/recovery/localization and complete admin editorial lifecycle | Passed in run `30310361078` | Pending final head |
| Stability | Three fresh isolated zero-retry critical iterations | Passed in run `30310366103` | Pending final head |
| Soak | Bounded read-only public surface calibration | Wrapper failed before job creation in run `30310370831` | Pending direct-profile rerun |
| Repository CI | Composer validation/audit, formatting, PHPStan and full automated tests | Passed in run `30310298452` | Pending final head |

## Findings ledger

### E2E-AUD-001 — Reusable full profile is overridden for pull-request callers

- Classification: test harness defect
- Severity: medium
- Status: confirmed; durable audit workaround implemented
- Evidence: `.github/workflows/acceptance-validation.yml` selects `critical` from `github.event_name == 'pull_request'` before considering `inputs.profile`.
- Impact: a reusable workflow called from a pull-request workflow cannot request `full`; it silently executes `critical` instead. This can create a false belief that the full primary Chromium baseline was executed.
- Disposition: the audit invokes `acceptance-validation.yml` through `workflow_dispatch` with an explicit profile and verifies the selected exact-head run. Ordinary pull-request behavior is not weakened.

### E2E-AUD-002 — Project-state documentation is behind the delivered portal state

- Classification: documentation drift
- Severity: low
- Status: confirmed; not a runtime defect
- Evidence: `docs/agents/PROJECT_STATE.md` is dated 2026-07-21 and describes continuous E2E closure only through PR #111, while `docs/agents/ACTIVE_WORK.md` records portal closure through PR #260 and current main includes PR #264.
- Impact: an agent reading only the required core startup file can underestimate current delivered coverage and repeat already completed work.
- Disposition: recorded here; correction remains a narrow documentation follow-up and must not alter runtime acceptance claims.

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

### E2E-AUD-005 — Direct critical and full profiles cancel each other

- Classification: test orchestration defect
- Severity: high for audit trustworthiness
- Status: confirmed; fixed in the audit orchestrator
- Evidence: preliminary `critical` run `30310320026` was cancelled seconds after `full` run `30310324691` started on the same ref. Direct `workflow_dispatch` invocations share the acceptance workflow's `direct` concurrency key.
- Impact: concurrent profile dispatch can silently remove required evidence while the later profile continues.
- Disposition: direct `critical`, `full` and `soak` profiles are now serialized and individually collected.

### E2E-AUD-006 — Specialized lifecycle suites leaked into the full baseline

- Classification: Playwright collection and fixture-isolation defect
- Severity: high for full-profile reliability
- Status: confirmed; fixed pending exact-head rerun
- Evidence: full run `30310324691` collected specialized Downloads, Events, Announcements, Support/Legal, Editorial Media and Wiki lifecycle specs that have independent reset scripts and dedicated workflows. Five full-run failures appeared after shared state was altered, while the dedicated Downloads, Events, Announcements, Support/Legal and Wiki runs passed on the same SHA.
- Impact: the generic full baseline did not represent an isolated baseline and produced misleading failures caused by cross-suite fixture resets and shared registration throttles.
- Disposition: specialized lifecycle specs are excluded from `chromium-primary`; they remain mandatory through their independent zero-retry workflows. Helper-driven registration now clears only isolated acceptance throttle state for serial account/full profiles.

### E2E-AUD-007 — Public game-data assertion used stale channel copy

- Classification: test expectation drift
- Severity: low
- Status: confirmed; fixed pending exact-head rerun
- Evidence: the page rendered `Acceptance (Channel ID 1)` while `public-game-data-acceptance.spec.mjs` expected `Acceptance (ID 1)` in full run `30310324691`.
- Impact: a correct and more explicit UI string failed the older browser assertion.
- Disposition: assertion now matches the delivered accessible text.

### E2E-AUD-008 — Editorial preview assertion did not wait for image decoding

- Classification: browser-test race
- Severity: medium
- Status: confirmed; fixed pending exact-head rerun
- Evidence: Editorial Media run `30310355821` showed a visible authenticated `<img>` with valid 320 × 180 metadata, but the tablet assertion read `naturalWidth` as `0` before decoding completed.
- Impact: a slow decode could fail a zero-retry test even though upload, authorization and rendered markup were correct.
- Disposition: the test now polls `image.complete` and `naturalWidth > 0` before continuing.

### E2E-AUD-009 — Soak wrapper failed before creating a job

- Classification: workflow composition defect
- Severity: medium
- Status: confirmed; bypassed pending exact-head rerun
- Evidence: `Acceptance E2E Public Soak` run `30310370831` failed in two seconds and produced zero jobs.
- Impact: the scheduled/manual wrapper did not provide the required soak evidence and offered no job-level diagnostics.
- Disposition: the audit now invokes the proven reusable acceptance workflow directly with `profile=soak` and `soak_seconds=300`. The wrapper remains a separately recorded defect rather than being treated as passed.

## Known missing capabilities and explicit nonclaims

These are not counted as newly discovered regressions, but they remain material gaps:

| ID | Missing or unproven capability | Current owner/boundary |
|---|---|---|
| GAP-001 | Audited administrator homepage-template selector is not delivered | Open Issue #244 |
| GAP-002 | Production Go-Live verification is not executed | Open Issue #91; explicit production authorization and access required |
| GAP-003 | Authoritative Platform-originated game-login bridge is not implemented/proven | Separate cross-repository authorization and contract required |
| GAP-004 | Full manual screen-reader compatibility is not proven by automated keyboard/focus checks | Manual assistive-technology validation |
| GAP-005 | Deferred account deletion, unlink/rebind/transfer, character rename/deletion and payments are not delivered | Explicitly outside the current delivered-surface contract |

## Preliminary runtime result

The first complete orchestration run was `30310298326` against exact SHA `bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab`. It proved the contract, account lifecycle, five specialized module gates, Wiki and three-iteration stability, but correctly failed overall because critical was cancelled, full and Editorial Media exposed harness defects, and the soak wrapper created no job.

No preliminary result is promoted to final evidence after code or test changes. Every remediation must pass again on one final exact head.

## Final evidence

Pending exact-head rerun after the recorded remediations.
