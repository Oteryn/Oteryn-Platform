---
task_id: OTERYN-20260802-delivery-closeout-v21
required_reads:
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - delivery completeness
  - vertical slice
  - pull request hygiene
optional_reads: []
---

# Delivery completeness and closeout v2.1

## Goal

Require prompt eval discipline, trust boundaries, complete frontend/backend vertical slices, independent audit, real E2E and terminal related-PR state before task completion.

## Policy

```yaml
policy_version: 2
task_kind: integration
implementation_authorized: false
context_pressure: low
context_growth: stable
decomposition_decision: single
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

## Scope

Owned paths:

- `docs/agents/AGENTS.md`
- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`
- `docs/agents/tasks/active/OTERYN-20260802-delivery-closeout-v21.md`

No application, authentication, database, payment, production, Canary or deployment mutation.

## Acceptance

- [x] Backend-only evidence cannot close a user-facing feature when frontend is required.
- [x] Independent audit, real E2E and exact-head CI are mandatory closeout gates.
- [x] Related and superseded PRs must reach terminal states.
- [x] Prompt changes gain eval and trust-boundary discipline.
- [ ] Pass governance and required CI.
- [ ] Merge and archive this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T00:26:00+02:00
head: 2d72363328029b42a04ebb76c43c8afff1765a21
branch: docs/agent-closeout-vertical-slice-v21-20260802
pr: 445
status: validating
phase: validate
session_id: chat-20260802-delivery-closeout-v21
session_role: coordinator
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
context_routes:
  - agent-governance
context_pressure: low
context_growth: stable
decomposition_decision: single
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
last_completed_step: added and routed delivery completeness and closeout contract
owned_paths:
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/tasks/active/OTERYN-20260802-delivery-closeout-v21.md
proven:
  - The contract requires prompt eval discipline, trust boundaries, vertical-slice completeness, independent audit, real E2E, exact-head CI and terminal related PR states.
  - Nested agent instructions require the contract for substantial implementation and closeout.
derived:
  - Backend-only evidence cannot close an Oteryn user-facing feature when frontend integration is required.
unknown:
  - Required exact-head workflow results after checkpoint repair.
conflicts: []
first_failure:
  marker: missing-context-checkpoint
  evidence: Agent Governance run 30721078703 rejected the initial active task because it lacked the required checkpoint section.
rejected_hypotheses:
  - treat worker summary as terminal evidence
  - allow completed tasks with open superseded PRs
changed_paths:
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/tasks/active/OTERYN-20260802-delivery-closeout-v21.md
validation:
  - command: Agent Governance run 30721078703
    result: FAIL
    evidence: first relevant failure was the missing checkpoint; this commit repairs it
blockers: []
next_action: verify exact-head workflows for PR 445
```
