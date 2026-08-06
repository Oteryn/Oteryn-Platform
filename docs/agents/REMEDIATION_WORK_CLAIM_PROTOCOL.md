# Remediation Work Claim Protocol

```yaml
claim_protocol_version: 4
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
  - audit-created implementation Issues
atomic_lock: deterministic Git branch ref
visibility_record: GitHub Issue claim comments
ownership_record: active task checkpoint
optional_delivery_record: one Issue-owned Pull Request
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
audit_gate_contract: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
```

## Purpose

This contract prevents two remediation agents from implementing the same Issue or editing overlapping paths concurrently. It also keeps one implementation owner accountable from claim through terminal closeout.

A valid active claim has three mandatory parts:

1. deterministic branch `repair/issue-<ISSUE_NUMBER>` as the atomic race arbiter;
2. machine-readable Issue claim evidence;
3. an active task checkpoint with exact ownership and one `next_action`.

A PR is a delivery/review artifact, not a lock. Labels, assignees, comments, arbitrary branch names, chat messages, unpushed files and UI state are not ownership.

## Ownership invariant

One Issue has one active implementation owner at a time. That owner remains responsible for:

- implementation;
- self-review and validation;
- delivery PR maintenance;
- review, CI and audit finding remediation;
- merge and outcome verification;
- Issue closure, task archival and ownership release.

An auditor does not become the Issue owner. A distinct integration owner is allowed only for an explicitly authorized exceptional repair train. Evidence-backed takeover is the only normal way to replace an implementation owner.

## Claim states

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

`active` means deterministic branch, activation marker and task agree. A PR may still be absent. `completed` requires a terminal accepted outcome and released ownership, not merely an open or merged PR.

## Deterministic lock

For Issue `#543`, the only valid initial branch is:

```text
repair/issue-543
```

Rules:

- derive the name only from repository and Issue number;
- create it from synchronized current `main` after eligibility preflight;
- first successful unique-ref creation wins;
- a ref conflict means the claim was not acquired;
- never add random suffixes, delete or move another owner's branch, or create a competing implementation branch;
- reopened work reuses the branch through valid takeover or requires a separately authorized Issue.

## Eligibility preflight

Before claim mutation verify:

1. Issue is implementation-authorized, ready and not blocked/triage.
2. Work is parallel-safe or selected as the sole serialized item.
3. No valid active claim or deterministic branch conflicts.
4. Dependencies and accepted contracts are resolved.
5. Active tasks, branches, related open/closed PRs, coordination keys and paths do not conflict.
6. Issue metadata matches current taxonomy and protocol versions.
7. Required ownership and audit-risk metadata is complete enough to fail closed.

Any material `UNKNOWN` or `CONFLICT` blocks the claim until resolved.

## Provisional marker

Post before the branch attempt:

```yaml
OTERYN_REMEDIATION_CLAIM:
  protocol_version: 4
  issue: <number>
  finding_id: <finding id>
  claim_nonce: <globally unique value>
  session_id: <agent/session>
  task_id: <task>
  lock_branch: repair/issue-<number>
  coordination_key: <key>
  intended_exclusive_paths:
    - <path>
  intended_shared_paths: []
  claimed_at: <timestamp>
  provisional_lease_expires_at: <timestamp>
  state: provisional
```

The marker is visibility only.

## Atomic acquisition

1. Re-read Issue, claim comments, active tasks, related PRs and deterministic branch.
2. Attempt exactly once to create the deterministic branch from verified current `main`.
3. On success, record exact base and activate.
4. On conflict:
   - valid active claim: release and select another eligible Issue;
   - stale candidate: follow takeover;
   - ambiguous state: stop with an ownership blocker.
5. Never edit product paths while only provisional.

Losing marker:

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 4
  issue: <number>
  claim_nonce: <nonce>
  state: released
  reason: deterministic branch lock not acquired
  winning_branch: repair/issue-<number>
  released_at: <timestamp>
  next_action: select another eligible Issue
```

## Activation

The branch winner activates before the provisional lease expires:

1. create the active task record on the deterministic branch;
2. record Issue, nonce, owner, paths, coordination key, lease, recovery and audit-gate state;
3. search related PRs and apply `REPAIR_PR_ECONOMY.md`;
4. remain branch-only until a PR is useful, or reuse/open one Issue-owned PR;
5. re-read ownership and remove `agent:ready`;
6. post:

```yaml
OTERYN_REMEDIATION_CLAIM_ACTIVATED:
  protocol_version: 4
  issue: <number>
  claim_nonce: <nonce>
  state: active
  implementation_owner: <session/claim>
  task_id: <task>
  task_path: docs/agents/tasks/active/<task>.md
  branch: repair/issue-<number>
  pull_request: none | <number>
  delivery_state: branch_only | reused_existing | issue_owned_pr | exceptional_train_candidate | exceptional_repair_train
  audit_requirement: UNCLASSIFIED | NOT_REQUIRED | OPTIONAL | REQUIRED
  base_head_at_claim: <sha>
  exact_head: <sha>
  activated_at: <timestamp>
  lease_expires_at: <timestamp>
