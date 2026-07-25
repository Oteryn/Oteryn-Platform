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

- [x] `docs/agents/EXECUTION_MODE_ROUTING.md` defines the supported `CHAT`, `CODEX` and `WORK` modes.
- [x] Mode selection, escalation and return rules are deterministic and subordinate to repository allowlists and stop conditions.
- [x] The standard routing fields used in task prompts are defined.
- [x] The policy forbids using mode selection as authorization or as evidence that execution occurred.
- [x] Documentation-only validation and required governance checks pass on the exact implementation head.

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
updated_at: 2026-07-25T20:47:00Z
head: 050c17ef6437dc702f678c9cb6bbf32ef414087c
branch: docs/OTERYN-20260725-execution-mode-routing
pr: 191
status: ready
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260725-execution-mode-routing.md
proven:
  - trusted base main is f6a5086ca1a3d291d0ba70f91d9229f4282d190a
  - docs/agents/EXECUTION_MODE_ROUTING.md was absent from trusted main
  - repository and organization searches found no existing equivalent document
  - public website programme task requires the missing document before substantive implementation
  - no open PR owned the selected paths before this task
  - execution mode routing now defines CHAT, CODEX and WORK
  - routing fields, deterministic selection, capability checks, escalation and return rules are documented
  - repository allowlists, task ownership, security controls and stop conditions remain higher priority than mode selection
  - mode labels are explicitly not authorization or execution evidence
  - PR 191 changes only the two declared governance paths
  - exact implementation head 050c17ef6437dc702f678c9cb6bbf32ef414087c passed all required triggered workflows
  - no write outside blakinio/Oteryn-Platform occurred
derived:
  - merging PR 191 will resolve the missing mandatory-read blocker recorded by the public website programme task
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: all triggered workflows passed on exact implementation head 050c17ef6437dc702f678c9cb6bbf32ef414087c
rejected_hypotheses:
  - copy an equivalent document from another repository: repository and organization searches returned no result
  - mode selection can grant otherwise unavailable permissions: contradicted by the new policy and AGENTS.md
changed_paths:
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260725-execution-mode-routing.md
validation:
  - command: repository overlap and equivalent-policy search
    result: PASS
    evidence: no matching file or overlapping governance PR found
  - command: focused documentation review against AGENTS.md, CONTEXT_ROUTING.md, BUILD_TEST_MATRIX.md and CONTEXT_HANDOFF.md
    result: PASS
    evidence: routing policy is subordinate to existing allowlist, evidence, validation and checkpoint rules
  - command: CI
    result: PASS
    evidence: run 30174083776 on 050c17ef6437dc702f678c9cb6bbf32ef414087c
  - command: Agent Governance
    result: PASS
    evidence: run 30174083786 on 050c17ef6437dc702f678c9cb6bbf32ef414087c
  - command: Platform DB Outage Validation
    result: PASS
    evidence: run 30174083773 on 050c17ef6437dc702f678c9cb6bbf32ef414087c
  - command: Game Auth Ticket Concurrency
    result: PASS
    evidence: run 30174083785 on 050c17ef6437dc702f678c9cb6bbf32ef414087c
  - command: Phase 7 Production-Like Validation
    result: PASS
    evidence: run 30174083800 on 050c17ef6437dc702f678c9cb6bbf32ef414087c
  - command: exact-head final checkpoint validation
    result: NOT_RUN
    evidence: this document-only checkpoint commit triggers the final authoritative pull-request workflows
blockers:
  - none
next_action: Validate the final document-only head, mark PR 191 ready and squash-merge.
```

## Notes

This task changes documentation and agent routing policy only. It does not grant repository, infrastructure, production or external-system permissions.