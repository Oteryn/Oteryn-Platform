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
  - PR #992 agent-governance ownership
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
- [ ] Agent Governance executes deterministic prompt/governance regression after PR #992 releases ownership.
- [x] Deep System Validation installs acceptance dependencies with committed lockfile via `npm ci`.
- [ ] Exact-final-head CI/E2E/self-review and fresh independent Codex review are green before merge.

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
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
deferred_overlap_paths:
  - path: .github/workflows/agent-governance.yml
    owner: PR #992
  - path: tools/agents/**
    owner: PR #992
modules:
  - build-test
  - ci
  - agent-governance
  - synology-staging-deployment
dependencies:
  - Issue #1002
  - terminal PR #992 with lifecycle ownership release before agent-governance edit
blockers:
  - PR #992 remains open and owns .github/workflows/agent-governance.yml plus tools/agents/**
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-12T06:56:40Z
head: a0a5d1c68c782e16b521e12e582f87f99d7cd47e
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
  - .github/workflows/ci.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - Protected main was ab43c4b47173e7208d34851c4091f79051379f7a when PR #1003 was synchronized; merge commit 3f0a5256a71605c11cd14a98e8314d2f2eeb2213 incorporated that main without force.
  - PR #1001 is merged/terminal and its previous .github/workflows/deep-system-validation.yml ownership is released.
  - Deep System Validation on a0a5d1c68c782e16b521e12e582f87f99d7cd47e contains the landed current-main workflow with exactly the acceptance install changed from npm install to npm ci.
  - Reconcile run 31571721768 generated a0a5d1c68c782e16b521e12e582f87f99d7cd47e with a one-line Deep System change and self-deletion of the temporary helper; connector fast-forwarded the branch after the workflow-token push was rejected for missing workflows permission.
  - PR #992 remains open at 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed and owns .github/workflows/agent-governance.yml and tools/agents/**.
  - Current PR #1003 remains draft and mergeable; no production deployment, protected-environment approval, secret mutation, live customer-data mutation, payment action, or external-repository write has been performed.
derived:
  - All currently safe non-overlapping implementation work is complete; final Agent Governance wiring must wait for PR #992 terminal lifecycle ownership release by explicit task instruction and repository ownership policy.
  - Exact-final-head CI and fresh final-head Codex review cannot be validly completed until the deferred Agent Governance edit and any resulting main synchronization are incorporated.
unknown:
  - Final landed current-main form of .github/workflows/agent-governance.yml after PR #992 becomes terminal.
  - Whether protected main or open-PR ownership will change before PR #992 releases the deferred paths.
conflicts: []
first_failure:
  marker: agent_governance_ownership_release
  evidence: PR #992 is open at 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed and explicitly owns the deferred Agent Governance surface.
rejected_hypotheses:
  - Add tests/Integration directly to phpunit.xml; the existing cross-repository test requires an external generated snapshot environment.
  - Treat prior Agent Governance failure as infrastructure; exact logs proved the task record previously omitted open PR #1003 identity.
  - Push the generated Deep System workflow commit with GITHUB_TOKEN; run 31571721768 proved GitHub rejects workflow mutation without workflows permission, while the generated commit object remained verifiable and was safely fast-forwarded with the GitHub connector.
changed_paths:
  - .github/dependabot.yml
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
  - command: live protected-main, PR, issue, active-task, ownership and open-PR preflight
    result: PASS
    evidence: Main ab43c4b47173e7208d34851c4091f79051379f7a, PR #1003, Issue #1002, PR #1001, PR #992 and all open PR changed-file inventories relevant to owned paths were inspected before mutation.
  - command: history-preserving current-main synchronization
    result: PASS
    evidence: 3f0a5256a71605c11cd14a98e8314d2f2eeb2213 merged protected main ab43c4b47173e7208d34851c4091f79051379f7a into the task branch with force=false; compare then showed zero commits behind.
  - command: deterministic Deep System workflow reconciliation
    result: PASS
    evidence: Run 31571721768 generated a0a5d1c68c782e16b521e12e582f87f99d7cd47e after git diff --cached --check and an exact 1:1 line-count assertion; commit diff proves only npm install -> npm ci plus removal of the temporary helper.
  - command: current-head applicable GitHub Actions on a0a5d1c68c782e16b521e12e582f87f99d7cd47e
    result: NOT_RUN
    evidence: Pull-request workflow generation was queued/pending when this checkpoint was written; it is supporting evidence only because Agent Governance wiring is still deferred and this is not the final delivery head.
  - command: final-head Codex review
    result: NOT_RUN
    evidence: Final delivery head does not yet exist because PR #992 still owns the required Agent Governance path.
blockers:
  - PR #992 remains open at 3dc0c3429f8b96c913109b1dabfd494d0bbe23ed and retains ownership of .github/workflows/agent-governance.yml and tools/agents/**.
next_action: When PR #992 is terminal and its lifecycle ownership is released, refresh protected main and open-PR ownership, then extend the landed .github/workflows/agent-governance.yml with prompt-eval regression and harness/contract triggers before exact-final-head validation and review.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: 20260812T064100Z-engineering-hardening
  session_started_at: 2026-08-12T06:41:00Z
  checkpointed_at: 2026-08-12T06:56:40Z
  last_progress_at: 2026-08-12T06:56:40Z
  phase: waiting-for-agent-governance-ownership-release
  exact_head: a0a5d1c68c782e16b521e12e582f87f99d7cd47e
  pull_request: 1003
  active_operation: none
  external_run_ids: [31571721768]
  operation_started_at: null
  wait_deadline_at: null
  check_generation: null
  checks_used: 0
  status: waiting
  safe_to_resume: true
  resume_condition: PR #992 is terminal and lifecycle ownership of .github/workflows/agent-governance.yml and tools/agents/** is released.
  next_action: Verify PR #992 terminal/archive state, protected main, PR #1003 head/base and open-PR ownership; if ownership is released, synchronize main without force and wire prompt evals into the landed Agent Governance workflow.
```

## Notes

Issue #1002. No production or staging deployment, protected-environment approval, credential mutation, live data mutation, payment action, or external-repository write is authorized by this task. The checkpoint `head` records the last material implementation head before this task-metadata-only checkpoint commit; live PR state remains authoritative for the current branch head.