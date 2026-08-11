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

Close Issue #1002 by making verification, E2E dependency installation, prompt/governance regression checks, workflow governance, Game Catalog staging validation, and Synology staging release identity deterministic without changing product behavior or performing live deployment.

## Acceptance criteria

- [x] Acceptance npm lockfile and npm Dependabot coverage exist.
- [x] `composer verify` is the canonical full developer verification command.
- [x] Integration tests are fail-closed registered to explicit proving workflows/commands.
- [x] Game Catalog cross-repository staging is selected semantically, not by historical PR number.
- [x] Deterministic prompt-contract regression fixtures and executable validator exist.
- [x] Workflow inventory/classification is machine enforced.
- [x] Synology staging resolves exact source SHA images to immutable digests before deployment.
- [ ] Agent Governance executes deterministic prompt/governance regression after PR #992 releases ownership.
- [ ] Deep System Validation installs acceptance dependencies with `npm ci` after PR #1001 releases ownership.
- [ ] Exact-head CI/E2E/self-review are green before merge.

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
  - tools/validation/**
  - docs/agents/evals/prompt-contract-v1.json
  - .github/workflows/ci.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
deferred_overlap_paths:
  - path: .github/workflows/deep-system-validation.yml
    owner: PR #1001
  - path: .github/workflows/agent-governance.yml
    owner: PR #992
modules:
  - build-test
  - ci
  - agent-governance
  - synology-staging-deployment
dependencies:
  - Issue #1002
  - terminal PR #1001 before deep-system edit
  - terminal PR #992 before agent-governance edit
blockers:
  - PR #1001 retains .github/workflows/deep-system-validation.yml ownership
  - PR #992 retains .github/workflows/agent-governance.yml ownership
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T15:05:00Z
head: 380fc60d748033c060de4db59257992b2c41ce7e
branch: fix/engineering-excellence-hardening
pr: 1003
status: validating
context_routes:
  - testing
  - agent-governance
  - ci-repair
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/**
  - docs/agents/evals/prompt-contract-v1.json
  - .github/workflows/ci.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - PR #1003 exists on branch fix/engineering-excellence-hardening and exact head incorporates protected main 0375285cfa964a0f0cbdcf56d65d7592ac41298a.
  - CI run 31504907961 passed lockfile, Game Catalog trigger, Synology release identity, Integration registry, prompt-contract and workflow-inventory executable validations before checkpoint validation.
  - Portal Exhaustive Trigger Coupling run 31504908155 passed after current-main synchronization.
  - PR #1004 merged and removed the unrelated stale active-task liveness blocker.
derived:
  - Remaining overlapping workflow edits must wait until PR #1001 and PR #992 become terminal under repository ownership policy.
unknown:
  - Final current-main form of deep-system and Agent Governance workflows after their owning PRs become terminal.
conflicts: []
first_failure:
  marker: checkpoint_validation_result_vocabulary
  evidence: CI run 31504907961 rejected noncanonical validation result values in this task record.
rejected_hypotheses:
  - Add tests/Integration directly to phpunit.xml; the existing cross-repository test requires external generated snapshot environment.
  - Treat prior Agent Governance failure as infrastructure; exact logs proved the task record previously omitted open PR #1003 identity.
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
  - command: repository and PR ownership preflight
    result: PASS
    evidence: Current main, active tasks and open PR changed-file inventories were inspected before path claims.
  - command: exact-head repository contract validators in CI run 31504907961
    result: PASS
    evidence: All newly added deterministic repository validators passed.
  - command: exact-head checkpoint validation in CI run 31504907961
    result: FAIL
    evidence: Only noncanonical task validation result vocabulary failed; this revision uses canonical values exclusively.
blockers:
  - PR #1001 retains deep-system-validation.yml ownership.
  - PR #992 retains agent-governance.yml ownership.
next_action: Validate this checkpoint revision, then complete PR #992 and PR #1001 under their existing ownership before applying the released deferred edits to PR #1003.
```

## Notes

Issue #1002. No production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized by this task.