---
task_id: OTERYN-20260805-adr-registry-validator
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 577
status: validating
branch: task/OTERYN-20260805-adr-registry-validator
base_branch: main
exact_base: 3f79987f47e5c7593daccdf1136e09d6641017de
created: 2026-08-05T16:18:00Z
updated: 2026-08-05T16:40:00Z
execution_mode: github-only
risk: medium
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: false
owned_paths:
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tests/Unit/Architecture/AdrRegistryValidationTest.php
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/tasks/archive/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/reports/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
shared_path_lease: []
excluded_overlap:
  - tools/agents/** and Issue 558
  - .github/workflows/** and active PR 542 workflow-bearing scope
  - PR 541 public-edge scope
  - runtime, migrations, dependencies, deployment and production systems
---

# ADR registry validator

## Goal

Add a fail-closed, compatibility-preserving validator for the Oteryn Platform ADR registry without renaming or renumbering historical decisions.

## Delivery matrix

```yaml
validator_cli: required
positive_negative_boundary_tests: required
existing_ci_integration_without_workflow_edit: required
legacy_duplicate_preservation: required
new_duplicate_rejection: required
lifecycle_validation: required
readme_inventory_validation: required
supersession_target_validation: required
runtime_e2e: not_applicable_non_runtime_repository_integrity_tool
```

## Acceptance

- [x] deduplicate against live Issues, PRs and repository code;
- [x] create Issue #577 with compatibility and scope boundaries;
- [x] select an exact-path closed allowlist for historical duplicate prefixes;
- [x] implement the standard-library validator;
- [x] add positive, negative and boundary fixture tests;
- [x] execute the validator from the existing PHPUnit suite without workflow changes;
- [x] update the ADR registry and authority documentation;
- [x] diagnose the first exact-head failure from uploaded PHPUnit evidence;
- [x] preserve all established lifecycle metadata formats without rewriting historical ADRs;
- [ ] pass repaired focused and exact-head validation;
- [ ] complete fresh post-implementation audit with zero material findings;
- [ ] merge, archive, close Issue #577 and release ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-05T16:40:00Z
invocation_started_at: 2026-08-05T15:50:00Z
last_progress_at: 2026-08-05T16:40:00Z
head: 2d1d59fffe8d0163ff49a42afb7c0c18d7521655
branch: task/OTERYN-20260805-adr-registry-validator
pr: 581
status: validating
phase: repair_validation
session_id: chat-20260805-architecture-continuation
session_role: implementer
execution_mode: github-only
execution_reason: bounded repository tooling and documentation changes can be implemented through GitHub objects and validated by GitHub Actions
lease_expires_at: 2026-08-05T17:25:00Z
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one validator, one compatibility contract and one existing CI consumer form a cohesive bounded slice
context_routes:
  - architecture
  - testing
owned_paths:
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tests/Unit/Architecture/AdrRegistryValidationTest.php
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/tasks/archive/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/reports/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
proven:
  - ADR 0022 requires a fail-closed follow-up validator while preserving accepted historical paths.
  - Duplicate prefixes currently exist for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021.
  - No exact existing Issue, PR or implementation owner was found before Issue 577 was created.
  - Issue 558 owns tools/agents and task-liveness governance, not ADR registry integrity.
  - Existing CI executes PHPUnit; workflow files need not be changed to consume this validator.
  - The initial focused Python fixture suite passed but omitted two established lifecycle syntax families.
  - CI run 31025277136 failed only in the live registry bridge because 17 accepted ADRs use plain or section lifecycle declarations.
  - The repaired focused suite passes 10 tests and covers bullet, plain and section forms plus ambiguous declarations.
derived:
  - A closed exact-path legacy allowlist is the least disruptive compatibility boundary.
  - Parser compatibility is safer and narrower than rewriting 17 historical ADRs.
unknown:
  - Repaired exact-head GitHub Actions result.
conflicts: []
first_failure:
  marker: CI PHPUnit ADR registry validation
  evidence: run 31025277136, job 92372884204, artifact 8938486455; test_repository_adr_registry_passes reported 17 established ADRs with zero recognized lifecycle declarations
rejected_hypotheses:
  - The failure was not caused by a new ADR collision, README inventory drift or invalid filename.
  - The failure was not a runtime, database, edge, game-auth or native-protocol regression.
  - Python and Symfony Process were available because the focused bridge test executed successfully.
  - Renumbering or normalizing historical ADRs is not required and would risk breaking references.
  - An open-ended duplicate-prefix allowlist would not fail closed.
  - A workflow edit is not required for CI enforcement.
changed_paths:
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tests/Unit/Architecture/AdrRegistryValidationTest.php
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/reports/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
validation:
  - command: python3 tools/validation/test_adr_registry.py after lifecycle parser repair
    result: PASS
    evidence: 10 focused positive, negative and boundary tests passed
  - command: exact failed-head GitHub Actions on 2d1d59fffe8d0163ff49a42afb7c0c18d7521655
    result: FAIL
    evidence: CI run 31025277136 isolated parser incompatibility; other unrelated exact-head checks passed
  - command: repaired exact repository ADR validation
    result: NOT_RUN
    evidence: new exact-head GitHub Actions required
ci_checks_for_current_head: 9
ci_check_generation: failed_head_diagnosed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 9
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
blockers: []
next_action: Commit the lifecycle parser repair and run all exact-head GitHub Actions on the repaired tree.
```

## E2E

`NOT_APPLICABLE`: this task validates repository ADR metadata and does not alter a runtime or user-facing path.
