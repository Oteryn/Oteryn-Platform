# Audit and Remediation Issue Taxonomy

```yaml
taxonomy_version: 1.3
repository: blakinio/Oteryn-Platform
programmes:
  audit: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  remediation: OTERYN_PLATFORM_REMEDIATION
claim_protocol:
  path: docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  version: 3
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
```

## Purpose

This contract lets the continuous auditor create consistently classified findings and lets remediation agents discover independent work without editing the same paths, contracts or migration sequence.

GitHub labels provide coarse filtering. The `oteryn_work_item` block provides exact routing and dependency data. The deterministic branch `repair/issue-<number>` is the atomic claim lock. Issue comments provide global visibility, and the active task checkpoint provides detailed ownership and continuation state. Pull Requests are optional delivery and review artifacts selected under `REPAIR_PR_ECONOMY.md`; they are not ownership primitives.

Labels, assignees, chat messages, arbitrary branches, Pull Requests and unpushed task files are not atomic ownership.

## Existing labels

Use only labels that exist in the repository:

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

State rules:

- `state:triage`: evidence, acceptance, dependencies or parallel safety are not yet resolved.
- `agent:ready`: confirmed, implementation-authorized, unblocked work with complete metadata and no valid claim.
- `state:blocked`: authority, dependency, decision, environment or contract prevents implementation.
- Remove `agent:ready` only after the deterministic branch is acquired and branch/Issue/task activation is confirmed.
- A valid active claim may remain `branch_only`; do not require a PR merely to prove activity.
- Restore `agent:ready` after release only when current evidence again proves the Issue is eligible and unclaimed.

Do not invent missing labels during Issue creation. A broader label set requires a separately reviewed governance change.

## Primary workstreams

Use exactly one primary workstream:

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

Secondary workstreams may be listed, but the primary workstream controls routing and ownership search.

## Issue identity

Title:

```text
[AUDIT][<WORKSTREAM>][<FINDING_ID>] <observable problem>
```

Use a stable ID such as `OPA-AUTH-0012` or `OPA-CI-0007`. Never reuse an ID after duplicate, false-positive or terminal disposition.

## Required Issue metadata

Place this machine-readable block near the top of every audit-created Issue:

```yaml
oteryn_work_item:
  schema_version: 3
  finding_id: OPA-<DOMAIN>-NNNN
  source_programme: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  repository: blakinio/Oteryn-Platform
  workstream: <one allowed workstream>
  secondary_workstreams: []
  finding_type: defect | missing_capability | incomplete_vertical_slice | architecture_gap | ci_gap | operability_gap | documentation_drift
  risk: critical | high | medium | low
  priority: P0 | P1 | P2 | P3
  evidence_state: PROVEN | DERIVED | UNKNOWN | CONFLICT
  implementation_authorized: true | false
  parallelization:
    classification: parallel_safe | serialized | blocked
    coordination_key: <stable key>
    reason: <evidence-backed reason>
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
  suggested_task_id: OTERYN-YYYYMMDD-<slug>
  claim:
    protocol_version: 3
    status: unclaimed | provisional | active | blocked | released | stale | completed
    lock_branch: repair/issue-<ISSUE_NUMBER>
    claim_nonce: none
    session_id: none
    task: none
    pull_request: none | <number>
    delivery_state: branch_only | reused_existing | dedicated_pr | train_candidate | repair_train
    claimed_at: none
    lease_expires_at: none
```

For an unclaimed Issue, use `pull_request: none` and `delivery_state: branch_only` as placeholders; activation updates the exact state. A PR is created or reused only when the mandatory selection order and review/CI needs require it.

Do not include secrets, raw tokens, production data or excessive logs.

## Parallel-safety classification

Use `parallel_safe` only when all are proven:

- exclusive paths do not overlap any ready or active Issue/task;
- shared paths are empty or have an explicit single integration owner/lease and merge order;
- migrations, schema, public contracts, generated types and rollout are independent;
- no atomic cross-repository change is required;
- each Issue can be implemented, validated, merged and rolled back independently;
- completing one Issue cannot invalidate the other's assumptions.

