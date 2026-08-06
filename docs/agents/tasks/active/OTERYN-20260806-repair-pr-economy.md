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
- [x] Prompt evaluation covers positive, negative and boundary cases.
- [x] Runtime E2E is `NOT_APPLICABLE` with a concrete governance-only reason.
- [ ] Fresh independent audit, exact-head required CI, PR hygiene, merge, archival and ownership release are completed.

## Ownership

```yaml
owned_paths:
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
updated_at: 2026-08-06T12:05:00Z
head: 191574415f377b2700c77bd5ff355a17ca523041
branch: docs/repair-pr-economy-20260806
pr: 743
status: ready
context_routes:
  - agent-governance
  - testing
owned_paths:
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
  - Static adversarial policy evaluation records 31 of 31 candidate cases passing with zero safety-critical regressions.
  - PR #743 contains exactly eight declared governance/task/evidence paths and no product/runtime/workflow/deployment paths.
derived:
  - A controlling specialization avoids duplicating complete train and audit schemas across every programme document.
  - Durable handoff plus ROTATE preserves recoverable ownership without holding an active worker slot.
unknown:
  - Required exact-head workflow results for the final branch tip.
  - Independent auditor identity and verdict.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Replacing the deterministic branch lock with labels, comments or assignees.
  - Making repair workers wait for an auditor or train peer.
  - Allowing an implementation worker or integration owner to self-approve final audit.
  - Treating fewer Pull Requests as more important than rollback, review or security boundaries.
changed_paths:
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
  - command: compare 5c06bb4f1b79459d41e04d9e185e17918b88a948...docs/repair-pr-economy-20260806
    result: PASS
    evidence: behind 0 and exactly eight declared governance/task/evidence paths before final checkpoint commit
  - command: static adversarial policy evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md; candidate 31/31 PASS
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and delivery-routing documentation only; no executable runtime or user journey changed
blockers:
  - none
next_action: A distinct eligible agent/session must audit PR #743 in AUDIT ONLY mode on its exact current base and head, recording whole-diff and Issue #742 verdicts without modifying the branch.
```

## Notes

The current implementation session cannot perform the required final independent audit. The exact final branch head and audit handoff are maintained in PR #743 after this self-referential checkpoint commit.
