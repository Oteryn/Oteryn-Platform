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

- [ ] Product/runtime fixes remain one coherent root cause per Issue, branch and PR.
- [ ] PASS-only independent audits default to review/comment on the existing target PR and do not create audit PRs.
- [ ] Eligible lifecycle-only/archive-only reconciliations may be grouped into one bounded wave PR.
- [ ] One fresh independent audit can validate the entire exact batch head with per-item verdicts.
- [ ] `ROTATE` is used when an implementer requires a fresh independent validator; `WAITING` remains reserved for real external waiting.
- [ ] The short-command registry and audit/remediation programme states require the new contract.
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
updated_at: 2026-08-06T07:50:00Z
head: 700fa5d0d75a7badd7cb8583d36341c711673942
branch: docs/lifecycle-closeout-batching-20260806
pr: none
status: implementing
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
  - Independent audit is required, but the existing contracts do not require PASS-only audit work to create a separate PR.
  - No open PR was found that already modifies the selected governance contracts for lifecycle batching.
derived:
  - A controlling specialization can reduce PR and CI multiplication while preserving exact-head independent review.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Removing independent audit to reduce repository noise.
  - Combining unrelated product or security changes into one PR.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260806-lifecycle-closeout-batching.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation in progress
blockers:
  - none
next_action: Add the controlling batching contract and bind it into the audit/remediation short-command routing.
```

## Notes

This task changes agent-governance documentation only. Runtime E2E is not applicable; exact routing, scope, review behavior and lifecycle outcomes must be validated on the final PR head.
