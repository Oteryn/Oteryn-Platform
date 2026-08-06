# Remediation Work Claim Protocol

```yaml
claim_protocol_version: 3
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
  - audit-created implementation Issues
atomic_lock: deterministic Git branch ref
visibility_record: GitHub Issue claim comments
ownership_record: active task checkpoint
optional_delivery_record: existing or newly opened Pull Request
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
```

## Purpose

This contract prevents two remediation agents from implementing the same Issue or editing overlapping paths concurrently. `REPAIR_PR_ECONOMY.md` controls PR selection, repair trains, independent audit separation and delivery mapping.

A valid active claim has three mandatory parts:

1. **Atomic branch lock** — deterministic Git ref `repair/issue-<ISSUE_NUMBER>`; first successful creation wins.
2. **Issue claim** — machine-readable comments record provisional, activation, renewal, takeover and release state.
3. **Active task ownership** — exact paths, coordination key, branch, lease, recovery state and one `next_action`.

A PR is a delivery/review artifact, not a mandatory ownership primitive. Labels, assignees, comments, chat, local branches, random branch names, unpushed task records and UI spinners are not atomic locks.

## Lock states

```yaml
claim_states:
  - unclaimed
  - provisional
  - active
  - releasing
  - released
  - stale
  - completed
```

- `unclaimed`: no deterministic branch and no valid live claim.
- `provisional`: eligibility verified and marker posted, but branch not yet acquired.
- `active`: deterministic branch, activation marker and active task agree; PR may be absent.
- `releasing`: owner is terminally reconciling or abandoning the claim.
- `released`: ownership terminally released; requeue requires revalidation.
- `stale`: lease/recovery deadline expired and live evidence proves no worker remains active.
- `completed`: merged or otherwise terminal accepted outcome independently verified and ownership released.

## Deterministic lock identity

For Issue `#543`, the only valid initial remediation branch is:

```text
repair/issue-543
```

Rules:

- derive the name only from repository and Issue number;
- never append agent names, timestamps or random suffixes to bypass a lock;
- create from synchronized current `main` after eligibility preflight;
- one successful GitHub create-ref operation acquires the lock;
- `already exists`, ref conflict or equivalent means the lock was not acquired;
- never delete or move a pre-existing branch merely to obtain the claim;
- reopened/follow-up work reuses the branch through evidence-backed takeover or requires a separately approved Issue; do not create silent `-v2` branches.

Issue comments, labels and assignees do not provide atomic compare-and-set ownership.

## Eligibility preflight

Before claim mutation verify:

1. Issue has `agent:ready` and neither `state:triage` nor `state:blocked`.
2. `implementation_authorized: true`.
3. Work is `parallel_safe`, or selected as the sole serialized item.
4. Claim is unclaimed/released/stale and no valid active marker exists.
5. Blocking Issues and required accepted decisions/contracts are resolved.
6. Active tasks, PRs, branches, coordination keys and paths show no conflict.
7. Issue metadata still matches current `main`.
8. Related open and closed PRs were searched so an authoritative existing delivery is not duplicated.

If any material fact is `UNKNOWN` or `CONFLICT`, do not claim. Correct metadata or stop with the exact blocker.

## Provisional marker

Before the branch attempt post:

```yaml
OTERYN_REMEDIATION_CLAIM:
  protocol_version: 3
  issue: <number>
  finding_id: <finding id>
  claim_nonce: <globally unique value>
  session_id: <agent/session identifier>
  task_id: <planned task id>
  lock_branch: repair/issue-<number>
  coordination_key: <key>
  intended_exclusive_paths:
    - <path or narrow glob>
  intended_shared_paths: []
  claimed_at: <ISO-8601 timestamp>
  provisional_lease_expires_at: <normally 15 minutes later>
  state: provisional
```

This marker provides visibility only.

## Atomic acquisition

1. Re-read Issue, claim/release comments, active tasks, related PRs and deterministic branch.
2. Attempt exactly once to create `repair/issue-<number>` from verified current `main`.
3. On success, record exact base SHA and activate.
4. On ref conflict:
   - valid active claim: post losing/release marker and select another eligible Issue when authorized;
   - stale candidate: use takeover; never create a competing branch;
   - ambiguous state: stop with an ownership blocker.
5. Never edit product paths while only provisional.

Losing marker:

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 3
  issue: <number>
  claim_nonce: <nonce>
  state: released
  released_at: <timestamp>
  reason: deterministic branch lock not acquired
  winning_branch: repair/issue-<number>
  next_action: select another eligible Issue
```

## Activation sequence

The branch winner must activate before the provisional lease expires:

1. Create the active task record on `repair/issue-<number>`.
2. Record Issue, claim nonce, coordination key, exact owned/shared paths, lease, recovery state and branch.
3. Apply the delivery selection order in `REPAIR_PR_ECONOMY.md`:
   - reuse an authoritative existing PR;
   - join a compatible train through its integration owner;
   - remain branch-only until coherent;
   - create one dedicated PR when needed.
4. Re-read Issue, branch, task, related PRs and ownership.
5. Remove `agent:ready`.
6. Post:

```yaml
OTERYN_REMEDIATION_CLAIM_ACTIVATED:
  protocol_version: 3
  issue: <number>
  claim_nonce: <same nonce>
  state: active
  task_id: <task id>
  task_path: docs/agents/tasks/active/<task>.md
  branch: repair/issue-<number>
  pull_request: none | <number>
  delivery_state: branch_only | reused_existing | dedicated_pr | train_candidate | repair_train
  base_head_at_claim: <sha>
  exact_head: <sha>
  activated_at: <timestamp>
  lease_expires_at: <timestamp>
