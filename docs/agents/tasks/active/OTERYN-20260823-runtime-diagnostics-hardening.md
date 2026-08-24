---
task_id: OTERYN-20260823-runtime-diagnostics-hardening
governing_issue: 1262
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/CONTEXT_HANDOFF.md
search_first:
  - scripts/acceptance diagnostics and intentional failure scenarios
optional_reads: []
---
```
# OTERYN-20260823-runtime-diagnostics-hardening
```
## Goal
```
Governing GitHub Issue: #1262 - canonical lifecycle authority for this existing task and PR #1242.

Make unexpected browser/runtime failures fail Platform Playwright acceptance without importing Atlas-specific WebGL, canvas, map, geometry or pixel-oracle behavior.
```
## Acceptance criteria
```
- [ ] Unexpected console errors, page errors, failed requests and HTTP 5xx are rejected by acceptance diagnostics.
- [ ] Intentional resilience/error-state failures can be explicitly consumed/allowed without weakening unrelated diagnostics.
- [ ] Existing secret-safe diagnostic attachments remain available before a diagnostic gate fails.
- [ ] Focused regression tests prove clean, unexpected-failure and expected-failure behavior.
- [ ] No Atlas-specific rendering or map test subsystem is introduced.
```
## Ownership
```
```yaml
owned_paths:
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - scripts/acceptance/tests/portability-critical.spec.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/support-moderation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/admin/wiki/categories/form.blade.php
  - resources/views/player-companion/session-analyses/show.blade.php
  - public/js/form-confirmations.js
  - tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php
  - tests/Feature/Wiki/AdminWikiAdministrationTest.php
  - app/EditorialMedia/Application/WikiEditorialMediaFileResponse.php
  - tests/Feature/Wiki/WikiEditorialMediaReferenceSyncTest.php
  - docs/agents/prompts/OTERYN-RUNTIME-DIAGNOSTICS-CLOSEOUT-AGENT-PROMPT.md
  - docs/superpowers/plans/2026-08-24-runtime-diagnostics-closeout.md
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
modules:
  - acceptance test infrastructure
  - bounded frontend defects exposed by runtime diagnostics
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```
`
## Context checkpoint
```
```yaml
checkpoint_version: 1
updated_at: 2026-08-24T17:01:02Z
head: 84a1c6b0e289b76ff9b6d1536bc40411394b4be7
branch: test/runtime-diagnostics-hardening-20260823
pr: 1242
status: validating
phase: validate
task_kind: implementation
execution_mode: local-terminal
execution_reason: resumed existing worktree; bounded acceptance diagnostics and directly exposed frontend repairs
project_lane: oteryn-platform-core
context_routes:
  - testing
  - web-cms
owned_paths:
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - scripts/acceptance/tests/portability-critical.spec.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/support-moderation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/admin/wiki/categories/form.blade.php
  - resources/views/player-companion/session-analyses/show.blade.php
  - public/js/form-confirmations.js
  - tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php
  - tests/Feature/Wiki/AdminWikiAdministrationTest.php
  - app/EditorialMedia/Application/WikiEditorialMediaFileResponse.php
  - tests/Feature/Wiki/WikiEditorialMediaReferenceSyncTest.php
  - docs/agents/prompts/OTERYN-RUNTIME-DIAGNOSTICS-CLOSEOUT-AGENT-PROMPT.md
  - docs/superpowers/plans/2026-08-24-runtime-diagnostics-closeout.md
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
  - scripts/acceptance/tests/payment-foundation-acceptance.spec.mjs
  - tests/Feature/Wiki/WikiEditorialMediaServingTest.php
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/resilience-critical.spec.mjs
  - lang/en/errors.php
  - lang/pl/errors.php
  - resources/views/errors/409.blade.php
  - docs/agents/tasks/active/OTERYN-20260823-platform-transfer-terminal-reconciliation.md
