---
task_id: OTERYN-20260806-repair-pr-economy
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: validating
base_head: 5c06bb4f1b79459d41e04d9e185e17918b88a948
branch: docs/repair-pr-economy-20260806
---

# OTERYN-20260806-repair-pr-economy

## Goal

Implement the repository-owned repair PR economy, repair-train, independent-audit role separation and parallel-worker routing contract discussed with the owner, without changing product/runtime behavior or weakening deterministic Issue locking, exact-head CI, E2E, rollback, review or closeout gates.

## Acceptance criteria

- [x] Deterministic `repair/issue-<number>` Git-ref arbitration remains the mandatory atomic Issue lock.
- [x] A valid activated claim no longer universally requires a draft PR.
- [x] Existing authoritative PR reuse and bounded repair-train selection order are normative.
- [x] Repair trains preserve exact accepted source heads, per-Issue provenance, rollback boundaries, freeze and drift rejection.
- [x] Dedicated-PR safety boundaries remain explicit.
- [x] PASS-only audit and lifecycle-only closeout rules remain compatible with PR #673.
- [x] The implementation owner, train integrator and contributing workers cannot perform the required final independent audit.
- [x] Exact-target audit handoff, audit generation invalidation and whole-diff/per-Issue verdicts are machine-readable.
- [x] Parallel short commands do not leave all repair workers waiting for each other; workers rotate after durable handoff and a separate audit role drains ready audits.
- [x] Existing short commands for platform audit, architecture review and remediation remain valid.
- [x] Prompt-evaluation evidence covers positive, negative and boundary cases, including worker races, train freeze/drift, independent-auditor separation and no wait-to-fill behavior.
- [x] Runtime E2E is recorded as `NOT_APPLICABLE` with a concrete governance-only reason.
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
updated_at: 2026-08-06T12:03:00Z
head: 59071b3d9631815790d50f65027d052a71fec507
branch: docs/repair-pr-economy-20260806
pr: pending
status: validating
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
  - PR #673 is merged and its lifecycle-only batching and PASS-only audit artifact rules are present on main.
  - Baseline claim protocol uses deterministic repair/issue-<number> branch creation as atomic arbitration.
  - Candidate claim protocol version 3 preserves that arbitration while permitting branch-only activation.
  - Candidate adds authoritative PR reuse, bounded repair trains, immutable exact-source-head acceptance, rollback mapping and freeze/drift rejection.
  - Candidate requires an AUDIT ONLY actor distinct from implementation owner, train integrator and all Issue workers.
  - Existing audit, remediation and architecture short commands remain valid.
  - New independent-repair-audit role command and total-slot allocation prevent internal worker waiting.
  - Static adversarial policy evaluation records 31 of 31 candidate cases passing with zero safety-critical regressions.
  - Effective candidate diff is exactly eight declared governance/task/evidence paths and is behind main by zero at pre-PR comparison.
derived:
  - The controlling specialization avoids duplicating full normative train and audit schemas across every programme document.
  - A separate repair-audit role and durable handoff allow implementation workers to return ROTATE without releasing recoverable ownership.
unknown:
  - Pull Request number and exact final candidate head until PR creation.
  - Required exact-head workflow results.
  - Independent auditor identity and verdict.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Replacing the deterministic branch lock with labels, comments or assignees.
  - Making every repair worker wait in an active session for an auditor or train peer.
  - Allowing a train integrator or implementation worker to self-approve the final audit.
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
    evidence: ahead 8, behind 0, exactly eight declared governance/task/evidence paths and no runtime/workflow/deployment paths
  - command: static adversarial policy evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260806-repair-pr-economy/prompt-eval.md; candidate 31/31 PASS, baseline 16 PASS/10 ambiguous/5 FAIL
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and delivery-routing documentation only; no executable runtime or user journey changed
blockers:
  - none
next_action: Open the single implementation Pull Request, verify exact-head CI, then publish a durable handoff for a distinct AUDIT ONLY validator.
```

## Notes

The current task cannot use its own unmerged governance changes to expand authority. Final audit must be performed by a distinct eligible agent/session in `AUDIT ONLY` mode on the exact candidate target.