Use `serialized` when Issues share a module root, route group, service, policy, migration chain, frontend shell, workflow, fixture, acceptance inventory, canonical contract or required merge order.

Use `blocked` when authority, architecture/product decision, environment, secret, external dependency or cross-repository contract is unresolved.

## Coordination keys

Use a stable collision-search key:

```text
module:<module-name>
route-group:<prefix>
contract:<contract-name>
database:<table-or-aggregate>
workflow:<workflow-name>
frontend-shell:<shell-name>
integration:<producer>-<consumer>
```

Two parallel Issues normally have different keys. Sharing a key requires explicit evidence that paths and acceptance remain independent.

## Claim eligibility

A remediation worker may attempt a claim only when:

1. `agent:ready` is present and triage/blocked labels are absent.
2. `implementation_authorized: true`.
3. Work is `parallel_safe`, or the coordinator selected it as the sole serialized item.
4. No `repair/issue-<number>` branch or valid live claim exists.
5. Blocking Issues and required accepted contracts are terminally resolved.
6. Active tasks, branches, related open and closed PRs, coordination keys and paths show no overlap or authoritative delivery that must be reused.
7. Metadata still matches current `main`.

Then execute `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md` version 3. The agent posts a provisional marker and attempts to create the deterministic branch. GitHub ref creation determines the winner. A losing agent releases without product mutation.

A winning claim becomes active only after deterministic branch, Issue activation marker and active task agree. PR selection then follows `docs/agents/REPAIR_PR_ECONOMY.md`: reuse authoritative delivery, join a compatible train, continue branch-only, or create one dedicated PR. Never create a PR solely as an activity signal.

## Parallel dispatch

A coordinator may dispatch several agents only when selected Issues:

- are all eligible and unclaimed;
- have distinct coordination keys;
- have non-overlapping exclusive/shared paths;
- have independent migrations, contracts and rollout order;
- do not depend on one unaccepted decision;
- do not require concurrent edits to shared root manifests, lockfiles, route registries, shells, migration chains, generated contracts, global indexes or CI workflows.

Assign one Issue per worker. Coordinator dispatch is not a claim; every worker must acquire `repair/issue-<number>`.

Shared paths should normally be handled later by one integration/closeout owner. A coherent repair does not wait merely to fill a train; a worker persists an exact audit or integration handoff and returns `ROTATE` at an internal role boundary.

## Auditor responsibilities

Before applying `agent:ready`, the auditor must:

- prove and deduplicate the finding;
- apply all required labels;
- fill every mandatory metadata field using the current taxonomy and claim-protocol versions;
- define exclusive/shared/forbidden paths conservatively;
- identify dependencies, coordination key and rollout order;
- state whether the complete feature fits one Issue or requires exact dependent tasks;
- set `implementation_authorized: false` when a decision remains;
- preserve uncertain claims as `UNKNOWN` and use triage/blocked instead of guessing;
- verify that no deterministic branch or valid active claim exists;
- search related PRs so existing authoritative delivery is not duplicated.

## Remediator responsibilities

Before claiming and again before editing, independently verify taxonomy version, claim protocol version, paths, dependencies, accepted decisions, branch lock, claim state and related PRs against current `main`. Stale labels or metadata never justify overlap. Cross-document protocol/schema drift is a blocker and must fail closed.

## Completion and release

After merge and independent verification:

- post the terminal claim marker;
- link the merged PR and merge commit;
- reconcile the finding;
- archive the task and release paths;
- make the deterministic branch terminal according to claim protocol version 3;
- close only when every acceptance criterion and required layer is proven;
- keep the parent capability open while any required producer or consumer remains incomplete.

A pre-merge archive may use only `completed_on_merge` bound to the exact PR/head. Closing the PR without merge cannot leave the task completed or ownership released.
