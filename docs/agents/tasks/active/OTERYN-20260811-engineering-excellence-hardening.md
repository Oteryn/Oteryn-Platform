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
- [x] Game Catalog cross-repository staging is selected semantically/manual, not by historical PR number.
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
updated_at: 2026-08-12T15:18:00+02:00
head: live-pr-1003-authoritative
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
  - PR #992 and Issue #991 are terminal and their Agent Governance/tools ownership is released.
  - Protected main 25ca5180849996462dfcfd2a0914eb6d765a47d3 was incorporated by history-preserving merge commit 794c1f2a86a1d81de13aac49ccfc0d9c3acec788 with force=false and zero commits behind at synchronization.
  - Exact PR head d9bde9362e9f2557e4c44cfeeb78f1f887b5c4d4 completed all 16 emitted pull-request workflows successfully before final Codex review.
  - Post-ready Codex review PRR_kwDOTcsYjs8AAAABJQ6E5g reviewed d9bde9362e and reported one P1 and two P2 findings; merge was not attempted.
  - P1 showed Integration registration markers could come from unrelated workflow paths; validator now binds a declared top-level event to a declared proving job, its condition, exact PHPUnit invocation and required environment in that same job.
  - Game Catalog registry schema v2 now truthfully declares workflow_dispatch, job cross-repository-staging, semantic opt-in run_cross_repository_staging and the dispatch-only job condition, matching Issue #1002 manual-contract scope.
  - P2 showed workflow event regex could mistake a job name for an event; workflow inventory now derives events only from direct children of the top-level on mapping and has regressions for job/nested-key masquerading.
  - The checkpoint uses live-pr-1003-authoritative rather than pretending a commit can embed its own future SHA; concrete reviewed/material SHAs remain in evidence and live PR state is authoritative by AGENTS.md.
derived:
  - The three Codex findings are addressed in the current branch generation, but the new head requires a complete fresh exact-head workflow generation and fresh Codex review before merge.
unknown:
  - Terminal result of every emitted workflow on the current live PR #1003 head after Codex repairs.
  - Fresh independent Codex review result for the current live PR #1003 head after Codex repairs.
conflicts: []
first_failure:
  marker: none-known-after-codex-repairs
  evidence: All three findings from review PRR_kwDOTcsYjs8AAAABJQ6E5g have corresponding code/contract/checkpoint repairs; fresh exact-head validation is now required.
rejected_hypotheses:
  - Treat the Codex P1 as requiring automatic cross-repository execution on every PR; Issue #1002 explicitly requires an explicit semantic/manual Game Catalog contract, so the correct repair is truthful event/job binding rather than changing scope.
  - Continue storing the parent SHA as an alleged exact checkpoint head; a task-record commit necessarily changes the branch SHA, so live PR state is the authoritative current-head source and immutable SHAs are retained as evidence.
  - Keep workflow-wide event regex matching; it admits unsupported events when unrelated two-space job keys resemble supported events.
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
  - command: exact-head pull-request workflows on d9bde9362e9f2557e4c44cfeeb78f1f887b5c4d4
    result: PASS
    evidence: All 16 emitted runs completed success, including CI, Agent Governance, Deep System Validation, browser/E2E, Phase 7, Game Catalog and deployment-build contracts.
  - command: post-ready Codex review on d9bde9362e9f2557e4c44cfeeb78f1f887b5c4d4
    result: FAIL
    evidence: Review PRR_kwDOTcsYjs8AAAABJQ6E5g reported P1 Integration trigger/job decoupling, P2 workflow-event scope, and P2 stale checkpoint head; all three are patched in the subsequent branch generation.
  - command: current-head required GitHub Actions after Codex repairs
    result: NOT_RUN
    evidence: This checkpoint update belongs to the new validation generation; live PR #1003 head is authoritative and must be checked after this commit.
  - command: current-head fresh Codex review after repairs
    result: NOT_RUN
    evidence: Fresh review must target the new live PR #1003 head after all repair commits.
blockers:
  - none
next_action: Read live PR #1003 head, validate every emitted exact-head workflow, request a fresh Codex full-diff review, resolve every material finding, then squash-merge with expected-head protection and perform lifecycle closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 4
  session_id: coordinator-20260812-engineering-hardening-final
  session_started_at: 2026-08-12T14:25:00+02:00
  checkpointed_at: 2026-08-12T15:18:00+02:00
  last_progress_at: 2026-08-12T15:18:00+02:00
  phase: codex-repair-validation
  exact_head: live-pr-1003-authoritative
  pull_request: 1003
  active_operation: validate Codex P1/P2 repairs on the current live PR head
  external_run_ids: []
  operation_started_at: 2026-08-12T15:18:00+02:00
  wait_deadline_at: 2026-08-12T16:03:00+02:00
  check_generation: post-codex-repairs
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: exact-head required checks and fresh review are terminal and branch head remains unchanged
  next_action: inspect live PR head, exact-head CI and fresh review, repair only material findings, then squash-merge and lifecycle-close the task
```

## Notes

Issue #1002. `feature_scope: internal_only`; no production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task.
