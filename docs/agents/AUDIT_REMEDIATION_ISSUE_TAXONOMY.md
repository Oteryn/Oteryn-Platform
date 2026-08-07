# Audit and Remediation Issue Taxonomy

```yaml
taxonomy_version: 1.5
repository: blakinio/Oteryn-Platform
programmes:
  audit: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  remediation: OTERYN_PLATFORM_REMEDIATION
claim_protocol:
  path: docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  version: 5
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
validation_gate_contract: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
external_repair_audit: disabled
```

## Purpose

This contract lets the continuous auditor create consistently classified findings and lets remediation agents claim one Issue each without path, contract or migration collisions.

The deterministic branch `repair/issue-<number>` is the atomic claim lock. Issue comments and active tasks provide durable visibility. A PR is a delivery/review artifact. One implementation owner remains accountable from claim through terminal closeout.

The continuous audit programme discovers defects and creates Issues. It does not approve each completed repair.

## Labels

Use only labels that exist:

```yaml
required_programme_labels:
  - programme:platform
  - programme:audit-repair
required_governance_label:
  - governance:managed
type_labels:
  defect: type:bug
  missing_capability: type:feature
  remediation: type:repair
risk_labels:
  critical: risk:critical
  high: risk:high
  medium: risk:medium
  low: risk:low
priority_labels:
  P0: priority:P0
  P1: priority:P1
  P2: priority:P2
  P3: priority:P3
state_labels:
  triage: state:triage
  ready: agent:ready
  blocked: state:blocked
```

Apply both programme labels, `governance:managed`, exactly one type, one risk and one priority.

- `state:triage`: evidence, acceptance, dependencies, validation intensity or parallel safety is unresolved.
- `agent:ready`: confirmed, implementation-authorized, unblocked work with complete current metadata and no valid claim.
- `state:blocked`: authority, dependency, decision, environment or contract prevents implementation.
- Remove `agent:ready` only after deterministic branch plus Issue/task activation is confirmed.
- A valid claim may remain branch-only; a PR is not required to prove activity.

## Workstreams

Use one primary workstream:

```yaml
allowed_workstreams:
  - identity-auth
  - accounts-characters
  - admin-rbac
  - public-web-cms
  - public-game-data
  - game-gateway-integration
  - database-persistence
  - api-contracts
  - frontend-ux
  - security
  - ci-build-test
  - deployment-operations
  - observability-recovery
  - architecture-governance
  - dependencies-tooling
```

## Issue identity

Title:

```text
[AUDIT][<WORKSTREAM>][<FINDING_ID>] <observable problem>
```

Use one stable finding ID and never reuse it after terminal disposition.

## Required work-item metadata

```yaml
oteryn_work_item:
  schema_version: 5
  finding_id: OPA-<DOMAIN>-NNNN
  source_programme: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  repository: blakinio/Oteryn-Platform
  workstream: <allowed workstream>
  secondary_workstreams: []
  finding_type: defect | missing_capability | incomplete_vertical_slice | architecture_gap | ci_gap | operability_gap | documentation_drift
  risk: critical | high | medium | low
  priority: P0 | P1 | P2 | P3
  evidence_state: PROVEN | DERIVED | UNKNOWN | CONFLICT
  implementation_authorized: true | false
  parallelization:
    classification: parallel_safe | serialized | blocked
    coordination_key: <stable key>
    reason: <evidence>
  ownership:
    exclusive_paths:
      - <path or narrow glob>
    shared_paths: []
    forbidden_paths:
      - <path outside this work item>
  dependencies:
    blocked_by_issues: []
    blocks_issues: []
    related_issues: []
    required_contracts: []
    rollout_order: independent | before:<issue> | after:<issue> | atomic_with:<issue>
  delivery_scope:
    type: full_stack | backend_only | frontend_only | contract_producer | infrastructure | data_pipeline | protocol | documentation
    complete_user_facing_feature: true | false
    required_layers: []
    dependent_tasks: []
  acceptance:
    - <observable criterion>
  validation_risk:
    preliminary_intensity: STANDARD | HEIGHTENED | BLOCKED | UNCLASSIFIED
    triggers: []
    unknown_or_conflict: []
    rationale: <evidence or unresolved classification>
  suggested_task_id: OTERYN-YYYYMMDD-<slug>
  claim:
    protocol_version: 5
    status: unclaimed | provisional | active | blocked | released | stale | completed
    lock_branch: repair/issue-<ISSUE_NUMBER>
    claim_nonce: none
    implementation_owner: none
    task: none
    pull_request: none | <number>
    delivery_state: branch_only | reused_existing | issue_owned_pr | exceptional_train_candidate | exceptional_repair_train
    claimed_at: none
    lease_expires_at: none
```

