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

- [x] Existing browser Download Center publication and immutable approved artifact references remain supported independently from updater state.
- [x] Updater-enabled releases receive opaque stable identities and positive channel-scoped monotonic integer sequences; display-version text is never security ordering.
- [x] Updater targets are exact platform + architecture targets with no fallback identity.
- [x] Immutable policy revisions model minimum support, optional/recommended/required update mode, release revocations, exact target revocations and explicit older-sequence rollback authorization.
- [x] Withdrawal is separate from revocation and preserves browser/release history.
- [x] Browser publication, approved policy/signing intent, reconciled protected-integration generation state and Platform-active updater state are separate facts.
- [x] Platform stores and validates a bounded PUBLIC generation projection only through an internal protected-integration boundary after required TUF verification outside Laravel; no ordinary web-admin route can import or activate it.
- [x] No private signing key or signing secret is accepted or persisted, including through direct internal action input.
- [x] Policy operation identity and protected-generation identity are idempotent; identity reuse with different intent fails closed.
- [x] Import rejects stale/replayed metadata, expired metadata, channel/policy mismatch, policy-target mismatch, duplicate/extra/missing platform targets and exact target/length/digest mismatch.
- [x] Metadata rollback is fail-closed while freshness-only Timestamp advancement can preserve unchanged Root/Targets/Snapshot versions.
- [x] Activation rejects stale, expired, superseded, non-latest-policy and withdrawn-current generations.
- [x] Browser/admin presentation describes administrator-supplied SHA-256 truthfully and never presents it as publisher-signature verification.
- [x] Admin diagnostics are behind the existing auth + confirmed MFA + `downloads.manage` boundary and explicitly state that Platform reconciliation is not TUF client verification or production activation.
- [x] Unit and Feature coverage include sequence ordering, rollback, withdrawal, revocation, idempotency, replay/staleness, freshness-only metadata advancement, mismatch handling, private-key rejection, absent web-admin generation mutation routes and browser/updater divergence.
- [x] Existing Downloads Playwright acceptance exercises browser publication plus updater enable → policy approval → acceptance-only protected-integration simulation → Platform-active read-only diagnostics and localized presentation; it explicitly does not claim cryptographic TUF signing or production activation.
- [x] Existing `downloads-acceptance.yml` is extended rather than creating a new workflow; workflow budget remains 53.
- [x] The isolated acceptance workflow proves migration rollback and re-apply before browser lifecycle execution.
- [ ] Exact final PR head passes required GitHub CI and Downloads Acceptance.
- [x] Whole-diff self-review and review-thread hygiene are complete with no unresolved material finding on the final material implementation head.
- [ ] Issue #1039, source branch and task lifecycle are terminal after implementation merge.
- [x] Real protected signer/repository integration and real first-party updater/client cross-repository E2E remain separately authorized future gates and are not claimed by this task.

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

- Platform persists browser metadata, updater policy intent, exact bounded public generation projection received from the protected integration and Platform-active reconciliation state.
- Platform does not persist private updater signing keys, expose an ordinary web-admin generation import/activation route, or perform/claim TUF signature verification.
- The internal generation reconciliation action is a receiving boundary for a future separately protected release-publishing integration after that integration has verified the required TUF signatures/repository coherence; the real integration itself is outside this task.
- Exact policy association is checked by canonical policy target path + byte length + SHA-256 and exact target tuple/path/length/digest reconciliation.
- The administrator-supplied artifact SHA-256 remains supplied metadata; matching it to the bounded public target projection proves association consistency, not independent publisher authenticity.
- External signer/repository writes, production activation and real first-party updater/client cross-repository E2E are outside this task's authority.

## Reconstruction evidence

