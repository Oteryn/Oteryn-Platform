---
task_id: OTERYN-20260805-required-ci-gate
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 552
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - overlapping active tasks and open pull requests for required checks or ci.yml
  - existing change classifier and CI validation tests
optional_reads:
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260805-required-ci-gate

## Goal

Repair Issue #552 by making the protected `test` context a stable, always-emitted aggregate gate: documentation-only changes may pass with an explicit proven `NOT_APPLICABLE` runtime-test result, while runtime/code changes cannot pass unless the full runtime test job succeeds.

## Delivery classification

```yaml
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: true
```

## Acceptance criteria

- [x] `classify-changes` remains an always-emitted fail-closed protected context.
- [x] `test` is always emitted and succeeds for a documentation-only PR only after classification proves runtime tests are not applicable.
- [x] Runtime/code classification cannot produce a successful `test` context unless the full runtime test job succeeds.
- [x] Classifier failure, malformed classifier output, skipped required runtime tests, failed runtime tests and inconsistent state fail the aggregate gate.
- [x] Positive, negative and boundary tests cover the gate evaluator.
- [x] Existing runtime test commands and MariaDB integration service remain unchanged in the conditional runtime job.
- [x] A runtime-classified exact implementation passed all emitted workflows, including deep system validation.
- [ ] Current-main synchronization checks pass and PR #626 merges through protected `main`.
- [ ] A real documentation-only PR proves the protected merge gate can be satisfied without bypass.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-ci-gate.md
modules:
  - CI required-check contract
dependencies:
  - Issue #552
  - protected main contexts classify-changes and test
blockers: []
cross_repository_tasks: []
shared_path_lease:
  - path: .github/workflows/ci.yml
    holder: OTERYN-20260805-required-ci-gate
    expires_at: 2026-08-06T00:20:00Z
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T21:44:00Z
head: a8635c5a992bc4732d90f89d6e033cd1c27786ef
branch: repair/issue-552-required-ci-gate
pr: 626
status: validating
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - .github/workflows/ci.yml
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-ci-gate.md
proven:
  - Main is protected and requires classify-changes and test.
  - The previous CI contract omitted the required test context for documentation-only changes.
  - PR 626 implements a conditional runtime-tests lane and an always-running aggregate test context.
  - The evaluator fails closed for classifier failure, malformed output, skipped or failed required runtime tests and inconsistent NOT_APPLICABLE state.
  - Exact implementation head 2738129190ef2e6173aff2cb699b584519f76922 passed CI, Agent Governance, Edge Security, Phase 7, DB outage, game-auth concurrency and Deep System Validation.
  - CI run 31047986939 proved runtime-tests and aggregate test both succeed for a runtime-classified change.
  - Main advanced after validation, and protected merge correctly refused an out-of-date branch.
  - The four implementation paths were reapplied without semantic change on current main 6ce4189ca2881c012332f24238cae9a35d35efb1.
derived:
  - A current-main synchronization generation can reuse the completed deep exact-content evidence while protected checks establish current ancestry.
unknown:
  - Whether main will remain stable until protected auto-merge completes.
  - Post-merge documentation-only behavior on PR 604.
conflicts: []
first_failure:
  marker: protected-merge-current-base-required
  evidence: Merge API rejected validated head after main advanced; branch was reconstructed from current main rather than bypassing protection.
rejected_hypotheses:
  - Remove test from branch protection.
  - Use an administrator bypass.
  - Run the full MariaDB/PHP suite for every documentation-only change.
changed_paths:
  - .github/workflows/ci.yml
  - docs/agents/tasks/active/OTERYN-20260805-required-ci-gate.md
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
validation:
  - command: exact implementation workflow generation on 2738129190ef2e6173aff2cb699b584519f76922
    result: PASS
    evidence: all seven emitted workflows passed, including full zero-retry browser matrix and soak evidence.
  - command: exact runtime-classified CI lane
    result: PASS
    evidence: classify-changes, runtime-tests and aggregate test all completed successfully in run 31047986939.
  - command: independent exact-head diff audit
    result: PASS
    evidence: review 4868831358 found no material finding.
  - command: current-main reconstruction
    result: PASS
    evidence: branch reset to main 6ce4189ca2881c012332f24238cae9a35d35efb1 and exactly four semantically identical paths reapplied.
blockers: []
next_action: Enable protected auto-merge for PR 626, verify required checks on the synchronized head, then exercise PR 604 as the documentation-only proof.
```

## Notes

Issue #552 is the root-cause owner. PR #604 is the real documentation-only case that exposed the skipped-context failure. No branch-protection bypass, production operation or external-repository change is authorized.