The auditor provides a conservative preliminary validation-intensity classification. The implementation owner recomputes the final gate from current scope and evidence before readiness.

For unclaimed work, use `pull_request: none`, `delivery_state: branch_only` and `implementation_owner: none`.

## Validation intensity

- Critical/high and sensitive boundaries normally imply preliminary `HEIGHTENED`.
- Medium or low may be `STANDARD`, `HEIGHTENED` or `BLOCKED` according to `REMEDIATION_AUDIT_RISK_GATE.md`.
- Material `UNKNOWN` or `CONFLICT` is `BLOCKED` until resolved.
- Documentation/test-only scope is not automatically low risk.
- Heightened validation requires stronger tests and evidence, not a different-agent PASS.

## Parallel-safety classification

Use `parallel_safe` only when all are proven:

- exclusive paths do not overlap ready or active work;
- shared paths are empty or have one explicit owner and merge order;
- migrations, schema, contracts, generated types and rollout are independent;
- no atomic cross-repository change is required;
- each Issue can be implemented, validated, merged and rolled back independently;
- completion of one cannot invalidate another's assumptions.

Use `serialized` for shared module roots, route groups, services, policies, migration chains, frontend shells, workflows, fixtures, canonical contracts or merge order.

Use `blocked` for unresolved authority, architecture/product decision, environment, secret, external dependency or cross-repository contract.

## Coordination keys

Examples:

```text
module:<module-name>
route-group:<prefix>
contract:<contract-name>
database:<table-or-aggregate>
workflow:<workflow-name>
frontend-shell:<shell-name>
integration:<producer>-<consumer>
```

## Claim eligibility

A remediation agent may attempt a claim only when:

1. `agent:ready` is present and triage/blocked labels are absent.
2. `implementation_authorized: true`.
3. Work is parallel-safe or selected as the sole serialized item.
4. No deterministic branch or valid active claim exists.
5. Blocking Issues and required contracts/decisions are resolved.
6. Active tasks, branches, related PRs, coordination keys and paths do not conflict.
7. Taxonomy, claim protocol and validation-gate metadata match current `main`.

Then execute claim protocol version 5. First successful deterministic branch creation wins. Losing agents release without mutation.

## One-Issue ownership

Assign one Issue to one implementation owner. The owner is responsible from activation through implementation, exact-head self-review, validation, merge, Issue closure, archival and release.

Do not create a different-agent repair approval handoff. Findings from self-review, CI or ordinary review return to the same owner.

## Parallel dispatch

A coordinator may dispatch several agents only when selected Issues are eligible, unclaimed, non-overlapping and independently releasable.

`N agentów naprawczych` means up to `N` implementation owners. Do not reserve an auditor or integrator slot.

Every worker must acquire its own deterministic branch. A losing worker selects another eligible Issue when authorized.

## Repair-train classification

Do not mark ordinary product work as a train candidate. `exceptional_train_candidate` is allowed only for explicitly coordinator-authorized homogeneous low-risk mechanical, documentation, test-fixture or governance work satisfying `REPAIR_PR_ECONOMY.md`.

## Continuous auditor responsibilities

Before applying `agent:ready`, the continuous auditor must:

- prove and deduplicate the finding;
- apply required labels and current schema;
- define paths, coordination key, dependencies and rollout conservatively;
- state complete delivery expectations;
- provide a preliminary validation-intensity classification;
- keep uncertainty explicit;
- verify no deterministic branch or valid claim exists;
- search related PRs to avoid duplicate delivery.

The continuous auditor does not implement the fix and does not certify every repaired Issue.

## Remediator responsibilities

Before claiming and before final readiness, verify current taxonomy/protocol versions, paths, dependencies, related PRs, accepted decisions and validation triggers.

Cross-document version drift is a blocker. Stale labels or metadata never authorize overlap or a weaker validation intensity.

## Completion

A finding becomes completed only after:

- accepted outcome is merged or otherwise terminally delivered;
- the implementation owner or valid takeover owner proves acceptance;
- exact-head self-review and applicable validation pass;
- applicable E2E and required exact-head CI pass;
- related PRs and reviews are terminal;
- task is archived and ownership released.

An open PR, green CI or audit comment alone does not close the Issue.
