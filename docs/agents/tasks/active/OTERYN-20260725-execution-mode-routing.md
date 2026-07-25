---
task_id: OTERYN-20260725-execution-mode-routing
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
search_first:
  - existing execution-mode routing document or equivalent mode terminology
  - active tasks and open pull requests touching agent governance paths
optional_reads:
  - docs/agents/GOVERNANCE_CONTRACT.json
---

# OTERYN-20260725-execution-mode-routing

## Goal

Add the missing repository-specific execution-mode routing policy required by the public website programme and future autonomous tasks, without changing application runtime, permissions, deployment behavior or external repositories.

## Acceptance criteria

- [ ] `docs/agents/EXECUTION_MODE_ROUTING.md` defines the supported `CHAT`, `CODEX` and `WORK` modes.
- [ ] Mode selection, escalation and return rules are deterministic and subordinate to repository allowlists and stop conditions.
- [ ] The standard routing fields used in task prompts are defined.
- [ ] The policy forbids using mode selection as authorization or as evidence that execution occurred.
- [ ] Documentation-only validation and required governance checks pass on the exact PR head.

## Ownership

```yaml
owned_paths:
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260725-execution-mode-routing.md
modules:
  - AgentGovernance
dependencies:
  - AGENTS.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T19:45:00Z
head: f6a5086ca1a3d291d0ba70f91d9229f4282d190a
branch: docs/OTERYN-20260725-execution-mode-routing
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260725-execution-mode-routing.md
proven:
  - trusted main is f6a5086ca1a3d291d0ba70f91d9229f4282d190a
  - docs/agents/EXECUTION_MODE_ROUTING.md is absent from trusted main
  - repository and organization searches found no existing equivalent document
  - public website programme task requires the missing document before substantive implementation
  - no open PR owns the selected paths
  - no write outside blakinio/Oteryn-Platform is authorized or required
derived:
  - a bounded documentation-only governance PR is sufficient to resolve the missing-read blocker
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - copy an equivalent document from another repository: repository and organization searches returned no result
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260725-execution-mode-routing.md
validation:
  - command: repository overlap and equivalent-policy search
    result: PASS
    evidence: no matching file or overlapping governance PR found
  - command: documentation and governance checks
    result: NOT_RUN
    evidence: implementation not yet complete
blockers:
  - none
next_action: Create docs/agents/EXECUTION_MODE_ROUTING.md with deterministic mode, escalation, return and safety rules.
```

## Notes

This task changes documentation and agent routing policy only. It does not grant repository, infrastructure, production or external-system permissions.