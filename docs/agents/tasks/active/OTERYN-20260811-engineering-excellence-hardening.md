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
updated_at: 2026-08-12T17:16:00+02:00
head: e035f32c85f8bbb56bac557a93bf878e807247f0
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
  - Live protected main was verified at 25ca5180849996462dfcfd2a0914eb6d765a47d3 and PR #1003 is based on that generation.
  - Exact-head Codex review PRR_kwDOTcsYjs8AAAABJROTLg explicitly reviewed 07fc86921e and reported three material P2 findings: proving-job condition comment spoofing, filename-first unsupported-event bypass, and non-immutable checkpoint aliases.
  - Material head e035f32c85f8bbb56bac557a93bf878e807247f0 binds the proving-job marker to its direct job-level if scalar, rejects standalone/inline comment spoofing, rejects unsupported top-level workflow events before filename classification, recognizes the repository's managed pull_request_target and issue_comment trigger classes, and carries deterministic regressions.
  - CI run 31611094891 on checkpoint successor 4792b1b63443045de4398564b0dda09a14dc00cd passed Integration registry regressions/validator but failed the repository workflow inventory because the first event allowlist omitted three pre-existing managed workflows using pull_request_target and issue_comment.
  - The decoded CI log for job 94162317623 identified exactly cloudflare-oteryn-edge-audit.yml, cloudflare-oteryn-endpoints.yml, and github-actions-storage-hygiene.yml as the managed-event omissions; e035f32c adds those event classes while keeping repository_dispatch and mixed unsupported shapes fail closed.
  - This checkpoint records immutable material head e035f32c85f8bbb56bac557a93bf878e807247f0. Its containing commit is intentionally a checkpoint-only successor whose future SHA cannot be embedded in its own contents; final CI/review attribution must use the live PR successor head.
  - Portal Exhaustive Acceptance run 31601106328 attempt 1 failed because Playwright WebKit returned an internal browserType.launch error before an application assertion; immediately preceding run 31598174695 passed the same 54-test portability profile and same WebKit test, and d9bde936..07fc8692 changed validators/checkpoint rather than portal/browser runtime.
  - Job 94128450395 was rerun only after that evidence-based portability classification; attempt 2 was cancelled by subsequent repair-head mutation and is not counted as passing evidence.
derived:
  - The 07fc8692 portability failure is classified as a transient WebKit/browser-runner launch flake; the final exact-head generation must still pass the portability profile.
  - All currently known exact-head Codex material findings have implementation/regression/checkpoint repairs in the e035f32c material generation.
unknown:
  - Terminal result of every required/emitted workflow on the checkpoint-only successor exact PR head.
  - Fresh independent Codex review result for that final exact PR head.
conflicts: []
first_failure:
  marker: final-exact-head-hosted-proof-pending
  evidence: CI 31611094891 exposed the managed-event allowlist omission; root cause was patched at e035f32c and requires a fresh exact-head workflow generation.
rejected_hypotheses:
  - Treat the WebKit portability failure as an application assertion regression: artifact evidence is a browser launch internal error, with the same test/profile green immediately before and no portal/browser runtime change in the relevant delta.
  - Accept arbitrary job comment text as a proving condition: direct job-level if parsing and comment-spoof regressions now fail closed.
  - Allow filename classification to bypass unknown triggers: repository_dispatch remains rejected before semantic classification, including mixed supported/unsupported triggers.
  - Treat pull_request_target and issue_comment as unmanaged merely because the first new allowlist omitted them: decoded CI evidence and live workflows prove they are existing explicitly governed trusted-trigger shapes and they are now classified without admitting repository_dispatch.
  - Store moving PR aliases in checkpoint head fields: immutable material SHA e035f32c85f8bbb56bac557a93bf878e807247f0 is stored and the checkpoint-only successor is explicitly called out for live verification.
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
    evidence: All 16 emitted workflows completed success.
  - command: exact-head Codex review on 07fc86921ea1cce60e222041c9a8e60bf6e15314
    result: FAIL
    evidence: Review PRR_kwDOTcsYjs8AAAABJROTLg reviewed 07fc86921e and reported three material P2 findings; all are addressed in the current repair generation.
  - command: CI run 31611094891 job 94162317623 on 4792b1b63443045de4398564b0dda09a14dc00cd
    result: FAIL
    evidence: Integration registry regressions/validator passed; workflow inventory failed only on pre-existing pull_request_target/issue_comment workflows omitted from the first allowlist. e035f32c repairs that root cause and adds managed-event regression coverage.
  - command: Portal Exhaustive Acceptance run 31601106328 attempt 1 versus green run 31598174695
    result: FAIL
    evidence: WebKit browserType.launch internal error was isolated and classified transient before rerun; final exact-head hosted proof remains required.
  - command: final checkpoint-successor GitHub Actions generation
    result: NOT_RUN
    evidence: Workflows must execute on the exact live PR successor head created by this checkpoint update.
  - command: final checkpoint-successor Codex review
    result: NOT_RUN
    evidence: Fresh review must be requested only after the final exact PR head and hosted gates are green.
blockers:
  - none
next_action: Query the live PR #1003 checkpoint-successor head, validate its complete exact-head GitHub Actions generation, investigate any red result, then request a fresh exact-head Codex review only after hosted proof is green.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 6
  session_id: coordinator-20260812-engineering-hardening-final
  session_started_at: 2026-08-12T16:39:00+02:00
  checkpointed_at: 2026-08-12T17:16:00+02:00
  last_progress_at: 2026-08-12T17:16:00+02:00
  phase: final-exact-head-validation
  exact_head: e035f32c85f8bbb56bac557a93bf878e807247f0
  pull_request: 1003
  active_operation: validate checkpoint-only successor after managed-event repair
  external_run_ids: []
  operation_started_at: 2026-08-12T17:16:00+02:00
  wait_deadline_at: null
  check_generation: final-managed-event-repair
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: live PR #1003 successor head is stable and its exact-head hosted checks can be observed
  next_action: query live PR #1003 head and aggregate exact-head checks, investigate any failure, then request fresh Codex review only after green hosted proof
```

## Notes

Issue #1002. `feature_scope: internal_only`; no production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task.
