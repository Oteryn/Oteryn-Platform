---
programme_id: OTERYN_PLATFORM_REMEDIATION
programme_version: 3
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
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
programme_state_version: 3
updated_at: 2026-08-06T07:55:00Z
status: ready
active_claims: []
active_tasks: []
active_pull_requests: []
serialized_coordination_keys: []
parallel_capacity: dynamic
ready_issue_query: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
proven:
  - Every product-remediation worker must obtain a valid Issue claim and repository task ownership before product mutation.
  - One product Issue is assigned to one remediation worker at a time.
  - Issue 547 was repaired through pull request 595 and closed completed by merge commit 5a04d055aa02b74cc741f69713d1ea26c91550c0.
  - Independent audit Issue 597 inspected the exact final pull-request head, found zero critical, high or material-medium findings and closed completed.
  - This programme is immutably scoped to blakinio/Oteryn-Platform by docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md.
  - Eligible lifecycle-only and archive-only reconciliation is governed by docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md and may use one bounded wave PR instead of one PR per completed task.
derived:
  - Parallel product dispatch remains safe only for Issues classified parallel_safe with different coordination keys and non-overlapping paths.
  - PASS-only independent audits should be reviews or comments on existing target PRs, not additional PRs.
unknown:
  - Current count and dependency graph of the remaining ready remediation Issues.
conflicts: []
blockers: []
next_action: Query live ready Issues, validate taxonomy and claim state, route eligible lifecycle-only items into a bounded batch, then select the highest-priority unclaimed non-overlapping product work.
```

## Parallel dispatch checkpoint

When several remediation agents are started, replace the empty structure below with the current wave. Each product worker still performs the claim protocol independently. A lifecycle-only reconciliation wave uses the separate coordinator structure required by `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md`.

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

- Keep one Issue, task, branch and PR per material product/runtime root cause.
- Claims and leases for product work are governed by `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md`.
- Eligible lifecycle-only/archive-only reconciliations are the sole exception: group 2–10 compatible items into one coordinator task, one wave branch, one PR, one fresh exact-head audit and one CI generation under `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md`.
- Do not create one audit PR merely to record PASS. Put the independent verdict on the existing implementation or lifecycle-batch PR and in the linked audit record.
- When an implementer reaches the fresh-audit gate, leave the task `ready` and return `ROTATE` if a separate validator cannot run in that session; do not report `WAITING` unless a real external dependency exists.
- Do not dispatch more product agents than the number of proven independent ready Issues.
- Shared root manifests, lockfiles, route registries, common shells, migrations, global catalogs and CI workflows are serialized unless an explicit integration owner is declared.
- Update this file after claims activate/release, a lifecycle wave changes, a parallel barrier completes, dependencies change, or before rotation.
- Exactly one `next_action` is required while work remains.
