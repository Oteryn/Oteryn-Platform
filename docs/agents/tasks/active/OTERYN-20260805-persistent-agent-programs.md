---
task_id: OTERYN-20260805-persistent-agent-programs
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/TRUST_AND_CONTEXT_BOUNDARIES.md
  - docs/agents/END_TO_END_FEATURE_COMPLETENESS.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
search_first:
  - existing audit, remediation, architecture and short-command prompts
  - active tasks and open pull requests touching docs/agents/prompts or programme registries
optional_reads:
  - docs/agents/PROMPT_EVAL_STANDARD.md
---

# OTERYN-20260805-persistent-agent-programs

## Goal

Create three durable, non-overlapping Oteryn Platform agent programmes for continuous product audit, audit remediation, and architecture/structure/CI review, plus short owner commands and repository-backed continuation state.

## Acceptance criteria

- [ ] Three versioned programme prompts exist and follow the current prompting, trust, completeness, audit, E2E, closeout and anti-stall contracts.
- [ ] Auditor, remediator and architecture adviser have explicit non-overlapping mutation authority.
- [ ] Each programme has a durable state record that allows continuation without chat history.
- [ ] One short-command registry maps Polish owner invocations to the canonical prompt and state files.
- [ ] Missing modules found by the auditor produce a confirmed Issue and a documentation/contract proposal PR, while runtime implementation remains owned by remediation.
- [ ] Documentation paths and internal references are verified on the exact final head.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
modules:
  - agent-governance
  - programme-coordination
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T13:30:00Z
head: UNKNOWN
branch: docs/persistent-agent-programs-20260805
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
proven:
  - Current prompting standard supports short programme invocations resolved from live repository state.
  - Existing audit and remediation prompts are narrower and contain historical state, so new durable programme contracts are required.
derived:
  - Separate immutable prompts and mutable programme-state records prevent continuation state from corrupting the behavioural contract.
unknown:
  - Exact documentation CI checks generated for the final branch head.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reusing only the two historical prompts would not cover the requested continuous whole-platform scope or architecture/CI adviser.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-persistent-agent-programs.md
validation:
  - command: documentation reference and live-path review
    result: NOT_RUN
    evidence: programme files are still being authored
blockers:
  - none
next_action: Create the three programme prompts, mutable state records and short-command registry.
```

## Notes

This is documentation and agent-governance work. Runtime E2E is not applicable; final validation must still verify exact paths, references, role separation, continuation semantics and PR hygiene.