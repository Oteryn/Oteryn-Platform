---
task_id: OTERYN-20260822-platform-runner-availability
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - runner recovery workflows
optional_reads: []
---

# OTERYN-20260822-platform-runner-availability

## Goal

Restore the existing organization-scoped Platform Synology runner to a schedulable state and prove the trusted-main `Synology Diagnostics` route for parent `Oteryn/Oteryn#34` without changing product runtime or organization ACLs.

## Acceptance criteria

- [ ] Observe exact Platform runner container identity/state through the retained legacy control route without exposing secrets.
- [ ] Perform only a bounded restart of `oteryn-organization-runners-platform-1` when the replacement runner is unschedulable.
- [ ] Prove trusted-main `Synology Diagnostics` runs successfully on `platform-runners` / `oteryn-platform` / `oteryn-synology-platform`.
- [ ] Preserve Platform application containers, state, volumes, and organization runner ACL/registration configuration.
- [ ] Archive this task and close Issue #1217 after terminal evidence.

## Ownership

```yaml
owned_paths:
  - .github/workflows/organization-runner-platform-recovery.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-availability.md
modules:
  - ci-runner-control
dependencies:
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T08:20:00Z
head: 62d134a71fa5b480249ffbffbb81079aede4be34
branch: infra/issue-1217-platform-runner-recovery
pr: none
status: implementing
context_routes:
  - ci-repair
  - testing
owned_paths:
  - .github/workflows/organization-runner-platform-recovery.yml
  - docs/agents/tasks/active/OTERYN-20260822-platform-runner-availability.md
proven:
  - Platform main Synology Diagnostics run 32548013556 job 96969892573 has remained queued with no assigned runner.
  - Previous organization seal evidence proved oteryn-synology-platform online before the regression.
  - Legacy oteryn-staging remains the documented rollback/control route.
derived:
  - Replacement Platform runner availability regressed after the prior seal evidence.
unknown:
  - Exact current Docker container state and reconnect behavior.
conflicts: []
first_failure:
  marker: platform-diagnostics-queued
  evidence: run 32548013556 job 96969892573
rejected_hypotheses: []
changed_paths: []
validation:
  - command: not-run
    result: NOT_RUN
    evidence: repair workflow not yet executed
blockers:
  - none
next_action: Push the guarded recovery workflow and observe its exact legacy-runner execution.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary bounded recovery PR
source_branch_evidence: pending
```

## Notes

Issue: `Oteryn/Oteryn-Platform#1217`. The recovery workflow may inspect and restart only the existing Platform organization-runner container. Product application containers and persistent data are out of scope.
