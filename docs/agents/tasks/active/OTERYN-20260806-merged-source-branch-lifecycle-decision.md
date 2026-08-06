---
task_id: OTERYN-20260806-merged-source-branch-lifecycle-decision
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: architecture
implementation_authorized: false
issue: 586
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
search_first:
  - duplicate branch lifecycle policy or ADR
  - overlapping open PR and active task ownership
  - current repository merge and branch-deletion settings
---

# OTERYN-20260806-merged-source-branch-lifecycle-decision

## Goal

Resolve `ARCH-DEC-0001` by recording the repository owner's selected merged source-branch lifecycle policy without deleting branches in the decision package.

## Delivery classification

```yaml
feature_scope:
  type: architecture-decision
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
```

## Acceptance criteria

- [x] Reconcile stale Issue #586 repository-setting evidence against live metadata.
- [x] Prove current merge-method and automatic-deletion settings.
- [x] Enumerate the complete current branch inventory count without deleting refs.
- [x] Compare Options A, B and C using current GitHub behavior.
- [x] Allocate ADR 0024 without reusing an ADR prefix.
- [x] Define fail-closed exception, recovery and one-time cleanup boundaries.
- [x] Record the repository owner's explicit selection of Option A.
- [x] Accept ADR 0024 and remove `ARCH-DEC-0001` from the active backlog in the same bounded package.
- [x] Create separate implementation and cleanup handoff Issue #658.
- [ ] Pass final exact-head workflows and independent audit.
- [ ] Merge PR #653, archive this task and release ownership.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/reports/OTERYN-20260806-merged-source-branch-lifecycle-review.md
  - docs/agents/tasks/active/OTERYN-20260806-merged-source-branch-lifecycle-decision.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
modules:
  - architecture-governance
  - repository-governance
dependencies:
  - Issue #586
  - ARCH-DEC-0001
  - accepted ADR 0023
  - implementation handoff Issue #658
blockers: []
forbidden_paths:
  - application code
  - workflows
  - repository settings
  - branch deletion
  - production and staging
  - external repositories
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T06:44:00Z
phase: validate
head: tracked-by-pr-653
branch: task/OTERYN-20260806-merged-branch-lifecycle-decision
pr: 653
status: validating
context_routes:
  - architecture
  - repository-governance
owned_paths:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/reports/OTERYN-20260806-merged-source-branch-lifecycle-review.md
  - docs/agents/tasks/active/OTERYN-20260806-merged-source-branch-lifecycle-decision.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
proven:
  - Live repository metadata reports delete_branch_on_merge=true.
  - Squash is the only enabled merge method; merge commits and rebase merges are disabled.
  - Complete branch enumeration returned 498 refs including main and the decision branch.
  - No open duplicate PR or canonical lifecycle policy was found.
  - The repository owner explicitly selected Option A on 2026-08-06.
  - ADR 0024 is Accepted and records the selected policy.
  - ARCH-DEC-0001 has been removed from the active backlog; the remaining IDs are ARCH-DEC-0002 and ARCH-DEC-0003.
  - Issue #658 owns implementation, deterministic dry-run classification, cleanup and recovery proof.
derived:
  - Remaining work for Issue #586 is operational implementation, not an unresolved architecture decision.
unknown:
  - Final exact-head workflow and merge conclusions for PR #653.
conflicts:
  - Issue #586 historical evidence says automatic deletion was disabled, while current repository metadata proves it is enabled.
first_failure:
  marker: missing-checkpoint-owned-paths
  evidence: Agent Governance run 31078216272 rejected the task because checkpoint owned_paths was absent; the same six owned paths are now declared inside the checkpoint.
rejected_hypotheses:
  - Treat the enabled toggle as a complete policy.
  - Bulk-delete branches by prefix or age.
  - Infer every historical-looking branch is terminal.
  - Add custom deletion automation inside the decision package.
changed_paths:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/reports/OTERYN-20260806-merged-source-branch-lifecycle-review.md
  - docs/agents/tasks/active/OTERYN-20260806-merged-source-branch-lifecycle-decision.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
validation:
  - command: current repository metadata reconciliation
    result: PASS
    evidence: automatic deletion true; squash-only merge policy
  - command: full branch enumeration
    result: PASS
    evidence: 498 branch refs returned across all pages
  - command: duplicate PR and repository policy search
    result: PASS
    evidence: no competing open PR or canonical policy found
  - command: Agent Governance run 31078216272
    result: FAIL_THEN_FIXED
    evidence: checkpoint owned_paths was missing and is now declared
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: decision-only documentation package; no runtime, workflow or setting change
blockers: []
next_action: Complete exact-head validation and protected merge of PR #653, archive the decision task, release ownership and activate Issue #658.
```
