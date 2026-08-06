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
- [x] Static adversarial evaluation passes.
- [ ] Fresh independent audit and exact-head required CI pass.
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
  - fresh independent audit of PR 673
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T08:05:00Z
head: resolved-from-live-pr-673
branch: docs/lifecycle-closeout-batching-20260806
pr: 673
status: ready
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
  - Initial exact-head workflows all passed on 40f9da479728e0ec475e07684a208fbe5e7499c7.
  - Implementer exact-diff preflight found no material scope or safety defect in PR 673.
derived:
  - This removes the recurring pattern of per-task closeout PR plus per-task audit Issue plus follow-up archive PR while retaining independent falsification.
unknown:
  - Fresh independent audit conclusion for the final immutable PR head.
  - Exact-head workflow result for this checkpoint generation.
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
  - command: compare main@700fa5d0d75a7badd7cb8583d36341c711673942 to PR 673
    result: PASS
    evidence: six declared agent-governance files only; no runtime, workflow, architecture, contract or product path
  - command: implementer exact-diff preflight
    result: PASS
    evidence: batching is narrowly gated; active ownership cannot be absorbed; material/security work remains separate
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: agent-governance routing and lifecycle policy only
  - command: initial exact-head GitHub Actions on 40f9da479728e0ec475e07684a208fbe5e7499c7
    result: PASS
    evidence: CI 31082813694, Agent Governance 31082812882, Edge 31082814016, DB Outage 31082811405, Phase 7 31082812984 and concurrency 31082810330 succeeded
  - command: fresh independent audit
    result: NOT_RUN
    evidence: must be performed by a separate session on the final immutable PR head
blockers:
  - fresh independent validator required before merge
next_action: A fresh audit-only session must inspect PR 673 at its exact final SHA, submit a review/comment with PASS_ZERO_MATERIAL_FINDINGS or exact findings, and close the linked audit Issue; merge only after that result and final exact-head CI pass.
```

## Notes

This task changes agent-governance documentation only. Runtime E2E is not applicable; exact routing, scope, review behavior and lifecycle outcomes must be validated on the final PR head.
