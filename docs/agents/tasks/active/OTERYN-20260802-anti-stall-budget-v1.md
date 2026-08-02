---
task_id: OTERYN-20260802-anti-stall-budget-v1
status: validating
branch: docs/anti-stall-budget-v1-20260802
base_branch: main
created: 2026-08-02T10:29:00+02:00
updated: 2026-08-02T10:49:00+02:00
feature_pr: "449"
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/tasks/active/OTERYN-20260802-anti-stall-budget-v1.md
---

# Anti-stall and execution budget v1

## Goal

Prevent autonomous agents from becoming unbounded polling, retry, repair, or task-selection loops while preserving platform safety and durable state.

## Acceptance

- [x] Add the normative anti-stall contract.
- [x] Require it from the automatically loaded root bootstrap.
- [x] Route local execution through it.
- [x] Limit CI checks, unchanged states, identical failures, repair cycles, context reconstruction, command duration, runtime and no-progress time.
- [ ] Pass exact-head governance and required CI.
- [ ] Merge and archive.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T10:49:00+02:00
head: 7d53ad691a6c40adeef90cc030f50359264fb293
branch: docs/anti-stall-budget-v1-20260802
pr: 449
status: validating
phase: validate
session_id: chat-20260802-anti-stall-budget-v1
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
last_completed_step: finalized validation command schema
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/tasks/active/OTERYN-20260802-anti-stall-budget-v1.md
proven:
  - Root and local agent routing require the anti-stall contract.
  - The contract bounds runtime, no-progress, CI checks, retries, repair cycles, context reconstruction and command duration.
  - All non-governance required workflows passed on the prior implementation head.
derived:
  - Pending platform workflows can no longer justify indefinite polling.
unknown:
  - Exact-head workflow outcome after final checkpoint schema repair.
conflicts: []
first_failure:
  marker: validation-command-required
  evidence: Agent Governance run 3955 required a command field in every validation item.
rejected_hypotheses:
  - the anti-stall contract content caused the governance failure
  - a check field is accepted in place of command
changed_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/tasks/active/OTERYN-20260802-anti-stall-budget-v1.md
validation:
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance run 3955 validator-test step
  - command: GitHub Actions required platform workflows
    result: PASS
    evidence: CI 4227; Phase 7 3229; DB Outage 3156; Edge Security 1650; Game Auth 2727
blockers: []
invocation_started_at: 2026-08-02T10:29:00+02:00
last_progress_at: 2026-08-02T10:49:00+02:00
runtime_limit_minutes: 60
no_progress_minutes: 15
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 3
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: verify final exact-head checks for PR 449; block without further repair if governance fails again
```
