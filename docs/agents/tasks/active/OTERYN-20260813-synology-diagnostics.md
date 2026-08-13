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
- [x] Repository CI passed on validation head `ce5127df6bc741cdef44501ec917f0ee06a29544` before this checkpoint-only update.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260813-synology-diagnostics.md
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
updated_at: 2026-08-13T06:45:00Z
head: ce5127df6bc741cdef44501ec917f0ee06a29544
branch: chore/synology-diagnostics-20260813
pr: 1017
status: ready
context_routes:
  - oteryn-platform-core
owned_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260813-synology-diagnostics.md
proven:
  - existing deploy workflow targets runs-on oteryn-staging
  - runner compose mounts /var/run/docker.sock and /volume1/docker/oteryn/state
  - open PR 1003 owns deploy-synology-staging.yml specifically; this task does not modify that path
  - open PR 1013 intentionally avoids deploy-synology-staging.yml; no overlap with this task-owned new workflow path was found
  - PR 1017 contains only the new diagnostic workflow and this task record
  - CI, Agent Governance, Game Auth Ticket Concurrency, Platform DB Outage Validation, Edge Security Emulation, and Phase 7 Production-Like Validation all passed on ce5127df6bc741cdef44501ec917f0ee06a29544
derived:
  - Docker Engine diagnostics can be executed through the existing runner without SSH
unknown:
  - required-check result for the checkpoint-only final PR head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260813-synology-diagnostics.md
validation:
  - command: full PR diff self-review at ce5127df6bc741cdef44501ec917f0ee06a29544
    result: PASS
    evidence: workflow is manual-only, fixed-command, read-only, does not emit environment/inspect/file contents, and has no Docker mutation commands
  - command: repository CI at ce5127df6bc741cdef44501ec917f0ee06a29544
    result: PASS
    evidence: CI + five additional PR workflows completed successfully
blockers:
  - none
next_action: verify required checks on the checkpoint-only final head, mark PR ready, and merge only if all exact-head gates remain green
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: ce5127df6bc741cdef44501ec917f0ee06a29544
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
    - no existing Synology deployment workflow is modified
```

## E2E

`NOT_APPLICABLE` for this repository-only workflow addition: executing it would access the live Synology-hosted runner, while this task is authorized to add the diagnostic path but does not need to perform a live diagnostic run to prove the static repository change.

## Notes

The workflow deliberately avoids arbitrary command input and full `docker inspect` output because either would unnecessarily expand authority or risk exposing secrets. It is a repository change only; this task does not itself dispatch the Synology diagnostic workflow or perform a protected deployment.
