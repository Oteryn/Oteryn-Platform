---
programme_id: OTERYN_PLATFORM_REMEDIATION
programme_version: 4
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Remediation — Programme State

## Mission

Consume confirmed implementation-authorized Platform findings and close them through bounded, non-overlapping, complete slices with atomic Issue ownership, economical PR delivery, eligible independent audit, real applicable E2E, exact-head CI and terminal cleanup.

## Durable state

```yaml
programme_state_version: 4
updated_at: 2026-08-06T11:45:00Z
status: ready
live_state_snapshot:
  mode: live_query_required
  exhaustive: false
  active_claims: unknown
  active_tasks: unknown
  active_pull_requests: unknown
  open_repair_trains: unknown
  ready_audit_handoffs: unknown
  serialized_coordination_keys: unknown
  reason: Mutable ownership, delivery and audit queues must be resolved from live Issues, branches, tasks, PRs, reviews and exact heads at invocation time; unknown never means empty.
parallel_capacity: dynamic
live_queries:
  ready_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
  active_claim_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:claimed'
  open_remediation_pull_requests: 'repo:blakinio/Oteryn-Platform is:pr is:open label:programme:audit-repair'
  active_tasks_path: docs/agents/tasks/active/
  audit_handoff_marker: audit_handoff
  repair_delivery_marker: repair_delivery
proven:
  - Every implementation worker must independently acquire deterministic branch repair/issue-<number> before product mutation.
  - The deterministic Git ref remains the atomic race arbiter; comments, labels and coordinator dispatch are not locks.
  - One Issue belongs to one active worker claim and source branch at a time.
  - Claim activation does not universally require a Pull Request.
  - Existing authoritative delivery PR reuse precedes train or dedicated PR creation.
  - Compatible independently claimed repairs may use a bounded repair train with one integration owner and immutable accepted source heads.
  - PASS-only independent audits are reviews/comments on the existing exact target PR, not additional PRs.
  - Eligible lifecycle-only/archive-only reconciliation follows docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md.
  - The remediation programme is immutably scoped to blakinio/Oteryn-Platform by docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md.
derived:
  - Parallel implementation dispatch is safe only for ready authorized Issues with distinct coordination keys and non-overlapping paths.
  - Repair workers must rotate after durable audit handoff rather than staying active while waiting for another role.
  - A dedicated audit role should drain valid ready audit handoffs independently from implementation workers.
  - A stored empty mutable queue is unsafe unless proven by same-generation live queries.
unknown:
  - Current ready, claimed, train and audit-handoff counts until live queries run.
conflicts: []
blockers: []
next_action: Query live ready/claimed Issues, deterministic branches, active tasks, related PRs, repair trains and ready audit handoffs; drain the highest-priority valid audit handoff or select the highest-priority safe unclaimed implementation Issue according to the invoked role.
```

## Role queues

```yaml
role_queues:
  implementation:
    source: ready authorized unclaimed Issues
    ownership: deterministic branch plus Issue activation plus active task
    terminal_phase_handoff: audit_handoff
  independent_audit:
    source: valid ready audit_handoff records
    mode: AUDIT_ONLY
    mutation_allowed: false
    result: PASS_ZERO_MATERIAL_FINDINGS or exact findings
  integration:
    source: compatible coherent train candidates
    owner_count: 1
    accepted_input: exact immutable worker source heads
  lifecycle_closeout:
    source: terminal governance-only items
    contract: docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
```

## Parallel dispatch template

```yaml
parallel_wave_template:
  wave_id: <live-state-derived>
  requested_mode: implementation_workers | total_slots
  requested_count: <owner command>
  implementation_workers: <proven safe count>
  audit_workers: <0 or at least 1 when valid handoffs exist>
  integration_coordinator: <0 or 1>
  dispatched_issues: <live-state-derived>
  ready_audit_handoffs: <live-state-derived>
  coordination_keys: <live-state-derived>
  shared_paths: <live-state-derived>
  repair_train_ids: <live-state-derived>
  barrier_state: <live-state-derived>
```

Literal `N agentów naprawczych` means up to N implementation workers. `N slotów naprawy` means total roles and uses the allocation in `SHORT_PROGRAM_INVOCATIONS.md`.

## Programme rules

- Claims and leases follow `REMEDIATION_WORK_CLAIM_PROTOCOL.md` version 3.
- PR selection, repair trains, delivery mapping, rollback, independent audit and role rotation follow `REPAIR_PR_ECONOMY.md`.
- Search open and closed related PRs before creating a new one.
- Do not create a PR solely because an Issue was claimed.
- Do not create duplicate implementation, audit-only, evidence-only, ownership-release-only or per-Issue archive-only PRs.
- Keep dedicated PR boundaries for security, auth, payments, migrations, protocol/API authority, dependencies, CI/workflow, architecture, protected rollout and unclear rollback.
- A coherent candidate does not wait for another candidate merely to fill a train.
- A worker that loses its deterministic branch race releases immediately and selects another eligible Issue when authorized.
- Exactly one integration owner writes a train branch; accepted worker source heads are immutable per train generation.
- Train freeze rejects new Issues and silent source-head drift.
- Final audit requires a distinct eligible `AUDIT ONLY` agent/session, exact PR/base/head, whole-diff verdict and per-Issue verdicts.
- Implementers, train workers and the integration owner cannot self-issue the required final PASS.
- A finding returns to implementation on the same delivery PR unless a separate root cause/authority boundary is proven.
- At the audit gate, persist handoff, set task `ready` and return `ROTATE` when a distinct auditor cannot run in the session.
- `WAITING` is only for genuine external dependency/actor, permission/environment, observation window, protected operation, owner decision or exhausted bounded terminal-CI procedure.
- Same-PR pre-merge archival uses `completed_on_merge`; closing without merge cannot leave completion or ownership release.
- Query live state before every claim, train acceptance, audit pickup, merge and terminal completion.
- Exactly one `next_action` is required while work remains.

## Success metrics

```yaml
metrics:
  duplicate_implementation_prs: 0
  audit_only_prs_per_repair: 0
  archive_only_prs_per_repair: 0
  unintentionally_open_related_prs_at_completion: 0
  workers_implementing_same_issue: 0
  trains_reverted_for_incoherent_scope: 0
  repair_prs_per_completed_issue: normally_lte_1
```

Metrics never override safety, reviewability, rollback, audit, E2E, exact-head CI or closeout.
