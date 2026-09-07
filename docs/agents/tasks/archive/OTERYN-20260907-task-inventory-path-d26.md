---
task_id: OTERYN-20260907-task-inventory-path-d26
governing_issue: 1299
required_reads: []
search_first: []
optional_reads: []
---

# Task inventory input boundary

## Goal

Issue #1299 / PR #1300. Reject missing or non-directory task inventories while preserving a valid existing empty directory. This is the independent D26 correctness repair under META #142, not central-policy adoption.

## Acceptance criteria

- [x] Missing and regular-file paths fail before Issue API evaluation.
- [x] Empty and README-only directories remain valid without API calls.
- [x] All ten focused tests, including the CLI error path, pass.
- [x] Exact-head repository checks, protected integration and readback pass.
- [x] Archive this packet and close Issue #1299 after verified integration.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T09:25:00Z
head: 1d2c8019e09e0d5c98acaab9ba25032eef17a01b
branch: fix/task-inventory-path-d26-20260907
pr: 1300
status: completed
context_routes:
  - agent-governance
owned_paths:
  - tools/agents/task_issue_liveness.py
  - tools/agents/test_task_issue_liveness.py
  - docs/agents/tasks/active/OTERYN-20260907-task-inventory-path-d26.md
proven:
  - Five new regressions fail in three cases before repair; all ten tests pass after repair.
  - Published source and test blobs equal locally executed bytes.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: D26-invalid-inventory-passes
  evidence: The old implementation passes missing and regular-file inputs as empty inventories; three RED tests reproduce it.
rejected_hypotheses: []
changed_paths:
  - tools/agents/task_issue_liveness.py
  - tools/agents/test_task_issue_liveness.py
validation:
  - command: python tools/agents/test_task_issue_liveness.py -v
    result: PASS
    evidence: All 10 tests pass; no network calls in path-boundary cases.
  - command: user-facing runtime E2E
    result: NOT_APPLICABLE
    evidence: Python governance input validation only; affected CLI error path is covered directly.
blockers:
  - none
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1300 merged normally and repository delete-after-merge removed the task branch.
source_branch_evidence: PR #1300 head 1d2c8019e09e0d5c98acaab9ba25032eef17a01b squash-merged to protected main as 3557085c20512d25576d8884cc54471665784b00; live branch inventory no longer contains fix/task-inventory-path-d26-20260907.
```
