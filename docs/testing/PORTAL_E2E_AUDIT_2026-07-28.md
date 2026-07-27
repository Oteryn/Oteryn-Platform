# Oteryn Portal E2E Audit — 2026-07-28

## Status

`REMEDIATING_AND_REVALIDATING`

This is the durable findings record for the exact task head on `test/OTERYN-20260728-portal-e2e-audit`. It records failures and missing capabilities without promoting repository or staging evidence to `PRODUCTION_PROVEN`.

## Audit target

- Base repository SHA: `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b`
- Base changes: merged PR #262 staging refresh and PR #264 container-namespace verification fix
- Delivered-surface baseline: PR #260 / Issue #240 closure
- Audit branch: `test/OTERYN-20260728-portal-e2e-audit`
- First preliminary tested SHA: `bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab`
- Second preliminary tested SHA: `418bb0939fea9b98753da14b0e0254e0afe37f3a`
- Final tested SHA: `PENDING`
- Audit PR: #265

## Required execution matrix

| Area | Required evidence | Latest preliminary state | Final state |
|---|---|---|---|
| Primary functional browser baseline | `full` Chromium profile with effective zero retries | Functional, resilience and accessibility profiles passed in run `30311830608`; legacy visual selector failed | Pending rerun |
| Critical risk matrix | Smoke, Chromium/Firefox/WebKit portability, desktop/tablet/mobile responsive, resilience and keyboard/focus accessibility, zero retries | Passed in run `30311500518` | Pending final head |
| Identity/account lifecycle | Registration, login/logout, account overview, provisioning, password recovery/change, MFA, sessions and character creation | Passed in run `30312252559` | Pending final head |
| Delivered route contract | Strict zero-gap route/evidence ledger against the exact Laravel route table | Passed in run `30312252559` | Pending final head |
| Downloads | Public/admin/localization/failure recovery and bounded Firefox/WebKit | Misconfigured collection produced zero lifecycle tests in runs `30311441459` and `30312256615` | Pending rerun |
| Events | Public/admin/localization/conflict lifecycle across declared browser/viewports | Passed in run `30312260632` | Pending final head |
| Announcements | Public/admin/localization/stale/conflict/audit lifecycle | Passed in run `30312264757` | Pending final head |
| Support and legal | Typed routes, legal versions, localization, RBAC/MFA and audit | Passed in run `30312268671` | Pending final head |
| Editorial media | Upload validation, private content, integrity, reference lock, deletion and audit | Passed in run `30312273170` after image-decode remediation | Pending final head |
| Wiki | Public/search/errors/recovery/localization and complete admin editorial lifecycle | Passed in run `30312277252` | Pending final head |
| Stability | Three fresh isolated zero-retry critical iterations | Passed in run `30312281574` | Pending final head |
| Soak | Bounded read-only public surface calibration | Direct 300-second soak passed in run `30312285888` | Pending final head |
| Repository CI | Composer validation/audit, formatting, PHPStan and full automated tests | Passed in run `30311441454` | Pending final head |

## Findings ledger

### E2E-AUD-001 — Reusable full profile is overridden for pull-request callers

- Classification: test harness defect
- Severity: medium
- Status: confirmed; durable audit workaround implemented
- Evidence: `.github/workflows/acceptance-validation.yml` selects `critical` from `github.event_name == 'pull_request'` before considering `inputs.profile`.
- Impact: a reusable workflow called from a pull-request workflow cannot request `full`; it silently executes `critical` instead.
- Disposition: the audit invokes `acceptance-validation.yml` through `workflow_dispatch` with an explicit profile and verifies the selected exact-head run.

### E2E-AUD-002 — Project-state documentation is behind the delivered portal state

- Classification: documentation drift
- Severity: low
- Status: confirmed; not a runtime defect
- Evidence: `docs/agents/PROJECT_STATE.md` is dated 2026-07-21 and describes continuous E2E closure only through PR #111, while current repository state includes the Issue #240 closure and later staging work.
- Impact: an agent reading only the required startup file can underestimate current delivered coverage and repeat completed work.
- Disposition: recorded for a narrow documentation follow-up.

### E2E-AUD-003 — Staging release identity text is stale after the final refresh

- Classification: documentation drift
- Severity: medium
- Status: confirmed
- Evidence: `docs/agents/ACTIVE_WORK.md` still names `415aa3febd04c8d9c61082d4a7451352bf084013` as the previously verified staging release, while PR #262 and PR #264 established later staging refresh and verification evidence.
- Impact: operators or agents may use an obsolete staging release identity.
- Disposition: record the exact PR #262/#264 deployment evidence before changing the authoritative release identity text.

### E2E-AUD-004 — Audit target changed while the first branch was being prepared

- Classification: process/evidence invalidation
- Severity: informational
- Status: handled
- Evidence: main advanced from `ccd45fdce3176bd1da97a264bbbaf19a68c1397b` to `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b` through PR #264 before PR #265 was opened.
- Impact: runs on the superseded task head cannot be used as final exact-head evidence.
- Disposition: branch was force-synchronized before the audit implementation was recreated.

### E2E-AUD-005 — Direct critical and full profiles cancel each other

- Classification: test orchestration defect
- Severity: high for audit trustworthiness
- Status: confirmed; fixed and preliminarily proven
- Evidence: `critical` run `30310320026` was cancelled seconds after `full` run `30310324691` started on the same ref because both used the direct concurrency key.
- Impact: concurrent profile dispatch can silently remove required evidence.
- Disposition: direct `critical`, `full` and `soak` profiles are serialized. The second critical run `30311500518` and direct soak run `30312285888` passed.

