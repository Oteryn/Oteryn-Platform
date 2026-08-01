# OTERYN-20260801 — Autonomous program continuation v2

## Objective

Make one short owner invocation authorize a long, low-noise autonomous programme run that checkpoints safely, completes and archives terminal tasks, crosses barriers, and continues with the next ready work until a real stop condition is reached.

## Scope

Documentation and agent-governance contracts only.

Owned paths:

- `docs/agents/PROMPTING_STANDARD.md`
- `docs/agents/PROMPTING_HANDOVER.md`
- `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md`
- this task record

No application, database, authentication, production, Canary, payment, deployment, or external-repository mutation is authorized.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-01T21:19:00Z
head: ca854ab4f6737532b7d3a2036e33bf680daeabee
branch: docs/autonomous-program-continuation-v2-20260801
pr: 440
status: validating
phase: validate
session_id: chat-20260801-autonomous-v2
session_role: coordinator
execution_mode: chat
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/tasks/active/OTERYN-20260801-autonomous-program-continuation-v2.md
proven:
  - The standard now distinguishes bounded worker sessions from a multi-task owner invocation.
  - The new contract requires terminal task finalization, archival, barrier review, and continuation with the next READY task.
  - The handover routes resolvable short commands into execution rather than returning a prompt.
  - Oteryn repository safety and task-archive requirements remain authoritative.
derived:
  - One short programme command can now drive long foreground work without treating each checkpoint or completed task as an owner-interaction boundary.
unknown:
  - Required exact-head CI result for PR 440.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
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