proven:
  - Fresh closeout validation on candidate 84a1c6b0e289b76ff9b6d1536bc40411394b4be7 passes runtime diagnostics 23/23, changed-JS syntax, git diff --check, PHP 8.5 syntax, three focused PHPUnit scenarios with --fail-on-warning, PHPStan 712/712 with zero errors, and Playwright enumeration for primary/Wiki/Editorial Media configs.
  - Latest origin/main was integrated into the task branch without force-push; the only merge conflict was the already-archived platform-transfer task, resolved by preserving main archive state.
  - Exact-head Editorial Media run 32708438620 isolated a classifier bug for repeated identical expected 403 /admin/media outcomes; no product/runtime suppression was added.
  - Exact-head CI exposed terminal PR #1243 as a global governance blocker; its task has no live ownership and is transitioned to archive_pending without changing Issue #1155 state.
  - GitHub main d0ffc93855cba744ca5dc654651f528c962970aa was integrated without conflicts before the candidate.
  - PR #1242 remains the task PR and no overlapping active owner was found for the changed runtime-diagnostics or frontend paths.
  - Previous exact-head CI failures were classified as invalid Wiki HTML pattern, explicit expected HTTP outcomes, browser navigation cancellation noise, CSP violations, or real request failures.
  - Runtime diagnostics now support explicit bounded HTTP 400-599 allowances by exact status, exact pathname and count while preserving unexpected 5xx failure behavior.
  - Chromium net::ERR_ABORTED, Firefox NS_BINDING_ABORTED and WebKit Load request cancelled are the only added navigation-cancellation signatures; ordinary request failures remain fatal.
  - Wiki stable-key/content-type HTML patterns escape the hyphen for current HTML pattern v-mode semantics.
  - Player Companion delete confirmation no longer uses an inline event handler and is implemented through external same-origin JavaScript.
  - Wiki stale-write conflicts render through a CSP-safe localized 409 error view without inline style.
  - Focused PHP feature tests pass in isolated PHP 8.5 Docker: Wiki stale conflict 32 assertions; Player Companion owner/delete 13 assertions.
  - Node runtime-diagnostics regression suite passes 23 tests, including expected 403/404/409/419/422/429/500/503, bounded consumption, surplus/wrong status/path, cancellation signatures, ordinary failures, CSP fatality, Wiki v-mode pattern semantics and the WebKit evidence regression.
  - Playwright 1.62.1 WebKit screenshot behavior was traced to a temporary inline style mutation; local commit 740ec45 avoids that screenshot operation for WebKit while preserving fail-closed CSP diagnostics and explicit evidence.
  - Wiki media focused feature test proves missing stored media returns 404 while corrupt/integrity-failed media remains 500; exit 0 with 10 assertions on the current WIP.
derived:
  - Cross-browser Wiki media behavior now uses missing-file 404 for the browser fallback and explicit authenticated request-context 500 assertion for corruption so generic Firefox request failures remain fatal rather than suppressed.
unknown:
  - Exact-head GitHub Actions have not yet run on the final pushed closeout candidate; branch push and exact-head CI inspection are the next action.
conflicts: []
first_failure:
  marker: Editorial Media exact-head CI repeated-identical-allowance bug repaired
  evidence: run 32708438620 on a95ba7c48c6067a6a7782a0b52eb30245e0a913b showed two distinct expected 403 /admin/media declarations competing for the first exhausted allowance; focused RED reproduced the same unexpected+missing pair and GREEN passes 22/22 after selecting the first unexhausted exact allowance
rejected_hypotheses:
  - blanket console/status/requestfailure suppression: rejected; classifier remains exact and bounded
  - Atlas renderer/map/WebGL/canvas/geometry migration: rejected as out of scope
