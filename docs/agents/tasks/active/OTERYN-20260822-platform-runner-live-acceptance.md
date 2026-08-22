---
task_id: OTERYN-20260822-platform-runner-live-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
search_first:
  - platform-runners
  - oteryn-synology-platform
  - synology-diagnostics
optional_reads: []
---

# OTERYN-20260822-platform-runner-live-acceptance

## Goal

Prove the Platform organization runner replacement route with one bounded trusted-main read-only workload, without changing Synology runtime state or retiring the legacy rollback runner.

## Acceptance criteria

- [x] registered `synology-diagnostics.yml` now routes by exact `platform-runners` + `oteryn-platform` only behind trusted-main/manual triggers;
- [ ] exact `oteryn-synology-platform` organization registration identity is verified;
- [ ] expected Docker-host and Platform staging-state capabilities are observed read-only;
- [ ] existing staging services are observed without create/update/start/stop/remove/prune or secret/environment output;
- [ ] exact-head repository validation passes and the PR merges normally;
- [ ] trusted-main diagnostics job passes and evidence is recorded to Issue #1215 and parent `Oteryn/Oteryn#34`.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-live-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260822-platform-runner-live-acceptance.md
modules:
  - deployment-operations
dependencies:
  - Oteryn/Oteryn-Platform#1199
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T02:58:00Z
head: UNKNOWN
branch: ops/issue-1215-platform-runner-acceptance
pr: 1216
status: validating`nterminal_pr_policy: archive_pending
context_routes:
  - deployment-operations
owned_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-live-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260822-platform-runner-live-acceptance.md
proven:
  - runner bootstrap #1199/#1213 is merged and archived on current main
  - Platform target contract requires platform-runners + oteryn-platform + oteryn-synology-platform
  - Platform intentionally retains Docker socket and staging-state capability
  - existing registered synology-diagnostics workflow is read-only and can be reused for the replacement proof
  - privileged diagnostics has no pull_request trigger and performs no runtime mutation
  - legacy oteryn-synology-staging remains rollback until all parent gates close
derived:
  - migrating the existing diagnostics workload proves a real Platform-owned replacement route without increasing workflow inventory
unknown:
  - exact trusted-main run/job identity until after merge
conflicts: []
first_failure:
  marker: workflow lifecycle validation
  evidence: CI run 32547593540 rejected an unregistered 54th workflow; root cause was creating a redundant task-specific workflow instead of reusing the registered diagnostics workflow
rejected_hypotheses:
  - PR-triggered privileged runner execution is unsafe and is not used
  - increasing the workflow budget for a one-task acceptance workflow is unnecessary; the registered diagnostics workflow is the reusable owner
changed_paths:
  - .github/workflows/synology-diagnostics.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-live-acceptance.md
validation:
  - command: GitHub exact-head hosted checks
    result: NOT_RUN
    evidence: repaired current head not yet validated
blockers:
  - none
next_action: Archive this implementation record after successor PR #1218 completes trusted-main acceptance.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository bounded task branch
source_branch_evidence: repository delete-branch-on-merge policy will be verified after merge
```

## Notes

Issue: `Oteryn/Oteryn-Platform#1215`. PR: `Oteryn/Oteryn-Platform#1216`. No production action, secret readback, runtime mutation or legacy-runner retirement is authorized by this task.