# Remediation Work Claim Protocol

```yaml
claim_protocol_version: 2
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
  - audit-created implementation Issues
atomic_lock: deterministic Git branch ref
visibility_record: GitHub Issue claim comments
ownership_record: active task checkpoint and pull request
```

## Purpose

This contract prevents two remediation agents from implementing the same Issue or editing overlapping paths concurrently.

A valid claim has three coordinated parts:

1. **Atomic branch lock** — the deterministic Git ref `repair/issue-<ISSUE_NUMBER>` is the race arbiter. GitHub can create one ref with that exact name; the first successful creation wins.
2. **Issue claim** — machine-readable comments make ownership globally visible and record activation, renewal, takeover and release.
3. **Active task ownership** — the repository-backed task checkpoint declares exact paths, coordination key, branch, PR, lease, recovery state and one `next_action`.

All three are required for an active claim. A label, assignee, chat message, local branch, arbitrary branch name, unpushed task record or UI spinner is not a lock.

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

- `unclaimed`: no deterministic branch and no valid live claim exist.
- `provisional`: eligibility is verified and a provisional Issue marker exists, but the deterministic branch has not yet been acquired.
- `active`: deterministic branch, Issue activation marker, active task and draft/open PR agree.
- `releasing`: the owner is closing or abandoning the claim without leaving ambiguous ownership.
- `released`: ownership is terminally released; the Issue may return to `agent:ready` after revalidation.
- `stale`: lease/recovery deadline expired and live state proves no worker is still writing.
- `completed`: merged outcome is independently verified and ownership is released.

## Deterministic lock identity

For Issue `#543`, the only valid initial remediation branch is:

```text
repair/issue-543
```

Rules:

- Branch name is derived only from the repository and Issue number.
- Do not append agent names, timestamps or random suffixes to bypass an existing lock.
- Create the branch from the current synchronized `main` head after eligibility preflight.
- A successful GitHub create-ref operation acquires the lock.
- `already exists`, ref conflict or equivalent response means the lock was not acquired. Inspect the existing claim; do not create another implementation branch.
- A pre-existing branch is never deleted or moved merely to obtain the claim.
- Reopened or follow-up work after a terminal Issue requires either reuse under evidence-backed takeover or a separately approved new Issue. Do not create `-v2` branches silently.

The Git ref is the race arbiter because Issue comments, labels and assignees do not provide atomic compare-and-set ownership.

## Eligibility preflight

Before any claim mutation, verify all of the following:

1. The Issue has `agent:ready` and neither `state:triage` nor `state:blocked`.
2. `implementation_authorized: true`.
3. The work item is `parallel_safe`, or the coordinator selected it as the sole serialized item.
4. `claim.status` is unclaimed/released/stale and no valid active claim marker exists.
5. Every blocking Issue and required accepted contract is resolved.
6. Active tasks, open PRs, branches, coordination keys and changed paths show no conflicting owner.
7. The Issue metadata still matches current `main`.

If any fact is `UNKNOWN` or conflicting, do not claim. Correct metadata or stop with the exact blocker.

## Provisional marker

Before attempting the branch lock, post this machine-readable Issue comment:

```yaml
OTERYN_REMEDIATION_CLAIM:
  protocol_version: 2
  issue: <number>
  finding_id: <finding id>
  claim_nonce: <globally unique value>
  session_id: <agent/session identifier>
  task_id: <planned task id>
  lock_branch: repair/issue-<number>
  coordination_key: <key from Issue metadata>
  intended_exclusive_paths:
    - <path or narrow glob>
  intended_shared_paths: []
  claimed_at: <ISO-8601 timestamp>
  provisional_lease_expires_at: <ISO-8601 timestamp, normally 15 minutes later>
  state: provisional
```

This marker is visibility, not ownership. It cannot beat an agent that successfully acquires the deterministic branch ref.

## Atomic acquisition procedure

1. Re-read the Issue, all claim/release comments, active tasks, open PRs and the deterministic branch.
2. Attempt exactly once to create `repair/issue-<number>` from the verified current `main` head.
3. If creation succeeds, record the exact base SHA and continue to activation.
4. If creation fails because the ref exists or raced, re-read its task/PR/claim state:
   - valid active claim: post a losing/release marker and select another Issue;
   - stale candidate: follow the takeover procedure; do not create a competing branch;
   - ambiguous state: stop with an ownership blocker.
5. Never begin product edits while only provisional.

When several agents race, GitHub's unique ref creation determines the winner. Comment chronology is supporting evidence only.

A losing claimant posts:

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 2
  issue: <number>
  claim_nonce: <nonce>
  state: released
  released_at: <timestamp>
  reason: deterministic branch lock not acquired
  winning_branch: repair/issue-<number>
  next_action: select another eligible Issue
