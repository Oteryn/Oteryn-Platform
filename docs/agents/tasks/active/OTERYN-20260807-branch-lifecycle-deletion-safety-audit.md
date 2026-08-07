---
task_id: OTERYN-20260807-branch-lifecycle-deletion-safety-audit
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
  - docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  - docs/agents/BRANCH_LIFECYCLE_POLICY.json
---

# OTERYN-20260807-branch-lifecycle-deletion-safety-audit

## Goal

Independently falsify the destructive branch-lifecycle apply path on current `main` and persist any confirmed material finding without modifying runtime or repair code.

## Acceptance criteria

- [x] Live programme ownership, open PRs and material audit/remediation Issues were refreshed before selection.
- [x] The package is non-overlapping with active Issue #558 ownership.
- [x] ADR 0024, lifecycle policy, workflow, primary implementation and focused tests were inspected on `main@021bf44d99de4430b2e054d25872eabfa322eba2`.
- [x] Destructive apply was checked for stale-SHA, newly opened PR, newly active claim/task and protection/retention race behavior.
- [x] Confirmed material findings were deduplicated and given one durable remediation Issue.
- [ ] Exact-head documentation/governance CI passes and this audit task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-deletion-safety-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - branch lifecycle governance audit records only
dependencies:
  - Issue #780 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T08:31:00Z
head: 021bf44d99de4430b2e054d25872eabfa322eba2
branch: audit/OTERYN-20260807-branch-lifecycle-deletion-safety
pr: 781
status: validating
context_routes:
  - ci
  - architecture
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-deletion-safety-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Current main is 75 commits ahead of the programme latest_audited_main baseline recorded before this invocation.
  - Issue #558 was independently claimed after PR #542 became terminal, so its Agent Governance paths were excluded from this package.
  - Completed branch-lifecycle implementation Issue #658 and its task have released ownership.
  - apply_manifest validates against one in-memory report and then deletes refs by branch name without a per-entry live revalidation.
  - Focused tests validate snapshot classification, manifest drift and non-main apply context but contain no race/pre-delete live-state test.
  - OPA-GOV-0019 is recorded as Issue #780 with risk high, priority P1 and implementation authorization.
derived:
  - A branch can become active or move after inventory and still be deleted later in the same apply loop.
unknown: []
conflicts:
  - ADR 0024 requires active/open/ambiguous branches to fail closed, while the current destructive apply path has a time-of-check/time-of-use gap.
first_failure:
  marker: OPA-GOV-0019
  evidence: tools/agents/branch_lifecycle.py apply_manifest performs name-only deletion after snapshot validation.
rejected_hypotheses:
  - The reviewed manifest hash alone prevents the race; rejected because it binds the earlier snapshot, not state immediately before DELETE.
  - Workflow concurrency prevents branch activity during apply; rejected because it serializes lifecycle workflow runs, not ordinary branch pushes, PR creation or task claims.
  - Existing focused tests cover the destructive race; rejected because no apply-time ref/PR/claim mutation test exists.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-deletion-safety-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-deletion-safety-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source static falsification on main@021bf44d99de4430b2e054d25872eabfa322eba2
    result: PASS
    evidence: issue #780 records exact functions, destructive call path and missing per-entry live guard.
  - command: tools/agents/test_branch_lifecycle.py source review
    result: PASS
    evidence: tests cover classification and manifest snapshot drift, but no apply-time TOCTOU boundary.
  - command: destructive live race reproduction
    result: NOT_APPLICABLE
    evidence: intentionally not performed because creating a real race against repository refs would add destructive risk; deterministic offline regression belongs to remediation acceptance.
blockers:
  - none
next_action: Complete exact-head checks for PR #781, then merge the documentation-only audit package and archive this task with ownership released.
```

## Notes

The auditor did not modify `.github/workflows/branch-lifecycle.yml`, `tools/agents/branch_lifecycle.py`, tests, repository settings or any live branch other than this dedicated audit branch.
