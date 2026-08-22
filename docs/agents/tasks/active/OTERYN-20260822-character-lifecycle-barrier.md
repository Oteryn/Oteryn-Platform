---
task_id: OTERYN-20260822-character-lifecycle-barrier
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/contracts/CHARACTER_TRANSFER_CONTRACT.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
search_first:
  - issues #317 #319 #320
  - active character lifecycle tasks and open PRs
  - accepted world-transfer product decision
optional_reads: []
---

# OTERYN-20260822-character-lifecycle-barrier

## Goal

Execute `OTERYN-CHARACTER-LIFECYCLE-BARRIER` as a bounded Platform-only discovery/barrier task: classify Issues #317, #319 and #320 independently from current protected-main authority, record the smallest truthful dependency matrix, and identify whether any executable Platform-only prerequisite is safe without inventing Oteryn-v2 game semantics.

This task does not implement deletion, rename or world/channel transfer runtime behavior and does not access or modify Oteryn-v2, Canary or another server/game repository.

## Acceptance criteria

- [ ] #317, #319 and #320 are independently classified from current accepted Platform evidence.
- [ ] Platform versus game-domain responsibility and canonical `AccountId`, `CharacterId`, `WorldId` / `ChannelId` semantics are explicit.
- [ ] Deletion/restore, rename and transfer each state which game-owned semantics are accepted and which remain missing.
- [ ] Mutual exclusion with Character Bazaar and sibling lifecycle mutations is explicit.
- [ ] #320 remains blocked unless durable current evidence proves player-selectable native world/channel transfer is an accepted product capability.
- [ ] No Canary compatibility behavior or stale historical record is promoted into native game authority.
- [ ] No runtime implementation is started while a material game-owned contract or product decision is `UNKNOWN`.
- [ ] The smallest durable Platform-side barrier result and one concrete unblock action are recorded.
- [ ] Exact-head documentation/governance validation, full-diff self-review and PR hygiene pass before merge.

## Ownership

```yaml
project_lane: oteryn-platform-core
task_kind: discovery
implementation_authorized: false
execution_mode: github
execution_reason: repository-only architecture and dependency classification with no permitted game-repository access
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260822-character-lifecycle-barrier.md
  - docs/agents/reports/OTERYN-20260822-character-lifecycle-barrier.md
modules:
  - Characters
  - Accounts
dependencies:
  - ADR 0029
  - ADR 0030
  - ADR 0031
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - Issues #317 #319 #320
blockers:
  - none for the bounded barrier classification
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T17:00:00Z
head: UNKNOWN
branch: agent/oteryn-20260822-character-lifecycle-barrier
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - accounts-characters
  - canary-integration
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260822-character-lifecycle-barrier.md
  - docs/agents/reports/OTERYN-20260822-character-lifecycle-barrier.md
proven:
  - protected main starts this task at 8e609f05278816102a08fcbeb9d102642c8380a0
  - Issues #317 #319 and #320 are open and blocked
  - Issue #919 is closed and the shared native Character Authority command/result semantic contract is accepted on main
  - no open PR found for the exact character lifecycle barrier or Issues #317 #319 #320
  - existing active-work index shows no character lifecycle task owner overlapping the declared paths
  - repository governance forbids server/game repository access for this invocation without separate explicit permission
  - ADR 0030 and ADR 0031 assign native character mutation authority to Oteryn-v2 Character Authority while Platform owns authenticated UX policy gates and orchestration
  - ADR 0029 assigns canonical WorldId and ChannelId to Platform World Registry while game-domain placement remains game authority
  - the shared command contract keeps world/channel transfer capability-gated rather than product-approved
  - Character Bazaar ownership transfer remains a separate higher-risk saga in which authoritative ownership transfer precedes wallet settlement
derived:
  - the previously missing generic operation identity idempotency typed-outcome ambiguity and reconciliation baseline is no longer the shared blocker
  - runtime lifecycle work must remain fail-closed wherever product-specific game semantics or exact producer mapping remain unknown
unknown:
  - exact native deletion lifecycle and automatic-versus-explicit finalization semantics
  - exact native rename producer mapping and game-owned name-policy/result mapping
  - whether player-selectable native world/channel transfer is an adopted product capability
  - exact native transfer placement and eligibility semantics
  - exact Oteryn-v2 transport endpoint IDL receipt persistence and reconciliation mechanism for these command families
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Canary compatibility contracts can define native lifecycle semantics: rejected by ADR 0030 ADR 0031 and the focused native lifecycle authority guide
  - the shared transfer command profile proves world transfer is approved: rejected by the command contract and Issue #320
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260822-character-lifecycle-barrier.md
validation:
  - command: exact-head documentation/governance validation
    result: NOT_RUN
    evidence: barrier report not yet persisted
blockers:
  - none for the bounded discovery task
next_action: Persist the three-operation dependency matrix and smallest truthful unblock handoff in the owned barrier report.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: bounded barrier task is still active
source_branch_evidence: pending
```

## Notes

The task is `discovery_first` and intentionally has no game-repository dependency execution. Material `UNKNOWN` stays unresolved rather than being converted into a Platform assumption.