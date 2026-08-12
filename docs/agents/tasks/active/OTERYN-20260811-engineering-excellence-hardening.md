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
updated_at: 2026-08-12T17:12:00+02:00
head: bdb20db896c5b7fa134fae6c437413761b54618a
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
  - Live protected main is 25ca5180849996462dfcfd2a0914eb6d765a47d3 and PR #1003 is based on that generation.
  - Exact PR head 07fc86921ea1cce60e222041c9a8e60bf6e15314 received Codex review PRR_kwDOTcsYjs8AAAABJROTLg; the review explicitly says Reviewed commit 07fc86921e.
  - That exact-head Codex review reported three material P2 findings: comments could spoof a proving-job condition marker, filename-first workflow classification could bypass unsupported events, and the moving live-pr-1003-authoritative checkpoint alias violated immutable checkpoint semantics.
  - Material implementation head bdb20db896c5b7fa134fae6c437413761b54618a parses the proving job direct if scalar, rejects commented condition spoofing, validates workflow event shapes before filename classification, and adds regressions for both bypass classes.
  - This task record stores the immutable material implementation head bdb20db896c5b7fa134fae6c437413761b54618a. The commit containing this checkpoint is intentionally a checkpoint-only successor whose SHA cannot be embedded in its own contents; live PR #1003 must be queried for the successor exact head before attributing final CI or review evidence.
  - Portal Exhaustive Acceptance run 31601106328 attempt 1 failed in the bounded portability profile because Playwright WebKit returned an internal error while launching the browser for tests/portal/support-moderation.spec.ts:389, before an application assertion executed.
  - The immediately preceding green generation d9bde9362e9f2557e4c44cfeeb78f1f887b5c4d4 passed the same 54-test portability profile and the same WebKit test; the d9bde936..07fc8692 change set contained validator/regression/checkpoint changes rather than portal or browser-runtime changes.
  - A rerun of job 94128450395 was started only after that evidence-based classification; attempt 2 was later cancelled by the new-head repair commits, not counted as a passing rerun.
derived:
  - The 07fc8692 portability failure is classified as a transient WebKit/browser-runner launch flake rather than a deterministic repository regression; the final repair generation must still prove the portability profile again on its own exact head.
  - All material findings currently known from exact-head Codex review PRR_kwDOTcsYjs8AAAABJROTLg have code, regression, or checkpoint repairs in this generation.
unknown:
  - Terminal result of every required/emitted workflow on the checkpoint-only successor exact PR head.
  - Fresh independent Codex review result for that final exact PR head.
conflicts: []
first_failure:
  marker: exact-head-Codex-P2-repairs-require-fresh-proof
  evidence: Review PRR_kwDOTcsYjs8AAAABJROTLg found three P2 issues on 07fc8692; material code repairs end at bdb20db896c5b7fa134fae6c437413761b54618a and this checkpoint records them before the final hosted proof generation.
rejected_hypotheses:
  - Treat the WebKit portability failure as an application assertion regression: the artifact records a browserType.launch internal WebKit error, while the same test/profile passed immediately before and no portal/browser runtime changed between d9bde936 and 07fc8692.
  - Accept job-condition marker text from arbitrary job comments: direct job-level if parsing is now required and covered by standalone and inline comment-spoof regressions.
  - Classify build/deploy/ci workflows by filename before checking event shape: unsupported top-level events now fail closed before semantic filename classification.
  - Store a moving PR-ref alias in checkpoint head fields: immutable material SHA bdb20db896c5b7fa134fae6c437413761b54618a is recorded and the checkpoint-only successor is explicitly identified as requiring live verification.
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
  - command: exact-head Codex review on 07fc86921ea1cce60e222041c9a8e60bf6e15314
    result: FAIL
    evidence: Review PRR_kwDOTcsYjs8AAAABJROTLg reviewed 07fc86921e and reported three material P2 findings; all are addressed by the bdb20db8 material repair generation plus this immutable checkpoint correction.
  - command: Portal Exhaustive Acceptance run 31601106328 attempt 1 plus preceding green run 31598174695
    result: FAIL
    evidence: Red artifact isolated WebKit browserType.launch internal error; preceding exact profile/test passed. Failure classified transient before rerun; rerun attempt 2 was cancelled by subsequent repair-head updates and is not green evidence.
  - command: final checkpoint-successor GitHub Actions generation
    result: NOT_RUN
    evidence: Hosted workflows must execute on the exact live PR head created by this checkpoint update.
  - command: final checkpoint-successor Codex review
    result: NOT_RUN
    evidence: Fresh review must be requested only after the final exact PR head and hosted gates are confirmed.
blockers:
  - none
next_action: Read the live PR #1003 successor head created by this checkpoint commit, then validate its complete exact-head GitHub Actions generation before requesting a fresh exact-head Codex review.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 5
  session_id: coordinator-20260812-engineering-hardening-final
  session_started_at: 2026-08-12T16:39:00+02:00
  checkpointed_at: 2026-08-12T17:12:00+02:00
  last_progress_at: 2026-08-12T17:12:00+02:00
  phase: final-exact-head-validation
  exact_head: bdb20db896c5b7fa134fae6c437413761b54618a
  pull_request: 1003
  active_operation: validate the checkpoint-only successor exact PR head after Codex P2 repairs
  external_run_ids: []
  operation_started_at: 2026-08-12T17:12:00+02:00
  wait_deadline_at: null
  check_generation: final-codex-p2-repairs
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: live PR #1003 successor head is stable and its exact-head hosted checks can be observed
  next_action: query live PR #1003 head and aggregate exact-head checks, investigate any failure, then request fresh Codex review only after green hosted proof
```

## Notes

Issue #1002. `feature_scope: internal_only`; no production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized or performed by this task.
