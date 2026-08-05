# Audit and Remediation Issue Taxonomy

```yaml
taxonomy_version: 1.1
repository: blakinio/Oteryn-Platform
programmes:
  audit: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  remediation: OTERYN_PLATFORM_REMEDIATION
claim_protocol: docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
```

## Purpose

This contract lets the continuous auditor create consistently classified findings and lets multiple remediation agents discover independent work without editing the same paths, contracts or migration sequence.

Labels provide coarse GitHub filtering. The machine-readable `oteryn_work_item` block in every audit-created Issue is the source of truth for workstream, paths, dependencies and parallel-safety classification. The Issue claim plus active task record defined by `REMEDIATION_WORK_CLAIM_PROTOCOL.md` is the ownership lock.

Labels, assignees, chat messages and local branches never prove that an Issue is safe or claimed.

## Required labels

Use only labels that exist in the repository. The baseline taxonomy uses:

```yaml
programme:
  - programme:platform
  - programme:audit-repair
type:
  defect: type:bug
  missing_capability: type:feature
  remediation: type:repair
risk:
  - risk:critical
  - risk:high
  - risk:medium
  - risk:low
priority:
  - priority:P0
  - priority:P1
  - priority:P2
  - priority:P3
state:
  triage: state:triage
  ready: agent:ready
  blocked: state:blocked
governance:
  - governance:managed
```

Apply exactly one `type:*`, one `risk:*` and one `priority:*` label, plus both programme labels and `governance:managed`.

State rules:

- New evidence needing deduplication, acceptance or dependency review: `state:triage`.
- Confirmed, unblocked, implementation-authorized work with complete metadata: `agent:ready` and no `state:triage`/`state:blocked`.
- Work that cannot proceed: `state:blocked`, with exact blockers and dependencies in the Issue body.
- A winning provisional claimant removes `agent:ready` only during activation after task/branch ownership is established.
- After a valid release, restore `agent:ready` only when the Issue is revalidated, unblocked, implementation-authorized and truly unclaimed.
- Close only after the merged outcome and every acceptance criterion are independently verified.

Do not invent a label in an Issue request when it does not exist. Extend labels through a separately reviewed governance change when more GitHub-side filtering is required.

## Workstream classification

Every Issue uses exactly one primary workstream:

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

A secondary workstream may be listed when necessary, but the primary workstream remains the ownership and queue-routing key.

## Required Issue title

```text
[AUDIT][<WORKSTREAM>][<FINDING_ID>] <observable problem>
```

Use stable uppercase finding IDs, for example `OPA-AUTH-0012` or `OPA-CI-0007`. Do not reuse an ID after duplicate, false-positive or terminal closure.

## Required machine-readable Issue body

Every Issue created by the audit programme must include this block near the top:

```yaml
oteryn_work_item:
  schema_version: 1
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
    coordination_key: <stable component or contract key>
    reason: <why it is or is not independent>
  ownership:
    exclusive_paths:
      - <path or narrow glob>
    shared_paths:
      - <path requiring lease or coordinated sequencing>
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
    protocol_version: 1
    status: unclaimed | provisional | active | blocked | released | stale | completed
    claim_nonce: none
    session_id: none
    task: none
    branch: none
    pull_request: none
    claimed_at: none
    lease_expires_at: none
```

Keep secrets, raw tokens, production data and excessive logs out of the Issue body.

## Parallel-safety rules

Set `parallel_safe` only when all are true:

- exclusive paths do not overlap another ready or active Issue/task;
- shared paths are empty, or a sequencing/lease mechanism is explicit;
- no common migration ordering, schema contract, generated artifact or public API must change concurrently;
- no atomic cross-repository rollout is required;
- acceptance can be independently implemented, validated, merged and rolled back;
- one Issue completing cannot invalidate the other's assumptions.

Set `serialized` when any apply:

- both Issues touch the same module root, route group, service, policy, migration chain, frontend shell, workflow, shared fixture or canonical contract;
- one changes a contract/type/schema consumed by another;
- merge order affects correctness;
- both alter the same user journey or acceptance inventory;
- independent PRs could produce repeated conflicts or a false-green integration state.

Set `blocked` when authority, product/architecture decision, external dependency, secret, environment or cross-repository contract is unresolved.

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

Two `parallel_safe` Issues must not share a coordination key unless exclusive paths and acceptance are proven independent.

## Claim eligibility

A remediation worker may attempt a claim only when:

1. the Issue has `agent:ready`;
2. it has neither `state:triage` nor `state:blocked`;
3. `implementation_authorized: true`;
4. it is `parallel_safe`, or the coordinator selected it as the sole serialized item;
5. Issue metadata and claim comments show no valid live claim;
6. every `blocked_by_issues` dependency is terminally resolved;
7. active tasks, branches, PRs and changed paths show no overlap;
8. required contracts/architecture decisions are accepted, not merely proposed.

The worker must then execute `docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md` exactly. It posts the provisional Issue claim before product edits. The earliest valid unexpired claim wins. The winner activates task/branch/PR ownership; later claimants release immediately and select another Issue.

## Multiple remediation agents

A coordinator may dispatch several workers only for Issues that:

- have different coordination keys;
- have non-overlapping exclusive and shared paths;
- have independent migrations, contracts and rollout order;
- do not depend on the same unaccepted ADR/decision;
- do not require concurrent edits to a common canonical index, lockfile, root manifest, shared shell, migration sequence or CI workflow.

Assign exactly one Issue number per worker. Coordinator dispatch is not a lock; each worker performs the global Issue claim protocol.

Shared documentation indexes, dependency lockfiles, route registries, module catalogs, architecture registries, generated contracts and CI workflows should normally be assigned to one integration/closeout owner after independent implementation PRs are ready.

## Auditor responsibilities

Before marking an Issue `agent:ready`, the auditor must:

- prove and deduplicate the finding;
- assign all required labels;
- fill every mandatory metadata field;
- define exclusive/shared/forbidden paths conservatively;
- identify dependencies, coordination key and rollout order;
- classify whether the complete user-facing feature is contained in the Issue or split across exact dependent tasks;
- set `implementation_authorized: false` when an owner/architecture decision is required;
- preserve uncertain paths/dependencies as UNKNOWN and use triage/blocked rather than guessing;
- verify that no valid active claim already exists.

## Remediator responsibilities

Before claiming and again before editing, the remediator independently verifies that taxonomy, paths, dependencies and claim state match current main. Stale metadata must be corrected or the claim stopped; labels never justify overlapping work.

## Completion and release

After merge and independent verification:

- post the terminal claim-release/completion marker;
- link the merged PR and exact merge commit;
- reconcile the originating finding;
- archive the task and release all owned/shared paths;
- close only when every acceptance criterion and required layer is proven;
- keep a parent capability open when a required producer or consumer remains incomplete.
