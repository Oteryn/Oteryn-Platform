---
task_id: OTERYN-20260801-autonomous-program-continuation-v2
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
search_first:
  - autonomous program continuation
  - task lifecycle archive
  - active work ownership
optional_reads: []
---

# OTERYN-20260801 — Autonomous program continuation v2

## Goal

Make one short owner invocation authorize a long, low-noise autonomous programme run that checkpoints safely, completes and archives terminal tasks, crosses barriers, and continues with the next ready work until a real stop condition is reached.

## Policy

```yaml
policy_version: 2
task_kind: integration
implementation_authorized: false
context_pressure: medium
context_growth: stable
decomposition_decision: single
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

## Scope

Documentation and agent-governance contracts only.

Owned paths:

- `docs/agents/PROMPTING_STANDARD.md`
- `docs/agents/PROMPTING_HANDOVER.md`
- `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`
- `docs/agents/tasks/active/OTERYN-20260801-autonomous-program-continuation-v2.md`

No application, database, authentication, production, Canary, payment, deployment, or external-repository mutation is authorized.

## Acceptance criteria

- [x] Distinguish one bounded worker session from one long owner invocation.
- [x] Define autonomous continuation until a real stop.
- [x] Require terminal task finalization, archival, ownership release, barrier review, and next-READY continuation.
- [x] Route resolvable short commands into execution instead of returning a long prompt.
- [x] Preserve Oteryn security, cross-repository, production, merge, and task-archive rules.
- [ ] Pass exact-head governance and required CI.
- [ ] Merge and archive this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-01T23:22:00+02:00
head: 42ac8e2914cd1388ae4fd4c16b89d7336efd1a08
branch: docs/autonomous-program-continuation-v2-20260801
pr: 440
status: validating
phase: validate
session_id: chat-20260801-autonomous-v2
session_role: coordinator
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
context_routes:
  - agent-governance
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
last_completed_step: added normative autonomous programme loop and short-command execution semantics
owned_paths:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/tasks/active/OTERYN-20260801-autonomous-program-continuation-v2.md
proven:
  - The standard distinguishes bounded worker sessions from a multi-task owner invocation.
  - The autonomous contract requires terminal task finalization, archival, barrier review, and continuation with the next READY task.
  - The handover routes resolvable short commands into execution rather than returning a prompt.
  - Oteryn repository safety and task-archive requirements remain authoritative.
derived:
  - One short programme command can drive long foreground work without treating each checkpoint or completed task as an owner-interaction boundary.
unknown:
  - Required exact-head governance and CI results for PR 440 after front-matter normalization.
conflicts: []
first_failure:
  marker: none
  evidence: no exact-head failure has been classified on the normalized task head
rejected_hypotheses:
  - weaken worker stop conditions to obtain long programme continuation
  - treat checkpoints as mandatory pauses
  - claim hidden background execution after the final response
changed_paths:
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/tasks/active/OTERYN-20260801-autonomous-program-continuation-v2.md
validation:
  - command: compare main...docs/autonomous-program-continuation-v2-20260801
    result: PASS
    evidence: four authorized documentation/governance paths only
blockers: []
next_action: verify required exact-head checks for PR 440 and complete the repository merge gate
```