```yaml
protected_main: 9336cd1f240196908a84cdea124992300bede59c
protected_main_tree: 5d1921a69091816b98e2c22b7e0621d1cd82b1b0
previous_pr_head: 72a0aea11c4b610a2b7e561f39a5fa68c0ad05cc
previous_material_implementation: none
overlap_resolution: reconstruct exact implementation tree from current protected main; do not replay stale implementation history
workflow_policy: reuse_or_extend
workflow_budget: 53
new_persistent_workflows: 0
clean_material_commit: da514f3eda178e904a6fc481616a737d1b12d8c4
first_published_reconstruction_head: 4a2fa4e3b19571e257d5462f6c6b58ef841a4ad0
formatter_repair_commit: aaec9fe4cda15c48da12d5a1646cd7eba018aad2
phase7_fixture_repair_commit: 2e8d81c9c8a957c1aeef4b5ff2ecffa8c9106fde
static_analysis_repair_commit: b67c361aa924e96b9ce54c2ba1c15d5bb104d724
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T16:09:00+02:00
head: b67c361aa924e96b9ce54c2ba1c15d5bb104d724
status: validating
phase: final_exact_head_validation_after_portal_route_classification_repair
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
  - .github/workflows/downloads-acceptance.yml
repository_mutation_authorization: PROVEN
external_repository_mutation_authorization: ABSENT_AND_NOT_REQUIRED
production_activation_authorization: ABSENT_AND_NOT_REQUIRED
owner_funded_agent_authorization: ABSENT_AND_NOT_USED
proven:
  - protected main was re-read at 9336cd1f240196908a84cdea124992300bede59c before continuation
  - PR #1073 is the sole open Downloads implementation owner found by live PR search
  - ADR 0035 and CLIENT_DISTRIBUTION_ARCHITECTURE remain the accepted authority for this slice
  - ordinary downloads.manage web administration cannot import or activate protected generation state
  - no private updater signing key or signing secret is accepted or persisted by the Platform implementation
  - canonical formatter repair commit aaec9fe4cda15c48da12d5a1646cd7eba018aad2 changed exactly the 22 PHP files reported by Pint
  - Phase 7 MFA fixture root cause was repaired by explicit forceFill persistence in 2e8d81c9c8a957c1aeef4b5ff2ecffa8c9106fde without weakening middleware
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed CI 31951774672 including Pint, PHPStan/Larastan and the full PHPUnit suite
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed Phase 7 Production-Like Validation 31951774659 including critical regression, backup/restore and upgrade/rollback coverage
  - exact head b67c361aa924e96b9ce54c2ba1c15d5bb104d724 passed Downloads Acceptance 31951774825 including migration rollback/replay, zero-retry Chromium lifecycle and Firefox/WebKit portability
  - Portal Acceptance Contract 31951774841 found exactly four unclassified new updater admin routes after all content-scale checks passed with zero gaps
  - whole-diff self-review was completed after architecture-authority repair and formatter cleanup; review threads and submitted reviews were empty at that checkpoint
derived:
  - the strict portal failure is a coverage-ledger completeness defect, not an application runtime defect
  - the canonical surface-fragment mechanism should classify the four updater admin routes with their existing Feature and Playwright evidence rather than weakening route discovery or exclusions
  - final exact-head validation must be regenerated after the coverage fragment is committed
unknown:
  - terminal conclusions of required CI and acceptance workflows for the coverage-fragment successor head until GitHub executes them
  - real protected signer/repository integration behavior because that separate authority and implementation are outside this task
  - real updater/client cross-repository behavior because that separate authority is not granted
conflicts: []
first_failure:
  marker: portal-strict-four-updater-routes-unclassified
  evidence: Portal Acceptance Contract 31951774841 strict coverage reported only admin.downloads.updater, admin.downloads.updater.enable, admin.downloads.updater.policies.store and admin.downloads.updater.withdraw as unclassified routes
rejected_hypotheses:
  - lexical display version can safely order updates
  - browser is_current can stand in for updater-active state
  - administrator-supplied SHA-256 proves publisher authenticity
  - Laravel should own private TUF signing keys
  - ordinary downloads.manage web administration should be able to mint reconciled signed-generation state
  - TUF Targets and Snapshot versions must increase on every Timestamp freshness refresh
  - a new persistent task-specific workflow is needed
  - the Pint failure represented application behavior rather than source formatting
  - the Phase 7 diagnostic should bypass MFA or downloads.manage authorization
  - updater admin routes should be excluded from strict portal route discovery
changed_paths:
  - .github/workflows/downloads-acceptance.yml
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - scripts/acceptance/coverage/surfaces/downloads-updater-admin-trust.json
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - tests/Feature/Downloads/**
  - tests/Unit/Downloads/**
validation:
  - command: CI 31951774672 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: Composer metadata/audit, Pint, PHPStan/Larastan and full PHPUnit suite succeeded
  - command: Phase 7 Production-Like Validation 31951774659 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: production-like dependency boundaries, critical regression suite, backup/restore and upgrade/rollback/redeploy succeeded
  - command: Downloads Acceptance 31951774825 on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: PASS
    evidence: migration rollback/replay, zero-retry Chromium lifecycle and Firefox/WebKit portability succeeded
  - command: Portal Acceptance Contract 31951774841 strict coverage on b67c361aa924e96b9ce54c2ba1c15d5bb104d724
    result: FAIL
    evidence: all baseline content-scale checks passed with zero gaps; exactly four new updater admin routes lacked portal classification
blockers: []
invocation_started_at: 2026-08-16T15:05:00+02:00
last_progress_at: 2026-08-16T16:09:00+02:00
ci_checks_for_current_head: 11
ci_check_generation: coverage_fragment_successor
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 1
stall_warnings: 0
next_action: Publish the canonical updater admin coverage surface fragment, then verify its successor exact head through all required CI and acceptance workflows before ready-for-review and exact expected-head squash merge.
```

## Recovery checkpoint

```yaml
checkpoint_type: RECOVERY
captured_at: 2026-08-16T16:09:00+02:00
current_state: application CI, Phase 7 and Downloads Acceptance are green on b67c361; only strict portal classification of four new updater admin routes requires a canonical coverage fragment before final exact-head validation
last_completed_step: diagnosed Portal Acceptance Contract 31951774841 to four exact unclassified updater admin route names with zero content-scale gaps
active_operation: publish canonical coverage surface fragment and regenerate final exact-head validation
external_run_ids:
  - 31951774672
  - 31951774659
  - 31951774825
  - 31951774841
expected_success_marker: successor head has strict portal coverage plus required CI, Downloads Acceptance and Phase 7 SUCCESS with zero unresolved material review findings
expected_failure_marker: first failing required job or final-head review finding
wait_deadline: bounded terminal-CI exception only after the final head is confirmed and non-CI gates remain satisfied
next_step_on_success: mark PR ready, exact expected-head squash merge, close Issue #1039, create lifecycle-only archive closeout, verify source/archive branch disposition and final main
next_step_on_failure: inspect the first relevant failing job log and repair the root cause on the same branch
parallel_work_allowed: false
parallel_work_scope: none during final exact-head validation
context_pressure: low
rotation_reason: null
```
