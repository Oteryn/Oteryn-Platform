---
task_id: OTERYN-20261011-deps-automerge-1
governing_issue: 1474
required_reads: []
search_first:
  - .github/dependabot.yml
optional_reads: []
---

# OTERYN-20261011 dependency update grouping and auto-merge

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task (owner approved the change in session; no dedicated issue exists).

Group Dependabot minor and patch updates into one weekly pull request per ecosystem (major stays separate) and enable auto-merge only for Dependabot semver-patch and semver-minor pull requests. Required checks and the merge queue still gate every merge.

## Acceptance criteria

- [x] `.github/dependabot.yml` groups minor and patch updates per ecosystem and keeps schedules and limits.
- [x] `dependabot-auto-merge.yml` triggers only for `dependabot[bot]`, uses a pinned `dependabot/fetch-metadata`, never checks out PR code, and has only `contents: write` and `pull-requests: write`.
- [x] Workflow is registered in the lifecycle policy (budget 53 to 54).
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/dependabot-auto-merge.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-automerge-1.md
modules:
  - repository automation
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T00:00:00Z
head: UNKNOWN
branch: claude/deps-automerge-1-20261011
pr: 1511
status: validating
context_routes:
  - repository-automation
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/dependabot-auto-merge.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-automerge-1.md
proven: []
derived: []
unknown:
  - Exact-head CI result.
  - Repository auto-merge setting is enabled (owner-controlled).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/dependabot-auto-merge.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-automerge-1.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-deps-automerge-1.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: all groups at or below baseline
  - command: python tools/validation/workflow_inventory.py and github_actions_pinning.py and their unit tests
    result: PASS
    evidence: 54 workflows within budget 54; 199 uses references pinned; 25 tests OK
blockers:
  - none
next_action: Read exact-head CI and hand the review packet to the lead.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The lifecycle budget is raised by one because the new workflow has a distinct trigger and permission lifecycle (pull_request_target with write token). Review condition: retire it if Dependabot auto-merge is moved to a native repository setting.
