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
- [ ] Exact-head repository checks, protected integration and readback pass.
- [ ] Archive this packet and close Issue #1299 after verified integration.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T06:26:00Z
head: 706b84000d4380a6b2a269939d43766b95a87f56
branch: fix/task-inventory-path-d26-20260907
pr: 1300
status: validating
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
unknown:
  - Exact final-head hosted qualification and merge outcome.
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
next_action: Qualify the published final candidate through existing repository CI and normal protected integration, then archive this packet.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR #1300 is active; ordinary delete-after-merge path applies after integration.
source_branch_evidence: Exact task revision, checks and closeout are recorded in the live Issue and PR, not self-referential checkpoint commits.
```
