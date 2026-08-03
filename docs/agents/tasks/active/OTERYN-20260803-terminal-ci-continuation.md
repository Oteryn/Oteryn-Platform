---
task_id: OTERYN-20260803-terminal-ci-continuation
status: validating
branch: docs/OTERYN-20260803-terminal-ci-continuation
base_branch: main
created: 2026-08-03T08:17:00+02:00
updated: 2026-08-03T08:27:00+02:00
related_pr: 484
project_lane: oteryn-platform
execution_mode: github-only
complete_user_facing_feature: false
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/tasks/active/OTERYN-20260803-terminal-ci-continuation.md
modules_touched:
  - agent-governance
depends_on: []
blocks: []
---

# Bounded terminal CI continuation

## Goal

Prevent autonomous Oteryn Platform workers from returning `WAITING` solely because final required exact-head CI is still running after two ordinary checks, while preserving bounded execution, branch protection, audit, E2E, review and direct-merge safety.

## Problem evidence

PR #477 reached a final audited head with eight required workflows still running. The trusted policy v2 forced `WAITING` after two checks even though CI was the only remaining gate and repository auto-merge is disabled.

## Acceptance criteria

- [x] ordinary/non-terminal CI retains the two-check limit;
- [x] eligible final exact-head CI receives a separate bounded wait budget;
- [x] unchanged terminal checks require a minimum interval and a per-generation cap;
- [x] the terminal wait budget does not reset across ready/current-base/merge generations on one head;
- [x] auto-merge availability is not required for eligibility;
- [x] when auto-merge is unavailable, direct squash merge is allowed only after every exact-head gate passes;
- [x] post-merge archival and Issue reconciliation remain part of the same entry task;
- [x] hidden background execution and unbounded polling remain forbidden;
- [x] all higher-priority routing documents are consistent;
- [ ] exact-head repository CI passes;
- [x] fresh contradiction/scope audit has no open material finding.

## Fresh contradiction and scope audit

```yaml
validator_role: fresh-governance-falsification
reviewed_inputs:
  - complete PR 484 diff
  - AGENTS.override.md priority rules
  - ordinary and terminal CI counters
  - autonomous continuation waiting rules
  - GitHub-only merge authority
result: PASS
open_material_findings: 0
scope:
  changed_paths: 6
  application_code: 0
  workflows_or_branch_protection: 0
  production_or_external_state: 0
invariants:
  ordinary_two_check_limit_preserved: true
  terminal_wait_bounded_to_45_minutes: true
  minimum_interval_3_minutes: true
  per_generation_cap_12: true
  total_budget_not_reset_between_generations: true
  dedicated_terminal_counters: true
  auto_merge_required: false
  direct_merge_before_exact_head_pass: forbidden
  branch_protection_bypass: forbidden
  hidden_background_execution: forbidden
  archive_and_issue_closeout_same_entry_task: true
e2e:
  result: NOT_APPLICABLE
  reason: documentation-only agent-governance change with no application or runtime behaviour
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-03T08:27:00+02:00
head: pending-final-task-record-commit
branch: docs/OTERYN-20260803-terminal-ci-continuation
pr: 484
status: validating
proven:
  - Oteryn Platform main uses policy v2 and caused PR 477 to stop after two final-CI checks.
  - Repository setting allow_auto_merge is false.
  - Policy 2.1 separates ordinary and terminal CI counters.
  - Eligible final CI may continue for at most 45 minutes, at intervals of at least 3 minutes and no more than 12 checks per materially new generation.
  - Direct squash merge remains forbidden until every exact-head gate passes.
  - Fresh contradiction and scope audit passed with zero open material findings.
unknown:
  - exact-head CI result for the final governance head
conflicts: []
changed_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/tasks/active/OTERYN-20260803-terminal-ci-continuation.md
validation:
  - command: complete diff contradiction and scope audit
    result: PASS
    evidence: six governance-only paths, zero open material findings
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only governance change
  - command: exact-head repository CI
    result: NOT_RUN
    evidence: final task-record commit triggers exact-head checks
blockers: []
next_action: Mark PR #484 ready, inspect required exact-head CI under the trusted-base limit, and squash-merge only after every required check passes.
```
