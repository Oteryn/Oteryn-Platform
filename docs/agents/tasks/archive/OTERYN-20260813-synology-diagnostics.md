---
task_id: OTERYN-20260813-synology-diagnostics
project_lane: oteryn-platform-core
task_profile: implementation
execution_mode: github_only
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - synology diagnostics
  - deploy/synology
optional_reads: []
---

# OTERYN-20260813-synology-diagnostics

## Goal

Add a manual, read-only GitHub Actions diagnostic path for the existing Synology-hosted `oteryn-staging` runner without requiring SSH or exposing runtime secrets.

## Acceptance criteria

- [x] Workflow is manual-only and targets `runs-on: oteryn-staging`.
- [x] Diagnostics cover runner identity, Docker Engine facts, containers, Docker disk usage, networks, volumes, and the mounted Oteryn state filesystem.
- [x] Workflow does not print container environments, inspect payloads, file contents, secrets, or application data.
- [x] Workflow contains no Docker mutation, deployment, prune, create, restart, stop, remove, or privileged helper-container action.
- [x] Exact final PR head `253d7a0e8f28c5d8bfb8c0a34bcc65a40ae26938` passed CI, Agent Governance, Game Auth Ticket Concurrency, Platform DB Outage Validation, Edge Security Emulation, and Phase 7 Production-Like Validation.
- [x] PR #1017 merged to `main` as `b9dbe244118107e3bc7ef7a6c5fce7ad51898fd2`.

## Ownership

```yaml
owned_paths: []
modules:
  - github-actions
  - synology-staging-runner
dependencies:
  - existing oteryn-staging self-hosted runner
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T06:50:00Z
head: b9dbe244118107e3bc7ef7a6c5fce7ad51898fd2
branch: main
pr: 1017
status: completed
context_routes:
  - oteryn-platform-core
owned_paths: []
proven:
  - merged workflow exists at .github/workflows/synology-diagnostics.yml
  - workflow is manual-only and targets the oteryn-staging runner
  - workflow exposes fixed read-only Docker and filesystem diagnostics only
  - exact final PR head 253d7a0e8f28c5d8bfb8c0a34bcc65a40ae26938 passed all six observed PR workflows
  - PR 1017 auto-merged as b9dbe244118107e3bc7ef7a6c5fce7ad51898fd2
derived:
  - the existing Synology runner now provides a controlled diagnostic path without SSH
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/synology-diagnostics.yml
validation:
  - command: full PR diff self-review at 253d7a0e8f28c5d8bfb8c0a34bcc65a40ae26938
    result: PASS
    evidence: no arbitrary command input, secret/environment dump, docker inspect payload, or mutation command exists
  - command: exact-head PR workflow suite at 253d7a0e8f28c5d8bfb8c0a34bcc65a40ae26938
    result: PASS
    evidence: CI plus five additional required workflows completed successfully before protected auto-merge
blockers:
  - none
next_action: none
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 253d7a0e8f28c5d8bfb8c0a34bcc65a40ae26938
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - workflow has no arbitrary command input and no full docker inspect output
    - diagnostic commands are read-only Docker and filesystem queries
    - no existing Synology deployment workflow was modified
```

## E2E

`NOT_APPLICABLE`: the delivered change is a repository-only manual diagnostic workflow. Dispatching it would query the live Synology-hosted runner and was not required to validate the static workflow delivery.

## Terminal closeout

- Resulting merged state verified at `b9dbe244118107e3bc7ef7a6c5fce7ad51898fd2`.
- PR #1017 is terminal and merged.
- No Issue was created for this owner-requested bounded task.
- No task-owned temporary containers, networks, volumes, images, deployments, or runner resources were created.
- Path ownership is released by this archival move.
