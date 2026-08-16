---
task_id: OTERYN-20260815-client-distribution-platform
repository: blakinio/Oteryn-Platform
mode: implementation
issue: 1039
programme: OTERYN_PORTAL_COMPLETION
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
---

# OTERYN-20260815-client-distribution-platform

## Goal

Implement the complete Platform-only vertical slice of ADR 0035 for first-party client distribution without taking custody of private signing keys, mutating an external updater repository, writing another repository, deploying, or activating production.

## Acceptance criteria

- [x] Browser Download Center publication remains independent from updater trust state.
- [x] Updater-enabled releases use opaque identities and channel-scoped positive monotonic integer sequences, never display-version ordering.
- [x] Updater targets are exact platform + architecture targets.
- [x] Immutable policy revisions model minimum support, optional/recommended/required mode, withdrawal, release revocation, target revocation and explicit rollback.
- [x] Browser publication, policy intent, protected-generation reconciliation and Platform-active state are separate facts.
- [x] Platform accepts only a bounded PUBLIC generation projection through an internal protected-integration boundary; ordinary web administration cannot import or activate signed-generation state.
- [x] No private signing key or signing secret is accepted or persisted.
- [x] Policy and generation identities are idempotent and ambiguity-safe; replay, stale metadata, expiry, channel/policy/target mismatch and rollback fail closed.
- [x] Administrator-supplied SHA-256 is presented truthfully and never as publisher-signature verification.
- [x] Admin diagnostics remain behind authentication + confirmed MFA + `downloads.manage`.
- [x] Unit, Feature and zero-retry Playwright coverage exercise the complete Platform lifecycle, negative boundaries, EN/PL presentation and Firefox/WebKit portability.
- [x] Existing `downloads-acceptance.yml` is extended; no new persistent task-specific workflow is created and workflow budget remains 53.
- [x] Migration rollback/re-apply is proven in Downloads Acceptance.
- [ ] Exact final PR head passes all required GitHub CI and acceptance contracts.
- [x] Whole-diff self-review found and repaired the ordinary-admin signed-generation authority defect; final review/thread hygiene will be reconfirmed on the final head.
- [ ] Issue #1039, source branch and task lifecycle are terminal after implementation merge.
- [x] Real protected signer/repository integration and real first-party updater/client cross-repository E2E remain separately authorized future gates and are not claimed.

## Ownership

```yaml
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/downloads-updater-admin-trust.json
  - docs/testing/portal-content-scale-surfaces/downloads-updater-admin-trust.json
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - .github/workflows/downloads-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - docs/agents/tasks/archive/OTERYN-20260815-client-distribution-platform.md
modules:
  - Downloads
  - client distribution Platform trust boundary
dependencies:
  - ADR 0035 accepted on protected main
  - existing immutable Download Center boundary
  - existing Downloads Acceptance workflow
blockers: []
```

## Architecture boundary

Platform stores browser metadata, updater policy intent, a bounded public generation projection received from a separately protected integration, and Platform-active reconciliation state. Laravel owns no private updater signing keys and performs no claimed TUF client verification. Matching target path/length/digest proves association consistency only; the first-party updater remains the authoritative TUF verifier. External signer/repository writes, production activation and cross-repository client E2E are outside this task's authorization.

## Reconstruction evidence

