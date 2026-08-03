---
task_id: OTERYN-20260803-terminal-ci-continuation
status: implementing
branch: docs/OTERYN-20260803-terminal-ci-continuation
base_branch: main
created: 2026-08-03T08:17:00+02:00
updated: 2026-08-03T08:17:00+02:00
related_pr: ""
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

- [ ] ordinary/non-terminal CI retains the two-check limit;
- [ ] eligible final exact-head CI receives a separate bounded wait budget;
- [ ] unchanged terminal checks require a minimum interval and a per-generation cap;
- [ ] the terminal wait budget does not reset across ready/merge generations on one head;
- [ ] auto-merge availability is not required for eligibility;
- [ ] when auto-merge is unavailable, direct squash merge is allowed only after every exact-head gate passes;
- [ ] post-merge archival remains part of the same entry task;
- [ ] hidden background execution and unbounded polling remain forbidden;
- [ ] all higher-priority routing documents are consistent;
- [ ] exact-head repository CI passes;
- [ ] fresh contradiction/scope audit has no open material finding.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-03T08:17:00+02:00
head: pending
branch: docs/OTERYN-20260803-terminal-ci-continuation
pr: none
status: implementing
proven:
  - Oteryn Platform main still uses anti-stall policy v2 with a hard two-check final-CI stop.
  - Repository setting allow_auto_merge is false.
  - PR 477 stopped with eight required workflows pending after the second check.
unknown:
  - exact final governance head and CI result
conflicts: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260803-terminal-ci-continuation.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: governance implementation in progress
blockers: []
next_action: Update the governing bootstrap, anti-stall, autonomous continuation and GitHub-only contracts consistently, then open a draft PR and run exact-head validation.
```