changed_paths:
  - public/js/form-confirmations.js
  - resources/views/admin/wiki/articles/form.blade.php
  - resources/views/admin/wiki/categories/form.blade.php
  - resources/views/player-companion/session-analyses/show.blade.php
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/portability-critical.spec.mjs
  - scripts/acceptance/tests/public-localization.spec.mjs
  - scripts/acceptance/tests/support-moderation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php
  - tests/Feature/Wiki/AdminWikiAdministrationTest.php
  - app/EditorialMedia/Application/WikiEditorialMediaFileResponse.php
  - tests/Feature/Wiki/WikiEditorialMediaReferenceSyncTest.php
  - docs/agents/prompts/OTERYN-RUNTIME-DIAGNOSTICS-CLOSEOUT-AGENT-PROMPT.md
  - docs/superpowers/plans/2026-08-24-runtime-diagnostics-closeout.md
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
  - scripts/acceptance/tests/payment-foundation-acceptance.spec.mjs
  - tests/Feature/Wiki/WikiEditorialMediaServingTest.php
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/resilience-critical.spec.mjs
  - lang/en/errors.php
  - lang/pl/errors.php
  - resources/views/errors/409.blade.php
  - docs/agents/tasks/active/OTERYN-20260823-platform-transfer-terminal-reconciliation.md
validation:
  - command: composer analyse in oteryn-php85-gd-validation:local with task vendor volume
    result: PASS
    evidence: PHPStan analysed 712 files with [OK] No errors on candidate 84a1c6b0e289b76ff9b6d1536bc40411394b4be7
  - command: focused PHPUnit Wiki media, Wiki stale-conflict, and Player Companion delete with --fail-on-warning
    result: PASS
    evidence: 8 + 33 + 13 assertions; zero failures and zero warnings after restoring a local ignored .env from .env.example and repairing only generated testing/disks permissions
  - command: Playwright --list for playwright.config.mjs, playwright.wiki-reconciliation.config.mjs, playwright.editorial-media.config.mjs
    result: PASS
    evidence: 192 + 20 + 12 tests enumerate cleanly
  - command: npm --prefix scripts/acceptance run test:runtime-diagnostics after repeated-allowance repair
    result: PASS
    evidence: 22/22 tests; dedicated regression was RED before the fix and GREEN after selecting an unexhausted identical allowance
  - command: npm --prefix scripts/acceptance run test:runtime-diagnostics
    result: PASS
    evidence: 23/23 Node tests pass after the WebKit screenshot evidence regression
  - command: node --check changed acceptance MJS and public/js/form-confirmations.js
    result: PASS
    evidence: changed JavaScript parses successfully
  - command: Playwright --list for primary, Wiki reconciliation and Editorial Media configs
    result: PASS
    evidence: changed acceptance suites enumerate successfully
  - command: git diff --check and git diff --cached --check
    result: PASS
    evidence: no whitespace errors
  - command: isolated PHP 8.5 Docker focused feature tests
    result: PASS
    evidence: Wiki stale-conflict test exit 0 with 32 assertions; Player Companion owner/delete test exit 0 with 13 assertions; language PHP lint PASS
  - command: isolated PHP 8.5 Docker Wiki media missing-vs-corrupt feature test
    result: PASS
    evidence: exit 0 with 10 assertions; missing storage is 404 and corrupt/integrity failure remains 500
  - command: node --check admin-wiki-editorial-media.spec.mjs plus git diff --check and PHP 8.5 php -l on current WIP PHP files
    result: PASS
    evidence: current handoff WIP parses cleanly and has no whitespace errors
blockers:
  - none
invocation_started_at: 2026-08-24T06:50:00Z
last_progress_at: 2026-08-24T17:01:02Z
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: commit this fresh checkpoint, push the existing branch, verify PR #1242 exact head, then inspect exact-head CI and close out only if all required gates are green
```
`
## Recovery checkpoint
```
```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: runtime-diagnostics-20260824-resume
  session_started_at: 2026-08-24T06:50:00Z
  checkpointed_at: 2026-08-24T17:01:02Z
  last_progress_at: 2026-08-24T17:01:02Z
  phase: validate
  exact_head: 84a1c6b0e289b76ff9b6d1536bc40411394b4be7
  pull_request: 1242
  active_operation: handoff to closeout agent from saved WIP checkpoint
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: dedicated branch remains owned by this task and PR #1242 remains open
  next_action: commit this fresh checkpoint, push the existing branch, verify PR #1242 exact head, then inspect exact-head CI and close out only if all required gates are green
```
`
## Source branch closeout
```
```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```
`
## Notes
```
Bounded port of Atlas E2E rigor only. No Atlas rendering/map subsystem and no production/staging mutation.
