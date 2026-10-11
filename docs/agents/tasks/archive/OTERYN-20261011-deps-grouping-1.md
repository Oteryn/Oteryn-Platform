---
task_id: OTERYN-20261011-deps-grouping-1
governing_issue: 1517
required_reads: []
search_first:
  - .github/dependabot.yml
optional_reads: []
---

# OTERYN-20261011 dependency update grouping (token-free)

## Goal

Governing GitHub Issue: #1517 — canonical lifecycle authority for this task.

Group Dependabot minor and patch updates into one weekly pull request per configured update entry (ecosystem and directory; major stays separate). Narrowed by D282: the auto-merge workflow is removed because `GITHUB_TOKEN` cannot enqueue into the merge queue and a queue-capable App/PAT secret needs an owner decision. The workflow is carried into follow-up DEPS-AUTOMERGE-2 (blocked on that owner decision).

## Acceptance criteria

- [x] `.github/dependabot.yml` groups minor and patch updates per ecosystem and keeps schedules and limits.
- [x] No workflow or lifecycle-policy change; no token or secret is introduced.
- [x] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - .github/dependabot.yml
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/archive/OTERYN-20261011-deps-grouping-1.md
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
updated_at: 2026-10-11T08:00:00Z
head: 635a09d6a1f9e460ff9c2cc46b8e3b09c84e8477
branch: claude/deps-automerge-1-20261011
pr: 1511
status: completed
context_routes:
  - repository-automation
owned_paths:
  - .github/dependabot.yml
  - docs/operations/GITHUB_REPOSITORY_GOVERNANCE.md
  - docs/agents/tasks/archive/OTERYN-20261011-deps-grouping-1.md
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
  - docs/agents/tasks/archive/OTERYN-20261011-deps-grouping-1.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/archive/OTERYN-20261011-deps-grouping-1.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
blockers:
  - none
next_action: none; PR #1511 merged and record archived.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1511 merged (REST pulls/1511 state closed, merged true) as aaf209112c78d17c8de3309624807de57d62fc24 at 2026-10-11T07:31:34Z from exact head 635a09d6a1f9e460ff9c2cc46b8e3b09c84e8477; source branch (claude/deps-automerge-1-20261011) absent (REST branches API returns 404)
```

## Notes

Follow-up DEPS-AUTOMERGE-2 (auto-merge workflow) is blocked on an owner decision about a merge-queue-capable token (GitHub App or PAT secret).
