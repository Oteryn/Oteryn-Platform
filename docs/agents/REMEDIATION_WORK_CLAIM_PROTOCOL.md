# Remediation Work Claim Protocol

```yaml
claim_protocol_version: 1
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
  - audit-created implementation Issues
```

## Purpose

This contract prevents two remediation agents from implementing the same Issue or editing overlapping paths concurrently.

The lock has two coordinated parts:

1. **Issue claim** — the globally visible reservation and race arbiter.
2. **Active task ownership** — the repository-backed declaration of exact paths, branch, PR, lease and continuation state.

Neither part alone is sufficient. A label, assignee, chat message, local branch, unpushed task record or agent UI spinner is not a valid lock.

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

- `unclaimed`: no valid live claim exists.
- `provisional`: an agent has posted a claim marker but has not yet confirmed task/branch ownership.
- `active`: the Issue claim, active task record, dedicated branch and draft/open PR agree.
- `releasing`: the owner is closing or abandoning the claim without product mutation.
- `released`: no agent owns the work; the Issue may return to `agent:ready` after revalidation.
- `stale`: the lease expired and live state proves no agent is still writing.
- `completed`: merged outcome is independently verified and ownership is released.

## Canonical Issue claim marker

The first action of a remediation agent, before product-code edits, is to post this machine-readable Issue comment:

```yaml
OTERYN_REMEDIATION_CLAIM:
  protocol_version: 1
  issue: <number>
  finding_id: <finding id>
  claim_nonce: <globally unique value>
  session_id: <agent/session identifier>
  task_id: <planned task id>
  coordination_key: <key from Issue metadata>
  intended_exclusive_paths:
    - <path or narrow glob>
  intended_shared_paths: []
  claimed_at: <ISO-8601 timestamp>
  provisional_lease_expires_at: <ISO-8601 timestamp, normally 15 minutes later>
  state: provisional
```

The claim comment must contain no secrets and must match the Issue metadata. A malformed or scope-expanding claim is invalid.

## Race arbitration

After posting the provisional marker, the agent must re-read:

- all Issue claim/release comments;
- current Issue labels and metadata;
- active task records;
- open PRs and their branches/changed paths;
- live ownership and leases.

The winner is the earliest chronologically valid, unexpired provisional claim whose requested paths and coordination key match the Issue and do not overlap another active owner.

When two agents race:

- the winner continues;
- every later claimant must post a `released` marker, create no product mutation, remove any disposable branch/task it created when safe, and select another Issue;
- timestamp order alone does not validate a claim whose scope, dependencies or authorization are invalid;
- an agent may not overwrite or reinterpret another valid claim.

The first valid global Issue claim wins. This rule is independent of which agent created a local branch first.

## Activation sequence

The winning provisional claimant must complete all steps before the provisional lease expires:

1. Create the active task record from `docs/agents/tasks/TASK_TEMPLATE.md` with exact `owned_paths`, coordination key, Issue, session and lease.
2. Create a dedicated branch from current synchronized `main`.
3. Open a draft PR early when GitHub PR workflow is available.
4. Re-read the Issue and live ownership one more time.
5. Remove `agent:ready` from the Issue.
6. Post an activation comment:

```yaml
OTERYN_REMEDIATION_CLAIM_ACTIVATED:
  protocol_version: 1
  issue: <number>
  claim_nonce: <same nonce>
  state: active
  task_id: <task id>
  task_path: docs/agents/tasks/active/<task>.md
  branch: <branch>
  pull_request: <number or none with reason>
  exact_head: <sha>
  activated_at: <timestamp>
  lease_expires_at: <timestamp derived from repository lease policy>
```

7. Update the Issue `oteryn_work_item.claim` block when the programme has authority to edit it; otherwise the claim comments and task/PR are authoritative.

If activation cannot complete before the provisional lease expires, release the claim. Do not begin product edits under an unconfirmed provisional claim.

## Active ownership and lease

An active claim remains valid while all are true:

