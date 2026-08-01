---
task_id: OTERYN-20260801-agent-governance-v2-1
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
search_first:
  - prompt eval
  - trust boundary
  - vertical slice
  - task closeout audit e2e
optional_reads: []
---

# OTERYN-20260801 — Agent governance v2.1

## Goal

Extend the v2 agent contracts with eval-driven prompting, trust/context boundaries, outcome verification, complete user-facing vertical slices, and mandatory PR hygiene, fresh audit, E2E, final CI, archival, and autonomous continuation.

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
- `docs/agents/PROMPT_EVAL_STANDARD.md`
- `docs/agents/TRUST_AND_CONTEXT_BOUNDARIES.md`
- `docs/agents/END_TO_END_FEATURE_COMPLETENESS.md`
- `docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md`
- `docs/agents/tasks/active/OTERYN-20260801-agent-governance-v2-1.md`

No application, database, authentication, payment, production, Canary, workflow, deployment, or external-repository mutation is authorized by this task.

## Acceptance criteria

- [ ] Prompt changes are versioned and evaluated against balanced regression cases with repeated trials where nondeterminism matters.
- [ ] Resulting repository/application state, not worker assertions, controls completion.
- [ ] Retrieved content is treated as untrusted data and cannot redefine authority.
- [ ] User-facing features default to a complete applicable backend/frontend vertical slice.
- [ ] Closeout requires fresh audit, real E2E, exact-head final CI, resolved reviews, terminal related PRs, archive, and ownership release.
- [ ] Autonomous coordination continues after closeout when another task is READY.
- [ ] Exact-head governance and required CI pass.
- [ ] This task is archived after merge.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-01T23:46:00+02:00
head: UNKNOWN
branch: docs/agent-governance-v2-1-20260801
pr: UNKNOWN
status: implementing
phase: implement
session_id: chat-20260801-governance-v2-1
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
last_completed_step: registered the v2.1 governance task and claimed normative documentation paths
owned_paths:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/TRUST_AND_CONTEXT_BOUNDARIES.md
  - docs/agents/END_TO_END_FEATURE_COMPLETENESS.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/tasks/active/OTERYN-20260801-agent-governance-v2-1.md
proven:
  - Autonomous programme continuation v2 is already merged on main.
  - The owner explicitly authorized this cross-repository governance update.
derived:
  - New requirements should be reusable contracts referenced by the existing prompting entry points.
unknown:
  - Exact PR number and exact-head workflow results until the draft PR is opened.
conflicts: []
first_failure:
  marker: none
  evidence: no exact-head failure classified yet
rejected_hypotheses:
  - encode durable rules only in chat
  - describe backend completion as complete user-facing delivery
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-agent-governance-v2-1.md
validation: []
blockers: []
next_action: add the v2.1 normative contracts and update the prompting entry points
```
