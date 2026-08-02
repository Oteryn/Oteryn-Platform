---
task_id: OTERYN-20260802-agent-governance-sync
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
search_first: []
optional_reads: []
---

# OTERYN-20260802-agent-governance-sync

## Goal

Synchronize the shared autonomous-agent governance contract across the five repositories without changing application code.

## Acceptance criteria

- [ ] Task status and invocation-result vocabularies are distinct and consistent.
- [ ] The next-task budget no longer contradicts autonomous continuation.
- [ ] Exact-head, temporary-workflow, independent-audit and authority-freeze rules are deterministic.
- [ ] Checkpoint validation accepts waiting/completed and NOT_APPLICABLE.
- [ ] Exact-head Agent Governance validation passes.

## Ownership

```yaml
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
modules:
  - agent-governance
dependencies:
  - coordinated changes in canary, otclient, Otheryn and freqtrade
blockers:
  - none
cross_repository_tasks:
  - CAN-20260802-agent-governance-sync
  - OTC-20260802-agent-governance-sync
  - OTH-20260802-agent-governance-sync
  - FTAI-20260802-agent-governance-sync
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-02T12:33:00Z
head: UNKNOWN
branch: docs/OTERYN-20260802-agent-governance-sync
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
proven:
  - The current validator rejects waiting and completed task states.
  - The anti-stall task-start limit conflicts with programme continuation.
derived:
  - A backward-compatible additive contract revision avoids migration of existing checkpoints.
unknown:
  - Exact Agent Governance workflow results on the future PR head.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260802-agent-governance-sync.md
validation:
  - command: Agent Governance workflow
    result: NOT_RUN
    evidence: PR not yet opened
blockers:
  - none
next_action: update the shared governance contracts and checkpoint schema
```

## Notes

This is a documentation and governance task. No application, database, deployment or production operation is authorized.
