---
task_id: OTERYN-20260808-native-character-portfolio-context
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
search_first:
  - Native Character Portfolio
  - CharacterProfiles
  - canary_player_id
optional_reads:
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
---

# OTERYN-20260808-native-character-portfolio-context

## Goal

Accept and canonically reconcile the Platform-side **Native Character Portfolio / Account Center v2** boundary selected by the repository owner as Option A, without changing runtime code, persistence, protocol wire format, external repositories or production state.

## Acceptance criteria

- [x] Current Canary compatibility behavior is distinguished from the native Oteryn-v2 target.
- [x] Repository owner explicitly selected Option A and the decision is durably recorded in Issue #857.
- [x] ADR 0030 records Accounts-owned authenticated portfolio composition inside the Laravel modular monolith.
- [x] Oteryn-v2 Character Authority remains authoritative for `CharacterId`, current `AccountId <-> CharacterId` ownership, lifecycle and native mutation outcomes.
- [x] `Characters`, `PublicGameData` and `CharacterProfiles` responsibilities remain non-overlapping and explicit.
- [x] Canary numeric identifiers remain compatibility-only pending a separately authorized additive migration.
- [x] Exact transport, cache TTL, capability-code vocabulary, entitlement exchange and migration implementation remain deferred rather than invented.
- [ ] Exact-head Agent Governance and architecture/documentation validation pass.
- [ ] Exact-head full diff review reports zero open material findings.
- [ ] PR #859 is merged, Issue #857 is closed and this task is archived.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
modules:
  - Accounts
  - Characters
  - CharacterProfiles
  - PublicGameData
dependencies:
  - Issue #857 owner decision: Option A accepted
  - Oteryn-v2 merged PR #90 / ADR-0012 read-only authority evidence
blockers:
  - none
cross_repository_tasks:
  - none; Oteryn-v2 is evidence-only for this Platform task
```

## Decision result

The accepted responsibility split is:

- `Accounts` owns authenticated Account Center / Character Portfolio composition and presentation-ready effective capability state;
- `Characters` owns Platform-side orchestration of explicitly approved character commands, while the native mutation itself remains game-owned;
- Oteryn-v2 Character Authority owns canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, character lifecycle, game-domain capability decisions and mutation receipts/results;
- `PublicGameData` owns public/general game projections and never becomes proof of authenticated Account Center ownership;
- `CharacterProfiles` owns only Platform presentation/privacy preferences and targets canonical `CharacterId` after a later additive migration;
- `PlayerCompanion`, `PlatformAPI`, Marketplace and Support consume module application boundaries rather than raw Canary/Oteryn-v2 character tables.

Current `IdentityCanaryAccount`, `canary_account_id`, `canary_player_id`, direct Canary read models and the current ten-character compatibility rule remain valid only for their already delivered Canary compatibility scope. Acceptance of ADR 0030 does not delete or migrate them.

## Validation classification

E2E is `NOT_APPLICABLE`: this task changes architecture/task documentation only and creates no executable user or integration journey.

Issue #858 remains a separate governance remediation for the merge-gate/regression-control defect that allowed PR #856 to merge while Agent Governance was red. This task repairs the malformed active-task checkpoint because it directly blocks PR #859, but it does not claim to close the broader Issue #858 acceptance inventory.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T22:48:19Z
head: UNKNOWN
branch: docs/OTERYN-20260808-native-character-portfolio-decision
pr: 859
status: validating
context_routes:
  - architecture
  - accounts-characters
  - canary-integration
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
proven:
  - Repository owner explicitly accepted Option A for the Native Character Portfolio / Account Center v2 boundary on 2026-08-08 and Issue 857 contains the durable decision record.
  - Current AccountOverviewReadModel and CharacterProfilePreference use Canary numeric identifiers in the delivered compatibility implementation.
  - Oteryn-v2 merged PR 90 keeps CharacterId, current AccountId-to-CharacterId ownership and native character lifecycle mutations in Character Authority.
  - PR 859 exists on branch docs/OTERYN-20260808-native-character-portfolio-decision.
derived:
  - Accounts is the narrowest existing Platform owner for authenticated Character Portfolio composition without introducing a new service boundary.
  - CharacterProfiles remains presentation and privacy state rather than authoritative character ownership state.
  - New native consumers must use canonical AccountId and CharacterId semantics instead of inheriting Canary numeric identifiers.
unknown:
  - Exact Character Portfolio transport, cache TTL, capability-code vocabulary, entitlement exchange and Canary-to-CharacterId migration mechanics remain deliberately deferred.
  - Final exact PR 859 head and exact-head CI result are not known until the coherent acceptance commit is created and validated.
conflicts: []
first_failure:
  marker: Agent Governance run 31224204311 checkpoint-validation
  evidence: The active task introduced by PR 856 lacked the canonical checkpoint_version and required Context checkpoint fields; Issue 858 records the same root cause.
rejected_hypotheses:
  - No Laravel runtime or product defect is required to explain the Agent Governance failure; the deterministic root cause is malformed task governance state.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
validation:
  - command: focused architecture source reconciliation against ADR 0001, 0008, 0025, 0028, 0029 and Oteryn-v2 PR 90
    result: PASS
    evidence: Option A is consistent with existing authority and introduces no new deployment or mutation authority.
  - command: owner decision gate
    result: PASS
    evidence: Repository owner explicitly selected Option A and the decision was recorded in Issue 857.
  - command: repository exact-head Agent Governance and architecture validators
    result: NOT_RUN
    evidence: The coherent accepted-decision commit has not yet been created; exact-head workflow evidence must run after this checkpoint is committed.
  - command: user or integration E2E
    result: NOT_APPLICABLE
    evidence: Documentation-only architecture reconciliation changes no executable user or system journey.
blockers:
  - none
next_action: Create the coherent accepted-decision commit, validate PR 859 exact head including Agent Governance and architecture registries, perform final diff review, then merge and close Issue 857 if every gate passes.
```
