---
task_id: OTERYN-20260807-world-channel-identity-boundary
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
task_kind: implementation
status: validating
agent: ChatGPT
branch: docs/oteryn-world-channel-identity-20260807
base_branch: main
created: 2026-08-07T19:36:00Z
updated: 2026-08-07T19:47:00Z
risk: high
execution_mode: github-only
implementation_authorized: documentation_only
production_activation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
search_first:
  - overlapping active tasks and open architecture PRs
  - current World Registry implementation and identifier representations
optional_reads:
  - blakinio/Oteryn-v2 docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
---

# OTERYN-20260807-world-channel-identity-boundary

## Goal

Persist the owner-accepted native World Registry identity/topology decision without changing runtime or database schema: Platform World Registry owns and issues canonical UUIDv7 `WorldId` and `ChannelId`; local integer row IDs and Canary numeric routing remain compatibility-only; Channel becomes a first-class topology identity independent from route, endpoint, protocol candidate, GameNode and deployment.

## Acceptance criteria

- [x] Record the accepted durable decision in a new Platform ADR.
- [x] Define the native Platform <-> Oteryn-v2 world/channel topology boundary contract.
- [x] Preserve current implemented World Registry behavior as legacy/current implementation evidence while removing its authority over native identifier semantics.
- [x] Update the ADR inventory without allocating a duplicate prefix.
- [x] Make no runtime, migration, deployment or Oteryn-v2 repository change.
- [x] Inspect the complete candidate diff and related PR/task ownership before final CI.
- [ ] Merge only after required exact-head CI passes, then archive this task in a separate lightweight closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-world-channel-identity-boundary.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
modules:
  - architecture
  - world-registry
  - cross-repository-contracts
dependencies:
  - owner decision accepted in the architecture continuation on 2026-08-07
  - ADR 0028 AccountId boundary
  - Oteryn-v2 FND-ID-01 WorldId/ChannelId consumer baseline as read-only coordination evidence
blockers:
  - none
cross_repository_tasks:
  - Oteryn-v2 follow-up reference/reconciliation is read-only in this task and requires separate repository write authority
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T19:47:00Z
head: 83f1ec7aefa9319cb026c391fd41559c0679de10
branch: docs/oteryn-world-channel-identity-20260807
pr: 852
status: validating
phase: validate
session_role: implementer-validator
execution_mode: github-only
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive documentation-only architecture decision with one Platform owner
context_routes:
  - architecture
  - canary-integration
  - api
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-world-channel-identity-boundary.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
proven:
  - current game_worlds primary identity is an integer Laravel row id
  - current GameWorldRoute exports integer id
  - current protocol policy exports integer channelId
  - current DatabaseWorldRegistry hardcodes channel_id 1 and consumes canary_account_id
  - Oteryn-v2 FND-ID-01 already expects Platform-owned UUIDv7 WorldId and ChannelId with ChannelRef = WorldId + ChannelId
  - owner explicitly accepted the Platform UUIDv7 WorldId/ChannelId and first-class Channel decision on 2026-08-07
  - ADR 0029 and OTERY N_V2_WORLD_TOPOLOGY_CONTRACT are bounded to architecture semantics and explicitly authorize no runtime/schema/deployment change
  - WORLD_REGISTRY_CONTRACT preserves the delivered compatibility text and only adds a native authority clarification
  - PR 852 changes only the five declared documentation paths
derived:
  - existing numeric world/channel identifiers must remain implementation/compatibility state rather than native durable identity
  - endpoint, protocol candidate and GameNode placement must be independently replaceable without changing WorldId or ChannelId
unknown:
  - exact future additive database column/table names and migration rollout
  - exact channel allocation/capacity algorithm
  - exact FND-02 transport and wire representation
conflicts: []
first_failure:
  marker: over-broad-compatibility-contract-edit
  evidence: initial self-review found the first candidate rewrote historical WORLD_REGISTRY_CONTRACT sections beyond the narrow authority clarification; commit 83f1ec7aefa9319cb026c391fd41559c0679de10 restores the compatibility evidence and retains only the authority note
rejected_hypotheses:
  - use game_worlds.id as permanent native WorldId
  - use channel_id=1 as permanent native ChannelId
  - make protocol candidate or endpoint define Channel identity
  - let GameNode mint canonical ChannelId
  - rewrite historical compatibility evidence when a narrow authority note resolves the conflict
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-world-channel-identity-boundary.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
validation:
  - command: targeted live repository/preflight and overlap inspection
    result: PASS
    evidence: no open architecture PR or active task owns the new ADR, ADR README or World Registry topology contract paths
  - command: complete PR 852 changed-path and patch review
    result: PASS
    evidence: exact five owned documentation paths; initial over-broad legacy-contract rewrite was corrected before readiness
  - command: ADR allocation/inventory review
    result: PASS
    evidence: 0028 was the highest allocated prefix on trusted main and no open architecture PR claimed 0029; README includes the new exact path
  - command: E2E
    result: NOT_APPLICABLE
    evidence: documentation-only architecture decision; no executable user/runtime integration behavior changed
  - command: exact-head required CI
    result: NOT_RUN
    evidence: final task checkpoint commit must be created before exact-head checks are evaluated
blockers:
  - none
next_action: Freeze the PR head, record exact-head self-review, verify required classify-changes/test checks, and merge only if every gate passes.
```

## Notes

This task records architecture only. It does not authorize Laravel/PHP changes, schema migrations, Gateway runtime changes, protocol activation, deployment, Canary mutation or writes to `blakinio/Oteryn-v2`.
