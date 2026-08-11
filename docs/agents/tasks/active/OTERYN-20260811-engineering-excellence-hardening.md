---
task_id: OTERYN-20260811-engineering-excellence-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - Issue #1002
  - PR #1001 Actions-major cleanup ownership
  - PR #992 agent-governance ownership
  - PR #338 Game Catalog consumer ownership
optional_reads: []
---

# OTERYN-20260811-engineering-excellence-hardening

## Goal

Close Issue #1002 by making repository verification, E2E dependency installation, prompt/governance regression checks, selected CI routing, Game Catalog staging validation, and Synology staging release identity more deterministic and maintainable without changing product behavior or performing protected live operations.

## Acceptance criteria

- [x] `scripts/acceptance/package-lock.json` is committed, consistent with `package.json`, and deterministic npm installation is available through the canonical verification command; the deep-system workflow edit remains deferred while PR #1001 owns that path.
- [x] Dependabot covers npm dependencies under `/scripts/acceptance`.
- [x] One canonical developer verification command exists and covers Composer validation/audit, formatting, static analysis, base tests, strict acceptance-contract checks, and integration-test registration.
- [x] Every `tests/Integration/**/*Test.php` is machine-registered to an explicit proving workflow/command; unregistered integration tests fail validation.
- [x] Game Catalog cross-repository staging is selected semantically and no longer keyed to historical PR #272.
- [x] Prompt/agent-governance changes have executable deterministic regression fixtures, with stochastic model-behavior limits stated truthfully.
- [ ] Agent Governance executes the applicable deterministic prompt/governance regression checks after overlapping PR #992 releases ownership.
- [x] Workflow complexity/drift is reduced with machine-enforced classification/consistency rather than additional narrative-only rules, reusing current-main governance after #992 rather than duplicating it.
- [x] Synology staging deployment resolves Platform/Gateway images from an explicit `sha-<40 hex>` release identity to immutable image digests before deployment.
- [ ] Exact-head self-review is PASS, applicable validation is green, E2E is PASS or specifically NOT_APPLICABLE, related PRs are intentional, and required exact-head CI passes before merge.

## Ownership

```yaml
project_lane: oteryn-platform-core
task_kind: implementation
risk_gate: HEIGHTENED
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/test_verify_integration_test_registration.py
  - tools/validation/prompt_eval.py
  - tools/validation/test_prompt_eval.py
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - docs/agents/evals/prompt-contract-v1.json
  - .github/workflows/ci.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
deferred_overlap_paths:
  - path: .github/workflows/deep-system-validation.yml
    owner: PR #1001
    rule: do not edit until PR #1001 is terminal and current main is refreshed
  - path: .github/workflows/agent-governance.yml
    owner: PR #992
    rule: do not edit until PR #992 is terminal and current main is refreshed
  - path: tools/agents/**
    owner: PR #992 for policy-consistency surfaces
    rule: extend current-main implementation after #992 instead of duplicating it
modules:
  - build-test
  - ci
  - agent-governance
  - synology-staging-deployment
dependencies:
  - Issue #1002
  - terminal state of PR #1001 before deep-system workflow edit
  - terminal state of PR #992 before agent-governance/tool extension
blockers:
  - PR #1001 retains ownership of .github/workflows/deep-system-validation.yml
  - PR #992 retains ownership of .github/workflows/agent-governance.yml and tools/agents policy-consistency surfaces
cross_repository_tasks:
  - none
```

PR #338 does not change `.github/workflows/game-catalog-contract.yml`; its changed-file inventory was checked before claiming that path. No external repository write is part of this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-11T15:00:00Z
head: f5e6ff66e6d3d87395b8e05404226895715381b5
branch: fix/engineering-excellence-hardening
pr: 1003
status: validating
phase: non_overlapping_validation_and_dependency_wait
session_id: agent-20260811T134504Z
session_role: implementer
execution_mode: github
execution_reason: repository writes and exact-head validation are available through the authenticated GitHub connector and Actions
context_routes:
  - testing
  - agent-governance
  - ci-repair
