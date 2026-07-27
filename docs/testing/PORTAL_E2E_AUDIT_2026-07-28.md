# Oteryn Portal E2E Audit — 2026-07-28

## Status

`COMPLETE_WITH_RECORDED_FOLLOW_UPS`

This is the durable findings record for the exact-head portal audit completed through PR #265. It records failures, remediations and missing capabilities without promoting repository or staging evidence to `PRODUCTION_PROVEN`.

## Audit target and outcome

- Base repository SHA: `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b`
- Delivered-surface baseline: PR #260 / Issue #240 closure
- Final exact tested SHA: `a5929d0725d6a99069abbc2faa42022d843e560d`
- Audit orchestration run: `30313332817`
- Audit PR: #265
- Squash merge SHA: `dd48947a0c6328dc5d361f9953c221df343ecb4d`
- Final result: every required repository, browser, contract, module, stability and soak gate passed on the same exact tested SHA.
- Product result: no confirmed product runtime regression was found within the declared repository/staging coverage boundary.
- Nonclaim: production behavior was not tested and remains owned by Issue #91.

## Final execution matrix

| Area | Final exact-head evidence | Result |
|---|---:|---|
| Critical browser matrix | `30313356495` | PASS |
| Full Chromium baseline, resilience, accessibility and exploratory visual UX | `30313569233` | PASS |
| Strict route ledger and complete account lifecycle | `30313963191` | PASS |
| Downloads lifecycle plus Firefox/WebKit portability | `30313967734` | PASS |
| Events public/admin/localization/conflict lifecycle | `30313972600` | PASS |
| Announcements public/admin/localization/audit lifecycle | `30313976960` | PASS |
| Support/Legal routes, localization, RBAC/MFA and audit | `30313981106` | PASS |
| Editorial Media upload, preview, integrity, reference lock, deletion and audit | `30313985873` | PASS |
| Wiki public/admin/editorial lifecycle | `30313990460` | PASS |
| Three-iteration zero-retry critical stability | `30313994976` | PASS |
| Bounded 300-second read-only public soak | `30313999674` | PASS |
| Parent exact-head audit collector | `30313332817` | PASS |
| Pull-request CI, governance and platform checks | PR #265 exact head | PASS |

The final full artifact reports 22 passing Chromium-primary tests, two passing resilience tests, six passing accessibility tests and `visual_result: success`. The visual artifact includes the generated contact sheet and captured desktop/tablet/mobile states.

## Findings ledger

### E2E-AUD-001 — Reusable full profile is overridden for pull-request callers

- Classification: test harness defect
- Severity: medium
- Final status: recorded follow-up; audit path is safe
- Evidence: `.github/workflows/acceptance-validation.yml` gives pull-request event context precedence over the explicit `workflow_call` profile.
- Impact: a PR caller can silently receive `critical` when requesting `full`.
- Current control: the audit invokes the workflow through explicit `workflow_dispatch` and verifies exact profile/run evidence.

### E2E-AUD-002 — Project-state documentation is behind the delivered portal state

- Classification: documentation drift
- Severity: low
- Final status: recorded follow-up
- Evidence: `docs/agents/PROJECT_STATE.md` predates the final Issue #240 closure and current audit work.
- Impact: startup context can understate current delivered coverage.

### E2E-AUD-003 — Staging release identity text was stale

- Classification: documentation drift
- Severity: medium
- Final status: resolved during audit closeout
- Evidence: `docs/agents/ACTIVE_WORK.md` named the earlier PR #232 staging SHA after PR #262/#264 refreshed and verified the staging target.
- Resolution: the closeout records `ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b` as the latest verified staging refresh identity while retaining the no-production-claim boundary.

### E2E-AUD-004 — Audit target changed during initial preparation

- Classification: process/evidence invalidation
- Severity: informational
- Final status: resolved
- Evidence: main advanced through PR #264 before the first audit PR was opened.
- Resolution: the branch was force-synchronized and all superseded runs were excluded from final evidence.

### E2E-AUD-005 — Direct critical and full profiles cancelled each other

- Classification: test orchestration defect
- Severity: high for audit trustworthiness
- Final status: fixed and proven
- Evidence: preliminary `critical` run `30310320026` was cancelled when `full` run `30310324691` reused the same direct concurrency key.
- Resolution: direct `critical`, `full` and `soak` profiles are serialized. Final runs `30313356495`, `30313569233` and `30313999674` passed.

