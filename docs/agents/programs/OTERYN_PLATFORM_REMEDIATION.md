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
updated_at: 2026-08-06T10:04:00Z
status: ready
live_state_snapshot:
  mode: live_query_required
  exhaustive: false
  active_claims: unknown
  active_tasks: unknown
  active_pull_requests: unknown
  serialized_coordination_keys: unknown
  reason: Mutable ownership and queue state must be resolved from live Issues, PRs, branches and active task records at invocation time; unknown must never be interpreted as an empty queue.
parallel_capacity: dynamic
live_queries:
  ready_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
  active_claim_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:claimed'
  open_remediation_pull_requests: 'repo:blakinio/Oteryn-Platform is:pr is:open label:programme:audit-repair'
  active_tasks_path: docs/agents/tasks/active/
proven:
  - Every product-remediation worker must obtain a valid Issue claim and repository task ownership before product mutation.
  - One product Issue is assigned to one remediation worker at a time.
  - Issue 547 was repaired through pull request 595 and closed completed by merge commit 5a04d055aa02b74cc741f69713d1ea26c91550c0.
  - Independent audit Issue 597 inspected the exact final pull-request head, found zero critical, high or material-medium findings and closed completed.
  - Lifecycle closeout PRs 598, 601 and 670 are terminal merged work and must not be represented as active claims.
  - This programme is immutably scoped to blakinio/Oteryn-Platform by docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md.
  - Eligible lifecycle-only and archive-only reconciliation is governed by docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md and may use one bounded wave PR instead of one PR per completed task.
derived:
  - Parallel product dispatch remains safe only for Issues classified parallel_safe with different coordination keys and non-overlapping paths.
  - PASS-only independent audits should be reviews or comments on existing target PRs, not additional PRs.
  - A stored empty mutable queue is unsafe unless it is proven from a same-generation live query; this file therefore records unknown live state rather than a false empty snapshot.
unknown:
  - Current count, ownership and dependency graph of ready or active remediation work until the live queries are executed.
conflicts: []
blockers: []
next_action: Query live ready and claimed Issues, open remediation PRs, branches and active task records; reconcile terminal work; then route eligible lifecycle-only items into a bounded batch or select the highest-priority safe unclaimed product Issue.
```

## Parallel dispatch checkpoint

The structure below is a template, not a claim that no wave exists. Resolve live state before dispatch. Each product worker still performs the claim protocol independently. A lifecycle-only reconciliation wave uses the separate coordinator structure required by `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md`.

```yaml
parallel_wave_template:
  wave_id: <resolve-from-live-state>
  requested_workers: <resolve-from-owner-command>
  dispatched_issues: <resolve-from-live-state>
  coordination_keys: <resolve-from-live-state>
  shared_paths: <resolve-from-live-state>
  integration_owner: <resolve-from-live-state>
  barrier_state: <resolve-from-live-state>
```

## Programme rules

- Keep one Issue, task, branch and PR per material product/runtime root cause.
- Claims and leases for product work are governed by `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md`.
- Eligible lifecycle-only/archive-only reconciliations are the sole exception: group 2–10 compatible items into one coordinator task, one wave branch, one PR, one fresh exact-head audit and one CI generation under `docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md`.
- Do not create one audit PR merely to record PASS. Put the independent verdict on the existing implementation or lifecycle-batch PR and in the linked audit record.
- When an implementer reaches the fresh-audit gate, leave the task `ready` and return `ROTATE` if a separate validator cannot run in that session; do not report `WAITING` unless a real external dependency exists.
- Never infer that no work is active from an absent, stale or unknown snapshot. Query live Issues, PRs, branches and active task records before claiming or dispatching.
- Do not dispatch more product agents than the number of proven independent ready Issues.
- Shared root manifests, lockfiles, route registries, common shells, migrations, global catalogs and CI workflows are serialized unless an explicit integration owner is declared.
- Update this file after a durable programme-policy change; mutable queue and ownership truth remains live-query-derived.
- Exactly one `next_action` is required while work remains.
