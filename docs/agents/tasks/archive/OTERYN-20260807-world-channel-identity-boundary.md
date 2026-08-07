---
task_id: OTERYN-20260807-world-channel-identity-boundary
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
task_kind: implementation
status: done
agent: ChatGPT
branch: docs/OTERYN-20260807-world-channel-identity-closeout
base_branch: main
created: 2026-08-07T19:36:00Z
updated: 2026-08-07T19:51:00Z
risk: high
execution_mode: github-only
implementation_authorized: documentation_only
production_activation_authorized: false
---

# OTERYN-20260807-world-channel-identity-boundary

## Goal

Persist the owner-accepted native World Registry identity/topology decision without changing runtime or database schema: Platform World Registry owns and issues canonical UUIDv7 `WorldId` and `ChannelId`; local integer row IDs and Canary numeric routing remain compatibility-only; Channel becomes a first-class topology identity independent from route, endpoint, protocol candidate, GameNode and deployment.

## Terminal result

`DONE`

The accepted architecture was delivered through PR #852 and squash-merged to protected `main` as:

```text
c88bc3e842fb18eb860a69d867938bcba48f8d55
```

The exact validated PR head was:

```text
7dbe141b3c8d9302e858e9890052045c4f63c28e
```

## Acceptance criteria

- [x] Record the accepted durable decision in ADR 0029.
- [x] Define the native Platform <-> Oteryn-v2 world/channel topology boundary contract.
- [x] Preserve current implemented World Registry behavior as legacy/current implementation evidence while removing its authority over native identifier semantics.
- [x] Update the ADR inventory without allocating a duplicate prefix.
- [x] Make no runtime, migration, deployment or Oteryn-v2 repository change.
- [x] Inspect the complete candidate diff and related PR/task ownership.
- [x] Validate the exact frozen PR head through protected required CI.
- [x] Squash-merge PR #852 and verify the result on protected `main`.
- [x] Archive this task in a separate lightweight closeout.

## Delivered architecture

Canonical Platform-owned native topology identity is now documented as:

```text
WorldId    = strongly typed UUIDv7, full 128 bits
ChannelId  = strongly typed UUIDv7, full 128 bits
ChannelRef = WorldId + ChannelId
```

Durable rules recorded by the accepted package include:

- Platform World Registry / topology control owns and issues `WorldId` and `ChannelId`;
- `game_worlds.id` and future local row IDs remain Platform-local persistence surrogates;
- the current numeric world identity and hard-coded `channel_id = 1` remain legacy/current implementation compatibility state;
- Channel is a first-class topology entity independent from Route, Endpoint, ProtocolCandidate, GameNode and Deployment;
- GameNode/process replacement and route replacement do not re-key a continuing logical channel;
- current mutation authority is fenced separately from identity;
- native account-aware topology authorization uses canonical Platform `AccountId` from ADR 0028;
- World Registry does not freeze the final `FND-02` gameplay transport/wire tuple;
- Platform topology authorization does not imply successful game-domain admission or canonical `GameSessionId` issuance;
- Oteryn-v2 is a consumer of these externally owned Platform identities and must preserve them losslessly.

## Delivered paths

PR #852 changed exactly these five documentation paths:

- `docs/agents/tasks/active/OTERYN-20260807-world-channel-identity-boundary.md`;
- `docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md`;
- `docs/architecture/adr/README.md`;
- `docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md`;
- `docs/contracts/WORLD_REGISTRY_CONTRACT.md`.

The closeout PR only moves this task record from `active/` to `archive/` and records terminal evidence.

## Validation evidence

```yaml
architecture_pr: 852
validated_head: 7dbe141b3c8d9302e858e9890052045c4f63c28e
merged_main: c88bc3e842fb18eb860a69d867938bcba48f8d55
required_checks:
  classify-changes: success
  runtime-tests: success
  test: success
self_review:
  result: PASS
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
e2e:
  result: NOT_APPLICABLE
  reason: documentation-only architecture decision; no executable runtime/user integration path changed
```

The initial candidate had over-edited historical `WORLD_REGISTRY_CONTRACT.md`. Exact-head self-review caught that before readiness; the final validated head restored the historical compatibility contract and retained only a 13-line native-authority clarification.

## Non-authorization preserved

This completed architecture task did not authorize or perform:

- Laravel/PHP runtime changes;
- database migrations or identifier backfills;
- first-class Channel runtime/storage implementation;
- Gateway API/runtime changes;
- route or protocol activation;
- Canary data/runtime mutation;
- GameNode allocation/fencing implementation;
- staging/production deployment;
- writes to `blakinio/Oteryn-v2`.

Those remain separately authorized follow-up work.

## Follow-up architecture work

A later architecture/implementation task may define the additive Platform migration for canonical `WorldId`, first-class Channel persistence, topology revision/fencing and versioned native Gateway projection. Cross-repository Oteryn-v2 reconciliation must remain under that repository's own authority.
