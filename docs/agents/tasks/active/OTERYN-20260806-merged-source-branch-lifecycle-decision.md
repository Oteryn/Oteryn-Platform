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

Resolve `ARCH-DEC-0001` by presenting a current, evidence-backed merged source-branch lifecycle policy to the repository owner without deleting branches or inferring acceptance.

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
- [x] Allocate proposed ADR 0024 without reusing an ADR prefix.
- [x] Define fail-closed exception, recovery and one-time cleanup boundaries.
- [ ] Record an explicit repository-owner selection of A, B or C.
- [ ] Accept/reject ADR 0024 and transition/remove `ARCH-DEC-0001` in the same bounded package.
- [ ] Create a separate implementation/cleanup handoff only after acceptance.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/README.md
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
blockers:
  - explicit repository-owner selection of A, B or C
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
updated_at: 2026-08-06T06:35:00Z
phase: decision
head: tracked-by-pr-653
branch: task/OTERYN-20260806-merged-branch-lifecycle-decision
pr: 653
status: waiting
context_routes:
  - architecture
  - repository-governance
proven:
  - Live repository metadata reports delete_branch_on_merge=true.
  - Squash is the only enabled merge method; merge commits and rebase merges are disabled.
  - Complete branch enumeration returned 498 refs including main and this task branch.
  - No open duplicate PR or canonical lifecycle policy was found.
  - GitHub documentation states branch protection and repository rules can prevent automatic branch deletion.
  - ADR prefix 0024 is unused and is the next value after 0023.
  - Draft PR 653 contains exactly the five bounded decision-documentation paths.
derived:
  - Option A provides the lowest recurring cost and strongest deterministic default when retention exceptions are protected and fail closed.
unknown:
  - Repository-owner selection of A, B or C.
conflicts:
  - Issue #586 historical evidence says automatic deletion was disabled, while current repository metadata proves it is enabled.
first_failure:
  marker: stale-setting-evidence
  evidence: Issue #586 recorded delete_branch_on_merge=false on 2026-08-05; live metadata on 2026-08-06 reports true.
rejected_hypotheses:
  - Treat the enabled toggle as a complete policy.
  - Bulk-delete branches by prefix or age.
  - Infer every historical-looking branch is terminal.
  - Add custom deletion automation before selecting a policy.
changed_paths:
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/architecture/adr/README.md
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
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: decision-only documentation package; no runtime, workflow or setting change
blockers:
  - explicit repository-owner choice A, B or C
next_action: Obtain the repository owner's explicit A, B or C selection, then update ADR 0024, ARCH-DEC-0001, Issue #586 and the implementation handoff without deleting branches in this decision package.
```
