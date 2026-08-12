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
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - Issue #1002
  - PR #1003 live head/base/CI/review state
  - open PR changed-file ownership for all owned paths
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
- [x] Agent Governance executes deterministic prompt-contract tests and the live evaluator and triggers on prompt-eval harness changes.
- [x] Deep System Validation installs acceptance dependencies with committed lockfile via `npm ci`.
- [ ] Exact-final-head CI/E2E/self-review and fresh independent Codex review are green before merge.
- [ ] Issue #1002 closes and this task archives after verified squash merge.

## Ownership

```yaml
project_lane: oteryn-platform-core
task_kind: implementation
risk_gate: HEIGHTENED
feature_scope: internal_only
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/**
  - docs/agents/evals/prompt-contract-v1.json
  - .github/workflows/agent-governance.yml
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_acceptance_lockfile_contract.py
  - tests/ci/test_game_catalog_cross_repo_trigger.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
  - docs/agents/tasks/archive/OTERYN-20260811-engineering-excellence-hardening.md
modules:
  - build-test
  - ci
  - agent-governance
  - synology-staging-deployment
dependencies:
  - Issue #1002
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T14:47:00+02:00
head: ea93f299c4a7b1f17bb5c5f105899c45f21e5bba
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
  - .github/workflows/agent-governance.yml
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - tests/ci/test_acceptance_lockfile_contract.py
  - tests/ci/test_game_catalog_cross_repo_trigger.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
  - docs/agents/tasks/archive/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - PR #992 and Issue #991 are terminal; the policy-consistency task is archived and its Agent Governance/tools ownership is released.
  - Protected main 25ca5180849996462dfcfd2a0914eb6d765a47d3 was incorporated by history-preserving merge commit 794c1f2a86a1d81de13aac49ccfc0d9c3acec788 with force=false; compare reported behind_by=0.
  - No open PR for Issue #1008 was visible before the Agent Governance edit; PR #1013 for Issue #1007 was previously verified to avoid the deploy-synology-staging workflow owned by this task.
  - Material head f1c04011018132c66b6005db9a10b280007e8ce9 extended landed Agent Governance with blocking prompt-eval unit/live steps and harness path triggers while preserving every #992 policy/checkpoint/liveness/Control Room gate.
  - Final-checkpoint CI on 507bbec52839e6199e89061a2d85475ef37fb119 exposed a real restack regression: CI referenced `tools/validation/test_verify_integration_test_registration.py` and `tools/validation/test_workflow_inventory.py`, but both files were absent from the branch.
  - Both missing test files existed in previously reviewed material commit 4229facd42de8b7ec2a2b5954d7358f734fe8a37 and were restored without redesign as commits a448e5b6fb75610d6cb407da3f07788427a17783 and ea93f299c4a7b1f17bb5c5f105899c45f21e5bba.
  - CI run 31598034929 on ea93f299c4a7b1f17bb5c5f105899c45f21e5bba passed `classify-changes`, including routing contracts, Integration registry tests/live validator, deterministic prompt tests/live evaluator, workflow inventory tests/live validator, checkpoint validation and path classification; runtime-tests proceeded afterward.
  - Agent Governance run 31598034956, job 94118258644, passed on ea93f299c4a7b1f17bb5c5f105899c45f21e5bba, including policy consistency, both prompt-contract steps, checkpoint validation, live ownership/liveness and Control Room.
  - Deep System Validation retains acceptance dependency installation through committed lockfile via `npm ci`.
derived:
  - The only discovered final-checkpoint implementation failure was the two missing regression files, and focused validation now proves that repair; full exact-head gates and independent review remain.
unknown:
  - Terminal results of all repository-required workflows on the checkpoint-only final head created by this update.
  - Fresh independent Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: none-known-after-restored-validation-regressions
  evidence: CI classify-changes 31598034929 and Agent Governance 31598034956 both passed the repaired validator surfaces on material head ea93f299c4a7b1f17bb5c5f105899c45f21e5bba.
rejected_hypotheses:
  - Treat the missing validator-test error as runner infrastructure; exact CI log proved the file path was absent from the checked-out repository.
  - Reimplement the missing tests from scratch; the exact intended tests already existed on previous reviewed commit 4229facd42de8b7ec2a2b5954d7358f734fe8a37 and were recoverable.
  - Add tests/Integration directly to phpunit.xml; the existing cross-repository test requires an external generated snapshot environment.
  - Treat deterministic prompt-contract checks as model-behavior proof; the harness explicitly limits itself to repository text contracts and requires repeated runtime/model trials when nondeterminism matters.
  - Rebuild Agent Governance from the pre-#992 branch version; the final edit extends the landed fail-closed governance workflow instead.
  - Force-push to synchronize main; current main was merged history-preservingly and the branch ref moved with force=false.
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/agent-governance.yml
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
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
  - command: history-preserving current-main synchronization
    result: PASS
    evidence: Merge commit 794c1f2a86a1d81de13aac49ccfc0d9c3acec788 has parents task head 9e3c01fa6cea7efb26409283e64affc1c138afa3 and protected main 25ca5180849996462dfcfd2a0914eb6d765a47d3; branch ref update used force=false and compare reported behind_by=0.
  - command: CI classify-changes validation on material head ea93f299c4a7b1f17bb5c5f105899c45f21e5bba
    result: PASS
    evidence: Run 31598034929 job 94118258213 passed routing, Integration registry, deterministic prompt contracts, workflow inventory, task checkpoint and change classification.
  - command: Agent Governance on material head ea93f299c4a7b1f17bb5c5f105899c45f21e5bba
    result: PASS
    evidence: Run 31598034956 job 94118258644 passed policy consistency, prompt evaluator tests/live suite, checkpoint, liveness, ownership and Control Room.
  - command: final checkpoint-head required GitHub Actions
    result: NOT_RUN
    evidence: This task-record-only update creates the final validation generation after restoring the intended validator regressions.
  - command: final checkpoint-head fresh Codex review
    result: NOT_RUN
    evidence: This task-record-only update creates the final review generation after restoring the intended validator regressions.
blockers:
  - none
next_action: Validate the unchanged checkpoint-only final head with every emitted/applicable workflow and a fresh full-diff Codex review; resolve every material finding, then mark PR #1003 ready, squash-merge with expected-head protection, verify main, archive this task and close Issue #1002.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 3
  session_id: coordinator-20260812-engineering-hardening-final
  session_started_at: 2026-08-12T14:25:00+02:00
  checkpointed_at: 2026-08-12T14:47:00+02:00
  last_progress_at: 2026-08-12T14:47:00+02:00
  phase: final-exact-head-validation
  exact_head: ea93f299c4a7b1f17bb5c5f105899c45f21e5bba
  pull_request: 1003
  active_operation: create final checkpoint generation after restored validator regressions passed focused validation
  external_run_ids:
    - 31598034929
    - 31598034956
  operation_started_at: 2026-08-12T14:47:00+02:00
  wait_deadline_at: 2026-08-12T15:32:00+02:00
  check_generation: final-checkpoint-after-restored-tests
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect exact-head CI and fresh review, repair only material findings, then squash-merge and lifecycle-close the task
```

## Notes

Issue #1002. `feature_scope: internal_only`; no production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task.