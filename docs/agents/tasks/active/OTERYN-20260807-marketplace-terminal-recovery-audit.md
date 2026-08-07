---
task_id: OTERYN-20260807-marketplace-terminal-recovery-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
---

# OTERYN-20260807 marketplace terminal-recovery audit

## Goal

Audit whether Character Bazaar reconciliation and recovery preserve terminal auction truth under retries, concurrent workers and failures across the Canary ownership-transfer / Platform wallet-settlement boundary.

## Acceptance criteria

- [x] Current main, active tasks, open PRs and live remediation ownership were refreshed.
- [x] Marketplace settlement, cancellation, recovery, Canary transfer and relevant tests were inspected.
- [x] Canary transfer duplicate/race safety was verified and the duplicate-transfer hypothesis rejected.
- [x] Terminal-state regression through the generic recovery fallback was proven.
- [x] Open and closed Issues were searched for the same root cause.
- [x] One material finding was routed to Issue #804 as `OPA-REC-0001`.
- [ ] Exact-head documentation/governance CI passes, the audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/reports/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/evidence/OTERYN-20260807-marketplace-terminal-recovery-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - Character Bazaar reconciliation/recovery audit records only
dependencies:
  - Issue #804 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T11:21:00Z
head: 751d2a56e60586764483932f9a4332f25d64af2e
branch: audit/OTERYN-20260807-marketplace-terminal-recovery
pr: 805
status: validating
context_routes:
  - continuous-audit
  - marketplace
  - architecture-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/reports/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/evidence/OTERYN-20260807-marketplace-terminal-recovery-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Issue #804 records OPA-REC-0001 as a high-risk P1 proven Character Bazaar terminal-state recovery defect.
  - Marketplace runtime evidence was inspected on main 1ab8d90be35745f8020b2026d6d75ed777ccf76f and later main changes through 6b0efc015812d699c20424c4048e2fdba570c2dd do not touch Marketplace runtime or Marketplace tests.
  - ReconcileCharacterAuctions markRecovery updates by auction primary key without locking, re-reading current status or limiting the write to eligible non-terminal states.
  - Canary character transfer uses row locks and same-target idempotency; the inspected MariaDB concurrency test proves competing transfers serialize at that boundary.
  - PR #805 is the single documentation/evidence delivery for this bounded audit and contains no runtime change.
  - Agent Governance run 31172998033 and CI run 31172998002 passed on audit head 751d2a56e60586764483932f9a4332f25d64af2e.
derived:
  - A stale reconciliation failure can overwrite a newer completed, cancelled or expired auction state with recovery_required.
  - A completed settlement regressed to recovery_required can contradict already-finalized character ownership, wallet ledger and winning-bid state.
  - Rebuilding the same audit diff directly on current main is the narrowest changed-input test for the branch-protection expected-check response after main advanced through PR #806.
unknown: []
conflicts:
  - CharacterAuction defines completed, cancelled and expired as terminal while the generic recovery fallback is able to replace those states without a current-state guard.
first_failure:
  marker: Agent Governance run 31172457925 failed active task checkpoint validation on audit head aa5348849725adc5df72c6fbb7287d91c7406310.
  evidence: The initial audit task checkpoint used ad-hoc scalar fields and omitted required compact checkpoint-v1 fields; the corrected checkpoint later passed Agent Governance run 31172998033.
rejected_hypotheses:
  - Duplicate character transfer is the primary defect; Canary transfer locks ownership rows and treats same-target ownership as already transferred.
  - Existing recovery tests prove stale-worker terminal monotonicity; inspected coverage has no late-error-after-terminal interleaving.
  - Repeating direct merge on unchanged head after required checks passed would resolve branch protection; the second identical attempt returned the same expected-check response and is not repeated again.
changed_paths:
  - docs/agents/evidence/OTERYN-20260807-marketplace-terminal-recovery-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/reports/OTERYN-20260807-marketplace-terminal-recovery-audit.md
  - docs/agents/tasks/active/OTERYN-20260807-marketplace-terminal-recovery-audit.md
validation:
  - command: full PR #805 documentation-only diff review on 751d2a56e60586764483932f9a4332f25d64af2e
    result: PASS
    evidence: Exactly four audit/governance documentation paths changed, with zero unresolved review threads and no Marketplace or workflow runtime mutation.
  - command: CI run 31172998002
    result: PASS
    evidence: Required classify-changes and test jobs passed on exact head 751d2a56e60586764483932f9a4332f25d64af2e; runtime-tests was skipped as expected for the documentation-only diff.
  - command: Agent Governance run 31172998033
    result: PASS
    evidence: Checkpoint schema, live task ownership, Control Room and liveness enforcement all passed on exact head 751d2a56e60586764483932f9a4332f25d64af2e.
  - command: protected direct merge attempts for PR #805 on 751d2a56e60586764483932f9a4332f25d64af2e
    result: BLOCKED
    evidence: GitHub returned 405 with two of two required status checks expected after main advanced through PR #806; no protection bypass was attempted.
blockers: []
next_action: Validate the same audit diff rebased directly onto current main 6b0efc015812d699c20424c4048e2fdba570c2dd, then merge only if the new exact-head required checks and branch protection pass.
```

## Evidence summary

`ReconcileCharacterAuctions::markRecovery()` updates an auction to `recovery_required` by primary key only. It neither locks/re-reads the current row nor limits the write to non-terminal states. A stale/failing worker can therefore overwrite `completed`, `cancelled` or `expired` after another worker has already committed the newer terminal result.

For completed settlement this creates contradictory state: character ownership is already with the winner, wallet settlement and the bid can already be finalized, yet the auction can be moved back to recovery. The ordinary recovery path then cannot replay the already-finalized bid because settlement expects a `leading` bid while the successful path has already changed it to `won`.

The Canary transfer implementation itself locks the relevant records and treats same-target ownership as already transferred. Its dedicated MariaDB concurrency test proves the transfer boundary serializes, so the finding is intentionally scoped to Platform recovery-state monotonicity.

## Main refresh

While the audit was running, `main` advanced first to `7dbb35e...` through PR #786 and then to `6b0efc0...` through the documentation-only lifecycle closeout PR #806. Neither change touches Marketplace runtime or Marketplace tests, so the finding remains current. The audit branch is rebuilt directly on `6b0efc0...` to remove base ancestry as a branch-protection variable.

## Safety

Audit-only. No Marketplace runtime, Wallet runtime, Canary database, workflow, production/staging environment or external repository is modified in this task.

## Validation plan

- verify audit package diff contains docs/evidence/task/programme state only;
- run repository-required pull-request checks on exact head;
- inspect review threads and mergeability;
- merge only through protected repository policy;
- archive this task in a lifecycle closeout after the audit PR is terminal.