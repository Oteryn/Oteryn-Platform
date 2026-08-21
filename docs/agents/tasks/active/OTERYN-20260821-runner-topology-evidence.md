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

Provide the live Platform/Synology evidence required by `Oteryn/Oteryn#32` to decide the Oteryn organization runner topology. Audit/validation only: no runner registration, Docker-resource, staging-runtime, secret or protected-configuration mutation.

## Acceptance criteria

- [x] Exact live runner version/name/group display and scheduling evidence recorded.
- [x] Repository-scoped/custom-label-only contract reconciled with current source and prior transfer evidence.
- [x] Safe live container facts prove Docker-socket, runtime user and persistent runner/work boundaries without secret disclosure.
- [x] Existing `oteryn-staging` workflow triggers are classified for public-repository trust exposure.
- [x] Node.js 24 / Actions Runner >=2.327.1 compatibility is decided from live version evidence.
- [x] No self-hosted use outside Platform is found.
- [x] Organization-vs-repository desired state and migration capability are recorded back to META #32.
- [x] Temporary probe workflow/retained-workflow changes are removed from the terminal diff.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/tasks/archive/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md
modules:
  - synology-staging-runner
  - github-actions-runner-routing
dependencies:
  - Oteryn/Oteryn#32
follow_ups:
  - Oteryn/Oteryn-Platform#1199
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#32
```

## Terminal audit verdict

`KEEP_REPOSITORY_SCOPED_PLATFORM_RUNNER`

- META, Game and Atlas remain GitHub-hosted only.
- Platform keeps the repository-scoped `oteryn-synology-staging` runner for trusted Synology/staging operations only.
- Do not create Game/Atlas self-hosted runners now.
- Do not migrate the current privileged Platform runner to organization scope now.
- Revisit organization runner groups only if another repository later proves a real host-local workload.

Canonical detailed evidence: `docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md`.
Hardening follow-up: #1199 (`RUNNER-001` and `RUNNER-002`).

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-21T07:55:00Z
head: 59c8bd6fd2e6ae537b10a864b07b241f42e7d07b
branch: audit/issue-1194-runner-topology-evidence
pr: 1198
status: completed
phase: closeout
task_kind: audit
implementation_authorized: false
session_id: runner-topology-20260821-001
session_role: auditor
execution_mode: github-only
execution_reason: trusted GitHub Actions plus repository evidence supplied the bounded live read-only execution path
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/tasks/archive/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md
proven:
  - Platform PR 1164 established repository-scoped runner attachment after transfer.
  - PR 1176 established custom-label-only scheduling with --no-default-labels.
  - trusted-main run 32454899481 job 96690198992 reports runner oteryn-synology-staging version 2.336.0.
  - bounded live probe run 32460223728 job 96705516889 reports Linux/X64, root container user, running/always state, Docker client 29.6.2, Docker server 24.0.2, Compose 5.3.1, and RW /runner, /work, Docker socket and staging-state mounts.
  - live runner image is ghcr.io/blakinio/oteryn-deploy-runner:main with image ID sha256:bad8dc119e39553f5a9d958834562a44add4978e16f9a46df7c89507c06c24b8.
  - current runner entrypoint accepts repository-shaped RUNNER_URL and has no organization/runner-group registration path.
  - no retained self-hosted routing was found in META, Game or Atlas.
  - permanent Platform pull-request paths do not execute on the privileged Synology runner; live jobs are manual/trusted-main/environment bounded.
  - temporary probe workflow and temporary retained-workflow extension are absent from the final branch diff.
derived:
  - runner 2.336.0 satisfies the >=2.327.1 Node.js 24 Actions prerequisite.
  - sharing the root/Docker-socket runner organization-wide would broaden trust without demonstrated need.
unknown:
  - live .credentials permission metadata was intentionally not captured; secret contents were never read.
  - current connector exposes no organization runner/group/registration-token mutation action; this is not needed by the chosen desired state.
conflicts: []
first_failure:
  marker: optional .runner JSON parse in probe
  evidence: run 32460223728 job 96705516889 failed on UTF-8 BOM after all required live host/container evidence had already been emitted
rejected_hypotheses:
  - runner_group_name Default proves organization scope.
  - one self-hosted runner per permanent repository is required.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260821-runner-topology-evidence.md
  - docs/agents/reports/OTERYN-20260821-runner-topology-evidence.md
validation:
  - command: trusted-main live runner job 96690198992
    result: PASS
    evidence: runner name/version/scheduling and running container state
  - command: extract required bounded runner/container facts from probe log 96705516889 before the optional metadata parser
    result: PASS
    evidence: required live facts were emitted completely; later UTF-8 BOM failure affected only non-required selected .runner JSON fields
  - command: compare main 3f1a0eeb42a777106bef466dbcb4150d8a1bb818 to cleaned audit branch before report finalization
    result: PASS
    evidence: temporary workflow changes absent; final intended diff is audit task/report only
  - command: publish organization verdict to Oteryn/Oteryn#32
    result: PASS
    evidence: META Issue #32 comment 5366930246
blockers:
  - none
next_action: Run exact-final-head repository checks for PR 1198, merge the audit evidence when policy gates permit, then archive this task and close #1194.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 5
  session_id: runner-topology-20260821-001
  session_started_at: 2026-08-21T07:40:00Z
  checkpointed_at: 2026-08-21T07:55:00Z
  last_progress_at: 2026-08-21T07:55:00Z
  phase: closeout
  exact_head: pending-this-commit
  pull_request: 1198
  active_operation: exact-final-head validation and protected merge of audit-only evidence
  external_run_ids:
    - 32454899481
    - 32460223728
  operation_started_at: 2026-08-21T07:55:00Z
  wait_deadline_at: null
  check_generation: final-audit-docs-v2
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR 1198 remains open on the audit branch and its exact-head checks are inspected
  next_action: validate final head, merge if allowed, then perform lifecycle archive closeout
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded audit evidence branch; repository delete_branch_on_merge owns terminal deletion
source_branch_evidence: pending protected merge
```
