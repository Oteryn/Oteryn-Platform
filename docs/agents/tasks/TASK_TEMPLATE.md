---
task_id: OTERYN-YYYYMMDD-short-slug
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  # Add task-specific architecture, contract, security, or program files here.
search_first: []
optional_reads: []
---

# OTERYN-YYYYMMDD-short-slug

## Goal

<bounded task goal>

## Acceptance criteria

- [ ] <criterion>

## Ownership

```yaml
owned_paths:
  - <path/glob>
modules:
  - <module>
dependencies:
  - <task/contract-or-none>
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Policy revision 2 adds accepted statuses `waiting` and `completed` and validation result `NOT_APPLICABLE` without invalidating existing checkpoints. Validate the completed checkpoint with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

`ROTATE` is a terminal invocation result, never a task status. Before returning `ROTATE`, persist `ready`, `waiting`, or `blocked` with one concrete `next_action`.

```yaml
checkpoint_version: 1
updated_at: YYYY-MM-DDTHH:MM:SSZ
head: UNKNOWN
branch: <task-branch>
pr: none
status: investigating # investigating|implementing|validating|ready|waiting|blocked|completed
context_routes:
  - <route>
owned_paths:
  - <path/glob>
proven: []
derived: []
unknown:
  - <first unresolved fact>
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths: []
validation:
  - command: not-run
    result: NOT_RUN # PASS|FAIL|BLOCKED|NOT_RUN|NOT_APPLICABLE
    evidence: task not yet implemented # concrete reason required for NOT_APPLICABLE
blockers:
  - none
next_action: <one concrete next step>
```

## Notes

Keep this section concise. Durable continuation state belongs in the checkpoint above. Do not paste secrets, full logs, or full diffs. Structural checkpoint validation does not replace live Git, PR, or CI verification.