---
programme_id: OTERYN_PLATFORM_REMEDIATION
programme_version: 5
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/REPAIR_PR_ECONOMY.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Remediation — Programme State

## Mission

Consume confirmed implementation-authorized findings through one accountable implementation owner per Issue, complete vertical-slice delivery, documented self-review, selective risk-gated independent audit, applicable real E2E, exact-head CI and terminal closeout.

## Durable state

```yaml
programme_state_version: 5
updated_at: 2026-08-06T14:10:00Z
status: ready
live_state_snapshot:
  mode: live_query_required
  exhaustive: false
  active_claims: unknown
  active_tasks: unknown
  active_pull_requests: unknown
  required_audit_handoffs: unknown
  optional_requested_audits: unknown
  serialized_coordination_keys: unknown
  reason: Mutable ownership, delivery and audit queues are resolved from live Issues, deterministic branches, tasks, PRs, audit gates, reviews and exact heads; unknown never means empty.
parallel_capacity: dynamic
live_queries:
  ready_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:ready'
  active_claim_issues: 'repo:blakinio/Oteryn-Platform is:issue is:open label:programme:platform label:programme:audit-repair label:agent:claimed'
  open_remediation_pull_requests: 'repo:blakinio/Oteryn-Platform is:pr is:open label:programme:audit-repair'
  active_tasks_path: docs/agents/tasks/active/
  audit_gate_marker: audit_gate
  required_audit_handoff_marker: audit_handoff
proven:
  - Deterministic branch repair/issue-<number> is the atomic Issue lock.
  - One Issue has one active implementation owner at a time.
  - The implementation owner remains responsible through implementation, self-review, validation, PR, findings remediation, merge, Issue closure, archival and release.
  - Claim activation does not require an activity-only Pull Request.
  - One Issue normally uses one delivery PR.
  - Every repair requires documented self-review.
  - Independent audit is selected by docs/agents/REMEDIATION_AUDIT_RISK_GATE.md.
  - Mandatory audit triggers cannot be waived by the implementation owner.
  - PASS-only audit evidence belongs on the existing delivery PR and durable audit record.
  - Active repair trains are exceptional and not the ordinary product-remediation path.
  - Lifecycle-only terminal reconciliation follows docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md.
derived:
  - Parallel commands should allocate end-to-end implementation owners rather than permanent specialist slots.
  - Audit workers are needed only while valid REQUIRED or explicitly requested OPTIONAL handoffs exist.
  - An audit finding returns to the same implementation owner and does not transfer Issue ownership.
  - A stored empty queue is unsafe without same-generation live proof.
unknown:
  - Current ready, claimed and required-audit counts until live queries run.
conflicts: []
blockers: []
next_action: Query live ready/claimed Issues, active tasks, deterministic branches, related PRs and audit gates; resume the current valid Issue-owned task, otherwise select the highest-priority safe unclaimed Issue, or drain a valid required audit only when the invoked role is AUDIT ONLY.
```

## Role queues

```yaml
role_queues:
  implementation_owner:
    source: ready authorized unclaimed Issues or valid owned continuation
    ownership: deterministic branch plus Issue activation plus active task
    responsibility: claim through terminal closeout
  selective_independent_audit:
    source: REQUIRED handoffs or OPTIONAL handoffs explicitly requested
    mode: AUDIT_ONLY
    mutation_allowed: false
    issue_ownership_transfer: forbidden
    result: PASS_ZERO_MATERIAL_FINDINGS or exact findings
  lifecycle_closeout:
    source: terminal governance-only items
    contract: docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
  exceptional_train_integration:
    source: explicitly authorized homogeneous low-risk candidates only
    default_enabled: false
```

## Parallel dispatch template

```yaml
parallel_wave_template:
  wave_id: <live-state-derived>
  requested_implementation_owners: <owner command>
  dispatched_issues: <live-state-derived>
  implementation_owners: <proven safe count>
  required_audit_handoffs: <live-state-derived>
  audit_workers_allocated_now: <0 or proven needed count>
  coordination_keys: <live-state-derived>
  shared_paths: <live-state-derived>
  barrier_state: <live-state-derived>
```

`N agentów naprawczych` means up to `N` end-to-end implementation owners. There is no default slot subtraction for auditors or integrators.

## Programme rules

- Claims and leases follow claim protocol version 4.
- Delivery and PR economy follow `REPAIR_PR_ECONOMY.md` version 2.
- Audit classification follows `REMEDIATION_AUDIT_RISK_GATE.md` version 1.
- Taxonomy and Issue metadata follow taxonomy version 1.4 / schema version 4.
- One implementation owner keeps one Issue from claim through closeout.
- Search open and closed related PRs before creating a new one.
- Do not create a PR solely to prove activity.
- Normally use one Issue-owned PR.
- Always perform documented self-review.
- `NOT_REQUIRED` needs evidence disproving all mandatory triggers.
- `OPTIONAL` may be requested; otherwise record `NOT_REQUESTED` with rationale.
- `REQUIRED` needs a distinct exact-target read-only auditor before merge.
- An implementation owner cannot waive a mandatory trigger or call self-review independent audit.
- Audit findings return to the same implementation owner.
- At a required audit boundary, persist handoff, set task ready and use `ROTATE`, not `WAITING`.
- No audit slot is permanently reserved; allocate one only for a valid handoff.
- A worker that loses its deterministic branch race releases immediately and selects another eligible Issue when authorized.
- Ordinary product/runtime work does not enter repair trains.
- Exceptional trains require explicit coordinator authorization and low-risk homogeneous scope.
- Same-PR pre-merge archival uses `completed_on_merge`; closure without merge cannot release ownership.
- Query live state before claim, audit pickup, merge and completion.
- Exactly one `next_action` is required while work remains.

## Success metrics

```yaml
metrics:
  issues_with_one_end_to_end_owner: 100_percent
  repairs_with_documented_self_review: 100_percent
  duplicate_implementation_prs: 0
  audit_only_prs_per_repair: 0
  archive_only_prs_per_repair: 0
  mandatory_audits_missing_independent_pass: 0
  self_reviews_mislabeled_independent: 0
  idle_reserved_audit_slots: 0
  workers_waiting_for_internal_roles: 0
  ordinary_product_repairs_in_trains: 0
  repair_prs_per_completed_issue: normally_lte_1
```

Metrics never override safety, E2E, exact-head CI, rollback, reviewability or complete closeout.
