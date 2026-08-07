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
- [ ] Exact-head documentation/governance CI passes, the audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
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
updated_at: 2026-08-07T13:07:30Z
head: dfa24083bf99c7d953d25910e2779d8c8279feb4
branch: audit/OTERYN-20260807-branch-lifecycle-remote-identity
pr: 816
status: validating
context_routes:
  - continuous-audit
  - ci-build-test
  - architecture-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-remote-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Audit input is current main 7ae96633871e1d970f22d8de69499adb3d1e6d37 after Issue #793 / PR #796 atomic deletion repair.
  - Branch Lifecycle safety reads use GitHubClient.repo while the destructive git push uses a local remote defaulting to origin.
  - The destructive subprocess does not validate the remote URL against GitHubClient.repo.
  - The CLI accepts and resolves --root but the destructive subprocess is not executed with cwd bound to that root.
  - Existing atomic-delete tests assert the literal origin command and expected-SHA lease behavior but do not cover wrong-CWD or foreign-origin identity.
  - Duplicate search found no actionable Issue for this repository-identity boundary; Issue #815 owns remediation.
  - PR #816 is the single documentation/evidence delivery for this bounded audit and contains no executable behavior change.
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
  - docs/agents/tasks/active/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-branch-lifecycle-remote-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-branch-lifecycle-remote-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: current-main destructive-path and focused regression-coverage review
    result: PASS
    evidence: Repository API identity and local git remote/CWD identity are independent in the current code path and no negative identity fixture exists.
blockers:
  - none
next_action: Validate exact-head required checks and review state for PR #816, merge only through protected repository policy, then archive this audit task in lifecycle closeout.
```

## Safety

Audit-only. No branch deletion, governance implementation, production/staging mutation or external-repository write was performed.
