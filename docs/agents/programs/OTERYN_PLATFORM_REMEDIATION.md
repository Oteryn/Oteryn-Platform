---
programme_id: OTERYN_PLATFORM_REMEDIATION
programme_version: 1
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
required_reads:
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Remediation — Programme State

## Mission

Consume confirmed, implementation-authorized Platform findings and close them through bounded, non-overlapping, complete vertical slices with independent audit, real E2E when applicable, exact-head CI and terminal PR/task cleanup.

## Durable queue

```yaml
programme_state_version: 1
updated_at: 2026-08-05T13:45:00Z
status: ready
active_claims: []
active_tasks: []
active_pull_requests: []
serialized_coordination_keys: []
parallel_capacity: dynamic
ready_issue_query: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
proven:
  - Every remediation worker must obtain a valid Issue claim and repository task ownership before product mutation.
  - One Issue is assigned to one remediation worker at a time.
derived:
  - Parallel dispatch is safe only for Issues classified parallel_safe with different coordination keys and non-overlapping paths.
unknown:
  - Current count and dependency graph of ready remediation Issues.
conflicts: []
blockers: []
next_action: Query live ready Issues, validate taxonomy and claim state, then select the highest-priority unclaimed non-overlapping work item or a parallel-safe set requested by the owner.
```

## Parallel dispatch checkpoint

When several remediation agents are started, replace the empty structure below with the current wave. Each worker still performs the claim protocol independently.

```yaml
parallel_wave:
  wave_id: none
  requested_workers: 0
  dispatched_issues: []
  coordination_keys: []
  shared_paths: []
  integration_owner: none
  barrier_state: not_started
```

## Programme rules

- Keep one Issue, task, branch and PR per remediation worker.
- Claims and leases are governed by `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md`.
- Do not dispatch more agents than the number of proven independent ready Issues.
- Shared root manifests, lockfiles, route registries, common shells, migrations, global catalogs and CI workflows are serialized unless an explicit integration owner is declared.
- Update this file after claims activate/release, a parallel barrier completes, dependencies change, or before rotation.
- Exactly one `next_action` is required while work remains.
