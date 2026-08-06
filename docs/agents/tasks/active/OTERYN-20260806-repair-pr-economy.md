---
task_id: OTERYN-20260806-repair-pr-economy
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: ready
base_head: 5c06bb4f1b79459d41e04d9e185e17918b88a948
branch: docs/repair-pr-economy-20260806
implementation_pull_request: 743
---

# OTERYN-20260806-repair-pr-economy

## Goal

Implement the repository-owned repair PR economy, repair-train, audit-role separation and parallel-worker routing contract without changing product/runtime behavior or weakening deterministic Issue locking, exact-head CI, rollback, review or closeout gates.

## Acceptance criteria

- [x] Deterministic `repair/issue-<number>` Git-ref arbitration remains the mandatory atomic Issue lock.
- [x] A valid activated claim no longer universally requires a draft PR.
- [x] Existing authoritative PR reuse and bounded repair-train selection order are normative.
- [x] Repair trains preserve exact accepted source heads, per-Issue provenance, rollback boundaries, freeze and drift rejection.
- [x] Dedicated-PR safety boundaries remain explicit.
- [x] PASS-only audit and lifecycle-only closeout remain compatible with PR #673.
- [x] Exact-target audit handoff, generation invalidation and whole-diff/per-Issue verdicts are machine-readable.
- [x] Parallel workers rotate after durable handoff; a separate audit role drains ready audits.
- [x] Existing platform audit, architecture review and remediation short commands remain valid.
- [x] Claim protocol v3, taxonomy 1.3 and work-item schema 3 are consistent and cross-document drift fails closed.
- [x] Static prompt evaluation covers 32 positive, negative and boundary cases.
- [x] Runtime E2E is `NOT_APPLICABLE` with a concrete governance-only reason.
- [x] `AUDIT-744-001` is remediated.
- [x] The repository owner explicitly waived the external-auditor requirement for terminal closeout; same-session verification is recorded without claiming independence.
- [ ] Required checks pass against the current `main`, PR #743 merges, the task is archived and ownership is released.

## Ownership

```yaml
owned_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
modules:
  - agent-governance
  - remediation-programme
dependencies:
  - PR #673 merged lifecycle closeout batching and PASS-audit artifact policy
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T12:31:00Z
head: 428ba4b35467560e5ac1338452f3c36e393a37ca
branch: docs/repair-pr-economy-20260806
pr: 743
status: ready
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
proven:
  - Candidate claim protocol version 3 preserves deterministic branch arbitration and permits branch-only activation.
  - Candidate adds authoritative PR reuse, bounded repair trains, immutable exact-source-head acceptance, rollback mapping and freeze/drift rejection.
  - Taxonomy version 1.3, claim protocol version 3 and work-item schema version 3 are aligned.
  - Static adversarial evaluation case 32 rejects cross-document protocol and schema drift.
  - Static adversarial policy evaluation records 32 of 32 candidate cases passing with zero safety-critical regressions.
  - Independent audit generation 1 returned AUDIT-744-001 and that finding is remediated.
  - The exact pre-refresh head passed all six required workflows and had zero unresolved review threads.
  - PR #743 contains exactly nine declared governance, task and evidence paths and no product, runtime, workflow, deployment, migration or production mutation.
  - Current main 93635566946729792ffdcb7e6e844cce5c03531a advanced only through disjoint lifecycle closeout paths.
  - The repository owner explicitly directed same-session completion and waived the external-auditor requirement.
  - Review 4874564214 and Issue #744 record OWNER_OVERRIDE_NON_INDEPENDENT_VERIFICATION_PASS without claiming independent-auditor status.
derived:
  - A new branch commit is required to regenerate protected checks against the current PR merge result after main advanced.
unknown:
  - Final workflow results for the refreshed PR head.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Falsely describing the owner-directed verification as an independent audit.
  - Force-updating the branch or bypassing required protected checks.
  - Merging while GitHub reports a required check as expected.
changed_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
validation:
  - command: independent audit generation 1
    result: FAIL
    evidence: Issue #744 and review 4874435874 recorded AUDIT-744-001
  - command: remediation of AUDIT-744-001
    result: PASS
    evidence: taxonomy 1.3, claim protocol 3, work-item schema 3 and static evaluation case 32
  - command: pre-refresh exact-head workflows
    result: PASS
    evidence: six workflows succeeded on 428ba4b35467560e5ac1338452f3c36e393a37ca
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and delivery-routing documentation only
  - command: owner-directed same-session verification
    result: PASS
    evidence: review 4874564214 and Issue #744; explicitly non-independent
blockers:
  - none
next_action: Wait for the refreshed protected checks only as required by GitHub, then live-recheck and merge PR #743, archive the task, close Issue #742 and release ownership.
```

## Notes

This checkpoint refresh exists to regenerate required checks against current `main` after disjoint lifecycle closeout commits advanced the base. It does not expand the nine-path governance-only scope.
