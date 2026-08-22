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
optional_reads: []
---

# OTERYN-20260822-platform-runner-live-acceptance

## Goal

Prove the Platform organization runner replacement route with one bounded trusted-main read-only workload, without changing Synology runtime state or retiring the legacy rollback runner.

## Acceptance criteria

- [ ] exact `platform-runners` + `oteryn-platform` routing is committed only behind trusted-main/manual triggers;
- [ ] exact `oteryn-synology-platform` organization registration identity is verified;
- [ ] expected Docker-host and Platform staging-state capabilities are observed read-only;
- [ ] existing staging services are observed without create/update/start/stop/remove/prune or secret/environment output;
- [ ] exact-head repository validation passes and the PR merges normally;
- [ ] trusted-main acceptance job passes and evidence is recorded to Issue #1215 and parent `Oteryn/Oteryn#34`.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-platform-runner-acceptance.yml
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
updated_at: 2026-08-22T02:54:00Z
head: UNKNOWN
branch: ops/issue-1215-platform-runner-acceptance
pr: none
status: implementing
context_routes:
  - deployment-operations
owned_paths:
  - .github/workflows/synology-platform-runner-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-live-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260822-platform-runner-live-acceptance.md
proven:
  - runner bootstrap #1199/#1213 is merged and archived on current main
  - Platform target contract requires platform-runners + oteryn-platform + oteryn-synology-platform
  - Platform intentionally retains Docker socket and staging-state capability
  - legacy oteryn-synology-staging remains rollback until all parent gates close
derived:
  - a trusted-main read-only workflow can prove the remaining Platform live-execution gate without product mutation
unknown:
  - exact trusted-main run/job identity until after merge
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - PR-triggered privileged runner execution is unsafe and is not used
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-live-acceptance.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: workflow not yet implemented
blockers:
  - none
next_action: add trusted-main-only read-only Platform runner acceptance workflow
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository bounded task branch
source_branch_evidence: repository delete-branch-on-merge policy will be verified after merge
```

## Notes

Issue: `Oteryn/Oteryn-Platform#1215`. No production action, secret readback, runtime mutation or legacy-runner retirement is authorized by this task.