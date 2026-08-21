---
task_id: OTERYN-20260821-runner-topology-evidence
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
search_first:
  - oteryn-staging
  - self-hosted
  - runner
optional_reads: []
---

# OTERYN-20260821-runner-topology-evidence

## Goal

Provide the live Platform/Synology evidence required by `Oteryn/Oteryn#32` to decide the Oteryn organization runner topology. This task is audit/validation only and must not mutate runner registration, Docker resources, staging runtime, secrets or protected configuration.

## Acceptance criteria

- [ ] Exact live runner version/name/group display and scheduling evidence recorded.
- [ ] Repository-scoped/custom-label-only contract reconciled with current source and prior transfer evidence.
- [ ] Safe live container facts prove Docker-socket, runtime user and persistent runner/work boundaries without secret disclosure.
- [ ] Existing `oteryn-staging` workflow triggers are classified for public-repository trust exposure.
- [ ] Node.js 24 / Actions Runner >=2.327.1 compatibility is decided from live version evidence.
- [ ] No self-hosted use outside Platform is found or every exception is recorded.
- [ ] Organization-vs-repository desired state and migration blocker/capability are recorded back to META #32.
- [ ] Temporary probe workflow is deleted before terminal merge/closeout.

## Ownership

```yaml
owned_paths:
  - .github/workflows/runner-topology-probe.yml
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/tasks/archive/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md
modules:
  - synology-staging-runner
  - github-actions-runner-routing
dependencies:
  - Oteryn/Oteryn#32
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#32
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-21T07:43:00Z
head: 59ddc57bf6611bced636ac8ad621415a4c462908
branch: audit/issue-1194-runner-topology-evidence
pr: 1198
status: validating
phase: validate
task_kind: audit
implementation_authorized: false
session_id: runner-topology-20260821-001
session_role: auditor
execution_mode: github-only
execution_reason: local/Remote Desktop access is unavailable; trusted GitHub Actions and existing repository evidence provide the bounded read-only execution path
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/runner-topology-probe.yml
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/tasks/archive/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md
proven:
  - Platform PR 1164 verified a repository-scoped Synology runner named oteryn-synology-staging using custom label oteryn-staging.
  - PR 1176 restored --no-default-labels and explicitly excludes generic self-hosted eligibility.
  - trusted-main run 32454899481 scheduled oteryn-synology-staging and logged runner version 2.336.0.
  - current runner compose mounts /var/run/docker.sock plus persistent /runner and /work and the staging-state path.
  - current runner entrypoint accepts only exact repository RUNNER_URL values and has no organization URL/runner-group registration path.
derived:
  - live runner version 2.336.0 satisfies the >=2.327.1 prerequisite for Node.js 24 based Actions.
unknown:
  - exact live runner container user/mount realization and sanitized .runner registration metadata on the current Synology container.
  - whether the connected GitHub App has organization Self-hosted runners write permission; no runner administration action is exposed by the current connector surface.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - runner_group_name Default alone proves organization scope; prior transfer evidence and registration source explicitly identify repository scope.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
validation:
  - command: inspect trusted-main job log 96690198992
    result: PASS
    evidence: runner oteryn-synology-staging, group display Default, version 2.336.0, run 32454899481
blockers:
  - none
next_action: Add the temporary read-only runner-topology probe workflow; inspect its single resulting runner job and then remove the workflow.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: runner-topology-20260821-001
  session_started_at: 2026-08-21T07:40:00Z
  checkpointed_at: 2026-08-21T07:43:00Z
  last_progress_at: 2026-08-21T07:43:00Z
  phase: validate
  exact_head: 59ddc57bf6611bced636ac8ad621415a4c462908
  pull_request: 1198
  active_operation: create one temporary read-only runner-topology probe workflow on the owned branch
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: 2026-08-21T08:03:00Z
  check_generation: runner-topology-probe
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: probe job is created or completed for the exact branch head
  next_action: inspect the probe run once, record sanitized evidence, then remove the temporary workflow
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: audit evidence branch; temporary probe must be removed before merge
source_branch_evidence: pending
```
