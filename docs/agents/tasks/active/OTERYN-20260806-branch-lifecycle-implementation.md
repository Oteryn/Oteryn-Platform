---
task_id: OTERYN-20260806-branch-lifecycle-implementation
programme_id: OTERYN_PLATFORM_REMEDIATION
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 658
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
search_first:
  - duplicate branch lifecycle tooling or workflow
  - overlapping open PR and active task ownership
  - existing GitHub API automation patterns
---

# OTERYN-20260806-branch-lifecycle-implementation

## Goal

Implement accepted ADR 0024 as deterministic, fail-closed branch inventory, dry-run classification, retention metadata, reviewed candidate approval and recovery evidence. No branch may be deleted merely by age or prefix.

## Acceptance criteria

- [x] Deterministic remediation lock `repair/issue-658` acquired from synchronized main.
- [x] Issue claim activated under protocol version 2.
- [x] Machine-readable exact-branch retention exceptions validate fail closed.
- [x] Standard-library classifier inventories branches and reconciles protection, open PRs, active tasks/claims and exact merged-PR head SHAs.
- [x] Focused tests cover classifications, policy defects, manifest drift and apply-context refusal.
- [x] Pull-request workflow produced a complete live dry-run artifact without write permission.
- [x] Candidate deletion set was generated with `apply_on_main=false`, reviewed and bound by exact count, policy hash and entries hash.
- [x] Apply mode is restricted to protected-main push, exact reviewed candidate digest and zero live drift.
- [ ] Every deletion preserves branch, SHA, PR and evidence; non-candidates remain untouched.
- [ ] Recovery create/delete/restore/delete test passes using an ephemeral test ref.
- [x] Ordinary merged closeout branch deletion is proven; protected `main` survives.
- [ ] Exact-head CI, independent audit, cleanup evidence, Issue #586/#658 closeout and task archival complete.

## Ownership

```yaml
claim_nonce: OTERYN-20260806-branch-lifecycle-658-01
coordination_key: repository:merged-branch-lifecycle
branch: repair/issue-658
owned_paths:
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/**
shared_paths:
  - docs/agents/PROJECT_STATE.md
forbidden_paths:
  - app/**
  - database/**
  - resources/**
  - routes/**
  - production and staging
  - external repositories
  - refs/heads/main deletion or force update
blockers: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-06T07:24:00Z
phase: validate
session_id: chatgpt-20260806-branch-lifecycle
session_role: implementer
execution_mode: github
execution_reason: repository governance files, GitHub API classifier and Actions validation are all repository-owned
lease_expires_at: 2026-08-06T09:00:00Z
status: validating
context_routes:
  - agent-governance
  - repository-governance
  - testing
head: tracked-by-pr-666
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one task requires tooling, reviewed live dry-run, protected-main apply and terminal evidence in sequential phases
validation_level: full
last_completed_step: reviewed and cryptographically bound the inert 354-branch candidate set from live artifact 8959037792
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 0
human_interruptions: 0
branch: repair/issue-658
pr: 666
issue: 658
claim_nonce: OTERYN-20260806-branch-lifecycle-658-01
coordination_key: repository:merged-branch-lifecycle
owned_paths:
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/**
proven:
  - ADR 0024 is accepted on main through PR 653.
  - Automatic merged-head deletion is enabled and squash is the sole merge method, verified through the owner-authorized repository connection.
  - Decision closeout branch disappeared automatically after PR 665 merged.
  - The deterministic repair/issue-658 branch was acquired from main 47c6caa6b35c2d2af08d06322c6911721370860d.
  - PR 666 owns exactly six implementation paths.
  - Pull-request jobs are read-only; deletion can run only on a protected-main push after exact candidate approval is activated.
  - Focused tests and canonical policy validation pass.
  - Live dry-run run 31079934408 inventoried 501 branches: 354 TERMINAL_MERGED, 85 UNMERGED_ORPHAN, 31 UNKNOWN, 22 OPEN_PR, 8 ACTIVE_CLAIM and 1 PROTECTED.
  - Artifact 8959037792 has digest sha256:3fe9c93022d867ea5c4d243b103dc37a2a3e31bd473eb4b915dff63f191c208d.
  - Reviewed entries hash eeb980e8baab019b592a21712e607f4c27bf8655ccfd5becfb1fd9cdc7cbfa0f binds exactly 354 branch/SHA/merged-PR records.
  - The 354 candidates contain no protected, open-PR, active-claim, reserved release, rollback, recovery or backup branch names.
  - The current approval remains inert with apply_on_main=false.
  - The workflow now fails closed unless main is live-protected and never a deletion candidate.
derived:
  - A reviewed candidate digest rebuilt and compared immediately before deletion is safer and more reviewable than committing a large mutable manifest.
unknown:
  - Final exact-head conclusions after the latest default-branch protection hardening.
  - Apply and recovery evidence until the activated approval merges through protected main.
conflicts: []
first_failure:
  marker: unavailable-actions-admin-metadata
  evidence: Branch Lifecycle run 31080435495 proved GITHUB_TOKEN omits repository administration fields; the unsupported probe was removed rather than treated as a false setting result. Live settings remain owner-connection evidence, while deletion safety is enforced through branch/PR/task state and protected-main classification.
rejected_hypotheses:
  - Delete branches by prefix or age.
  - Give pull-request validation write permission.
  - Apply an unreviewed candidate list.
  - Treat a merged PR with a mismatched current branch SHA as terminal.
  - Treat missing administration fields in GITHUB_TOKEN metadata as repository-setting drift.
changed_paths:
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
validation:
  - command: deterministic branch acquisition
    result: PASS
    evidence: repair/issue-658 created from synchronized main
  - command: python3 tools/agents/test_branch_lifecycle.py
    result: PASS
    evidence: eleven focused tests passed
  - command: Branch Lifecycle run 31079934408
    result: PASS
    evidence: full read-only live inventory and inert candidate manifest artifact produced
  - command: Branch Lifecycle run 31080663708
    result: PASS
    evidence: reviewed 354-entry candidate digest matched unchanged live evidence
  - command: Agent Governance run 31080435489
    result: PASS
    evidence: active checkpoint schema and ownership passed
  - command: live repository settings
    result: PASS
    evidence: owner-authorized connection reports delete_branch_on_merge=true, squash=true, merge=false and rebase=false
  - command: apply and recovery
    result: NOT_RUN
    evidence: candidate approval is intentionally inert pending final exact-head audit
blockers: []
next_action: Complete exact-head CI and independent audit on the fail-closed implementation, activate the reviewed approval, synchronize with current main and merge PR 666 through protection.
```
