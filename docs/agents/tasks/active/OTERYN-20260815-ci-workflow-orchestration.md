---
task_id: OTERYN-20260815-ci-workflow-orchestration
policy_version: 2
project_lane: oteryn-platform-core
task_kind: implementation
execution_mode: github-only
implementation_authorized: true
parent_issue: 1085
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
search_first:
  - open PRs and active tasks touching CI/workflow paths
optional_reads: []
---

# OTERYN-20260815-ci-workflow-orchestration

## Goal

Make Oteryn Platform CI risk/path-aware and lifecycle-managed so complete validation is preserved while unrelated heavy workflows, historical one-off orchestration and unmeasured test execution no longer accumulate or delay normal delivery.

## Acceptance criteria

- [ ] Documentation/governance-only main pushes do not execute Portal Acceptance runtime/account-lifecycle jobs.
- [ ] Editing an ordinary workflow no longer fans out to every heavy runtime gate; central routing changes remain fail-closed.
- [ ] Repository tests fail when domain workflows regress to unbounded push/PR triggering or lose superseded-run cancellation where applicable.
- [ ] Proven obsolete diagnostic/task-wrapper workflows are removed without deleting unique test coverage.
- [ ] Completed deep/exhaustive audit programmes retained for diagnostic value no longer run automatically on ordinary product PRs/main pushes.
- [ ] PHP application coverage is measured outside the blocking PR fast path with a ratchet-ready policy and durable report artifact.
- [ ] Workflow lifecycle rules make temporary/task-specific workflow retention explicit and testable.
- [ ] Exact-head CI, self-review, diff scope, review hygiene and source-branch closeout satisfy repository gates.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/portal-exhaustive-acceptance.yml
  - .github/workflows/portal-exhaustive-trigger-coupling.yml
  - .github/workflows/account-security-format-diagnostics.yml
  - .github/workflows/account-security-static-diagnostics.yml
  - scripts/ci/classify_changes.py
  - tests/ci/test_classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_workflow_trigger_economy.py
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20260815-ci-workflow-orchestration.md
modules:
  - ci
  - testing
  - agent-governance
dependencies:
  - Issue #1085
blockers:
  - branch-protection required-check configuration is not readable through the connected GitHub integration; preserve existing check names for retained automatic gates and avoid deleting unique required product checks
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-15T07:42:00Z
head: 88b3dce7b822ed27d9f61c412493c57ba6608a38
branch: ci/issue-1085-workflow-orchestration
pr: none
status: implementing
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/portal-exhaustive-acceptance.yml
  - .github/workflows/portal-exhaustive-trigger-coupling.yml
  - .github/workflows/account-security-format-diagnostics.yml
  - .github/workflows/account-security-static-diagnostics.yml
  - scripts/ci/classify_changes.py
  - tests/ci/test_classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_workflow_trigger_economy.py
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/tasks/active/OTERYN-20260815-ci-workflow-orchestration.md
proven:
  - protected main at task start is 88b3dce7b822ed27d9f61c412493c57ba6608a38
  - workflow inventory currently reports 58 workflows, including 45 domain-validation workflows
  - portal-acceptance-contract.yml has an unfiltered push-to-main trigger and executed complete account lifecycle for a docs-only main commit
  - scripts/ci/classify_changes.py currently maps generic workflow changes to ALL_GATES
  - account-security format/static diagnostic workflows are self-trigger-only diagnostics whose proving checks are already blocking in central CI
  - archived portal-exhaustive and deep-system tasks have released ownership; retained workflows are historical validation capabilities rather than active task ownership
  - PR #1074 owns historical-branch-audit.yml and PR #1083 owns parallel-coordinator-prompt-eval.yml; this task does not edit either file
  - current GitHub integration returns 403 for classic branch-protection configuration and therefore exact required-check policy remains unreadable
  - repository rulesets endpoint returns no repository rulesets
  - no open PR or Issue already owns this CI consolidation scope
  - Issue #1085 owns this remediation
derived:
  - workflow cleanup must preserve stable retained check names until required-check configuration can be independently verified
  - manualizing completed exhaustive programmes is safer than deleting their unique diagnostic capability
unknown:
  - exact classic branch-protection required-status-check list
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - delete broad product acceptance workflows solely to reach an arbitrary workflow count
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ci-workflow-orchestration.md
validation:
  - command: repository preflight and live overlap inspection
    result: PASS
    evidence: main/Issue/open-PR ownership and workflow evidence inspected through GitHub connector
blockers:
  - none
next_action: implement the bounded CI routing, lifecycle and coverage changes on this branch
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chat-20260815-ci-1085-01
  session_started_at: 2026-08-15T07:41:00Z
  checkpointed_at: 2026-08-15T07:42:00Z
  last_progress_at: 2026-08-15T07:42:00Z
  phase: implement
  exact_head: 88b3dce7b822ed27d9f61c412493c57ba6608a38
  pull_request: none
  active_operation: repository implementation
  external_run_ids: []
  operation_started_at: 2026-08-15T07:42:00Z
  wait_deadline_at: null
  check_generation: draft
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: branch remains owned by Issue #1085 with no overlapping path owner
  next_action: implement the bounded CI routing, lifecycle and coverage changes on this branch
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

This task deliberately does not touch `.github/workflows/historical-branch-audit.yml` or `.github/workflows/parallel-coordinator-prompt-eval.yml`, which are owned by active PRs #1074 and #1083 respectively. Workflow-count reduction is evidence-driven; unique product/security/operations checks are retained unless safely consolidated without changing their externally visible gate contract.