```

A PR must not be created only to prove activity.

## Active claim and renewal

A claim remains valid only while branch, nonce, implementation owner, task, paths, lease and live delivery state agree.

Renew only after measurable progress:

```yaml
OTERYN_REMEDIATION_CLAIM_RENEWED:
  protocol_version: 4
  issue: <number>
  claim_nonce: <nonce>
  implementation_owner: <session/claim>
  branch: repair/issue-<number>
  pull_request: none | <number>
  delivery_state: <state>
  audit_requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
  exact_head: <sha>
  checkpoint_updated_at: <timestamp>
  lease_expires_at: <timestamp>
  next_action: <one action>
```

Do not create activity-only commits or comments.

## End-to-end ownership

The implementation owner must continue the same Issue through every safe phase available in the current invocation. Completing implementation, opening a PR, obtaining CI, receiving audit findings or merging is not by itself a reason to abandon ownership.

When a different execution session resumes the same valid task, it is a continuation/takeover of the implementation-owner role, not a new parallel owner.

## Audit gate

Before final readiness, record the machine-readable decision from `REMEDIATION_AUDIT_RISK_GATE.md`.

- `NOT_REQUIRED`: complete self-review, applicable E2E and exact-head CI; do not create an audit handoff.
- `OPTIONAL`: request a distinct audit only when chosen; otherwise record `NOT_REQUESTED` with rationale.
- `REQUIRED`: publish the exact handoff, keep ownership, set checkpoint `ready` and return `ROTATE` if no eligible auditor is available.

The owner cannot downgrade a mandatory trigger. Self-review is never independent audit.

If an auditor returns findings, the same implementation owner resumes, repairs them, reruns affected validation and emits a new candidate/audit generation. The auditor never owns or mutates the target.

## Exceptional repair train

A worker may offer an exact source head to a repair train only when `REPAIR_PR_ECONOMY.md` eligibility and explicit coordinator authorization are proven. The worker keeps its Issue ownership; exactly one integration owner controls the train branch.

Ordinary product repairs do not wait for or join trains by default.

## Stale-claim takeover

Silence or a UI spinner never proves abandonment. Takeover requires proof that:

- lease and recovery deadline expired;
- no worker is writing branch, PR, paths, runner or protected state;
- live state shows no unrecorded progress;
- no external operation remains active;
- dependencies, audit generations and accepted source heads remain safe.

Normally reuse the same branch, task and PR. Increment recovery generation and preserve exact evidence.

```yaml
OTERYN_REMEDIATION_CLAIM_TAKEOVER:
  protocol_version: 4
  issue: <number>
  previous_claim_nonce: <nonce>
  previous_implementation_owner: <owner>
  branch: repair/issue-<number>
  new_implementation_owner: <session>
  recovery_generation: <number>
  evidence: <expired lease and live-state identifiers>
  taken_over_at: <timestamp>
  next_action: <one action>
```

Ambiguous ownership blocks takeover.

## Release and completion

Before abandoning, blocking or completing:

1. preserve coherent work and exact evidence;
2. reconcile PR, audit gate, validation, reviews and related attempts;
3. set an accurate task status with one `next_action` when incomplete;
4. post release evidence;
5. archive/release only after terminal conditions are proven;
6. delete the deterministic branch only when no recovery/evidence dependency remains;
7. restore `agent:ready` only after revalidation.

```yaml
OTERYN_REMEDIATION_CLAIM_RELEASED:
  protocol_version: 4
  issue: <number>
  claim_nonce: <nonce>
  implementation_owner: <owner>
  branch: repair/issue-<number>
  state: released | completed | blocked
  released_at: <timestamp>
  task_id: <task>
  pull_request: <number or none>
  audit_requirement: NOT_REQUIRED | OPTIONAL | REQUIRED
  audit_result: NOT_REQUIRED | NOT_REQUESTED | PASS | FAILED | PENDING
  reason: <exact reason>
  branch_terminal_state: retained | deleted
  next_action: <one action or none>
```

`completed` requires merged/accepted outcome, completed self-review, required audit result, applicable E2E, exact-head CI, terminal related PRs, archived task and released ownership.

## Parallel dispatch

A coordinator may dispatch several implementation owners only for distinct ready Issues with non-overlapping paths, coordination keys, migrations, contracts and rollout.

`N agentów naprawczych` means up to `N` implementation owners. No audit slot is permanently reserved. Allocate an independent auditor only when a valid `REQUIRED` handoff exists.

## Forbidden patterns

Do not:

- edit before deterministic branch acquisition and activation;
- use labels/comments/PRs as atomic ownership;
- bypass a branch with random suffixes;
- claim several Issues speculatively;
- create a second active implementation owner;
- transfer Issue ownership to an auditor;
- let an auditor repair and then self-PASS the same generation;
- waive a mandatory audit trigger;
- call self-review independent;
- use an ordinary repair train to reduce PR count;
- keep a claim alive with activity-only evidence;
- delete or force-move another owner's branch.
