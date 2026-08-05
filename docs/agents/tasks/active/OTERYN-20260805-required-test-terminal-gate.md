---
task_id: OTERYN-20260805-required-test-terminal-gate
coordination_id: ci:required-test-terminal-gate
status: validating
branch: repair/issue-623
base_branch: main
created: 2026-08-05
updated: 2026-08-05
related_issue: "623"
related_pr: "624"
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
search_first:
  - .github/workflows/ci.yml
  - tests/ci/test_ci_required_gate.py
optional_reads: []
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_ci_required_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-test-terminal-gate.md
  - docs/agents/tasks/archive/OTERYN-20260805-required-test-terminal-gate.md
modules_touched:
  - ci
  - agent-governance
depends_on: []
blocks:
  - PR 613
cross_repo_tasks: []
---

# Required test terminal gate repair

## Goal

Ensure the branch-protection context `test` always terminates without running the MariaDB-backed application suite for classifier-approved documentation-only changes.

## Acceptance criteria

- [x] Preserve required check names `classify-changes` and `test`.
- [x] Move the conditional expensive suite behind internal job `full-test-suite`.
- [x] Add an always-running `test` aggregation job.
- [x] Fail closed when classification fails.
- [x] Fail closed when CI is required and the expensive suite does not succeed.
- [x] Succeed without the expensive suite when CI is not required.
- [x] Add a focused regression test for the workflow contract.
- [ ] Prove behavior on the exact final head and unblock real documentation-only PR #613.

## Design

```yaml
protected_contexts:
  - classify-changes
  - test
internal_jobs:
  full-test-suite:
    runs_when: classification success and ci=true
    services:
      - MariaDB
  test:
    runs_when: always
    services: []
    fails_when:
      - classification did not succeed
      - ci=true and full-test-suite did not succeed
    succeeds_when:
      - classification succeeded and ci=false
      - classification succeeded and ci=true and full-test-suite succeeded
```

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_ci_required_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-test-terminal-gate.md
  - docs/agents/tasks/archive/OTERYN-20260805-required-test-terminal-gate.md
shared_paths: []
forbidden_paths:
  - application runtime
  - migrations
  - production systems
  - external repositories
claim:
  protocol_version: 2
  issue: 623
  claim_nonce: issue-623-c7914282-20260805T2107Z
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T21:12:00Z
head: 4fc85c81d4907df6b61f33b140c4406a7c010bfd
branch: repair/issue-623
pr: 624
status: validating
phase: validate
session_id: chatgpt-20260805T2307+0200-required-test-gate
session_role: implementer
execution_mode: github
lease_expires_at: 2026-08-05T21:57:00Z
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
validation_level: full
context_routes:
  - ci-repair
  - agent-governance
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_ci_required_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-test-terminal-gate.md
  - docs/agents/tasks/archive/OTERYN-20260805-required-test-terminal-gate.md
proven:
  - main protection requires classify-changes and test.
  - PR 613 reproduced classify-changes=success, test=skipped and a protected merge rejection.
  - PR 624 preserves both protected context names.
  - The conditional MariaDB-backed work is isolated as full-test-suite.
  - The required test job always materializes and evaluates classification and full-suite results fail closed.
  - The focused regression contract executes successfully inside classify-changes.
  - Fresh independent review 4868781237 found zero material findings.
derived:
  - After this repair reaches main, a synchronized documentation-only PR can prove the terminal success path with full-test-suite skipped.
unknown:
  - Exact final PR 624 head and complete emitted check results.
  - Live PR 613 proof after main contains the repair.
conflicts: []
first_failure:
  marker: none
  evidence: implementation classifier and focused contract succeeded on 4fc85c81d4907df6b61f33b140c4406a7c010bfd
rejected_hypotheses:
  - A conditionally skipped required job is merge-safe; PR 613 disproved this.
  - Running the full suite for every documentation change is necessary; the terminal aggregation model preserves fail-closed behavior without that cost.
changed_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_ci_required_gate.py
  - docs/agents/tasks/active/OTERYN-20260805-required-test-terminal-gate.md
validation:
  - command: python tests/ci/test_classify_changes.py
    result: PASS
    evidence: classify-changes job 92446224350
  - command: python tests/ci/test_ci_required_gate.py
    result: PASS
    evidence: classify-changes job 92446224350
  - command: fresh CI repair audit
    result: PASS
    evidence: review 4868781237; zero material findings
  - command: exact-final-head emitted workflows
    result: PENDING
    evidence: final checkpoint commit triggers a fresh exact-head generation
  - command: documentation-only live proof on PR 613
    result: PENDING
    evidence: requires repaired workflow on main
blockers: []
next_action: Reconcile with current main, pass all exact-head checks, merge PR 624, then synchronize PR 613 and prove test=success with full-test-suite=skipped.
```
