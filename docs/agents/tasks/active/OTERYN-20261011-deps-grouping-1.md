---
task_id: OTERYN-20261011-deps-grouping-1
governing_issue: 1474
required_reads: []
search_first:
  - .github/dependabot.yml
optional_reads: []
---

# OTERYN-20261011 dependency update grouping (token-free)

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task (owner approved the change in session; no dedicated issue exists).

Group Dependabot minor and patch updates into one weekly pull request per ecosystem (major stays separate). Narrowed by D282: the auto-merge workflow is removed because `GITHUB_TOKEN` cannot enqueue into the merge queue and a queue-capable App/PAT secret needs an owner decision. The workflow is carried into follow-up DEPS-AUTOMERGE-2 (blocked on that owner decision).

## Acceptance criteria

- [x] `.github/dependabot.yml` groups minor and patch updates per ecosystem and keeps schedules and limits.
- [x] No workflow or lifecycle-policy change; no token or secret is introduced.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - .github/dependabot.yml
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-grouping-1.md
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
branch: claude/deps-grouping-1-20261011
pr: 1511
status: validating
context_routes:
  - repository-automation
owned_paths:
  - .github/dependabot.yml
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-grouping-1.md
proven: []
derived: []
unknown:
  - Exact-head CI result.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/dependabot.yml
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/active/OTERYN-20261011-deps-grouping-1.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-deps-grouping-1.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
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

Follow-up DEPS-AUTOMERGE-2 (auto-merge workflow) is blocked on an owner decision about a merge-queue-capable token (GitHub App or PAT secret).
