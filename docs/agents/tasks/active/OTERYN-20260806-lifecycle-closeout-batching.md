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
- [ ] Fresh independent audit and exact-head required CI pass on the final checkpoint head.
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
  - fresh independent re-audit of final PR 673 head through Issue 719
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:13:00Z
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
  - Multiple historical closeout PRs each changed only lifecycle task paths and generated separate audit Issues, creating avoidable governance churn.
  - Independent audit is required, but a PASS-only audit can be recorded on the existing target PR without creating another PR.
  - The new contract preserves one-root-cause isolation for product/runtime/security work.
  - Eligible lifecycle reconciliation is bounded to 2–10 items, one coordinator task, one wave PR, one exact-head audit and one CI generation.
  - The registry and both programme states require the contract.
  - Static adversarial evaluation passes 18 of 18 candidate cases.
  - Implementer exact-diff preflight found no material scope or safety defect in the batching contract.
  - Independent audit Issue 674 reported material finding LCB-AUDIT-01 on 7da00538239f633d993497cb454c9ceba1d3ef85: refreshed programme files preserved false exhaustive mutable queue snapshots.
  - The remediation programme now marks active claims, tasks, pull requests and coordination keys as live-query-required UNKNOWN rather than false empty arrays.
  - The continuous-audit programme now separates its historical finding identity ledger from mutable live queue state and removes the resolved Issue 547 blocker contradiction.
  - PRs 598, 601 and 670 are terminal merged closeout work and are no longer represented as active remediation claims.
  - Candidate head 77f2eadda9afc56f2feca72a19d420a9eb2ee434 included protected main d12a4f4a14db0319a8563cb16b1d92a7b1e117b8, was behind_by 0 and changed exactly six declared governance paths.
  - All six emitted workflows passed on candidate head 77f2eadda9afc56f2feca72a19d420a9eb2ee434: Agent Governance 31092282515, CI 31092283076, Edge 31092280833, Phase 7 31092280883, Game Auth Concurrency 31092281683 and DB Outage 31092280506.
  - Fresh audit-only rotation target is Issue 719 and the implementation branch remains read-only for that validator.
derived:
  - Live-query-derived UNKNOWN state is safer than a stale empty snapshot and prevents duplicate dispatch or hidden ownership.
  - This removes the recurring pattern of per-task closeout PR plus per-task audit Issue plus follow-up archive PR while retaining independent falsification.
unknown:
  - Exact final SHA created by this checkpoint-only commit; resolve from live PR 673.
  - Exact-head workflow result for that final checkpoint head.
  - Fresh independent re-audit conclusion for the final immutable PR head.
conflicts: []
first_failure:
  marker: LCB-AUDIT-01
  evidence: PR 673 independent audit comment and closed audit Issue 674 on exact head 7da00538239f633d993497cb454c9ceba1d3ef85
rejected_hypotheses:
  - Removing independent audit to reduce repository noise.
  - Combining unrelated product or security changes into one PR.
  - Treating an active individually owned closeout PR as safe to absorb without coordination.
  - Preserving mutable queue arrays as empty without same-generation live repository evidence.
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
  - command: implementer exact-diff preflight
    result: PASS
    evidence: batching remains narrowly gated; active ownership cannot be absorbed; material/security work remains separate
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: agent-governance routing and lifecycle policy only
  - command: independent audit on 7da00538239f633d993497cb454c9ceba1d3ef85
    result: FAIL
    evidence: LCB-AUDIT-01; false durable programme queue state
  - command: corrected exact-head GitHub Actions on 77f2eadda9afc56f2feca72a19d420a9eb2ee434
    result: PASS
    evidence: all six emitted workflows succeeded; review threads zero
  - command: final checkpoint-head GitHub Actions
    result: NOT_RUN
    evidence: pending exact head created by this checkpoint-only commit
  - command: fresh independent re-audit
    result: NOT_RUN
    evidence: Issue 719; must be performed by a separate validator role on the final immutable PR head
blockers:
  - fresh independent validator required before merge
next_action: Update Issue 719 and PR 673 to the exact live head created by this checkpoint commit, observe one aggregate exact-head CI snapshot, then a fresh audit-only session must validate that immutable head and record PASS_ZERO_MATERIAL_FINDINGS or exact findings without modifying the implementation branch.
```

## Notes

This task changes agent-governance documentation only. Runtime E2E is not applicable; exact routing, scope, live-state semantics, review behavior and lifecycle outcomes must be validated on the final PR head.