```

It creates no product mutation and does not touch the winner's branch, task or PR.

## Activation sequence

The branch winner must activate before the provisional lease expires:

1. On `repair/issue-<number>`, create the active task record from `docs/agents/tasks/TASK_TEMPLATE.md`.
2. Record Issue, claim nonce, coordination key, exact `owned_paths`, shared paths, lease, recovery state and branch.
3. Open one draft PR targeting `main`.
4. Re-read the Issue, deterministic branch, task and open PRs once more.
5. Remove `agent:ready` from the Issue.
6. Post:

```yaml
OTERYN_REMEDIATION_CLAIM_ACTIVATED:
  protocol_version: 2
  issue: <number>
  claim_nonce: <same nonce>
  state: active
  task_id: <task id>
  task_path: docs/agents/tasks/active/<task>.md
  branch: repair/issue-<number>
  pull_request: <number>
  base_head_at_claim: <sha>
  exact_head: <sha>
  activated_at: <timestamp>
  lease_expires_at: <timestamp from repository policy>
```

7. Update the Issue `oteryn_work_item.claim` block when authorized; otherwise task, PR and claim comments remain authoritative.

If activation cannot complete, preserve exact state and release or mark blocked. Do not keep an unactivated branch as an indefinite lock.

## Active ownership and lease

An active claim remains valid only while:

- `repair/issue-<number>` is the task branch;
- the Issue claim nonce is not released or superseded;
- the active task names the same Issue, branch, PR, coordination key and paths;
- the task lease and recovery checkpoint are fresh;
- live branch/PR state does not contradict the checkpoint;
- no unresolved ownership or safety conflict exists.

The Issue is the global discovery surface. The task checkpoint is the detailed ownership and continuation source of truth. The deterministic branch is the atomic exclusion mechanism.

Renew only after measurable progress or a material checkpoint:

```yaml
OTERYN_REMEDIATION_CLAIM_RENEWED:
  protocol_version: 2
  issue: <number>
  claim_nonce: <same nonce>
  branch: repair/issue-<number>
  exact_head: <sha>
  checkpoint_updated_at: <timestamp>
  lease_expires_at: <timestamp>
  next_action: <one action>
```

Do not create activity-only commits/comments to keep a claim alive.

## Stale-claim takeover

Chat silence or a UI spinner never proves abandonment.

A replacement agent may take over the existing deterministic branch only after verifying:

- task lease and recovery deadline expired;
- no worker is writing to the branch, PR, paths, runner or protected state;
- live Git/PR state shows no unrecorded progress;
- no external operation remains validly active;
- takeover preserves dependencies and safety.

Takeover normally reuses the same task, branch and PR. Increment recovery generation and preserve all counters, deadlines, run IDs, findings and exact heads.

```yaml
OTERYN_REMEDIATION_CLAIM_TAKEOVER:
  protocol_version: 2
  issue: <number>
  previous_claim_nonce: <nonce>
  branch: repair/issue-<number>
  new_session_id: <session>
  recovery_generation: <number>
  evidence: <expired lease and live-state identifiers>
  taken_over_at: <timestamp>
  next_action: <one action>
```

If live ownership is ambiguous, stop. Never delete the branch and create a competing replacement.

## Release protocol

Before abandoning, superseding, blocking or completing work:

1. Preserve/close coherent work accurately.
2. Set task status to `ready`, `waiting`, `blocked` or `completed` with one `next_action` when incomplete.
3. Reconcile the PR and reviews.
4. Post a release marker.
5. Archive/release task ownership according to repository policy.
6. Delete the deterministic branch only after its PR is terminal and no recovery or evidence dependency requires it.
7. Restore `agent:ready` only after the Issue is revalidated as implementation-authorized, unblocked and unclaimed.

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 2
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

## Shared-path serialization

These paths are serialized by default:

- root manifests and dependency lockfiles;
- shared route registries;
- common frontend shells/layouts;
- module/architecture catalogs and global indexes;
- migration chains or shared schema aggregates;
- generated contracts/types;
- common fixtures and acceptance inventories;
- CI workflows and deployment manifests.

Parallel workers avoid them. Assign one integration/closeout owner or an explicit shared-path lease and merge order.

## Coordinator dispatch

Several remediation workers may be dispatched only when every Issue has:

- `agent:ready`;
- `implementation_authorized: true`;
- `parallelization.classification: parallel_safe`;
- a distinct coordination key;
- non-overlapping exclusive/shared paths;
- independent migrations, contracts and rollout;
- no common unaccepted decision;
- no deterministic branch and no valid active claim.

Assign one Issue per worker. Coordinator dispatch is not a lock; each worker must acquire its deterministic branch.

## Forbidden patterns

Do not:

- edit product code before branch-lock acquisition and activation;
- treat labels, assignees or comments as atomic ownership;
- use random branch suffixes to bypass an existing claim;
- claim several Issues speculatively;
- keep a claim while working on unrelated tasks;
- renew without measurable progress;
- create a second PR for a valid active claim;
- steal a claim from chat/UI inactivity;
- delete or force-move another claim branch;
- use claim metadata to override repository safety, accepted architecture or path ownership.
