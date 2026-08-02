---
task_id: OTERYN-20260802-root-agent-bootstrap-v21
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
search_first:
  - mandatory Codex bootstrap
  - short-command contract
  - delivery completeness
optional_reads: []
---

# Root agent bootstrap v2.1

## Goal

Make the automatically loaded root instruction entry point force all Codex agents to read the full Oteryn governance stack and treat the short autonomous command consistently.

## Policy

```yaml
policy_version: 2
task_kind: documentation
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

- `AGENTS.override.md`
- `docs/agents/tasks/active/OTERYN-20260802-root-agent-bootstrap-v21.md`

No application, authentication, payment, database, production, Canary or deployment mutation.

## Acceptance

- [x] Root bootstrap requires all governing local instruction files.
- [x] Safety and authority boundaries remain more restrictive and authoritative.
- [x] The short autonomous command resolves to the durable foreground coordinator loop.
- [x] Vertical slice, audit, E2E, exact-head CI, PR hygiene and archival remain mandatory.
- [ ] Pass governance and required CI.
- [ ] Merge and archive this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T08:57:00+02:00
head: 2d435c083e20c1b145061edbd1b3026010f36469
branch: docs/root-agent-bootstrap-v21-20260802
pr: null
status: implementing
phase: implementation
session_id: chat-20260802-root-agent-bootstrap-v21
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
last_completed_step: added mandatory root bootstrap
owned_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260802-root-agent-bootstrap-v21.md
proven:
  - The root override routes Codex to the repository and nested instructions plus delivery and autonomous contracts.
derived:
  - A root Codex invocation no longer depends on implicit discovery of nested governance.
unknown:
  - Exact-head workflow results after PR creation.
conflicts: []
first_failure:
  marker: none
  evidence: validation pending
rejected_hypotheses:
  - rely on a long owner prompt for each new agent
changed_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260802-root-agent-bootstrap-v21.md
validation: []
blockers: []
next_action: open the feature PR and verify exact-head workflows
```
