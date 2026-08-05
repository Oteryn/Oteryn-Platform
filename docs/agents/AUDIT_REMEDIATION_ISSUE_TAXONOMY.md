# Audit and Remediation Issue Taxonomy

```yaml
taxonomy_version: 1
repository: blakinio/Oteryn-Platform
programmes:
  audit: OTERYN_PLATFORM_CONTINUOUS_AUDIT
  remediation: OTERYN_PLATFORM_REMEDIATION
```

## Purpose

This contract lets the continuous auditor create consistently classified findings and lets multiple remediation agents claim independent work without editing the same paths, contracts or migration sequence.

Labels are used for coarse GitHub filtering. The machine-readable `oteryn_work_item` block in every audit-created Issue is the source of truth for exact workstream, path ownership, dependencies and parallel-safety classification.

Labels alone never prove that work is safe to execute in parallel. Every remediation agent must verify active tasks, PRs and live changed paths before claiming an Issue.

## Required labels

Use only labels that exist in the repository. The current baseline taxonomy uses:

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

Apply exactly one `type:*`, one `risk:*` and one `priority:*` label, plus both programme labels.

State rules:

- New evidence that still needs deduplication, acceptance or dependency review: `state:triage`.
- Confirmed, unblocked, implementation-authorized work with complete metadata: `agent:ready` and no `state:triage`/`state:blocked`.
- Work that cannot proceed: `state:blocked`, with exact blockers and dependencies in the Issue body.
- Claimed work is represented by an active task record, branch/PR link and a claim comment. Remove `agent:ready` after the claim is verified.
- When remediation finishes and the merged outcome is independently verified, close the Issue as completed. Do not use labels as a substitute for verification.

Do not invent a label in an Issue create/update request when it does not exist. Extend the repository label taxonomy through a separately reviewed governance change when more GitHub-side filtering is required.

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

A secondary workstream may be listed when necessary, but one workstream remains the ownership and queue-routing key.

## Required Issue title

```text
[AUDIT][<WORKSTREAM>][<FINDING_ID>] <observable problem>
```

Use stable uppercase finding IDs, for example `OPA-AUTH-0012` or `OPA-CI-0007`. Do not reuse an ID after an Issue is closed as duplicate or false positive.

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
    status: unclaimed | claimed | blocked | completed
    task: none
    branch: none
    pull_request: none
    claimed_at: none
```

Keep secrets, raw tokens, production data and excessive logs out of the Issue body.

## Parallel-safety rules

Set `parallel_safe` only when all are true:

- exclusive paths do not overlap another ready or active Issue/task;
- shared paths are empty, or a sequencing/lease mechanism is explicit;
- no common migration ordering, schema contract, generated artifact or public API must be changed concurrently;
- no atomic cross-repository rollout is required;
- acceptance can be independently implemented, validated, merged and rolled back;
- one Issue completing cannot invalidate the other's assumptions.

Set `serialized` when any apply:

- both Issues touch the same module root, route group, service, policy, migration chain, frontend shell, workflow, shared test fixture or canonical contract;
- one Issue changes a contract/types/schema consumed by another;
- merge order affects correctness;
- both alter the same user journey or acceptance inventory;
- independent PRs would require repeated conflict resolution or could produce a false-green integration state.

Set `blocked` when authority, product/architecture decision, external dependency, secret, environment or cross-repository contract is unresolved.

## Coordination keys

Use a stable key that lets agents search for collisions before claiming work:

```text
module:<module-name>
route-group:<prefix>
contract:<contract-name>
database:<table-or-aggregate>
workflow:<workflow-name>
frontend-shell:<shell-name>
integration:<producer>-<consumer>
```

Two `parallel_safe` Issues must not have the same coordination key unless their exclusive paths and acceptance are proven independent.

## Claim protocol for remediation agents

A remediation agent may claim an Issue only when:

1. it has `agent:ready`;
2. it does not have `state:triage` or `state:blocked`;
3. `implementation_authorized: true`;
4. `parallelization.classification: parallel_safe`, or it is the sole serialized item selected by the coordinator;
5. `claim.status: unclaimed`;
6. all `blocked_by_issues` are terminally resolved;
7. active task records, branches, PRs and changed paths show no overlap;
8. required contracts/architecture decisions are accepted, not merely proposed.

Claim sequence:

1. create the active task record with exact `owned_paths` and coordination key;
2. create a dedicated branch;
3. add a claim comment containing task ID, branch, intended paths and timestamp;
4. re-read the Issue, active tasks and open PRs;
5. if another valid claim won the race, archive/close the duplicate claim without product mutation and select another Issue;
6. otherwise remove `agent:ready`, update the Issue claim block when permitted, and open a draft PR early.

The first valid durable claim wins. A chat message, local branch or unpushed task file is not a claim.

## Multiple remediation agents

The coordinator may run several remediation agents concurrently by selecting Issues that:

- have different coordination keys;
- have non-overlapping exclusive and shared paths;
- have independent migration and rollout order;
- do not depend on the same unaccepted ADR/contract;
- do not require edits to a common canonical index or shared workflow in the same wave.

Shared documentation indexes, dependency lockfiles, route registries, module catalogs, architecture registries and CI workflows should normally be assigned to one integration/closeout agent after independent implementation PRs are ready, rather than edited concurrently by every worker.

## Auditor responsibilities

Before marking an Issue `agent:ready`, the auditor must:

- prove and deduplicate the finding;
- assign all required labels;
- fill every mandatory metadata field;
- define exclusive/shared/forbidden paths conservatively;
- identify dependencies and rollout order;
- classify whether the complete user-facing feature is in one Issue or split across exact dependent tasks;
- set `implementation_authorized: false` when an owner/architecture decision is still needed;
- keep uncertain path or dependency claims as `UNKNOWN` and use `state:triage` or `state:blocked` rather than guessing.

## Remediator responsibilities

Before editing, the remediator must independently verify that the auditor's path and dependency classification still matches current main. If it does not, update the durable metadata or stop the claim; never rely on stale labels to justify overlapping work.

## Completion and release

After merge and independent verification:

- update the Issue claim state to `completed` or add exact terminal evidence when body editing is unsuitable;
- link the merged PR and exact head/merge commit;
- reconcile the originating finding;
- archive the task and release owned/shared paths;
- close the Issue only when every acceptance criterion and required layer is proven;
- if a dependent consumer remains, keep the parent capability open and report producer/consumer status truthfully.
