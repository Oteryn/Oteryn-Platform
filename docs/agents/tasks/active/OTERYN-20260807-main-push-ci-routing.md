---
task_id: OTERYN-20260807-main-push-ci-routing
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - issue #783 and related open PRs
  - overlapping active tasks and repair branches
optional_reads: []
---

# OTERYN-20260807-main-push-ci-routing

## Goal

Repair Issue #783 so documentation/governance-only pushes to `main` do not run or preempt runtime-heavy CI/Acceptance work, while product-affecting and ambiguous pushes remain fail-closed and required pull-request/manual behavior is preserved.

## Acceptance criteria

- [x] Main-push CI classifies the exact pushed base/head range instead of unconditionally forcing `--all`.
- [x] Missing, zero, empty or unusable push ranges fail closed to all heavy CI gates.
- [x] Documentation/governance-only pushes to `main` do not emit full Acceptance E2E runs.
- [x] Existing product-path Acceptance triggers, pull-request routing and manual `workflow_dispatch` remain intact.
- [x] Deterministic tests cover docs-only/product main routing, zero/ambiguous range fallback, Acceptance path filtering and concurrency/preemption safety.
- [x] Exact implementation-head self-review, focused validation and repository-required CI pass with zero unresolved material findings; final checkpoint-only head still requires its selected exact-head checks before merge.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
modules:
  - ci-build-test
  - architecture-governance
dependencies:
  - Issue #783
  - docs/agents/BUILD_TEST_MATRIX.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T10:18:00Z
head: 55aecd772447709355437c44e810f82c2b4fdbf0
branch: repair/issue-783
pr: 786
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
proven:
  - Issue #783 is implementation-authorized and PR #786 is the active one-Issue repair delivery.
  - Exact implementation head 55aecd772447709355437c44e810f82c2b4fdbf0 is one coherent repair commit directly over main fed4ea954cd34d8b1b3a06a3de6f7307f619c747.
  - CI push routing uses the GitHub push before/head range through a dedicated fail-closed adapter while pull-request routing remains on the canonical classifier.
  - Acceptance push routing mirrors its existing product-path pull-request filter, so docs/governance-only main pushes do not create the full Acceptance workflow generation.
  - All eight emitted workflows on implementation head 55aecd772447709355437c44e810f82c2b4fdbf0 completed successfully.
derived:
  - Trigger-level Acceptance filtering removes docs-only main pushes from the shared main Acceptance concurrency generation and therefore prevents them from preempting a required prior runtime generation.
  - Reusing the canonical path classifier through a push adapter keeps path policy single-sourced while adding push-range failure handling.
unknown: []
conflicts: []
first_failure:
  marker: Agent Governance run 31165291464 failed active task checkpoint validation.
  evidence: The original task checkpoint contained unsupported nested validation_gate metadata; replacing it with the version-1 compact contract made active checkpoint validation pass, and the inherited terminal audit task was later archived on main.
rejected_hypotheses:
  - Keep unconditional main --all and only tune concurrency; documentation-only pushes would still allocate heavy CI.
  - Add only an in-job Acceptance guard; the workflow would still be created and could preempt an in-progress main run before the guard executes.
changed_paths:
  - .github/workflows/ci.yml
  - .github/workflows/acceptance-validation.yml
  - scripts/ci/classify_push_changes.py
  - tests/ci/test_push_change_routing.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing.md
validation:
  - command: exact base/head full-diff self-review on 55aecd772447709355437c44e810f82c2b4fdbf0
    result: PASS
    evidence: Scope is limited to the six owned paths; acceptance, negative paths, rollback by reverting the workflow/classifier commit, PR/manual compatibility and related-PR state were checked with no material findings.
  - command: CI run 31165893804
    result: PASS
    evidence: Exact implementation-head CI routing tests and selected runtime validation completed successfully.
  - command: Acceptance E2E and Visual UX run 31165892762
    result: PASS
    evidence: Existing pull-request/manual product acceptance behavior remained executable on the implementation head.
  - command: Agent Governance run 31165891179
    result: PASS
    evidence: Active-task checkpoint and live ownership evaluation completed successfully on the implementation head.
  - command: remaining exact-head workflows
    result: PASS
    evidence: Game Auth Ticket Concurrency 31165890952, Edge Security Emulation 31165891600, Platform DB Outage Validation 31165891559, Phase 7 Production-Like Validation 31165891639 and Deep System Validation 31165891386 all completed successfully.
blockers: []
next_action: Validate the final checkpoint-only head, reverify reviews/mergeability and required exact-head checks, then squash-merge and complete Issue/task closeout.
```

## Self-review

```yaml
result: PASS
exact_head: 55aecd772447709355437c44e810f82c2b4fdbf0
acceptance_checked: true
full_diff_checked: true
negative_paths_checked: true
rollback_checked: true
compatibility_checked: true
related_prs_checked: true
findings: []
evidence:
  - CI 31165893804 PASS
  - Acceptance E2E and Visual UX 31165892762 PASS
  - Agent Governance 31165891179 PASS
  - all other emitted exact-head workflows PASS
```

E2E result: `PASS` via Acceptance E2E and Visual UX run `31165892762`. No production or external-service mutation was required or performed.

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: repair-issue-783-20260807T1018Z
  session_started_at: 2026-08-07T10:15:00Z
  checkpointed_at: 2026-08-07T10:19:00Z
  last_progress_at: 2026-08-07T10:18:28Z
  phase: validate-and-merge
  exact_head: a175d388db6b2bfd12dd621c4e754e53f6e19f5f
  pull_request: 786
  active_operation: final exact-head CI after ready-for-review transition
  external_run_ids:
    - 31169518770
    - 31169518719
    - 31169518736
    - 31169518799
    - 31169518910
    - 31169518939
    - 31169518843
    - 31169518713
  operation_started_at: 2026-08-07T10:18:28Z
  wait_deadline_at: 2026-08-07T11:03:28Z
  check_generation: ready
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: Reconcile the live PR head created by this checkpoint-only commit, then observe its aggregate required-check state no sooner than the minimum terminal-CI interval.
  next_action: Observe the aggregate required checks for the live PR head; if all pass, reverify merge gates and squash-merge PR #786.
```

## Notes

Issue #784 recorded the finding and its audit task has already been archived on `main`; it does not own or implement these repair paths.
