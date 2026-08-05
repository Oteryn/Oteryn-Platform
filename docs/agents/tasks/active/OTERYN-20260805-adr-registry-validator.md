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
updated: 2026-08-05T16:55:00Z
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
  - phpunit.xml
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tools/validation/phpunit/AdrRegistryValidationTest.php
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
- [x] move the PHPUnit bridge outside `tests/**` to respect the global native-contract documentation boundary;
- [ ] pass repaired focused and exact-head validation;
- [ ] complete fresh post-implementation audit with zero material findings;
- [ ] merge, archive, close Issue #577 and release ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-05T16:55:00Z
invocation_started_at: 2026-08-05T15:50:00Z
last_progress_at: 2026-08-05T16:55:00Z
head: b541e7a7c54f73a186cdc8cc2da3491c4acc729f
branch: task/OTERYN-20260805-adr-registry-validator
pr: 581
status: validating
phase: integration_boundary_repair
session_id: chat-20260805-architecture-continuation
session_role: implementer
execution_mode: github-only
execution_reason: bounded repository tooling and documentation changes can be implemented through GitHub objects and validated by GitHub Actions
lease_expires_at: 2026-08-05T17:40:00Z
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one validator, one compatibility contract and one existing CI consumer form a cohesive bounded slice
context_routes:
  - architecture
  - testing
owned_paths:
  - phpunit.xml
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tools/validation/phpunit/AdrRegistryValidationTest.php
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
  - CI run 31025277136 failed only because 17 accepted ADRs use plain or section lifecycle declarations in addition to bullet metadata.
  - The repaired focused suite passes 10 tests and covers bullet, plain and section forms plus ambiguous declarations.
  - Deep System Validation on b541e7a7c54f73a186cdc8cc2da3491c4acc729f passed the complete PHP regression and concurrency suites, proving the repaired validator against the live registry.
  - Native protocol contract audits run 31026544250 failed only because Audit 1 treats any `tests/**` change as a forbidden runtime path; its other four audits passed.
  - Registering the bridge from `tools/validation/phpunit/**` in `phpunit.xml` preserves CI enforcement without changing workflow files or entering the forbidden `tests/**` root.
derived:
  - A closed exact-path legacy allowlist is the least disruptive compatibility boundary.
  - Parser compatibility is safer and narrower than rewriting 17 historical ADRs.
  - The PHPUnit bridge belongs with the repository validation tool rather than the application test tree.
unknown:
  - Exact-head result after moving the bridge outside the global native-contract forbidden roots.
conflicts: []
first_failure:
  marker: CI PHPUnit ADR registry validation
  evidence: run 31025277136, job 92372884204, artifact 8938486455; test_repository_adr_registry_passes reported 17 established ADRs with zero recognized lifecycle declarations
secondary_failure:
  marker: Native protocol architecture boundary audit
  evidence: run 31026544250, job 92376378411 rejected tests/Unit/Architecture/AdrRegistryValidationTest.php solely because Audit 1 forbids every tests/** path
rejected_hypotheses:
  - Neither failure was caused by a new ADR collision, README inventory drift or invalid filename.
  - Neither failure was a runtime, database, edge, game-auth or native-protocol behavior regression.
  - Python and Symfony Process are available because the focused bridge and full PHP regression executed.
  - The native protocol audit did not identify a protocol invariant violation; all four substantive companion audits passed.
  - Renumbering or normalizing historical ADRs is not required and would risk breaking references.
  - An open-ended duplicate-prefix allowlist would not fail closed.
  - A workflow edit is not required for CI enforcement.
changed_paths:
  - phpunit.xml
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - tools/validation/phpunit/AdrRegistryValidationTest.php
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/reports/OTERYN-20260805-adr-registry-validator.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
validation:
  - command: python3 tools/validation/test_adr_registry.py after lifecycle parser repair
    result: PASS
    evidence: 10 focused positive, negative and boundary tests passed
  - command: Deep System Validation complete PHP regression on b541e7a7c54f73a186cdc8cc2da3491c4acc729f
    result: PASS
    evidence: run 31026544499 completed PHP regression and concurrency steps successfully
  - command: Native protocol contract audits on b541e7a7c54f73a186cdc8cc2da3491c4acc729f
    result: FAIL
    evidence: run 31026544250 rejected only the tests/** bridge path; four other audit jobs passed
  - command: exact validation after moving bridge to tools/validation/phpunit
    result: NOT_RUN
    evidence: new exact-head GitHub Actions required
ci_checks_for_current_head: 9
ci_check_generation: integration_boundary_repair
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 9
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
context_reconstruction_attempts: 0
stall_warnings: 0
blockers: []
next_action: Commit the PHPUnit bridge relocation and run all exact-head GitHub Actions on the new tree.
```

## E2E

`NOT_APPLICABLE`: this task validates repository ADR metadata and does not alter a runtime or user-facing path.
