---
task_id: OTERYN-20261005-platform-completion-lead
governing_issue: 1441
required_reads:
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - OTERYN_PLATFORM_COMPLETION_LEAD
optional_reads: []
---

# OTERYN-20261005-platform-completion-lead

## Goal

Governing GitHub Issue: #1441.

Make `Oteryn: platform lead` a real reusable technical lead, following the same subordinate-lead pattern used by Oteryn Game #1829: lead the Platform programme, reuse existing Platform ownership, and route Game-owned blockers through the Game Work Coordinator and Supervising Architect rather than taking Game authority.

## Acceptance criteria

- [ ] Alias resolves to one canonical reusable prompt.
- [ ] Programme routing document exists.
- [ ] Lead is explicitly not a second Platform implementation owner or Game control plane.
- [ ] Game implementation blockers route through `OTV2_WORK_DELIVERY_COORDINATOR` / Oteryn-Game #1622.
- [ ] Material Game architecture routes through the Game coordinator to `OTV2_SOL_SUPERVISING_ARCHITECT`.
- [ ] Documentation IA and deterministic prompt eval cover the reusable lead.
- [ ] Fresh-chat invocation reconstructs state from protected main/Jira/task/PR facts.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
modules:
  - agent-governance
dependencies:
  - KAN-41
  - Oteryn/Oteryn-Game#1622 escalation route
blockers:
  - none
cross_repository_tasks:
  - Game is coordination/read-only unless separately authorized; no Game code mutation in this task.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T09:30:00Z
head: UNKNOWN
branch: docs/platform-completion-lead-20261005
pr: 1442
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
proven:
  - PR #1440 registered the short alias on protected main.
  - Oteryn Game PR #1829 defines the desired subordinate technical lead pattern and coordinator-to-architect escalation route.
derived:
  - Platform should mirror that lead pattern while keeping its existing programme ownership.
unknown:
  - exact candidate CI result
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Alias-only registration is sufficient for fresh-session lead reconstruction.
  - Platform Completion Lead should become a second Game or Platform control plane.
changed_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
validation:
  - command: python tools/agents/documentation_ia.py
    result: NOT_RUN
    evidence: exact-candidate validation pending
  - command: python tools/validation/prompt_eval.py --suite docs/agents/evals/oteryn-platform-completion-lead-v1.json
    result: NOT_RUN
    evidence: exact-candidate validation pending
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md --require-checkpoint
    result: NOT_RUN
    evidence: exact-candidate validation pending
blockers:
  - none
next_action: rerun exact-head CI and reconcile the governed PR
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository prompt-governance PR
source_branch_evidence: pending protected integration
```

## Notes

Documentation/governance only. Product runtime E2E is `NOT_APPLICABLE`; the reusable lead itself later requires real cross-repository E2E for claims it coordinates.
