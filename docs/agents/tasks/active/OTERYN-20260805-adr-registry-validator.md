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
updated: 2026-08-05T17:06:00Z
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
- [x] diagnose and repair historical lifecycle syntax compatibility;
- [x] move the PHPUnit bridge outside `tests/**` to respect the native-contract boundary;
- [x] pass CI, formatting, static analysis, PHPUnit, native contract, native contract audits, edge and database-outage validation on the repaired implementation head;
- [ ] pass the corrected exact-head governance checkpoint and remaining exact-head workflows;
- [ ] complete fresh post-implementation audit with zero material findings;
- [ ] merge, archive, close Issue #577 and release ownership.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-05T17:06:00Z
invocation_started_at: 2026-08-05T15:50:00Z
last_progress_at: 2026-08-05T17:06:00Z
head: 0ac4500edc7fd6bcd8c6613ac14f47c248e18a7a
branch: task/OTERYN-20260805-adr-registry-validator
pr: 581
status: validating
phase: final_checkpoint_repair
session_id: chat-20260805-architecture-continuation
session_role: implementer
execution_mode: github-only
execution_reason: bounded repository tooling and documentation changes can be implemented through GitHub objects and validated by GitHub Actions
lease_expires_at: 2026-08-05T17:51:00Z
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
  - The validator preserves all accepted ADR paths through a closed exact-path legacy allowlist.
  - Ten focused tests cover accepted lifecycle forms, ambiguity, new duplicates, allowlist drift, inventory drift, filenames and supersession targets.
  - CI run 31027624692 passed formatting, static analysis and the complete PHPUnit suite on head 0ac4500edc7fd6bcd8c6613ac14f47c248e18a7a.
  - Native protocol contract run 31027624602 and contract audits run 31027624703 passed after relocating the PHPUnit bridge to tools/validation/phpunit.
  - Edge Security Emulation run 31027628085 and Platform DB Outage Validation run 31027624838 passed on the same head.
  - Agent Governance run 31027624871 failed only because the checkpoint used an unsupported nested key named secondary_failure.
derived:
  - A closed exact-path legacy allowlist is the least disruptive compatibility boundary.
  - Parser compatibility is safer and narrower than rewriting historical ADRs.
  - The PHPUnit bridge belongs with the repository validation tool rather than the application test tree.
unknown:
  - Final exact-head conclusions after this checkpoint-only correction.
conflicts: []
first_failure:
  marker: CI PHPUnit ADR registry validation
  evidence: run 31025277136, job 92372884204, artifact 8938486455; the first parser did not recognize established plain and section lifecycle declarations
rejected_hypotheses:
  - The repaired implementation does not introduce a new ADR collision, README inventory drift or invalid filename.
  - The native protocol audit failure on the prior head was a path-boundary violation, not a protocol invariant failure.
  - The latest governance failure is not a validator, runtime, database, edge, game-auth or protocol regression.
  - Renumbering or normalizing historical ADRs is not required and would risk breaking references.
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
  - command: python3 tools/validation/test_adr_registry.py
    result: PASS
    evidence: 10 focused positive, negative and boundary tests
  - command: CI on 0ac4500edc7fd6bcd8c6613ac14f47c248e18a7a
    result: PASS
    evidence: run 31027624692 passed formatting, static analysis and complete PHPUnit
  - command: Native protocol contract audits on 0ac4500edc7fd6bcd8c6613ac14f47c248e18a7a
    result: PASS
    evidence: run 31027624703
  - command: Agent Governance on 0ac4500edc7fd6bcd8c6613ac14f47c248e18a7a
    result: FAIL
    evidence: run 31027624871 rejected only unsupported checkpoint key secondary_failure; key removed in the next commit
ci_checks_for_current_head: 8
ci_check_generation: final_checkpoint_repair
terminal_ci_wait_started_at: 2026-08-05T17:06:00Z
terminal_ci_checks_for_current_generation: 8
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 0
stall_warnings: 0
blockers: []
next_action: Commit the checkpoint schema correction and verify all exact-head workflows before final PR audit and merge.
```

## E2E

`NOT_APPLICABLE`: this task validates repository ADR metadata and does not alter a runtime or user-facing path.
