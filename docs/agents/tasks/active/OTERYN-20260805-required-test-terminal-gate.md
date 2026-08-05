---
task_id: OTERYN-20260805-required-test-terminal-gate
coordination_id: ci:required-test-terminal-gate
status: implementing
branch: repair/issue-623
base_branch: main
created: 2026-08-05
updated: 2026-08-05
related_issue: "623"
related_pr: none
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
search_first:
  - .github/workflows/ci.yml
  - tests/ci
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

- [ ] Preserve required check names `classify-changes` and `test`.
- [ ] Move the conditional expensive suite behind an internal job name.
- [ ] Add an always-running `test` aggregation job.
- [ ] Fail closed when classification fails.
- [ ] Fail closed when CI is required and the expensive suite does not succeed.
- [ ] Succeed without the expensive suite when CI is not required.
- [ ] Add a focused regression test for the workflow contract.
- [ ] Prove behavior on exact head and unblock a real documentation-only PR.

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
updated_at: 2026-08-05T21:08:00Z
head: UNKNOWN
branch: repair/issue-623
pr: none
status: implementing
phase: implement
session_id: chatgpt-20260805T2307+0200-required-test-gate
session_role: implementer
execution_mode: github
lease_expires_at: 2026-08-05T21:53:00Z
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
validation_level: focused
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
  - PR 613 exact head has classify-changes success and test skipped.
  - GitHub rejects merge because required checks remain expected.
derived:
  - A stable always-running aggregation job named test is required.
unknown:
  - Exact final implementation head and validation results.
conflicts: []
first_failure:
  marker: required test context
  evidence: protected merge returns 405 with 2 of 2 required checks expected
rejected_hypotheses:
  - All workflow conclusions being successful is sufficient; the skipped required job disproves this.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-required-test-terminal-gate.md
validation:
  - command: live protection, PR and check-run inspection
    result: PASS
    evidence: required context mismatch reproduced on PR 613
blockers: []
next_action: Implement the terminal aggregation job and focused workflow test.
```
