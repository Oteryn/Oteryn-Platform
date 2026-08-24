---
task_id: OTERYN-YYYYMMDD-short-slug
required_reads: []
# Governing AGENTS/bootstrap instructions are loaded by the bounded bootstrap.
# Add only task-routed architecture, contract, security, programme, validation or handoff files not already required by higher-priority instructions.
search_first: []
optional_reads: []
---

# OTERYN-YYYYMMDD-short-slug

## Goal

Governing GitHub Issue: <number/url> — canonical lifecycle authority for this task.

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

The checkpoint is a durable context/evidence/ownership/handoff mirror. Its lifecycle and PR fields must be reconciled to the governing live GitHub Issue and live PR when those are newer; it is not an independent competing mutable lifecycle authority.

`ROTATE` is a terminal invocation result, never a task status. Before returning `ROTATE`, persist `ready`, `waiting` or `blocked` with one concrete `next_action`.

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

## Source branch closeout

Every task branch must have an intentional terminal disposition. This section is closeout evidence and does not add fields to the shared Context checkpoint contract.

```yaml
source_branch_disposition: pending # pending|auto_delete_after_merge|delete_on_close|retain
source_branch_reason: task is still active
source_branch_evidence: pending
```

- Use `auto_delete_after_merge` for the ordinary same-repository PR path and verify the source ref disappears after merge.
- Before intentionally closing a same-repository PR without merge, put exactly one `Branch-Disposition: delete` or `Branch-Disposition: retain` marker in the PR body and add a non-empty `Branch-Disposition-Reason: ...`.
- `delete_on_close` requires the trusted-main terminal branch lifecycle to prove exact-head terminal state and verify deletion.
- `retain` requires a concrete owner, purpose and review trigger; unexplained retained refs are not terminal.
- Never merge abandoned, superseded, diagnostic, temporary, backup or recovery-only work merely to trigger automatic branch deletion.

## Notes

Keep this section concise. Durable continuation state belongs in the checkpoint above. Do not paste secrets, full logs, or full diffs. Structural checkpoint validation does not replace live Git, PR, CI or source-branch closeout verification.