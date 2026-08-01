---
task_id: OTERYN-20260802-agent-quality-closeout-v21
required_reads:
  - AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/AGENT_QUALITY_AND_CLOSEOUT.md
search_first:
  - agent quality closeout
  - vertical slice audit e2e pr hygiene
optional_reads: []
---

# OTERYN-20260802 — Agent quality and closeout v2.1

## Goal

Make outcome-based evals, trust boundaries, full-stack vertical slices, independent audit, real E2E, exact-final-head CI, related-PR cleanup, and terminal task archival mandatory for substantial agent work.

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
feature_scope:
  type: documentation
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
```

## Owned paths

- `docs/agents/AGENT_QUALITY_AND_CLOSEOUT.md`
- `docs/agents/PROMPTING_HANDOVER.md`
- `docs/agents/tasks/active/OTERYN-20260802-agent-quality-closeout-v21.md`

## Acceptance

- [x] Add the normative v2.1 quality and closeout contract.
- [x] Make the handover require it for substantial work.
- [x] Cover prompt evals, trust boundaries, context engineering, outcome verification, acceptance inventory, full-stack vertical slices, audit, E2E, exact-head CI, PR hygiene, archival, and continuation.
- [ ] Pass exact-head required workflows.
- [ ] Merge and archive this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T00:24:00+02:00
head: c66d6b4156162222adf9a5d9a248aff41dd3c1d6
branch: docs/agent-quality-closeout-v21-20260802
pr: 443
status: validating
phase: validate
session_id: chat-20260802-quality-v21
session_role: coordinator
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
owned_paths:
  - docs/agents/AGENT_QUALITY_AND_CLOSEOUT.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/tasks/active/OTERYN-20260802-agent-quality-closeout-v21.md
proven:
  - The v2.1 contract exists and the handover declares it mandatory for substantial phases.
  - PR 443 owns the governance contract, handover integration, and task record.
derived:
  - Future task prompts must treat full-stack completeness, audit, E2E, exact-head CI, PR cleanup, and archival as one completion gate.
unknown:
  - Exact-head workflow results after this PR binding update.
conflicts: []
changed_paths:
  - docs/agents/AGENT_QUALITY_AND_CLOSEOUT.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/tasks/active/OTERYN-20260802-agent-quality-closeout-v21.md
validation: []
blockers: []
next_action: verify exact-head workflows for PR 443, then complete merge and archive gates
```
