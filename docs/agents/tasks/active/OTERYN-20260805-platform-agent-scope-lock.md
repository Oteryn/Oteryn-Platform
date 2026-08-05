---
task_id: OTERYN-20260805-platform-agent-scope-lock
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - OTERYN_PLATFORM_CONTINUOUS_AUDIT
  - OTERYN_PLATFORM_REMEDIATION
  - OTERYN_PLATFORM_ARCHITECTURE_REVIEW
optional_reads: []
---

# OTERYN-20260805-platform-agent-scope-lock

## Goal

Make the three durable Oteryn Platform programmes permanently repository-scoped so that no invocation, Issue, comment, task, PR, programme state or later owner wording can redirect those programme identities to another repository or product area.

## Acceptance criteria

- [ ] One canonical immutable scope contract names `blakinio/Oteryn-Platform` as the sole execution and write repository.
- [ ] External repositories and systems may be inspected only read-only when directly necessary to verify a Platform-owned boundary.
- [ ] The three programme identities cannot accept cross-repository write authorization; such work requires a different programme/task identity.
- [ ] The short-command registry and all three programme states require the scope contract.
- [ ] Documentation/governance validation and exact-head CI pass.
- [ ] The task is archived and ownership released after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
  - docs/agents/tasks/archive/OTERYN-20260805-platform-agent-scope-lock.md
modules:
  - agent-governance
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
updated_at: 2026-08-05T14:24:00Z
head: 8306a2d79e475e023a69fd3145db5f3c296369b7
branch: docs/platform-agent-scope-lock-20260805
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
  - docs/agents/tasks/archive/OTERYN-20260805-platform-agent-scope-lock.md
proven:
  - Each canonical programme prompt currently names blakinio/Oteryn-Platform and restricts writes to it.
  - Each canonical prompt currently contains a possible separate-authorisation exception, so programme identity is not absolutely immutable.
derived:
  - A shared mandatory higher-restriction scope contract is the smallest non-duplicated correction.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Labels, programme names or repository metadata alone are not a sufficient immutable authority boundary.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation in progress
blockers:
  - none
next_action: Create the immutable programme scope contract and require it from the registry and all three programme states.
```

## Notes

Runtime E2E is not applicable because this is an agent-governance documentation change; outcome validation must verify routing and authority text on the exact PR head.