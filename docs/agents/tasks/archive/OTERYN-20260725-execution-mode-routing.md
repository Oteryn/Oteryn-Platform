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
- [x] Documentation-only validation and required governance checks passed.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T20:50:00Z
status: completed
branch: docs/OTERYN-20260725-execution-mode-routing
pr: 191
validated_head: 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
merge_sha: 6c3b9fedcd07844315c51f59b7a4f8a3abc557e1
context_routes:
  - agent-governance
proven:
  - docs/agents/EXECUTION_MODE_ROUTING.md defines CHAT, CODEX and WORK
  - deterministic selection, capability verification, escalation, return and budget rules are documented
  - mode selection remains subordinate to repository allowlists, active ownership, security controls and stop conditions
  - mode labels are explicitly not authorization or execution evidence
  - PR 191 changed only the routing document and this bounded governance task
  - PR 191 was squash-merged after all exact-head checks passed
  - no application runtime, migration, deployment, production, infrastructure or external-repository change occurred
validation:
  - command: CI
    result: PASS
    evidence: run 30174171818 on 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
  - command: Agent Governance
    result: PASS
    evidence: run 30174171802 on 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
  - command: Platform DB Outage Validation
    result: PASS
    evidence: run 30174171810 on 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
  - command: Game Auth Ticket Concurrency
    result: PASS
    evidence: run 30174171815 on 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
  - command: Phase 7 Production-Like Validation
    result: PASS
    evidence: run 30174171849 on 8dbb558ef09b16e5c8e93be61207bbc0dfc95ec6
blockers:
  - none
next_action: none
```

## Notes

The missing mandatory-read blocker for the public website programme is resolved on trusted `main`.