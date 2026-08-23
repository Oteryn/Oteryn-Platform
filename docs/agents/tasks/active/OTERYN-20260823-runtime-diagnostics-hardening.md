---
task_id: OTERYN-20260823-runtime-diagnostics-hardening
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
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/*acceptance*.spec.mjs
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - scripts/acceptance/package.json
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
modules:
  - acceptance test infrastructure
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
updated_at: 2026-08-23T12:02:31Z
head: 822a516c049bbd519ec903d1d060538806976752
branch: test/runtime-diagnostics-hardening-20260823
pr: 1242
status: validating
phase: validate
task_kind: implementation
execution_mode: local-terminal
execution_reason: bounded multi-file acceptance harness change with required test loop
project_lane: oteryn-platform-core
context_routes:
  - testing
owned_paths:
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/*acceptance*.spec.mjs
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - scripts/acceptance/package.json
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
proven:
  - Base SHA 0ccbcdc48401e28360f6f814386319cf2c6e7f5d collected runtime failures but attachDiagnostics did not fail the test.
  - Existing resilience and error-state scenarios intentionally create HTTP 5xx and must retain explicit exception handling.
  - Existing Platform visual acceptance already collects UI screenshots/evidence; Atlas WebGL/canvas pixel oracles are out of scope.
  - Unexpected console/page/request/5xx diagnostics now fail after secret-safe attachments are written.
  - All current intentional HTTP 5xx acceptance paths are explicitly bounded by status, pathname and occurrence count.
  - Browser navigation net::ERR_ABORTED remains non-fatal; other request failures remain fatal.
  - Exact-head self-review found and repaired the stale Wiki raw-serverErrors assertion without clearing expected evidence.
derived:
  - The smallest useful port is a strict unexpected-runtime-failure gate with explicit expected-failure consumption.
unknown: []
conflicts: []
first_failure:
  marker: SELF-REVIEW-001 repaired
  evidence: wiki public reconciliation retained the expected 503 in diagnostics but still asserted raw serverErrors == []; repaired by 822a516c049bbd519ec903d1d060538806976752
rejected_hypotheses:
  - importing Atlas map/WebGL/canvas/geometry visual tests
changed_paths:
  - scripts/acceptance/package.json
  - scripts/acceptance/runtime-diagnostics.mjs
  - scripts/acceptance/unit/runtime-diagnostics.test.mjs
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/community-data-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/error-state-acceptance.spec.mjs
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/tests/resilience-critical.spec.mjs
  - scripts/acceptance/tests/wiki-reconciliation-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
validation:
  - command: npm --prefix scripts/acceptance run test:runtime-diagnostics
    result: PASS
    evidence: 4 Node regression tests pass, including bounded expected 5xx and real request-failure behavior
  - command: node --check changed acceptance .mjs files
    result: PASS
    evidence: all changed executable JavaScript parsed successfully
  - command: npx --prefix scripts/acceptance playwright test --config=scripts/acceptance/playwright.config.mjs --list
    result: PASS
    evidence: Playwright enumerated the suite successfully
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors
blockers:
  - none
invocation_started_at: 2026-08-23T11:06:16Z
last_progress_at: 2026-08-23T12:02:31Z
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: push the repaired candidate and inspect aggregate PR #1242 required checks
```
`
## Recovery checkpoint
```
```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: runtime-diagnostics-20260823
  session_started_at: 2026-08-23T11:06:16Z
  checkpointed_at: 2026-08-23T12:02:31Z
  last_progress_at: 2026-08-23T12:02:31Z
  phase: validate
  exact_head: 822a516c049bbd519ec903d1d060538806976752
  pull_request: 1242
  active_operation: push candidate and inspect PR #1242 required checks
  external_run_ids: []
  operation_started_at: 2026-08-23T12:02:31Z
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: dedicated branch remains unowned and based on current main
  next_action: push the repaired candidate and inspect aggregate PR #1242 required checks
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
