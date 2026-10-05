---
task_id: OTERYN-20261005-platform-lead-self-execution
governing_issue: 1443
required_reads:
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - OTERYN_PLATFORM_COMPLETION_LEAD
optional_reads: []
---

# OTERYN-20261005-platform-lead-self-execution

## Goal

Governing GitHub Issue: #1443.

Correct `Oteryn: platform lead` so it self-executes eligible Platform work by default and escalates only real architecture, Game-ownership or owner-only boundaries.

## Acceptance criteria

- [x] Canonical prompt defaults to first-line Platform self-execution.
- [x] Valid active Platform ownership is preserved; stale terminal ownership is not treated as live.
- [x] Platform Architecture Review is exception-only for new/conflicting durable decisions.
- [x] Game implementation/proof routes only through the Game Work Coordinator.
- [x] Material Game architecture routes through that coordinator to the Supervising Architect.
- [x] Owner interruption is last resort.
- [x] Short invocation and Documentation IA describe the corrected model.
- [x] Deterministic prompt eval covers self-execution and escalation boundaries.
- [x] Terminal task #1441/#1442 is removed from active task ownership.
- [ ] Exact candidate CI/validation is green.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/evals/oteryn-platform-completion-lead-v2.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/archive/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-lead-self-execution.md
modules:
  - agent-governance
dependencies:
  - KAN-41
  - Oteryn/Oteryn-Game#1622 escalation route
blockers:
  - none
cross_repository_tasks:
  - no Game repository mutation in this task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T09:50:54Z
head: UNKNOWN
branch: docs/platform-lead-self-execution-1443
pr: 1444
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/evals/oteryn-platform-completion-lead-v2.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/archive/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-lead-self-execution.md
proven:
  - Issue #1441 and PR #1442 are terminal.
  - Protected main 7fd0be6eb572a979218961e43c6b98417452b17c contains the original lead prompt.
  - The original prompt defaulted to programme coordination/read-only preparation.
  - The owner explicitly requested that the lead work on Platform problems itself and escalate only when needed.
derived:
  - The reusable lead contract must make self-execution the default while preserving live ownership and Game non-interference.
unknown:
  - exact candidate CI result
conflicts: []
first_failure:
  marker: branch_pr_identity_omitted
  evidence: Agent Governance run 37293744572 rejected the initial candidate because the live open PR #1444 was not yet recorded in the active task packet
rejected_hypotheses:
  - Coordination-only behavior is the intended Platform Lead model.
  - Every Platform problem should be delegated to a specialist alias.
changed_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_COMPLETION_LEAD.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/evals/oteryn-platform-completion-lead-v1.json
  - docs/agents/evals/oteryn-platform-completion-lead-v2.json
  - docs/agents/tasks/active/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/archive/OTERYN-20261005-platform-completion-lead.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-lead-self-execution.md
validation:
  - command: python tools/agents/documentation_ia.py
    result: NOT_RUN
    evidence: exact-candidate validation pending
  - command: python tools/validation/prompt_eval.py --suite docs/agents/evals/oteryn-platform-completion-lead-v2.json
    result: NOT_RUN
    evidence: exact-candidate validation pending
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261005-platform-lead-self-execution.md --require-checkpoint
    result: NOT_RUN
    evidence: exact-candidate validation pending
blockers:
  - none
next_action: archive this task after PR #1444 reaches protected main; do not start new implementation in this task
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository prompt-governance corrective PR
source_branch_evidence: pending protected integration
```

## Notes

Documentation/governance only. Product runtime E2E is `NOT_APPLICABLE` for this prompt-contract correction.