### E2E-AUD-006 — Specialized lifecycle suites leaked into the full baseline

- Classification: Playwright collection and fixture-isolation defect
- Severity: high for full-profile reliability
- Status: confirmed; fixed and preliminarily proven
- Evidence: full run `30310324691` collected module suites with independent reset scripts, while their dedicated same-SHA workflows passed. After isolation, the full functional baseline in run `30311830608` passed all 22 Chromium-primary tests plus resilience and accessibility.
- Impact: the generic baseline previously produced misleading cross-suite fixture failures.
- Disposition: specialized lifecycle suites remain mandatory in their independent workflows and are excluded only when `ACCEPTANCE_PROFILE=full`.

### E2E-AUD-007 — Public game-data assertion used stale channel copy

- Classification: test expectation drift
- Severity: low
- Status: fixed and preliminarily proven
- Evidence: the delivered page renders `Acceptance (Channel ID 1)`, while the former assertion expected `Acceptance (ID 1)`.
- Impact: correct and more explicit UI copy failed an older assertion.
- Disposition: the browser expectation now matches the accessible delivered text; full functional run `30311830608` passed.

### E2E-AUD-008 — Editorial preview assertion did not wait for image decoding

- Classification: browser-test race
- Severity: medium
- Status: fixed and preliminarily proven
- Evidence: run `30310355821` showed a visible authenticated 320 × 180 image, but the tablet assertion read `naturalWidth` before decoding completed.
- Impact: a slow decode could fail a zero-retry test despite correct upload, authorization and markup.
- Disposition: the test polls `image.complete` and `naturalWidth > 0`; exact-head Editorial Media run `30312273170` passed.

### E2E-AUD-009 — Soak wrapper failed before creating a job

- Classification: workflow composition defect
- Severity: medium
- Status: confirmed; direct-profile bypass proven
- Evidence: wrapper run `30310370831` failed in two seconds and produced zero jobs.
- Impact: the wrapper supplied no soak evidence and no job-level diagnostics.
- Disposition: the audit invokes the acceptance workflow directly with `profile=soak`; 300-second run `30312285888` passed. The wrapper remains a separately recorded defect.

### E2E-AUD-010 — Full-profile exclusions disabled the dedicated Downloads lifecycle

- Classification: Playwright collection regression introduced during audit remediation
- Severity: high
- Status: confirmed; fixed pending final rerun
- Evidence: Downloads runs `30311441459` and `30312256615` generated JUnit with `tests=0` after the Downloads lifecycle spec was added to the unconditional `chromium-primary` ignore list.
- Impact: the workflow failed before Firefox/WebKit portability and produced no Downloads lifecycle evidence.
- Disposition: specialized lifecycle exclusions now apply only when `ACCEPTANCE_PROFILE=full`; dedicated workflows retain their declared tests.

### E2E-AUD-011 — Exploratory visual audit used a removed homepage selector

- Classification: visual harness drift
- Severity: medium
- Status: confirmed; fixed pending final rerun
- Evidence: full run `30311830608` passed functional, resilience and accessibility execution but failed before writing visual evidence. The visual harness targeted `#character-name`; the current homepage control is `#home-character-name`.
- Impact: visual/UX evidence was marked not proven despite the functional baseline passing.
- Disposition: the visual wrapper resolves the legacy capture selector to the current homepage search control without modifying application markup.

### E2E-AUD-012 — Direct audit profiles did not explicitly force global zero retries

- Classification: retry-policy ambiguity
- Severity: medium
- Status: fixed pending final evidence
- Evidence: full evidence for run `30311830608` recorded `global_zero_retries_requested: "0"`, even though bounded specs used zero-retry declarations.
- Impact: audit evidence could not unambiguously prove that every collected profile used zero retries.
- Disposition: Playwright configuration now forces `critical`, `full` and `soak` profiles to `retries: 0`, while specialized workflows continue to use `ACCEPTANCE_ZERO_RETRIES=1`.

## Known missing capabilities and explicit nonclaims

These are not counted as newly discovered regressions, but they remain material gaps:

| ID | Missing or unproven capability | Current owner/boundary |
|---|---|---|
| GAP-001 | Audited administrator homepage-template selector is not delivered | Open Issue #244 |
| GAP-002 | Production Go-Live verification is not executed | Open Issue #91; explicit production authorization and access required |
| GAP-003 | Authoritative Platform-originated game-login bridge is not implemented/proven | Separate cross-repository authorization and contract required |
| GAP-004 | Full manual screen-reader compatibility is not proven by automated keyboard/focus checks | Manual assistive-technology validation |
| GAP-005 | Deferred account deletion, unlink/rebind/transfer, character rename/deletion and payments are not delivered | Explicitly outside the current delivered-surface contract |

## Preliminary runtime results

- Orchestration `30310298326` on `bb1e2a395169d0000ed33ad4e325e4bfc7fb12ab` exposed the initial concurrency, fixture, copy, media and soak-wrapper defects.
- Orchestration `30311441485` on `418bb0939fea9b98753da14b0e0254e0afe37f3a` proved the serialized critical and soak runs, the functional full baseline, contract/account lifecycle, Events, Announcements, Support/Legal, Editorial Media, Wiki and stability. It also exposed the zero-test Downloads regression and stale visual selector.

No preliminary result is promoted to final evidence after code or test changes. Every required gate must pass again on one final exact head.

## Final evidence

Pending exact-head rerun after the recorded remediations.
