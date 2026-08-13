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

Close Issue #1002 by making verification, E2E dependency installation, prompt/governance regression checks, workflow governance, Game Catalog staging validation, and Synology staging release identity deterministic without changing product runtime behavior or performing live deployment.

## Acceptance criteria

- [x] Acceptance npm lockfile and npm Dependabot coverage exist.
- [x] `composer verify` is the canonical full developer verification command.
- [x] Integration registry binds each Integration test to the declared event, proving job, executable PHPUnit run step, and executable environment provisioning.
- [x] Game Catalog cross-repository staging is selected semantically/manual, not by historical PR number.
- [x] Deterministic prompt-contract regression fixtures and executable validator exist.
- [x] Workflow inventory/classification is machine enforced and rejects unsupported/quoted event bypasses.
- [x] Synology staging resolves exact source SHA images to immutable digests and its production-target preflight accepts digest runtime references.
- [x] Agent Governance executes deterministic prompt-contract checks.
- [x] Deep System Validation uses committed-lockfile `npm ci`.
- [ ] Exact-final-head CI/E2E/self-review is green after the 2026-08-13 repair and main synchronization.
- [ ] All material review threads are resolved.
- [ ] Any owner-funded Codex review gate is either explicitly authorized for this exact use or remains an explicit blocker; it must not be invoked implicitly.
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
  - deploy/synology/scripts/production-target-preflight.sh
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
  - owner-funded Codex review authorization if that review remains a required merge gate
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T09:04:00+02:00
head: 692fd76e46891e67dba5b35e5703ec4a89b55f01
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
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_acceptance_lockfile_contract.py
  - tests/ci/test_game_catalog_cross_repo_trigger.py
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - PR #1003 was synchronized history-preservingly with protected main 9b9497ba9147f24e751f3632ba0fad8c8c4b220e at merge commit 85198c9b6308f1520f6b5dfb0208a126141f1452.
  - PR #1013 explicitly avoids .github/workflows/deploy-synology-staging.yml and its changed-file list does not overlap this repair's modified validator paths or production-target-preflight path.
  - Material head 692fd76e46891e67dba5b35e5703ec4a89b55f01 restricts Integration proving detection to executable run steps, rejects inline-comment step-name and echo spoofing, and requires executable environment provisioning before the proving step.
  - The focused synthetic validator suite contains 23 cases and passed locally before the material commit, including the four new invocation-spoof regressions and three environment-marker spoof regressions.
  - Existing exact-head d39d04c090dcfe2fb52c3c19a0e810c6ff4c3320 had 17 successful pull-request workflow runs before the 2026-08-13 synchronization and repair; those results are historical supporting evidence only.
derived:
  - The two audit findings for false-green Integration invocation and required_environment substring matching are addressed in the current material implementation and require hosted exact-head proof.
unknown:
  - Exact-head hosted workflow results for the checkpoint-successor PR head containing material head 692fd76e46891e67dba5b35e5703ec4a89b55f01.
  - Whether the repository owner authorizes a fresh owner-funded Codex review for the exact final head if that review remains required.
conflicts: []
first_failure:
  marker: integration-registration-false-green
  evidence: Audit reproduced comment step-name echo and environment-marker substring cases that the previous validator accepted without executable proof.
rejected_hypotheses:
  - Treating any job substring as executable PHPUnit proof is sufficient.
  - Treating an environment variable name in comments step names or plain echo output as environment provisioning is sufficient.
changed_paths:
  - AGENTS.md
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/archive/OTERYN-20260813-synology-diagnostics.md
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/test_verify_integration_test_registration.py
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
validation:
  - command: python3 tools/validation/test_verify_integration_test_registration.py in isolated pre-commit regression harness
    result: PASS
    evidence: 23 of 23 synthetic regression tests passed, including executable-run and environment-provisioning negative paths.
  - command: exact-head GitHub Actions generation after 2026-08-13 repair
    result: NOT_RUN
    evidence: The checkpoint successor must be observed after this durable checkpoint commit is created.
blockers:
  - Fresh owner-funded Codex review cannot be invoked without explicit authorization for this exact use if it remains required by the merge gate.
next_action: Query the live PR #1003 checkpoint-successor head and aggregate its exact-head GitHub Actions results, then investigate any failing gate before final self-review and review-thread cleanup.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 7
  session_id: chatgpt-20260813-pr1003-repair
  session_started_at: 2026-08-13T09:04:00+02:00
  checkpointed_at: 2026-08-13T09:04:00+02:00
  last_progress_at: 2026-08-13T09:04:00+02:00
  phase: exact-head-validation
  exact_head: 692fd76e46891e67dba5b35e5703ec4a89b55f01
  pull_request: 1003
  active_operation: observe checkpoint-successor exact-head hosted validation
  external_run_ids: []
  operation_started_at: 2026-08-13T09:04:00+02:00
  wait_deadline_at: 2026-08-13T09:49:00+02:00
  check_generation: current-base-integration-registry-repair
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: live PR #1003 successor head is stable and exact-head checks are observable
  next_action: aggregate exact-head checks for PR #1003 and investigate the first failing gate if any
```

## Self-review

```yaml
self_review:
  result: PENDING
  exact_head: 692fd76e46891e67dba5b35e5703ec4a89b55f01
  acceptance_checked: true
  full_diff_checked: false
  related_prs_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  findings:
    - final exact-head hosted validation and final full-diff review still pending
  evidence:
    - focused synthetic regression suite passed before material commit
```

## Notes

Issue #1002. `feature_scope: internal_only`. No production/staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task. Owner-funded Codex or other paid/limited AI review services must not be invoked without explicit permission for the exact use.