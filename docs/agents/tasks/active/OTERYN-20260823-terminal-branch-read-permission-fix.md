---
task_id: OTERYN-20260823-terminal-branch-read-permission-fix
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
search_first:
  - terminal-branch-lifecycle-reusable
  - test_terminal_branch_reusable
optional_reads: []
---

# OTERYN-20260823-terminal-branch-read-permission-fix

## Goal

Repair the reusable lifecycle permission chain exposed by the first META/Game/Atlas adoption runs without weakening read-only inventory authority.

## Acceptance criteria

- [x] Reproduce the caller startup failure on all three adoption PRs.
- [x] Add a dedicated reusable read-only workflow with no `contents: write` surface.
- [x] Restrict the write-capable reusable workflow to `close` and `apply`.
- [x] Extend deterministic contracts and workflow inventory.
- [x] Pass exact-head Platform CI on the implementation candidate.
- [ ] Merge PR #1234 and repin/requalify META #52, Game #66 and Atlas #95.

## Ownership

```yaml
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-read-reusable.yml
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - tools/agents/test_terminal_branch_reusable.py
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/agents/tasks/active/OTERYN-20260823-terminal-branch-read-permission-fix.md
modules:
  - agent-governance
  - ci
dependencies:
  - Oteryn/Oteryn-Platform#1230
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#51
  - Oteryn/Oteryn-Game#65
  - Oteryn/Oteryn-Atlas#93
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T23:15:00Z
head: 61af24fbfa2e84c67f7e852d60471a9fbc2ce9d1
branch: fix/org-terminal-read-permissions
pr: 1234
status: ready
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
  - testing
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-read-reusable.yml
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - tools/agents/test_terminal_branch_reusable.py
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/agents/tasks/active/OTERYN-20260823-terminal-branch-read-permission-fix.md
proven:
  - META run 32604201073, Game run 32604209418 and Atlas run 32604218004 all failed at Terminal Branch Lifecycle workflow startup.
  - The first reusable workflow mixed a caller read job with called jobs declaring contents:write.
  - RED contract on eadfd938397e6b9d3327ee4b85e2005bd888e7f0 failed 3 assertions for the missing split permission boundary.
  - GREEN contract passes 5/5 after physically separating read-only inventory from write-capable close/apply jobs.
  - Existing cleanup 7/7, guarded 8/8, approval 10/10 and workflow inventory 18/18 tests pass locally.
  - Platform CI run 32604581522 completed classify-changes, runtime-tests, test and platform-gate successfully on 61af24fbfa2e84c67f7e852d60471a9fbc2ce9d1.
  - Agent Governance run 32604581562, Terminal Branch Lifecycle run 32604581529 and CodeQL run 32604581525 succeeded on the same implementation candidate.
derived:
  - A physically separate read reusable prevents a read-only caller from invoking a workflow definition that can request stronger token permissions.
unknown:
  - exact merged repair SHA callers will pin after PR #1234 merges.
conflicts: []
first_failure:
  marker: Terminal Branch Lifecycle startup_failure
  evidence: caller runs 32604201073, 32604209418, 32604218004
rejected_hypotheses:
  - grant contents:write to the read inventory caller; rejected because it violates least privilege.
changed_paths:
  - .github/workflows/terminal-branch-lifecycle-read-reusable.yml
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - tools/agents/test_terminal_branch_reusable.py
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
validation:
  - command: python3 tools/agents/test_terminal_branch_reusable.py
    result: PASS
    evidence: 5 tests, 0 failures
  - command: python3 tools/agents/test_terminal_branch_cleanup.py
    result: PASS
    evidence: 7 tests, 0 failures
  - command: python3 tools/agents/test_terminal_branch_guarded.py
    result: PASS
    evidence: 8 tests, 0 failures
  - command: python3 tools/agents/test_terminal_branch_approval.py
    result: PASS
    evidence: 10 tests, 0 failures
  - command: python3 tools/validation/test_workflow_inventory.py && python3 tools/validation/workflow_inventory.py
    result: PASS
    evidence: 18 tests; 55 registered workflows within budget 55
  - command: git diff --check origin/main...HEAD
    result: PASS
    evidence: no output
  - command: GitHub Actions CI run 32604581522
    result: PASS
    evidence: classify-changes, runtime-tests, test and platform-gate SUCCESS
  - command: GitHub Actions Agent Governance run 32604581562
    result: PASS
    evidence: SUCCESS
  - command: GitHub Actions Terminal Branch Lifecycle run 32604581529
    result: PASS
    evidence: SUCCESS
  - command: GitHub Actions CodeQL run 32604581525
    result: PASS
    evidence: SUCCESS
blockers:
  - none
next_action: merge PR #1234 after the new checkpoint-only head requalifies, then repin caller PRs to the exact merge SHA
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR #1234 is still active; archive is pending terminal merge
source_branch_evidence: pending
```
