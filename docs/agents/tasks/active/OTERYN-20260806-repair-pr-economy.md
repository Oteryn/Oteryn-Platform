---
task_id: OTERYN-20260806-repair-pr-economy
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: validating
base_head: 5c06bb4f1b79459d41e04d9e185e17918b88a948
branch: docs/repair-pr-economy-20260806
implementation_pull_request: 743
---

# OTERYN-20260806-repair-pr-economy

## Goal

Implement the repository-owned repair PR economy, repair-train, independent-audit role separation and parallel-worker routing contract without changing product/runtime behavior or weakening deterministic Issue locking, exact-head CI, E2E, rollback, review or closeout gates.

## Acceptance criteria

- [x] Deterministic `repair/issue-<number>` Git-ref arbitration remains the mandatory atomic Issue lock.
- [x] A valid activated claim no longer universally requires a draft PR.
- [x] Existing authoritative PR reuse and bounded repair-train selection order are normative.
- [x] Repair trains preserve exact accepted source heads, per-Issue provenance, rollback boundaries, freeze and drift rejection.
- [x] Dedicated-PR safety boundaries remain explicit.
- [x] PASS-only audit and lifecycle-only closeout remain compatible with PR #673.
- [x] Implementation owners, train integrators and contributing workers cannot perform the required final independent audit.
- [x] Exact-target audit handoff, generation invalidation and whole-diff/per-Issue verdicts are machine-readable.
- [x] Parallel workers rotate after durable handoff; a separate audit role drains ready audits.
- [x] Existing platform audit, architecture review and remediation short commands remain valid.
- [x] Claim protocol v3, taxonomy 1.3 and work-item schema 3 are consistent and cross-document drift fails closed.
- [x] Prompt evaluation covers positive, negative and boundary cases, including `AUDIT-744-001`.
- [x] Runtime E2E is `NOT_APPLICABLE` with a concrete governance-only reason.
- [ ] Fresh independent audit generation 2, exact-head required CI, PR hygiene, merge, archival and ownership release are completed.

## Ownership

```yaml
owned_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
modules:
  - agent-governance
  - remediation-programme
dependencies:
  - PR #673 merged lifecycle closeout batching and PASS-audit artifact policy
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md protocol version 2 baseline
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T12:17:00Z
head_before_checkpoint_commit: 0fa864e62ac0e20f8031db6edfd951940d7ac4bd
branch: docs/repair-pr-economy-20260806
pr: 743
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
proven:
  - Main head at task start is 5c06bb4f1b79459d41e04d9e185e17918b88a948.
  - PR #673 is merged and its lifecycle-only batching and PASS-only audit artifact rules are preserved.
  - Candidate claim protocol version 3 preserves deterministic branch arbitration and permits branch-only activation.
  - Candidate adds authoritative PR reuse, bounded repair trains, immutable exact-source-head acceptance, rollback mapping and freeze/drift rejection.
  - Candidate requires an AUDIT ONLY actor distinct from the implementation owner, integration owner and all Issue workers.
  - Existing audit, remediation and architecture short commands remain valid.
  - New independent-repair-audit role and total-slot allocation prevent internal worker waiting.
  - Independent audit generation 1 returned material finding AUDIT-744-001 on exact head 1d8e7d0d40a662b964d852a6a29769efeee5ab69.
  - AUDIT-744-001 is remediated by taxonomy 1.3, claim protocol v3/work-item schema v3 alignment and explicit cross-document drift fail-closed rules.
  - Static adversarial policy evaluation now records 32 of 32 candidate cases passing with zero safety-critical regressions; repeated model trials remain NOT_RUN.
  - PR #743 now contains exactly nine declared governance/task/evidence paths and no product/runtime/workflow/deployment paths.
derived:
  - A controlling specialization avoids duplicating complete train and audit schemas across every programme document.
  - Durable handoff plus ROTATE preserves recoverable ownership without holding an active worker slot.
unknown:
  - Required exact-head workflow results for the post-remediation checkpoint head.
  - Independent auditor identity and generation 2 verdict.
conflicts: []
first_failure:
  marker: AUDIT-744-001
  evidence: docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md declared protocol v2 while the candidate governing claim protocol was v3
resolved_audit_findings:
  - AUDIT-744-001 remediated pending exact-head validation and generation 2 reaudit
  - taxonomy_version 1.3 and claim_protocol version 3 aligned
  - oteryn_work_item schema_version 3 with delivery_state and optional pull_request metadata
  - static evaluation case 32 added
rejected_hypotheses:
  - Replacing the deterministic branch lock with labels, comments or assignees.
  - Making repair workers wait for an auditor or train peer.
  - Allowing an implementation worker or integration owner to self-approve final audit.
  - Treating fewer Pull Requests as more important than rollback, review or security boundaries.
  - Leaving taxonomy at v2 while relying on document precedence to conceal the contradiction.
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
  - command: live governance preflight
    result: PASS
    evidence: current main, PR #673, active tasks, related open PRs and controlling contracts inspected
  - command: independent audit generation 1
    result: FINDING
    evidence: Issue #744 and review 4874435874; AUDIT-744-001
  - command: static adversarial policy evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md; candidate 32/32 PASS
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and delivery-routing documentation only; no executable runtime or user journey changed
blockers:
  - none
next_action: Verify all required workflows on the exact post-checkpoint PR head; if successful, publish the exact generation 2 audit handoff and rotate to a distinct AUDIT ONLY session.
```

## Notes

The session that remediated `AUDIT-744-001` is an implementation session and cannot perform the required generation 2 independent audit. The exact final head and audit handoff are maintained in PR #743 after the checkpoint commit.
