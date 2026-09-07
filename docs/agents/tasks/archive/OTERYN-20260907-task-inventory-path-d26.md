---
task_id: OTERYN-20260907-task-inventory-path-d26
governing_issue: 1299
required_reads: []
search_first: []
optional_reads: []
---

# Task inventory input boundary

## Goal

Issue #1299 / merged PR #1300 repaired instruction-debt finding D26.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T08:30:00Z
head: 3557085c20512d25576d8884cc54471665784b00
branch: none
pr: 1300
status: completed
context_routes:
  - agent-governance
owned_paths: []
proven:
  - PR #1300 merged to protected main as 3557085c20512d25576d8884cc54471665784b00.
  - Missing and regular-file inventory paths fail before Issue API evaluation.
  - Existing empty and README-only directories remain valid without Issue API calls.
  - The ten focused tests, PR-head platform-gate and PR-head Agent Governance passed.
  - Protected-main source readback contains the is_dir boundary and five focused regressions.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: resolved-d26-invalid-inventory-pass
  evidence: The former exists/glob behavior was replaced by an explicit is_dir boundary.
rejected_hypotheses:
  - An extra Issue API read was needed to distinguish an invalid local path.
changed_paths:
  - tools/agents/task_issue_liveness.py
  - tools/agents/test_task_issue_liveness.py
validation:
  - command: python tools/agents/test_task_issue_liveness.py -v
    result: PASS
    evidence: Ten focused tests passed on PR head 1d2c8019e09e0d5c98acaab9ba25032eef17a01b.
  - command: platform-gate at PR head 1d2c8019e09e0d5c98acaab9ba25032eef17a01b
    result: PASS
    evidence: GitHub check completed successfully before protected merge.
  - command: protected-main source readback
    result: PASS
    evidence: Main 3557085c20512d25576d8884cc54471665784b00 contains source blob bd78d63e051b32615a81074651dff8c9e1036cb4.
blockers: []
next_action: Close Issue #1299 after this archive transition reaches protected main through Issue #1302.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1300 is merged and the ordinary task branch has no remaining owner or purpose.
source_branch_evidence: GitHub branch search on 2026-09-07 found no retained fix/task-inventory-path-d26-20260907 branch.
```
