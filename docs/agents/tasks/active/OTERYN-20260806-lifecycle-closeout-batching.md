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
- [ ] Fresh independent audit and exact-head required CI pass on the current-main-synchronized final checkpoint head.
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
  - fresh independent re-audit after current-main synchronization
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T10:28:00Z
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
  - Exact head c6563ec1ec1b6d5f79174f5e5aa3ff90d80c58df passed all six emitted workflows and independent audit Issue 719 recorded PASS_ZERO_MATERIAL_FINDINGS with zero unresolved review threads.
  - Protected main advanced to 1919f7eb55f6c2a08058652f422b47f841467009 through unrelated lifecycle and architecture documentation after that audit.
  - Merge commit e229fcac2912cca62a0c5bf528018dc74778b635 overlays exactly the six declared lifecycle-batching paths on current main and preserves the unrelated main changes.
derived:
  - Live-query-derived UNKNOWN state is safer than a stale empty snapshot and prevents duplicate dispatch or hidden ownership.
  - This removes the recurring pattern of per-task closeout PR plus per-task audit Issue plus follow-up archive PR while retaining independent falsification.
  - The earlier PASS on c6563ec1ec1b6d5f79174f5e5aa3ff90d80c58df remains supporting evidence only because current-main synchronization changed the exact head.
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
  - Merging an exact-head audit result after protected main advanced without synchronizing and re-auditing.
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
  - command: corrected exact-head GitHub Actions and independent audit on c6563ec1ec1b6d5f79174f5e5aa3ff90d80c58df
    result: PASS
    evidence: all six emitted workflows succeeded; Issue 719 and PR review recorded PASS_ZERO_MATERIAL_FINDINGS; review threads zero
  - command: current-main overlay tree
    result: PASS
    evidence: merge commit e229fcac2912cca62a0c5bf528018dc74778b635 uses current main 1919f7eb55f6c2a08058652f422b47f841467009 as second parent and overlays only the six declared governance blobs
  - command: final checkpoint-head GitHub Actions
    result: NOT_RUN
    evidence: pending exact head created by this checkpoint-only commit
  - command: fresh independent re-audit
    result: NOT_RUN
    evidence: must be performed by a separate validator role on the final immutable PR head
blockers:
  - fresh independent validator required before merge
next_action: Resolve the exact live PR 673 head, update the audit target to that immutable SHA, observe exact-head checks, then a fresh audit-only session must record PASS_ZERO_MATERIAL_FINDINGS or exact findings without modifying the implementation branch.
```

## Notes

This task changes agent-governance documentation only. Runtime E2E is not applicable; exact routing, scope, live-state semantics, review behavior and lifecycle outcomes must be validated on the final PR head.