- the Issue claim nonce is not released or superseded;
- the active task checkpoint names the same Issue, branch, PR, coordination key and owned paths;
- the task lease is fresh under `docs/agents/PROJECT_LANES.json`;
- live branch/PR state does not contradict the checkpoint;
- no unresolved higher-priority ownership conflict exists.

The task checkpoint is the detailed ownership source. The Issue claim is the global discovery source.

Renew ownership only after measurable progress or a material checkpoint. When renewal is needed, update the task checkpoint and post one compact renewal comment:

```yaml
OTERYN_REMEDIATION_CLAIM_RENEWED:
  protocol_version: 1
  issue: <number>
  claim_nonce: <same nonce>
  exact_head: <sha>
  checkpoint_updated_at: <timestamp>
  lease_expires_at: <timestamp>
  next_action: <one action>
```

Do not create activity-only commits or comments merely to keep a claim alive.

## Stale-claim takeover

A claim is not stale merely because the chat appears inactive.

A replacement agent may take over only after it verifies:

- the task lease and any recovery deadline have expired;
- no worker is currently writing to the branch, PR, paths, runner or protected state;
- the latest task checkpoint and recovery record are stale or explicitly waiting/ready for takeover;
- live Git and PR state show no unrecorded progress;
- takeover does not violate dependencies or cross-repository safety.

Takeover uses the existing task/branch/PR when safe, increments recovery generation and preserves counters/deadlines. It does not create a second implementation branch by default.

The takeover agent posts:

```yaml
OTERYN_REMEDIATION_CLAIM_TAKEOVER:
  protocol_version: 1
  issue: <number>
  previous_claim_nonce: <nonce>
  new_session_id: <session>
  recovery_generation: <number>
  evidence: <expired lease and live-state identifiers>
  taken_over_at: <timestamp>
  next_action: <one action>
```

If live ownership is ambiguous, stop with a blocker. Never guess that another agent is gone.

## Release protocol

Release before selecting another Issue when work is abandoned, superseded, blocked outside the task, or duplicated.

The owner must:

1. preserve or close coherent work accurately;
2. update the task checkpoint to `ready`, `waiting`, `blocked` or `completed`;
3. reconcile/close the PR when appropriate;
4. post a release marker;
5. archive or release task ownership according to repository policy;
6. restore `agent:ready` only when the Issue is still implementation-authorized, unblocked, revalidated and truly unclaimed.

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 1
  issue: <number>
  claim_nonce: <nonce>
  state: released | completed | blocked
  released_at: <timestamp>
  task_id: <task>
  pull_request: <number or none>
  reason: <exact reason>
  next_action: <one action or none>
```

## Shared-path rule

Paths such as these are serialized by default:

- dependency lockfiles and root manifests;
- shared route registries;
- common frontend shells/layouts;
- module/architecture catalogs and global indexes;
- shared migrations or schema aggregates;
- generated contracts/types;
- common test fixtures and acceptance inventories;
- CI workflows and deployment manifests.

Independent agents should avoid these paths during parallel implementation. Assign them to one integration/closeout task or require an explicit shared-path lease and merge order.

## Coordinator dispatch rule

A coordinator may start several remediation agents only when every selected Issue has:

- `agent:ready`;
- `implementation_authorized: true`;
- `parallelization.classification: parallel_safe`;
- a different coordination key;
- non-overlapping exclusive/shared paths;
- independent migrations, contracts and rollout order;
- no common unaccepted architecture decision;
- `claim.status: unclaimed` and no valid live claim marker.

The coordinator should dispatch one Issue number per agent. Each agent still performs the claim protocol; coordinator selection is not itself a lock.

## Forbidden patterns

Do not:

- start product edits before the claim becomes active;
- treat `agent:ready` as ownership;
- rely only on assignees when multiple agents use the same GitHub identity;
- claim several Issues speculatively;
- keep a claim while working on unrelated tasks;
- renew an idle claim without measurable progress;
- create a second PR for a valid active claim;
- steal a claim based on chat silence or a UI spinner;
- use labels or Issue comments to override repository safety, accepted architecture or path ownership.