### E2E-AUD-006 — Specialized lifecycle suites leaked into the full baseline

- Classification: Playwright collection and fixture-isolation defect
- Severity: high
- Final status: fixed and proven
- Evidence: preliminary full execution collected independently seeded module lifecycle suites and produced cross-suite state contamination.
- Resolution: specialized lifecycle suites remain mandatory in their dedicated workflows and are excluded only from `ACCEPTANCE_PROFILE=full`. Final full run `30313569233` passed.

### E2E-AUD-007 — Public game-data assertion used stale channel copy

- Classification: expectation drift
- Severity: low
- Final status: fixed and proven
- Evidence: the UI renders `Acceptance (Channel ID 1)` while the old assertion expected `Acceptance (ID 1)`.
- Resolution: the expectation matches the current accessible copy; final full run passed.

### E2E-AUD-008 — Editorial preview assertion did not wait for image decoding

- Classification: browser-test race
- Severity: medium
- Final status: fixed and proven
- Evidence: preliminary tablet execution read `naturalWidth` before a valid authenticated image finished decoding.
- Resolution: the test polls image completion and decoded width. Final Editorial Media run `30313985873` passed.

### E2E-AUD-009 — Soak wrapper failed before creating a job

- Classification: workflow composition defect
- Severity: medium
- Final status: recorded follow-up; direct soak path is proven
- Evidence: wrapper run `30310370831` failed before job creation and supplied no diagnostics.
- Current control: the audit invokes the underlying acceptance workflow directly with `profile=soak`; final 300-second run `30313999674` passed.

### E2E-AUD-010 — Full-profile exclusions disabled the dedicated Downloads lifecycle

- Classification: audit-remediation collection regression
- Severity: high
- Final status: fixed and proven
- Evidence: preliminary Downloads runs produced JUnit `tests=0` after an unconditional project ignore was introduced.
- Resolution: lifecycle exclusions apply only to profile `full`. Final Downloads run `30313967734` executed lifecycle and Firefox/WebKit tests successfully.

### E2E-AUD-011 — Exploratory visual audit used a removed homepage selector

- Classification: visual harness drift
- Severity: medium
- Final status: fixed and proven
- Evidence: the old harness targeted `#character-name`; the refreshed homepage uses `#home-character-name`.
- Resolution: the visual wrapper resolves the legacy capture selector to the current control without changing application markup. Final full run `30313569233` produced successful visual evidence and a contact sheet.

### E2E-AUD-012 — Direct audit profiles did not unambiguously force zero retries

- Classification: retry-policy ambiguity
- Severity: medium
- Final status: fixed and proven
- Evidence: preliminary full evidence did not explicitly request global zero retries.
- Resolution: Playwright configuration forces `critical`, `full` and `soak` profiles to `retries: 0`; specialized workflows retain `ACCEPTANCE_ZERO_RETRIES=1`. Final matrix passed with bounded retries equal to zero.

## Known missing capabilities and explicit nonclaims

| ID | Missing or unproven capability | Owner/boundary |
|---|---|---|
| GAP-001 | Audited administrator homepage-template selector is not delivered | Issue #244 |
| GAP-002 | Production Go-Live verification is not executed | Issue #91; production authorization and access required |
| GAP-003 | Authoritative Platform-originated game-login bridge is not implemented/proven | Separate cross-repository authorization and contract required |
| GAP-004 | Full manual screen-reader compatibility is not proven by automated keyboard/focus checks | Manual assistive-technology validation |
| GAP-005 | Deferred account deletion, unlink/rebind/transfer, character rename/deletion and payments are not delivered | Outside the current delivered-surface contract |

## Follow-up boundary

`E2E-AUD-001`, `E2E-AUD-002` and `E2E-AUD-009` remain bounded harness/documentation follow-ups. They do not invalidate the final exact-head audit because the audit used explicit profile dispatch, exact run collection and the proven direct soak path.

## Conclusion

The repository now contains a persistent audit workflow and findings ledger. All required final gates passed on `a5929d0725d6a99069abbc2faa42022d843e560d`, and PR #265 merged the tested remediations. This is repository/staging evidence only, not production proof.
