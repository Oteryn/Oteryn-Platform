---
task_id: OTERYN-20260807-native-protocol-audit-routing
issue: 829
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: implementing
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: repair/issue-829
base_branch: main
pr: pending
production_activation_authorized: false
cross_repository_mutation_authorized: false
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
modules:
  - ci-native-protocol-audit-routing
coordination_key: ci:native-protocol-audit-routing
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 native protocol audit routing

## Goal

Repair Issue #829 so the native-protocol architecture boundary audit does not reject unrelated runtime changes merely because an unrelated file under `docs/contracts/**` changed, while true native-protocol producer corrections remain fail-closed.

## Acceptance criteria

- [ ] Unrelated contract + unrelated runtime changes do not invoke native-protocol producer ownership enforcement.
- [ ] A native-protocol producer correction with runtime changes still requires the canonical active producer task record.
- [ ] Native-protocol producer runtime changes outside the existing allowlist still fail closed.
- [ ] Focused deterministic regression fixtures cover unrelated-change PASS, missing-task FAIL, escaped-runtime FAIL, and valid producer PASS.
- [ ] Existing architecture, security/downgrade, parser/schema, Canary regression, rollout and rollback audits are not weakened.
- [ ] Required exact-head CI and Agent Governance pass; workflow-specific validation passes.
- [ ] E2E is recorded as NOT_APPLICABLE with a concrete CI/governance-only reason.

## Context checkpoint

```yaml
policy_version: 2
phase: implement
updated_at: 2026-08-07T18:33:00+02:00
status: implementing
branch: repair/issue-829
head: pending-initial-task-commit
pr: pending
execution_mode: github_only
execution_reason: narrow workflow/script/task change is fully executable through GitHub and GitHub Actions
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one CI routing defect with one workflow and deterministic classifier regressions
validation_level: focused
heavy_validation_runs: 0
ci_checks_for_current_head: 0
ci_check_generation: other
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
proven:
  - Issue #829 reproduces on PR #825 because generic docs/contracts triggering causes Audit 1 to classify every runtime path in the PR as a native-protocol producer correction.
  - Main branch protection requires classify-changes and test.
  - No branch, open PR or active task currently owns Issue #829 or this CI-routing scope.
derived:
  - Producer ownership enforcement must be conditional on a specific native-protocol producer signal, not generic docs/contracts membership.
unknown: []
conflicts: []
next_action: implement a deterministic native-protocol change-boundary classifier with focused fixtures and wire Audit 1 to it without altering Audits 2-5
```

## Safety

Repository-only CI/governance repair. No production deployment, protected-environment operation, secret access, runtime product behavior, Canary mutation or external-repository mutation is authorized.
