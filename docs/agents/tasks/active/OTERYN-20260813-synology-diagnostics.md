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
- [ ] Exact-head repository CI validates the final workflow/task record.

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
updated_at: 2026-08-13T06:41:00Z
head: 928cc8bf79dd149675613ce2f71c4259b70c89f7
branch: chore/synology-diagnostics-20260813
pr: 1017
status: validating
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
derived:
  - Docker Engine diagnostics can be executed through the existing runner without SSH
unknown:
  - exact-head CI result for the final task head
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260813-synology-diagnostics.md
validation:
  - command: full PR diff self-review at 928cc8bf79dd149675613ce2f71c4259b70c89f7
    result: PASS
    evidence: workflow is manual-only, fixed-command, read-only, does not emit environment/inspect/file contents, and has no Docker mutation commands
  - command: repository exact-head CI
    result: NOT_RUN
    evidence: PR workflows are running; final exact-head result not yet observed
blockers:
  - none
next_action: inspect exact-head PR CI once it reaches a terminal state; repair only an evidenced task-owned failure
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 928cc8bf79dd149675613ce2f71c4259b70c89f7
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
