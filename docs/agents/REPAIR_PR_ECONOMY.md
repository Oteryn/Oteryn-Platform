# Repair Pull Request Economy and Issue-Owned Delivery Contract

```yaml
repair_pr_economy_version: 2
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
audit_gate: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
controlling_specialization_over:
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/agents/LIFECYCLE_CLOSEOUT_BATCHING.md
objective: deliver one Issue through one accountable implementation owner and normally one PR while using independent audit only when risk requires it
```

## Default model

Ordinary remediation uses:

```text
one Issue → one implementation owner → one deterministic branch → one delivery PR → one terminal closeout
```

The implementation owner is accountable for:

- reproducing or proving the root cause;
- defining observable acceptance;
- implementing the complete bounded repair;
- self-review, focused/component validation and applicable E2E;
- creating and maintaining the delivery PR;
- repairing CI, review and independent-audit findings;
- verifying exact-head gates;
- merging when authorized;
- closing the Issue, archiving the task and releasing ownership.

An auditor, reviewer or integration helper does not take ownership of the Issue. Findings return to the same implementation owner unless an explicit evidence-backed takeover is required.

## Atomic ownership

The deterministic branch `repair/issue-<ISSUE_NUMBER>` remains the atomic Issue lock. Labels, assignees, comments, task files, PRs and coordinator intent are not replacements for Git-ref arbitration.

Every Issue retains one active claim nonce, task, branch, path set, acceptance inventory, rollback mapping and owner. A worker that loses the deterministic branch race releases immediately and selects another eligible Issue when authorized.

## Pull Request policy

A PR is a delivery and review artifact, not an ownership lock. It is not required merely to prove activity.

Before creating a PR:

1. search open and closed related PRs;
2. reuse an authoritative compatible PR when one exists;
3. otherwise continue branch-only until a coherent reviewable candidate exists;
4. open one dedicated delivery PR for the Issue.

One Issue normally has at most one active delivery PR. A replacement PR does not terminally reconcile the previous one; duplicates and superseded attempts must be closed accurately.

Open or reuse the PR early when PR-triggered CI, early high-risk review, integration visibility or a coherent candidate makes it useful.

## Delivery mapping

Every remediation PR records:

```yaml
repair_delivery:
  version: 2
  mode: issue_owned | reused_existing | exceptional_repair_train
  implementation_owner: <claim/session>
  issue:
    number: <issue>
    claim_nonce: <claim>
    task_id: <task>
    source_branch: repair/issue-<number>
    source_head: <sha>
    coordination_key: <key>
    owned_paths:
      - <path>
    acceptance_evidence:
      - <evidence>
    rollback:
      strategy: <bounded strategy>
  self_review:
    result: PENDING | PASS | FAIL
    exact_head: <sha or pending>
    evidence: []
  audit_gate:
    requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
    mandatory_triggers: []
    optional_triggers: []
    rationale: <evidence>
    independent_audit:
      result: NOT_REQUIRED | NOT_REQUESTED | PENDING | PASS | FAILED
      generation: 0
      evidence: []
  e2e:
    result: PASS | NOT_APPLICABLE | PENDING | FAILED
    reason: <reason or null>
  final_ci_head: <sha or pending>
  archive_state:
    status: active | completed_on_merge | completed
    effective_when:
      pull_request: <number>
      exact_head: <sha>
      merged: true
```

Human prose may supplement but cannot replace this mapping.

## Self-review

Every implementation owner performs a complete self-review on the exact candidate head. Self-review inspects acceptance, full diff, negative paths, rollback, related PRs, task/Issue state and validation evidence.

Self-review findings are repaired by the same owner. Self-review must never be described as independent audit.

## Selective independent audit

`REMEDIATION_AUDIT_RISK_GATE.md` decides whether a distinct auditor is required.

- `NOT_REQUIRED`: self-review plus applicable E2E and exact-head CI may satisfy the audit gate.
- `OPTIONAL`: the owner/coordinator may request a distinct audit; otherwise record `NOT_REQUESTED` with rationale.
- `REQUIRED`: publish an exact audit handoff and obtain an unchanged-target independent PASS before merge.

The implementation owner cannot waive mandatory triggers. When audit is required, Issue ownership remains with the implementation owner. Findings return to that owner, and a changed target creates a new audit generation.

