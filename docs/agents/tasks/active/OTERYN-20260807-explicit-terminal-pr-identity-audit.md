---
task_id: OTERYN-20260807-explicit-terminal-pr-identity-audit
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

# OTERYN-20260807 explicit terminal PR identity audit

## Goal

Audit the post-Issue-788 live task-liveness implementation for exact task→branch→PR identity on the explicit numeric-PR terminal path, without implementing the remediation.

## Acceptance criteria

- [x] Current main, open audit-remediation Issues, blocked findings, open PRs and recent audit delta were refreshed from live GitHub state.
- [x] The numeric-PR open/draft and terminal liveness paths were inspected on current main.
- [x] Existing regression coverage was inspected for terminal branch/head mismatch behavior.
- [x] Open and closed Issues were searched for the same root cause.
- [x] One material finding was routed to Issue #811 as `OPA-GOV-0023`.
- [ ] Exact-head documentation/governance CI passes, the audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-explicit-terminal-pr-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - Agent Governance task-liveness audit records only
dependencies:
  - Issue #811 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T12:55:00Z
head: ae716e3b955808916cb203bb97b59df0b44070cf
branch: audit/OTERYN-20260807-explicit-terminal-pr-identity
pr: none
status: investigating
context_routes:
  - continuous-audit
  - ci-build-test
  - architecture-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-explicit-terminal-pr-identity-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Current audit input is main ae716e3b955808916cb203bb97b59df0b44070cf after Issue #788 repair PR #808 and lifecycle closeout PR #810.
  - Numeric open/draft PR handling compares task.branch with PR head.ref and emits branch_pr_mismatch on disagreement.
  - Numeric terminal PR handling releases ownership and validates archive-pending metadata without first comparing task.branch with PR head.ref.
  - Existing terminal numeric-PR test coverage exercises only the matching branch case; no negative terminal branch/head mismatch fixture exists.
  - Duplicate search found no actionable Issue for this explicit terminal numeric-PR identity gap; Issue #811 now owns remediation.
derived:
  - A task can reference an unrelated same-repository terminal PR and be treated as terminal archive-pending with ownership inactive despite its declared branch belonging to different work.
unknown: []
conflicts:
  - Exact task→branch→PR identity required by Issue #558 is enforced for numeric open/draft PRs but not for numeric terminal PRs.
first_failure:
  marker: none
  evidence: static audit finding; no runtime mutation or destructive validation was required.
rejected_hypotheses:
  - Issue #788 fully closed all branch/PR identity paths; it repaired omitted-PR branch-history reconciliation but did not add the terminal numeric-PR branch comparison.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/reports/OTERYN-20260807-explicit-terminal-pr-identity-audit.md
  - docs/agents/evidence/OTERYN-20260807-explicit-terminal-pr-identity-audit/index.md
validation:
  - command: current-main static path and regression-coverage review
    result: PASS
    evidence: The mismatch check is present only in the non-terminal numeric-PR branch and the terminal mismatch fixture is absent.
blockers:
  - none
next_action: Open the single audit PR, record its exact identity in this checkpoint and programme state, then validate exact-head required checks before protected merge.
```

## Safety

Audit-only. No governance implementation, product runtime, workflow, production/staging environment or external repository is modified in this task.
