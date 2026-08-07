---
task_id: OTERYN-20260807-branch-lifecycle-atomic-delete-audit
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

# OTERYN-20260807 branch lifecycle atomic-delete audit

## Goal

Independently falsify the destructive branch-deletion boundary after the completed Issue #780 repair, focusing on whether the reviewed expected SHA is enforced atomically by the remote ref deletion rather than only by client-side reads.

## Acceptance criteria

- [x] Current main, active ownership and open PRs were refreshed before selection.
- [x] PR #789, current `branch_lifecycle.py`, focused tests, workflow and ADR 0024 were inspected.
- [x] The final GET-to-DELETE race boundary was checked against the documented remote API contract.
- [x] Existing open/closed Issues were searched for an atomic-delete duplicate.
- [x] One material finding was routed to Issue #793.
- [ ] Exact-head documentation/governance CI passes, audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-atomic-delete-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - branch-lifecycle audit records only
dependencies:
  - Issue #793 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T09:49:00Z
head: a340b4c2026a6b150bf9102d12894f5a9a5a9c50
branch: audit/OTERYN-20260807-branch-lifecycle-atomic-delete
pr: 794
status: validating
context_routes:
  - ci
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-atomic-delete-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Current main contains the completed Issue #780 repair from PR #789.
  - `GitHubClient.delete_branch()` performs `get_ref()` and then a separate name-addressed REST DELETE.
  - `revalidate_delete_entry()` performs another final `get_ref()` before calling `delete_branch()`, but neither read is part of the destructive remote operation.
  - Current focused tests mutate before the client-side expected-SHA comparison and do not prove rejection of a ref change after the final read but before the remote DELETE.
  - GitHub's documented Delete a reference REST operation accepts the ref identity and exposes no expected-old-SHA request precondition.
  - OPA-GOV-0022 is recorded as Issue #793 with risk high, priority P1 and implementation authorization.
derived:
  - The Issue #780 repair substantially narrows the original stale-snapshot race but leaves a smaller final check-then-delete TOCTOU window.
  - The safe terminal invariant requires a server-enforced expected-old-ref condition or equivalent atomic remote ref transition.
unknown: []
conflicts:
  - ADR 0024 forbids deleting active or changed work, while the current REST DELETE request contains no reviewed expected SHA and can target a ref that advanced after the final read.
first_failure:
  marker: OPA-GOV-0022
  evidence: `delete_branch()` checks expected SHA with GET, then sends a separate unconditional REST DELETE by ref name.
rejected_hypotheses:
  - The second GET inside `delete_branch()` is atomic with DELETE; rejected because they are separate HTTP requests.
  - The existing final-race test proves remote CAS; rejected because its fake client mutates before the client-side comparison.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-atomic-delete-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-atomic-delete-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source static falsification on main@993b3561feb75644d4a07f3e3377020be051eed6
    result: PASS
    evidence: current implementation and tests expose the two-request GET then DELETE boundary.
  - command: official GitHub REST delete-reference contract review
    result: PASS
    evidence: documented DELETE endpoint has owner/repo/ref parameters and no expected-old-SHA request parameter.
  - command: runtime/product E2E
    result: NOT_APPLICABLE
    evidence: this audit package changes documentation/evidence only.
blockers: []
next_action: Complete exact-head checks on PR #794, merge it, then archive this task and release ownership.
```