A PASS-only independent audit is a review/comment on the delivery PR plus durable evidence. It never creates an audit-only PR.

## Audit handoff

For `REQUIRED` audit:

```yaml
audit_handoff:
  version: 2
  audit_generation: <integer>
  target:
    repository: blakinio/Oteryn-Platform
    pull_request: <number>
    base_sha: <sha>
    head_sha: <sha>
  implementation:
    owner: <claim/session>
    issue: <number>
    claim_nonce: <claim>
    task_id: <task>
  audit_gate:
    requirement: REQUIRED
    mandatory_triggers:
      - <trigger>
  scope:
    changed_paths:
      - <path>
    required_falsification_cases:
      - <case>
  evidence:
    self_review: PASS
    validation_runs:
      - <run or artifact>
    unresolved_threads: 0
  auditor_requirements:
    mode: AUDIT_ONLY
    independent_from_implementation_owner: true
    exact_target_required: true
```

The auditor records exact identity, independence, target, whole-diff verdict, Issue verdict and material findings. An auditor who mutates or remediates the target loses final-auditor eligibility for that generation.

## Repair trains

Active repair trains are not the normal product-remediation path.

A repair train requires explicit coordinator authorization and is limited to homogeneous low-risk mechanical, documentation, test-fixture or governance work. It is forbidden for authentication/security authority, payments, migrations/schema authority, public API/protocol/generated contracts, dependencies/lockfiles, CI/deployment, production, durable architecture or atomic cross-repository work.

Train rules:

- default size 2–3 Issues;
- every Issue keeps its own owner, claim, task and source branch;
- exactly one integration owner writes the train branch;
- exact accepted source heads and per-Issue rollback remain verifiable;
- no coherent repair waits merely to fill a train;
- any mandatory audit trigger applies to the combined target;
- lifecycle-only terminal reconciliation uses `LIFECYCLE_CLOSEOUT_BATCHING.md` instead.

Without explicit authorization and full eligibility, use one Issue-owned PR.

## Parallel remediation

A request for `N` repair agents means up to `N` independent end-to-end implementation owners. Each claims one distinct Issue and continues it through terminal closeout.

No audit slot is permanently reserved. A separate audit role is allocated only when a valid `REQUIRED` handoff exists. A worker does not wait for peers, train capacity or an auditor; it persists durable state and returns `ROTATE` at a required role boundary.

## Merge gate

Every repair requires:

- exact coherent scope and ownership;
- self-review PASS;
- applicable focused/component validation;
- real applicable E2E or justified `NOT_APPLICABLE`;
- required exact-head CI PASS;
- zero unresolved material findings and review threads;
- terminal related PR states;
- bounded rollback;
- complete Issue/task/ownership closeout.

A `REQUIRED` audit additionally requires a valid independent PASS on the unchanged target. A `NOT_REQUIRED` or unrequested `OPTIONAL` decision requires complete audit-gate evidence proving no mandatory trigger remains.

## Lifecycle closeout

Task archival included before merge uses `completed_on_merge` and becomes completed only when the named PR exact head merges. Closing without merge cannot release ownership or mark completion.

Unavoidable post-merge governance-only reconciliation is consolidated under `LIFECYCLE_CLOSEOUT_BATCHING.md`; do not create one archive PR per Issue.

## Prohibited patterns

Do not:

- split one ordinary Issue between implementation, integration and closeout owners without a proven takeover need;
- require independent audit for every trivial repair;
- waive a mandatory audit trigger;
- label self-review as independent;
- create a PR solely as an activity signal;
- create duplicate delivery, audit-only, evidence-only or archive-only PRs;
- use repair trains as the default product path;
- keep coherent work open to wait for a train;
- use `WAITING` for an internal role transition;
- close an Issue merely because a PR exists.

## Metrics

```yaml
repair_economy_targets:
  issues_with_one_end_to_end_owner: 100_percent
  repairs_with_documented_self_review: 100_percent
  duplicate_implementation_prs: 0
  audit_only_prs_per_repair: 0
  archive_only_prs_per_repair: 0
  mandatory_audit_triggers_without_independent_pass: 0
  self_reviews_mislabeled_independent: 0
  ordinary_product_repairs_in_trains: 0
  workers_waiting_for_internal_roles: 0
  repair_prs_per_completed_issue: normally_lte_1
```