```

7. Update Issue claim metadata when authorized; otherwise branch, task and machine comments remain authoritative.

A PR must not be created solely to demonstrate activity. If PR-triggered CI, early high-risk review, authoritative PR reuse, coherent reviewability or train integration requires a PR, create/reuse it and update the activation/renewal record.

If activation cannot complete, preserve exact state and release or mark blocked. Do not keep an unactivated branch as an indefinite lock.

## Active ownership and lease

A claim remains valid only while:

- deterministic branch is the source branch;
- nonce is not released/superseded;
- active task names the same Issue, branch, coordination key and paths;
- task lease and recovery checkpoint are fresh;
- optional PR/delivery mapping does not contradict the task;
- no ownership or safety conflict exists.

Renew only after measurable progress:

```yaml
OTERYN_REMEDIATION_CLAIM_RENEWED:
  protocol_version: 3
  issue: <number>
  claim_nonce: <same nonce>
  branch: repair/issue-<number>
  pull_request: none | <number>
  delivery_state: branch_only | reused_existing | dedicated_pr | train_candidate | repair_train
  exact_head: <sha>
  checkpoint_updated_at: <timestamp>
  lease_expires_at: <timestamp>
  next_action: <one action>
```

Do not create activity-only commits/comments.

## PR creation and train handoff

When a PR becomes necessary, update task and Issue claim evidence; do not create a second delivery PR when a compatible authoritative PR exists.

A worker joining a train keeps its own deterministic branch and task. It publishes the exact immutable source-head acceptance block required by `REPAIR_PR_ECONOMY.md`; only the train integration owner writes the train branch.

The worker must not silently advance an accepted source head. New source content requires a new handoff and revalidation generation.

## Independent audit role rotation

When a coherent candidate reaches the final audit gate:

1. publish exact PR/base/head audit handoff;
2. set checkpoint `status: ready` with one audit `next_action`;
3. preserve the Issue claim, branch and task for findings/recovery;
4. return `ROTATE` when no distinct eligible auditor can run in the session;
5. never use `WAITING` merely because another agent must perform the audit.

The implementing agent, contributing workers and train integration owner cannot perform the required final independent PASS. The auditor is `AUDIT ONLY`, does not mutate the target and follows `REPAIR_PR_ECONOMY.md`.

## Stale-claim takeover

Chat silence or a spinner never proves abandonment. Takeover is allowed only after verifying:

- lease and recovery deadline expired;
- no worker writes branch, PR, paths, runner or protected state;
- live Git/PR state shows no unrecorded progress;
- no external operation remains validly active;
- takeover preserves dependencies, accepted train heads and audit/CI counters.

Normally reuse the task, branch and PR. Increment recovery generation and preserve deadlines, runs, findings and exact heads.

```yaml
OTERYN_REMEDIATION_CLAIM_TAKEOVER:
  protocol_version: 3
  issue: <number>
  previous_claim_nonce: <nonce>
  branch: repair/issue-<number>
  new_session_id: <session>
  recovery_generation: <number>
  evidence: <expired lease and live-state identifiers>
  taken_over_at: <timestamp>
  next_action: <one action>
```

Ambiguous ownership blocks takeover.

## Release protocol

Before abandoning, superseding, blocking or completing:

1. preserve/close coherent work accurately;
2. set task to `ready`, `waiting`, `blocked` or `completed` with one `next_action` when incomplete;
3. reconcile delivery PR, train mapping, audits and review threads;
4. post release marker;
5. archive/release ownership according to policy;
6. delete deterministic branch only after terminal delivery and no recovery/evidence dependency;
7. restore `agent:ready` only after revalidation.

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 3
  issue: <number>
  claim_nonce: <nonce>
  branch: repair/issue-<number>
  state: released | completed | blocked
  released_at: <timestamp>
  task_id: <task>
  pull_request: <number or none>
  reason: <exact reason>
  branch_terminal_state: retained | deleted
  next_action: <one action or none>
```

A pre-merge task archive may use only `completed_on_merge` semantics. A PR closed without merge cannot leave ownership released or the task completed.

## Shared-path serialization

Serialized by default:

- root manifests and lockfiles;
- shared route registries;
- common frontend shells/layouts;
- module/architecture catalogs and global indexes;
- migration chains/shared schema aggregates;
- generated contracts/types;
- common fixtures/acceptance inventories;
- CI workflows/deployment manifests.

Assign one integration owner or explicit shared-path lease and merge order.

## Coordinator dispatch

Multiple workers may be dispatched only when every Issue is ready, authorized, parallel-safe, dependency-resolved, distinct in coordination key, non-overlapping in paths and independent in migration/contract/rollout, with no existing deterministic branch or valid claim.

Coordinator dispatch is not a lock. Each worker acquires its own ref. A worker that loses immediately releases and selects another eligible Issue when authorized. Coordinators must not hold coherent repairs open merely to fill a repair train.

## Forbidden patterns

Do not:

- edit product code before lock acquisition and activation;
- replace atomic ref ownership with labels, assignees or comments;
- bypass locks with random branch suffixes;
- claim several Issues speculatively;
- keep a claim while doing unrelated work;
- renew without progress;
- create a duplicate PR for a valid active claim;
- create a PR solely as an activity signal;
- steal a claim based on chat/UI inactivity;
- delete/force-move another claim branch;
- silently change an accepted train source head;
- let an implementer self-approve the required independent audit;
- use claim metadata to override safety, architecture or path ownership.
