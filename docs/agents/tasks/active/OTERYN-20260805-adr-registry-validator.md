---
task_id: OTERYN-20260805-adr-registry-validator
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 577
status: implementing
branch: task/OTERYN-20260805-adr-registry-validator
base_branch: main
exact_base: 3f79987f47e5c7593daccdf1136e09d6641017de
created: 2026-08-05T16:18:00Z
updated: 2026-08-05T16:18:00Z
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
- [ ] implement the standard-library validator;
- [ ] add positive, negative and boundary fixture tests;
- [ ] execute the validator from the existing PHPUnit suite without workflow changes;
- [ ] update the ADR registry documentation;
- [ ] run focused and exact-head validation;
- [ ] complete fresh post-implementation audit with zero material findings;
- [ ] merge, archive, close Issue #577 and release ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-05T16:18:00Z
invocation_started_at: 2026-08-05T15:50:00Z
last_progress_at: 2026-08-05T16:18:00Z
head: 3f79987f47e5c7593daccdf1136e09d6641017de
branch: task/OTERYN-20260805-adr-registry-validator
pr: none
status: implementing
phase: implement
session_id: chat-20260805-architecture-continuation
session_role: implementer
execution_mode: github-only
execution_reason: bounded repository tooling and documentation changes can be implemented through GitHub objects and validated by GitHub Actions
lease_expires_at: 2026-08-05T17:03:00Z
context_pressure: medium
context_growth: stable
context_score: 6
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
derived:
  - A closed exact-path legacy allowlist is the least disruptive compatibility boundary.
unknown:
  - Whether every historical ADR already uses a lifecycle line that matches the documented lifecycle token contract; exact validation will prove or identify bounded normalization needs.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Renumbering existing ADRs is not required and would risk breaking inbound references.
  - An open-ended duplicate-prefix allowlist would not fail closed.
  - A workflow edit is not required for CI enforcement.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-adr-registry-validator.md
validation:
  - command: live Issue, PR and repository duplicate search
    result: PASS
    evidence: no exact existing validator owner found; Issue 577 created
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
blockers: []
next_action: Implement the validator and its focused fixture tests on the task branch.
```

## E2E

`NOT_APPLICABLE`: this task validates repository ADR metadata and does not alter a runtime or user-facing path.
