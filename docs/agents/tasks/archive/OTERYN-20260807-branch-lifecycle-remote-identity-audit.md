---
task_id: OTERYN-20260807-branch-lifecycle-remote-identity-audit
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

# OTERYN-20260807 branch lifecycle remote identity audit

## Goal

Audit whether the Branch Lifecycle destructive force-with-lease operation is bound to the same repository identity and working tree that were audited through GitHub API state.

## Acceptance criteria

- [x] Current main and the completed Issue #793 atomic-delete repair were inspected.
- [x] Destructive git push construction, CLI root handling and post-delete verification were traced end to end.
- [x] Regression coverage for atomic deletion and repository identity was inspected.
- [x] Open and closed Issues were searched for the same root cause.
- [x] One material finding was routed to Issue #815 as `OPA-GOV-0024`.
- [x] Exact-head documentation/governance CI passed, the audit package merged, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-remote-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - Branch Lifecycle destructive-boundary audit records only
dependencies:
  - Issue #815 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T13:11:30Z
head: cb9fc25675cf8a1687b73be1eb019909ac58be0f
branch: audit/OTERYN-20260807-branch-lifecycle-remote-identity
pr: 816
status: completed
context_routes:
  - continuous-audit
  - ci-build-test
  - architecture-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-remote-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Audit input was main 7ae96633871e1d970f22d8de69499adb3d1e6d37 after Issue #793 / PR #796 atomic deletion repair.
  - Branch Lifecycle safety reads use GitHubClient.repo while the destructive git push uses a local remote defaulting to origin.
  - The destructive subprocess does not validate the remote URL against GitHubClient.repo.
  - The CLI accepts and resolves --root but the destructive subprocess is not executed with cwd bound to that root.
  - Existing atomic-delete tests assert the literal origin command and expected-SHA lease behavior but do not cover wrong-CWD or foreign-origin identity.
  - Duplicate search found no actionable Issue for this repository-identity boundary; Issue #815 owns remediation.
  - PR #816 was the single documentation/evidence delivery for this bounded audit and changed no executable behavior.
  - Agent Governance run 31181372898 and CI run 31181372937 passed on exact audit head cb9fc25675cf8a1687b73be1eb019909ac58be0f; CI skipped runtime-tests as expected for the documentation-only diff.
  - PR #816 merged through protected repository policy as 3bc3e24da10c832b1c7efcc466d071aee4128cf9.
derived:
  - Safety checks can describe repository A while a successful force-with-lease deletion is executed against repository B selected by local CWD/origin.
unknown: []
conflicts:
  - Autonomous external-repository mutation is forbidden, while the destructive operation is not currently proven to target the configured repository identity before it executes.
first_failure:
  marker: none
  evidence: static control-flow finding; no destructive validation was required.
rejected_hypotheses:
  - Post-delete API verification prevents all damage; it detects that repository A was not deleted only after repository B may already have been modified.
changed_paths:
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-remote-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
validation:
  - command: current-main destructive-path and focused regression-coverage review
    result: PASS
    evidence: Repository API identity and local git remote/CWD identity are independent in the current code path and no negative identity fixture exists.
  - command: Agent Governance run 31181372898
    result: PASS
    evidence: Exact-head checkpoint validation, liveness tests, live ownership validation and Control Room validation passed on cb9fc25675cf8a1687b73be1eb019909ac58be0f.
  - command: CI run 31181372937
    result: PASS
    evidence: classify-changes and required test gate passed on cb9fc25675cf8a1687b73be1eb019909ac58be0f; runtime-tests was skipped by documentation-only routing.
  - command: protected squash merge PR #816
    result: PASS
    evidence: Exact head cb9fc25675cf8a1687b73be1eb019909ac58be0f merged as 3bc3e24da10c832b1c7efcc466d071aee4128cf9.
blockers:
  - none
next_action: No further audit-task action; Issue #815 owns the independent remediation and the continuous-audit programme returns to live-query selection.
```

## Safety

Audit-only. No branch deletion, governance implementation, production/staging mutation or external-repository write was performed.
