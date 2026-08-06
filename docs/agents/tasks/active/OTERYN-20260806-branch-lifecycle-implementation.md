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

Implement accepted ADR 0024 as deterministic, fail-closed branch inventory, dry-run classification, retention metadata, reviewed deletion manifests and recovery evidence. No branch may be deleted merely by age or prefix.

## Acceptance criteria

- [x] Deterministic remediation lock `repair/issue-658` acquired from synchronized main.
- [x] Issue claim activated under protocol version 2.
- [x] Machine-readable exact-branch retention exceptions validate fail closed.
- [x] Standard-library classifier inventories branches and reconciles protection, open PRs, active tasks/claims and exact merged-PR head SHAs.
- [x] Focused tests cover classifications, policy defects, manifest drift and apply-context refusal.
- [ ] Pull-request workflow produces a complete live dry-run artifact without write permission.
- [ ] Candidate deletion manifest is generated with `apply_on_main=false` and independently reviewed before activation.
- [ ] Apply mode runs only from protected main with an exact reviewed manifest and refuses drift.
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
  - docs/agents/BRANCH_DELETION_MANIFEST.json
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
updated_at: 2026-08-06T07:09:00Z
phase: validate
session_id: chatgpt-20260806-branch-lifecycle
session_role: implementer
execution_mode: github
execution_reason: repository governance files, GitHub API classifier and Actions validation are all repository-owned
lease_expires_at: 2026-08-06T08:30:00Z
status: validating
context_pressure: high
context_growth: stable
context_score: 10
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one task requires tooling, reviewed live dry-run, protected-main apply and terminal evidence in sequential phases
validation_level: component
last_completed_step: opened draft PR 666 with fail-closed policy, classifier, tests and live dry-run workflow
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
branch: repair/issue-658
pr: 666
issue: 658
claim_nonce: OTERYN-20260806-branch-lifecycle-658-01
coordination_key: repository:merged-branch-lifecycle
owned_paths:
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/BRANCH_DELETION_MANIFEST.json
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
  - docs/agents/evidence/OTERYN-20260806-branch-lifecycle/**
proven:
  - ADR 0024 is accepted on main through PR 653.
  - Automatic merged-head deletion is enabled and squash is the sole merge method.
  - Decision closeout branch disappeared automatically after PR 665 merged.
  - The deterministic repair/issue-658 branch was acquired from main 47c6caa6b35c2d2af08d06322c6911721370860d.
  - Draft PR 666 owns exactly the declared phase-one files.
  - The policy permits exact protected exceptions only and currently contains only main.
  - Pull-request jobs are read-only; apply is restricted to a push on protected main with a committed exact manifest.
  - Eleven focused unit tests passed in preflight against the committed classifier content.
derived:
  - A reviewed manifest merged through protected main is the safest candidate-specific authorization boundary available to GitHub-only execution.
unknown:
  - Exact live classification counts and candidate set until PR 666 publishes its read-only dry-run artifact.
conflicts: []
first_failure:
  marker: none
  evidence: no implementation failure yet
rejected_hypotheses:
  - Delete branches by prefix or age.
  - Give pull-request validation write permission.
  - Apply an unreviewed candidate list.
  - Treat a merged PR with a mismatched current branch SHA as terminal.
changed_paths:
  - .github/workflows/branch-lifecycle.yml
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
  - docs/agents/tasks/active/OTERYN-20260806-branch-lifecycle-implementation.md
  - tools/agents/branch_lifecycle.py
  - tools/agents/test_branch_lifecycle.py
validation:
  - command: deterministic branch acquisition
    result: PASS
    evidence: repair/issue-658 created from synchronized main
  - command: duplicate implementation search
    result: PASS
    evidence: no competing open PR or canonical tool found
  - command: python3 tools/agents/test_branch_lifecycle.py
    result: PASS
    evidence: eleven focused tests passed in preflight
  - command: python3 -m py_compile tools/agents/branch_lifecycle.py tools/agents/test_branch_lifecycle.py
    result: PASS
    evidence: both Python files compile
  - command: live dry-run
    result: NOT_RUN
    evidence: PR workflow generation is starting on exact head after this checkpoint update
blockers: []
next_action: Inspect PR 666 exact-head checks and live dry-run artifact, repair any fail-closed defect, then commit an inert reviewed candidate manifest with apply_on_main=false.
```
