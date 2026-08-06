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

Implement accepted ADR 0024 as deterministic, fail-closed branch inventory, reviewed cleanup, durable deletion evidence and recovery proof. No branch may be deleted merely by age or prefix.

## Acceptance criteria

- [x] Deterministic remediation lock and protocol-v2 claim acquired.
- [x] Exact-branch retention policy and fail-closed classifier implemented.
- [x] Focused classification, drift and apply-context tests pass.
- [x] Read-only live inventory produced and independently reviewed.
- [x] Exact 354-entry candidate set bound by policy and entries digests.
- [x] Protected-main apply deleted exactly the reviewed refs.
- [x] Every deletion preserves branch, SHA, merged PR and timestamp evidence.
- [x] Recovery create/delete/restore/delete proof passed.
- [x] One-time approval removed after successful apply.
- [ ] Fresh post-cleanup dry-run proves no reviewed terminal batch remains.
- [ ] Evidence PR merges, Issues #586/#658 close and task is archived.

## Ownership

```yaml
claim_nonce: OTERYN-20260806-branch-lifecycle-658-01
coordination_key: repository:merged-branch-lifecycle
branch: docs/OTERYN-20260806-branch-lifecycle-evidence
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
updated_at: 2026-08-06T07:47:00Z
phase: validate
session_id: chatgpt-20260806-branch-lifecycle
session_role: implementer
execution_mode: github
execution_reason: repository governance, GitHub API classification and Actions evidence are repository-owned
lease_expires_at: 2026-08-06T09:30:00Z
status: validating
context_routes:
  - agent-governance
  - repository-governance
  - testing
head: tracked-by-evidence-pr
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: implementation, protected apply, evidence persistence and archival require sequential proof
validation_level: full
last_completed_step: preserved exact apply and recovery evidence and retired the one-time approval
session_rotation_count: 0
heavy_validation_runs: 3
stale_takeover_count: 0
human_interruptions: 0
branch: docs/OTERYN-20260806-branch-lifecycle-evidence
pr: null
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
  - PR 666 merged implementation as 700fa5d0d75a7badd7cb8583d36341c711673942 after all final checks passed.
  - Branch Lifecycle push run 31081595058 rebuilt and exactly matched the reviewed 354-entry candidate set.
  - Apply job 92551500995 completed successfully and deleted exactly 354 refs.
  - Apply artifact 8959831558 has digest sha256:391a5a030fa4bfa7c2e0fac197b491925de8006f931577f5318a77de78a91848.
  - Exact deletion evidence contains 354 branch, head SHA, merged PR and merged timestamp records.
  - Recovery test recovery-test/issue-658-31081595058 recreated SHA 700fa5d0d75a7badd7cb8583d36341c711673942 and passed cleanup.
  - The implementation source branch repair/issue-658 was automatically deleted after merge.
  - One-time BRANCH_DELETION_APPROVAL.json is removed in this evidence package.
  - Exact workflow evidence is preserved under docs/agents/evidence/OTERYN-20260806-branch-lifecycle/.
derived:
  - A fresh PR dry-run after cleanup is the authoritative post-state proof because it re-enumerates live refs without write permission.
unknown:
  - Post-cleanup classification counts until the evidence PR artifact is produced.
  - Evidence PR exact-head validation and protected merge outcome.
conflicts: []
first_failure:
  marker: unavailable-actions-admin-metadata
  evidence: Actions GITHUB_TOKEN omitted repository-administration fields; unsupported probing was removed and owner-connection evidence was retained separately.
rejected_hypotheses:
  - Delete by branch age or prefix.
  - Retain an activated approval after one-time apply.
  - Treat artifact success alone as durable evidence without repository persistence.
changed_paths:
  - docs/agents/BRANCH_DELETION_APPROVAL.json
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/README.md
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/branch-deletion-evidence.json.xz
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/branch-recovery-test-evidence.json
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
validation:
  - command: PR 666 exact-head workflows
    result: PASS
    evidence: Branch Lifecycle, CI, governance, Phase 7 and all emitted checks passed
  - command: independent review 4872141953
    result: PASS
    evidence: no material finding
  - command: Branch Lifecycle run 31081595058
    result: PASS
    evidence: exact reviewed apply and recovery proof completed
  - command: durable evidence SHA-256 verification
    result: PASS
    evidence: README records artifact and decompressed evidence hashes
  - command: post-cleanup live dry-run
    result: NOT_RUN
    evidence: will be emitted by the evidence PR
blockers: []
next_action: Open the evidence PR, inspect its fresh post-cleanup inventory artifact, merge through protection, then archive the task and close Issues 586 and 658.
```
