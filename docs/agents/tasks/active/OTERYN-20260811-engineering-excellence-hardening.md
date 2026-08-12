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
updated_at: 2026-08-12T14:25:00+02:00
head: f1c04011018132c66b6005db9a10b280007e8ce9
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
  - Protected main 25ca5180849996462dfcfd2a0914eb6d765a47d3 was incorporated into the task branch by history-preserving merge commit 794c1f2a86a1d81de13aac49ccfc0d9c3acec788 with force=false.
  - Compare after synchronization reported behind_by=0 and preserved the bounded engineering-hardening diff while taking newer governance and portal evidence from main.
  - No open PR for Issue #1008 was visible before the Agent Governance edit; PR #1013 for Issue #1007 was previously verified to avoid the deploy-synology-staging workflow owned by this task.
  - Material head f1c04011018132c66b6005db9a10b280007e8ce9 extends the landed Agent Governance workflow without removing policy-consistency, checkpoint, liveness or Control Room gates.
  - Agent Governance now executes `python tools/validation/test_prompt_eval.py` and `python tools/validation/prompt_eval.py`, and pull-request/push path filters include both prompt-eval harness files; the eval contract itself is already covered by `docs/agents/**`.
  - Deep System Validation retains the deterministic acceptance `npm ci` edit based on the committed lockfile.
  - The current PR diff contains exactly 18 task-related paths after synchronization; BUILD_TEST_MATRIX.md and TEST_STRATEGY.md no longer differ from current main.
derived:
  - All known material implementation work for Issue #1002 is present; only exact-final-head validation, independent review, merge and lifecycle closeout remain.
unknown:
  - Terminal results of repository-required workflows on the checkpoint-only final head created by this update.
  - Fresh independent Codex review result for the checkpoint-only final head created by this update.
conflicts: []
first_failure:
  marker: none-known-before-final-gates
  evidence: Current implementation is synchronized with main and all previously deferred Agent Governance wiring is present; final exact-head checks have not yet completed.
rejected_hypotheses:
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
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/workflow_inventory.py
validation:
  - command: history-preserving current-main synchronization
    result: PASS
    evidence: Merge commit 794c1f2a86a1d81de13aac49ccfc0d9c3acec788 has parents task head 9e3c01fa6cea7efb26409283e64affc1c138afa3 and protected main 25ca5180849996462dfcfd2a0914eb6d765a47d3; branch ref update used force=false and compare reported behind_by=0.
  - command: Agent Governance prompt-eval wiring inspection
    result: PASS
    evidence: Material head f1c04011018132c66b6005db9a10b280007e8ce9 adds blocking prompt evaluator unit/live steps and harness path triggers while retaining all landed #992 governance gates.
  - command: final checkpoint-head required GitHub Actions
    result: NOT_RUN
    evidence: This task-record-only update creates the final validation generation.
  - command: final checkpoint-head fresh Codex review
    result: NOT_RUN
    evidence: This task-record-only update creates the final review generation.
blockers:
  - none
next_action: Validate the unchanged checkpoint-only final head with all repository-required workflows and a fresh full-diff Codex review; resolve every material finding, then mark PR #1003 ready, squash-merge with expected-head protection, verify main, archive this task and close Issue #1002.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: coordinator-20260812-engineering-hardening-final
  session_started_at: 2026-08-12T14:25:00+02:00
  checkpointed_at: 2026-08-12T14:25:00+02:00
  last_progress_at: 2026-08-12T14:25:00+02:00
  phase: final-exact-head-validation
  exact_head: f1c04011018132c66b6005db9a10b280007e8ce9
  pull_request: 1003
  active_operation: create final checkpoint generation after current-main synchronization and Agent Governance prompt-eval wiring
  external_run_ids: []
  operation_started_at: 2026-08-12T14:25:00+02:00
  wait_deadline_at: 2026-08-12T15:10:00+02:00
  check_generation: final-checkpoint
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect exact-head CI and fresh review, repair only material findings, then squash-merge and lifecycle-close the task
```

## Notes

Issue #1002. `feature_scope: internal_only`; user-facing/runtime E2E is not introduced by this change. No production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task.