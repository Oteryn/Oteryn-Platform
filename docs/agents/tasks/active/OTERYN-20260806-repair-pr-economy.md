---
task_id: OTERYN-20260806-repair-pr-economy
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: implementing
base_head: 5c06bb4f1b79459d41e04d9e185e17918b88a948
branch: docs/repair-pr-economy-20260806
---

# OTERYN-20260806-repair-pr-economy

## Goal

Implement the repository-owned repair PR economy, repair-train, independent-audit role separation and parallel-worker routing contract discussed with the owner, without changing product/runtime behavior or weakening deterministic Issue locking, exact-head CI, E2E, rollback, review or closeout gates.

## Acceptance criteria

- [ ] Deterministic `repair/issue-<number>` Git-ref arbitration remains the mandatory atomic Issue lock.
- [ ] A valid activated claim no longer universally requires a draft PR.
- [ ] Existing authoritative PR reuse and bounded repair-train selection order are normative.
- [ ] Repair trains preserve exact accepted source heads, per-Issue provenance, rollback boundaries, freeze and drift rejection.
- [ ] Dedicated-PR safety boundaries remain explicit.
- [ ] PASS-only audit and lifecycle-only closeout rules remain compatible with PR #673.
- [ ] The implementation owner, train integrator and contributing workers cannot perform the required final independent audit.
- [ ] Exact-target audit handoff, audit generation invalidation and whole-diff/per-Issue verdicts are machine-readable.
- [ ] Parallel short commands do not leave all repair workers waiting for each other; workers rotate after durable handoff and a separate audit role drains ready audits.
- [ ] Existing short commands for platform audit, architecture review and remediation remain valid.
- [ ] Prompt-evaluation evidence covers positive, negative and boundary cases, including worker races, train freeze/drift, independent-auditor separation and no wait-to-fill behavior.
- [ ] Runtime E2E is recorded as `NOT_APPLICABLE` with a concrete governance-only reason.
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
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md protocol version 2
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T11:45:00Z
head: 5c06bb4f1b79459d41e04d9e185e17918b88a948
branch: docs/repair-pr-economy-20260806
pr: none
status: implementing
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
  - Claim protocol version 2 uses deterministic repair/issue-<number> branch creation as atomic arbitration.
  - Current claim activation universally requires a draft PR and therefore needs correction.
  - Current short commands preserve the three owner-facing programme invocations.
derived:
  - A separate repair-audit role and durable handoff are required so parallel repair workers rotate instead of waiting.
  - A controlling specialization can minimize duplicated edits across governance documents.
unknown:
  - Final PR number and exact final head.
  - Independent auditor identity and audit generation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Replacing the deterministic branch lock with labels, comments or assignees.
  - Making every repair worker wait in an active session for an auditor or train peer.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260806-repair-pr-economy.md
validation:
  - command: live governance preflight
    result: PASS
    evidence: main, PR #673, active tasks, open related PRs and controlling contracts inspected
  - command: runtime E2E classification
    result: NOT_APPLICABLE
    evidence: repository agent-governance and delivery-routing documentation only; no runtime or user journey changes
blockers:
  - none
next_action: Add the controlling repair PR economy contract and integrate the minimum required routing and claim-policy changes.
```

## Notes

The current task cannot use its own unmerged governance changes to expand authority. Final audit must be performed by a distinct eligible agent/session in `AUDIT ONLY` mode on the exact candidate target.