```yaml
protected_main: 9336cd1f240196908a84cdea124992300bede59c
workflow_policy: reuse_or_extend
workflow_budget: 53
new_persistent_workflows: 0
clean_material_commit: da514f3eda178e904a6fc481616a737d1b12d8c4
formatter_repair_commit: aaec9fe4cda15c48da12d5a1646cd7eba018aad2
phase7_fixture_repair_commit: 2e8d81c9c8a957c1aeef4b5ff2ecffa8c9106fde
static_analysis_repair_commit: b67c361aa924e96b9ce54c2ba1c15d5bb104d724
route_classification_commit: e1c4084615883d4456bd133db8a2045cb525c084
complete_portal_ledgers_commit: 1852ba017255d1e42251f174a16d735e571a0928
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T16:22:00+02:00
head: 1852ba017255d1e42251f174a16d735e571a0928
status: validating
phase: final_exact_head_validation_after_content_scale_fixture_count_repair
branch: feat/issue-1039-client-distribution-platform
pr: 1073
context_routes:
  - agent-governance
  - security
  - testing
  - api
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/downloads-updater-admin-trust.json
  - docs/testing/portal-content-scale-surfaces/downloads-updater-admin-trust.json
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - .github/workflows/downloads-acceptance.yml
repository_mutation_authorization: PROVEN
external_repository_mutation_authorization: ABSENT_AND_NOT_REQUIRED
production_activation_authorization: ABSENT_AND_NOT_REQUIRED
owner_funded_agent_authorization: ABSENT_AND_NOT_USED
proven:
  - protected main was re-read at 9336cd1f240196908a84cdea124992300bede59c before continuation
  - ADR 0035 remains the accepted authority for this slice
  - ordinary downloads.manage web administration cannot import or activate protected generation state
  - no private updater signing key or signing secret is accepted or persisted
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed CI 31951774672 including Pint, PHPStan/Larastan and full PHPUnit
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed Phase 7 31951774659
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed Downloads Acceptance 31951774825 including migration rollback/replay, zero-retry Chromium lifecycle and Firefox/WebKit portability
  - route classification commit e1c4084615883d4456bd133db8a2045cb525c084 classified exactly the four new updater admin routes without exclusions
  - complete portal ledgers commit 1852ba017255d1e42251f174a16d735e571a0928 classified downloads.updater-admin-trust in content-scale, dimension and media contracts
  - Portal Acceptance Contract 31952358341 proved content-scale validator status complete with portal_surface_count 30, classified_surface_count 30, zero errors and zero gaps
  - the sole 31952358341 failure was the deterministic content-scale fixture asserting the previous fixed count 29 instead of 30
  - repository search found no other coverage fixture hardcode requiring the old 29-surface count
  - whole-diff self-review repaired the architecture authority issue; submitted reviews and review threads were empty before final evidence-only repairs
derived:
  - the current strict failure is a fixture cardinality update required by the intentionally added portal surface, not a runtime or evidence-contract defect
  - changing the fixture expectation from 29 to 30 preserves strictness because the validator independently proves exact classification completeness and zero gaps
unknown:
  - terminal conclusions of all required CI and acceptance workflows for the fixture-repair successor head until GitHub executes them
  - real protected signer/repository integration and real updater/client cross-repository behavior because those separate authorities are not granted
conflicts: []
first_failure:
  marker: content-scale-fixture-cardinality-repaired
  evidence: Portal Acceptance Contract 31952358341 validator produced complete 30/30 with no errors, then test-portal-content-scale-evidence.mjs line 23 failed only because it expected 29
rejected_hypotheses:
  - display version can safely order security updates
  - browser is_current can substitute for updater-active state
  - administrator-supplied SHA-256 proves publisher authenticity
  - Laravel should own TUF private signing keys
  - ordinary web administration should mint signed-generation state
  - strict route, scale, dimension or media validators should be weakened or exclude updater admin routes
  - the 30-surface result is a coverage gap
changed_paths:
  - .github/workflows/downloads-acceptance.yml
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/portal-content-scale-surfaces/downloads-updater-admin-trust.json
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - scripts/acceptance/coverage/surfaces/downloads-updater-admin-trust.json
  - scripts/acceptance/coverage/test-portal-content-scale-evidence.mjs
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - tests/Feature/Downloads/**
  - tests/Unit/Downloads/**
validation:
  - command: CI 31951774672 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: Composer audit, Pint, PHPStan/Larastan and full PHPUnit succeeded
  - command: Phase 7 31951774659 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: production-like boundaries, critical regression, backup/restore and upgrade/rollback succeeded
  - command: Downloads Acceptance 31951774825 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: rollback/replay, zero-retry Chromium and Firefox/WebKit portability succeeded
  - command: Portal Acceptance Contract 31952358341 on 1852ba017255d1e42251f174a16d735e571a0928
    result: FAIL
    evidence: content-scale validator was complete 30/30 with zero errors; deterministic fixture alone expected obsolete count 29
blockers: []
invocation_started_at: 2026-08-16T15:05:00+02:00
last_progress_at: 2026-08-16T16:22:00+02:00
ci_checks_for_current_head: 11
ci_check_generation: fixture_count_successor
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: Publish the deterministic content-scale fixture count repair in one fast-forward commit, then validate that exact successor head through all required CI and acceptance contracts before final review hygiene and expected-head squash merge.
```

## Recovery checkpoint

```yaml
checkpoint_type: RECOVERY
captured_at: 2026-08-16T16:22:00+02:00
current_state: application validation is green on the material implementation and portal ledgers validate 30/30; only the deterministic fixture still expected the historical 29-surface cardinality
last_completed_step: audited all coverage fixture tests and isolated the single obsolete hardcoded count
active_operation: publish fixture cardinality repair and regenerate exact-head validation
external_run_ids:
  - 31951774672
  - 31951774659
  - 31951774825
  - 31952358341
expected_success_marker: successor head has strict portal coverage plus required CI, Downloads Acceptance and Phase 7 SUCCESS with zero unresolved material review findings
expected_failure_marker: first failing required job or final-head review finding
wait_deadline: bounded terminal-CI exception only after final head is confirmed and non-CI gates remain satisfied
next_step_on_success: final review hygiene, mark PR ready, exact expected-head squash merge, close Issue #1039, lifecycle-only archive closeout, branch deletion and selector rerun
next_step_on_failure: inspect first relevant failing job log and repair the root cause on the same branch
parallel_work_allowed: false
parallel_work_scope: none during final exact-head validation
context_pressure: low
rotation_reason: null
```
