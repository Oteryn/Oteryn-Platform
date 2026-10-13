---
task_id: OTERYN-20261013-platform-agents-jira
governing_issue: 1595
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261013 Platform Jira rule alignment

## Goal

Governing GitHub Issue: #1595 is the canonical lifecycle authority for this task.

Align the Platform `AGENTS.md` "Jira programme coordination" section with the Game owner decision of 2026-09-29: only the programme coordinator writes to Jira; workers do not load the Jira connector.

## Acceptance criteria

- [x] The rule that let ordinary workers update their mapped Story is replaced by the coordinator-only rule, pointing to the Game decision.
- [x] No other Platform doc carries the worker-writes-Jira text; no validator pins it.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/archive/OTERYN-20261013-platform-agents-jira.md
modules:
  - governance
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
updated_at: 2026-10-13T12:17:01Z
head: 2359b67651c8ebb5e2014a0d308b4f16c714aa7e
branch: claude/platform-agents-jira-20261013
pr: 1596
status: completed
context_routes:
  - governance AGENTS.md
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/archive/OTERYN-20261013-platform-agents-jira.md
proven:
  - PLATFORM_AGENT_BOOTSTRAP.md has no Jira text
  - completion-lead programme section 7 is already consistent
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - AGENTS.md
  - docs/agents/tasks/archive/OTERYN-20261013-platform-agents-jira.md
validation:
  - command: python3 tools/agents/checkpoint.py <this record> --require-checkpoint
    result: PASS
    evidence: see PR
  - command: python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: see PR
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only change
blockers:
  - none
next_action: none; merge PR #1596 and this record is already archived.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1596 is terminal and the same-repository branch has no durable post-merge purpose.
source_branch_evidence: pending until PR #1596 merges; verify the source ref disappears.
```

## Notes

Docs-only. Decision source: Oteryn-Game docs/agents/JIRA_PROGRAMME_COORDINATION.md.
