# Remediation Work Claim Protocol

```yaml
claim_protocol_version: 5
repository: blakinio/Oteryn-Platform
applies_to:
  - OTERYN_PLATFORM_REMEDIATION
  - audit-created implementation Issues
atomic_lock: deterministic Git branch ref
visibility_record: GitHub Issue claim comments
ownership_record: active task checkpoint
optional_delivery_record: one Issue-owned Pull Request
repair_delivery_contract: docs/agents/REPAIR_PR_ECONOMY.md
validation_gate_contract: docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
repair_audit_handoff: prohibited
implementation_owner_end_to_end: true
```

## Purpose

This contract prevents two remediation agents from implementing the same Issue or editing overlapping paths concurrently. It keeps one implementation owner accountable from claim through terminal closeout without transferring the Issue to a second agent for approval.

## Atomic claim

A valid active claim has three mandatory parts:

1. deterministic branch `repair/issue-<ISSUE_NUMBER>` as the atomic race arbiter;
2. machine-readable Issue claim evidence;
3. an active task checkpoint with exact ownership and one concrete `next_action`.

A PR is a delivery and review artifact, not a lock. Labels, assignees, comments, arbitrary branch names, chat messages, unpushed files and UI state are not atomic ownership.

## Claim procedure

1. Query current open implementation-authorized Issues and active tasks.
2. Exclude Issues with a valid deterministic branch claim, active ownership conflict, blocker or overlapping serialized dependency.
3. Select one eligible Issue.
4. Attempt to create `repair/issue-<ISSUE_NUMBER>` from the trusted current base exactly once.
5. When branch creation succeeds, publish the Issue claim and activate the task checkpoint.
6. Remove `agent:ready` only after branch, Issue and task activation agree.
7. When branch creation loses the race, do not modify that Issue or branch; select another eligible Issue.

## Ownership invariant

One Issue has one active implementation owner at a time. That owner remains responsible for:

- root-cause analysis and acceptance;
- implementation;
- focused validation and exact-head self-review;
- delivery PR maintenance;
- findings from self-review, CI and ordinary PR review;
- applicable E2E and exact-head required CI;
- merge, resulting-state verification and Issue closure;
- task archival and ownership release.

No different-agent PASS is required. Do not create an audit handoff, audit Issue, frozen audit generation or ownership transfer for repair approval.

A separate continuous-audit programme may independently inspect the platform and create new Issues. It does not own or approve the active repair.

## Claim record

Record at least:

```yaml
repair_claim:
  protocol_version: 5
  issue: <number>
  owner: <stable session/task identity>
  task_id: <task id>
  branch: repair/issue-<number>
  base_sha: <sha>
  claimed_at: <ISO-8601>
  owned_paths: []
  coordination_key: <key or none>
  validation_intensity: STANDARD | HEIGHTENED | BLOCKED
  status: active
```

## Heartbeat and durable state

Update the task checkpoint after material discoveries, implementation packages, validation results, head changes, blockers and before session exhaustion. Do not create a new commit merely to repeat unchanged status.

Chat presence, a UI spinner or an assignee is not a heartbeat. Durable Git/Issue/task evidence is authoritative.

## Recovery and stale claims

A replacement session may recover the same claim only after verifying:

- deterministic branch identity and current head;
- active task identity and owned paths;
- no competing valid owner or newer takeover record;
- the last durable checkpoint and concrete `next_action`;
- preserved anti-stall and CI wait counters.

Release or take over a stale claim only with evidence. Never infer abandonment solely from silence.

## Parallel safety

A command requesting `N` repair agents means up to `N` distinct end-to-end Issue owners. Before parallel execution, verify:

- different deterministic branches;
- non-overlapping owned paths or explicitly safe coordination;
- independent migrations, shared contracts and rollout order;
- no shared coordination key requiring serialization.

A losing claimant selects another Issue rather than waiting.

## PR and Actions economy

- one coherent Issue normally uses one authoritative PR;
- build coherent edits before pushing;
- avoid one commit per file, checkpoint, comment or evidence line;
- checkpoint-only changes must not start unrelated heavy runtime workflows;
- superseded same-PR workflow runs are cancelled;
- use focused checks during construction and one full applicable exact-head validation at readiness.

## Release

Release ownership only after terminal closeout or an explicitly recorded safe abandonment:

```yaml
repair_release:
  issue: <number>
  owner: <identity>
  branch: repair/issue-<number>
  final_head: <sha>
  reason: merged_completed | duplicate | invalidated | blocked_released | safe_abandonment
  released_at: <ISO-8601>
  next_state: closed | agent_ready | triage | blocked
```

Never silently abandon a claimed Issue. Restore `agent:ready` only when current evidence proves it is eligible and unclaimed.