context_pressure: high
context_growth: stable
context_score: 10
decomposition_decision: phased
decomposition_reason: one cohesive engineering-verification contract with temporary path-release dependencies on two current PRs
invocation_started_at: 2026-08-11T13:40:48Z
last_progress_at: 2026-08-11T15:00:00Z
ci_checks_for_current_head: 16
ci_check_generation: exact_head_in_progress
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 0
stall_warnings: 0
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/test_verify_integration_test_registration.py
  - tools/validation/prompt_eval.py
  - tools/validation/test_prompt_eval.py
  - tools/validation/workflow_inventory.py
  - tools/validation/test_workflow_inventory.py
  - docs/agents/evals/prompt-contract-v1.json
  - .github/workflows/ci.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - current PR is #1003 on branch fix/engineering-excellence-hardening
  - current PR merge ref is tested against main 0375285cfa964a0f0cbdcf56d65d7592ac41298a
  - scripts/acceptance/package-lock.json is committed and npm Dependabot coverage is present
  - composer verify is the canonical full developer verification command
  - integration-test registration, deterministic prompt-contract evaluation and workflow inventory validators are executable and wired to CI classification
  - game-catalog cross-repository staging no longer depends on historical PR #272
  - Synology staging deploy input uses exact release SHA and resolves immutable image digests before environment generation
  - workflow inventory validator was repaired to accept valid workflow-scope permissions mappings including permissions: {}, read-all and write-all
  - PR #1004 was exact-head green and merged to remove an unrelated stale active-task liveness blocker
derived:
  - remaining overlapping edits must wait for PR #1001 and PR #992 to become terminal under the repository ownership contract
  - current Agent Governance failure is caused by this task record omitting PR #1003 identity and should be removed by this checkpoint update
unknown:
  - final current-main form of overlapping workflow/governance files after PRs #1001 and #992 become terminal
  - terminal outcome of the current exact-head validation generation
conflicts: []
first_failure:
  marker: workflow_inventory_permissions_shape
  evidence: CI rejected .github/workflows/repair-synology-autostart.yml because the validator did not accept valid top-level `permissions: {}`
rejected_hypotheses:
  - adding tests/Integration directly to phpunit.xml; the existing cross-repository test requires external generated snapshot environment and would make the default suite invalid
  - treating the Agent Governance failure as an external infrastructure fault; the exact log proved this task record omitted the already-open PR #1003 identity
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/ci.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/game-catalog-contract.yml
  - composer.json
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
  - scripts/acceptance/package-lock.json
  - tests/Integration/REGISTRY.json
  - tests/ci/test_acceptance_lockfile_contract.py
  - tests/ci/test_game_catalog_cross_repo_trigger.py
  - tests/ci/test_synology_deploy_release_identity.py
  - tools/validation/prompt_eval.py
  - tools/validation/test_prompt_eval.py
  - tools/validation/test_verify_integration_test_registration.py
  - tools/validation/test_workflow_inventory.py
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/workflow_inventory.py
validation:
  - command: repository/PR ownership preflight
    result: PASS
    evidence: live main, active tasks and open PR changed-file inventories inspected before path claim
  - command: exact-head Agent Governance run 31504259600
    result: FAIL_REPAIRED
    evidence: task liveness reported only branch_pr_identity_omitted for this task; checkpoint now records pr: 1003
  - command: exact-head workflow inventory regression repair
    result: IN_PROGRESS
    evidence: validator and regression tests updated on branch; current CI generation pending
blockers:
  - PR #1001 retains deep-system-validation.yml ownership, preventing the final owned-path `npm ci` workflow edit
  - PR #992 retains agent-governance.yml/tools/agents ownership, preventing the final Agent Governance prompt-eval integration
next_action: Validate this checkpoint repair, then inspect terminal status of PR #1001 and PR #992; only after each releases ownership, refresh from current main and complete its deferred edit.
```

## Notes

Issue: #1002. This task changes repository and staging deployment definitions only. It does not authorize a staging deployment, production deployment, environment approval, credential mutation, live database mutation, payment action, or cross-repository write.