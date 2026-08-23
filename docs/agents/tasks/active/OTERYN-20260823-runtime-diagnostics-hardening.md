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
  - scripts/acceptance/tests/runtime-diagnostics.test.mjs
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
updated_at: 2026-08-23T11:06:16Z
head: 0ccbcdc48401e28360f6f814386319cf2c6e7f5d
branch: test/runtime-diagnostics-hardening-20260823
pr: none
status: investigating
phase: investigate
task_kind: implementation
execution_mode: local-terminal
execution_reason: bounded multi-file acceptance harness change with required test loop
project_lane: oteryn-platform-core
context_routes:
  - testing
owned_paths:
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/*acceptance*.spec.mjs
  - scripts/acceptance/tests/runtime-diagnostics.test.mjs
  - scripts/acceptance/package.json
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
proven:
  - Platform diagnostics currently collect runtime failures but attachDiagnostics does not fail the test.
  - Existing resilience and error-state scenarios intentionally create HTTP 5xx and must retain explicit exception handling.
  - Existing Platform visual acceptance already collects UI screenshots/evidence; Atlas WebGL/canvas pixel oracles are out of scope.
derived:
  - The smallest useful port is a strict unexpected-runtime-failure gate with explicit expected-failure consumption.
unknown:
  - exact set of intentional failure scenarios requiring consumption helpers
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - importing Atlas map/WebGL/canvas/geometry visual tests
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not started
blockers:
  - none
invocation_started_at: 2026-08-23T11:06:16Z
last_progress_at: 2026-08-23T11:06:16Z
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: inventory intentional runtime failures, then add a failing diagnostics regression test
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
  checkpointed_at: 2026-08-23T11:06:16Z
  last_progress_at: 2026-08-23T11:06:16Z
  phase: investigate
  exact_head: 0ccbcdc48401e28360f6f814386319cf2c6e7f5d
  pull_request: none
  active_operation: none
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: dedicated branch remains unowned and based on current main
  next_action: inventory intentional runtime failures, then add a failing diagnostics regression test
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
