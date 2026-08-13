---
task_id: OTERYN-20260811-engineering-excellence-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
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
- [x] Integration tests are fail-closed registered to an explicit event, proving job, unmasked executable PHPUnit command, and executable environment provisioning.
- [x] Game Catalog cross-repository staging is selected semantically/manual, not by historical PR number.
- [x] Deterministic prompt-contract regression fixtures and executable validator exist.
- [x] Workflow inventory/classification rejects malformed, unsupported and quoted-event bypasses.
- [x] Synology staging resolves exact source-SHA images to immutable digests and the production-target preflight accepts digest runtime references.
- [x] Agent Governance executes deterministic prompt-contract checks.
- [x] Deep System Validation uses committed-lockfile `npm ci`.
- [x] Full material diff self-review and negative-path audit pass with zero remaining material findings.
- [x] All PR review threads are resolved.
- [ ] Exact-final-head applicable CI/E2E is terminal green.
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
  - exact-head applicable GitHub Actions generation is still running
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T09:19:00+02:00
head: b747d139d353519c7fbc9a8b387d7c67910cce3d
branch: fix/engineering-excellence-hardening
pr: 1003
status: waiting
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
  - Protected main remains 9b9497ba9147f24e751f3632ba0fad8c8c4b220e; PR #1003 was synchronized history-preservingly at 85198c9b6308f1520f6b5dfb0208a126141f1452 and is not behind that base generation.
  - PR #1001 and PR #992 are merged/terminal; PR #1013 explicitly avoids .github/workflows/deploy-synology-staging.yml and does not overlap the final Integration-validator repair.
  - Material head b747d139d353519c7fbc9a8b387d7c67910cce3d repairs the audit false-green paths: invocation text in comments/step names/echo output, directory-only invocation, commented dispatch input/condition spoofing, required-environment substring spoofing, step-scoped env leakage, same-step GITHUB_ENV misuse, lookalike GITHUB_ENV assignment, masked `|| true`, unreachable invocation and pipeline-without-pipefail.
  - A pipeline-backed proving command is accepted only when pipefail is active; the current Game Catalog proving step satisfies that shape and receives its three required paths/SHA through earlier GITHUB_ENV exports.
  - Workflow inventory parses only direct on: children, recognizes bounded quoted keys and fails closed on unsupported events before filename classification.
  - Synology deployment resolves Platform/Gateway/Canary to immutable digest references; Platform/Gateway OCI revisions must match release_sha; production-target-preflight accepts those digest refs and recovers the matching release SHA from OCI metadata.
  - The complete PR diff and all acceptance criteria were re-audited after synchronization; no material P1/P2 finding remains in the material head.
  - Every GitHub review thread on PR #1003 is resolved.
  - Trusted-base AGENTS.override.md and REMEDIATION_AUDIT_RISK_GATE.md both declare external_repair_auditor_required: false. Issue #1002 requires exact-head self-review/CI, not a Codex review. Therefore historical PR/task narrative naming Codex as a merge gate is superseded by the controlling remediation policy; no owner-funded Codex use is required or authorized for this closeout.
derived:
  - The two user-facing audit findings are repaired and regression-hardened beyond the original substring checks.
  - The remaining stop condition is hosted exact-head validation, not implementation or review remediation.
unknown:
  - Terminal result of the still-running applicable workflow generation on material head b747d139d353519c7fbc9a8b387d7c67910cce3d and of the checkpoint-only successor generation selected by repository path routing.
conflicts: []
first_failure:
  marker: none-current
  evidence: all material audit findings were repaired before this checkpoint; hosted validation is pending rather than failed
rejected_hypotheses:
  - Any occurrence of the PHPUnit test path in a job proves execution.
  - A commented or inline-comment workflow_dispatch input/condition is executable configuration.
  - A required environment name in arbitrary job text proves the variable exists for the proving step.
  - Step-local env from an earlier step leaks into the proving step.
  - Writing GITHUB_ENV in the proving step makes the variable available retroactively to that same step.
  - `vendor/bin/phpunit ... || true` or a pipeline without pipefail is a proving command.
  - A fresh Codex review is mandatory for this remediation despite the trusted-base one-owner self-review policy explicitly setting external_repair_auditor_required: false.
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/agent-governance.yml
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/game-catalog-contract.yml
  - composer.json
  - deploy/synology/scripts/production-target-preflight.sh
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
  - command: full material diff self-review at b747d139d353519c7fbc9a8b387d7c67910cce3d
    result: PASS
    evidence: acceptance, negative paths, review findings, rollback/compatibility and related PR ownership were checked; no material finding remains
  - command: focused Integration validator regression suite before hosted final generation
    result: PASS
    evidence: original 23-case repair suite passed before the later self-review hardening; the final expanded suite is part of current hosted CI and must be terminal before merge
  - command: Agent Governance run 31677042948 on b747d139d353519c7fbc9a8b387d7c67910cce3d
    result: PASS
    evidence: exact-head Agent Governance completed successfully
  - command: aggregate applicable workflows on b747d139d353519c7fbc9a8b387d7c67910cce3d
    result: PENDING
    evidence: observations 1 and 2 found 17 emitted workflows; Agent Governance was PASS and remaining CI/E2E/build/validation runs were queued/in-progress with no observed failure
blockers:
  - ordinary exact-head CI observation budget is exhausted while workflows remain non-terminal
next_action: On continuation, inspect the live checkpoint-successor PR head and its applicable exact-head generation; investigate any terminal failure, otherwise complete merge/Issue/archive closeout when repository gates are green.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 8
  session_id: chatgpt-20260813-pr1003-repair
  session_started_at: 2026-08-13T08:55:00+02:00
  checkpointed_at: 2026-08-13T09:19:00+02:00
  last_progress_at: 2026-08-13T09:19:00+02:00
  phase: final-exact-head-validation
  exact_head: b747d139d353519c7fbc9a8b387d7c67910cce3d
  pull_request: 1003
  active_operation: applicable GitHub Actions generation running for the material head
  external_run_ids:
    - 31677042915
    - 31677042973
    - 31677042946
    - 31677042951
    - 31677042916
    - 31677042939
    - 31677042947
    - 31677043003
    - 31677042972
    - 31677042975
    - 31677042919
    - 31677042917
    - 31677042952
    - 31677043134
    - 31677042948
    - 31677042933
    - 31677042914
  operation_started_at: 2026-08-13T09:15:40+02:00
  wait_deadline_at: 2026-08-13T10:00:40+02:00
  check_generation: final-material-b747d139
  checks_used: 2
  status: waiting
  safe_to_resume: true
  resume_condition: live PR successor exact head and applicable workflow generation are terminal enough to classify
  next_action: inspect the live PR #1003 successor head and reconcile its exact-head workflow generation before merge
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: b747d139d353519c7fbc9a8b387d7c67910cce3d
  acceptance_checked: true
  full_diff_checked: true
  related_prs_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  findings: []
  evidence:
    - all audit P1/P2 findings and follow-on negative-path bypasses were repaired in the material head
    - all PR review threads are resolved
    - current protected main generation was merged without history rewrite
    - deployment/preflight producer-consumer identity semantics were checked together
```

## E2E

Applicable exact-head hosted E2E is emitted as part of the current PR workflow generation and remains pending at this checkpoint. Repository/static checks do not substitute for its terminal result.

## Notes

Issue #1002. `feature_scope: internal_only`. No production/staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, owner-funded AI use, or external-repository write is authorized or performed by this task.