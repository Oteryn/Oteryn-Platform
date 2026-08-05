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

- [x] One canonical immutable scope contract names `blakinio/Oteryn-Platform` as the sole execution and write repository.
- [x] External repositories and systems may be inspected only read-only when directly necessary to verify a Platform-owned boundary.
- [x] The three programme identities cannot accept cross-repository write authorization; such work requires a different programme/task identity.
- [x] The short-command registry and all three programme states require the scope contract.
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
  - docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md
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
updated_at: 2026-08-05T14:34:00Z
head: 84b4fe751ff70014777ea44b0425c68e7000e016
branch: docs/platform-agent-scope-lock-20260805
pr: 545
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
  - docs/agents/tasks/archive/OTERYN-20260805-platform-agent-scope-lock.md
proven:
  - Each canonical programme prompt names blakinio/Oteryn-Platform and restricts writes to it.
  - The mandatory scope contract removes the former separate-authorisation ambiguity for these programme identities.
  - The registry and all three programme states require the same scope contract.
  - Static adversarial evaluation passes 15 of 15 scope-routing cases.
  - PR 545 contains only the declared agent-governance paths.
derived:
  - A separate programme/task identity remains available for separately authorized work without weakening these Platform programmes.
unknown:
  - Exact-head GitHub Actions result for the final PR head.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Labels, programme names or repository metadata alone are not a sufficient immutable authority boundary.
  - Editing only one of the three programme states would leave inconsistent routing.
changed_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
validation:
  - command: static adversarial scope evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md; 15/15 candidate cases pass
  - command: fresh exact-diff review
    result: PASS
    evidence: PR 545 contains only the immutable scope contract, registry bindings, programme-state bindings, evaluation and task checkpoint
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: agent-governance documentation only; no runtime product behavior changed
  - command: exact-head GitHub Actions
    result: NOT_RUN
    evidence: final-head checks pending
blockers:
  - none
next_action: Verify every required GitHub Actions workflow on the exact final PR head, then merge if all gates remain satisfied.
```

## Notes

Runtime E2E is not applicable because this is an agent-governance documentation change; outcome validation verifies routing and authority text on the exact PR head.