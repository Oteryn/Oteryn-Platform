---
task_id: OTERYN-20260806-lifecycle-closeout-batching
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - open lifecycle-only closeout PRs
  - open independent re-audit Issues
  - active tasks overlapping agent governance
optional_reads: []
---

# OTERYN-20260806-lifecycle-closeout-batching

## Goal

Reduce governance-only PR and Issue churn without weakening independent validation by introducing one controlling lifecycle-closeout batching contract for Oteryn Platform audit and remediation programmes.

## Acceptance criteria

- [x] Product/runtime fixes remain one coherent root cause per Issue, branch and PR.
- [x] PASS-only independent audits default to review/comment on the existing target PR and do not create audit PRs.
- [x] Eligible lifecycle-only/archive-only reconciliations may be grouped into one bounded wave PR.
- [x] One fresh independent audit can validate the entire exact batch head with per-item verdicts.
- [x] `ROTATE` is used when an implementer requires a fresh independent validator; `WAITING` remains reserved for real external waiting.
- [x] The short-command registry and audit/remediation programme states require the new contract.
- [ ] Static adversarial evaluation and exact-head required CI pass.
- [ ] The task is archived and ownership released after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/evidence/OTERYN-20260806-lifecycle-closeout-batching/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260806-lifecycle-closeout-batching.md
  - docs/agents/tasks/archive/OTERYN-20260806-lifecycle-closeout-batching.md
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
updated_at: 2026-08-06T08:01:00Z
head: a62fc94eba30375661656531cffe0530a8fe1c1d
branch: docs/lifecycle-closeout-batching-20260806
pr: none
status: validating
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/evidence/OTERYN-20260806-lifecycle-closeout-batching/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260806-lifecycle-closeout-batching.md
  - docs/agents/tasks/archive/OTERYN-20260806-lifecycle-closeout-batching.md
proven:
  - Multiple current open PRs each change exactly three lifecycle-only task paths and each has a separate independent audit Issue.
  - Independent audit is required, but a PASS-only audit can be recorded on the existing target PR without creating another PR.
  - The new contract preserves one-root-cause isolation for product/runtime/security work.
  - Eligible lifecycle reconciliation is bounded to 2–10 items, one coordinator task, one wave PR, one exact-head audit and one CI generation.
  - The registry and both programme states now require the contract.
  - Static adversarial evaluation passes 18 of 18 candidate cases.
derived:
  - This removes the recurring pattern of per-task closeout PR plus per-task audit Issue plus follow-up archive PR while retaining independent falsification.
unknown:
  - Exact-head GitHub Actions result for the final PR head.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Removing independent audit to reduce repository noise.
  - Combining unrelated product or security changes into one PR.
  - Treating an active individually owned closeout PR as safe to absorb without coordination.
changed_paths:
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/evidence/OTERYN-20260806-lifecycle-closeout-batching/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260806-lifecycle-closeout-batching.md
validation:
  - command: static adversarial routing evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260806-lifecycle-closeout-batching/prompt-eval.md; 18/18 candidate cases pass
  - command: compare main@700fa5d0d75a7badd7cb8583d36341c711673942 to task branch
    result: PASS
    evidence: six declared agent-governance files only; no runtime, workflow, architecture, contract or product path
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: agent-governance routing and lifecycle policy only
  - command: exact-head GitHub Actions
    result: NOT_RUN
    evidence: PR not opened yet
blockers:
  - none
next_action: Open the bounded governance PR, perform fresh exact-diff review and verify required checks on its exact final head.
```

## Notes

This task changes agent-governance documentation only. Runtime E2E is not applicable; exact routing, scope, review behavior and lifecycle outcomes must be validated on the final PR head.
