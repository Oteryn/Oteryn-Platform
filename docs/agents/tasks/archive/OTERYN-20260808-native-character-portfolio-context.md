---
task_id: OTERYN-20260808-native-character-portfolio-context
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
search_first:
  - Native Character Portfolio
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
- [x] Exact-head Agent Governance and architecture/documentation validation passed.
- [x] Exact-head full diff review reported zero open material findings.
- [x] PR #859 merged, Issue #857 closed and this task is archived.

## Decision result

Option A is Accepted:

- `Accounts` owns authenticated Account Center / Character Portfolio composition inside the Laravel modular monolith;
- `Characters` owns Platform-side orchestration of explicitly approved character commands;
- Oteryn-v2 Character Authority remains authoritative for canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, lifecycle and native mutation outcomes;
- `PublicGameData` remains public/general projection and is not authenticated ownership proof;
- `CharacterProfiles` owns Platform presentation/privacy preferences and targets canonical `CharacterId` after a separately authorized additive migration;
- Canary numeric identifiers and direct Canary paths remain compatibility-only until that migration/cutover is separately authorized.

The canonical architecture authority is ADR 0030 and the focused architecture documents merged by PR #859. This archived task record is historical execution evidence, not a replacement architecture authority.

## Closeout

PR #859 exact head `b3e08b2251a755baddacfe709504227b8534dfb5` passed Agent Governance, CI and the documented architecture validation set, then merged to `main` as `73c2426b37cfd5028fe9fbcec8254cc8aab3bc80` on 2026-08-07T22:54:43Z. Issue #857 is closed as completed.

The resulting-main Agent Governance run `31225419202` found only that this already-completed task was still represented in `tasks/active` with a stale merge next action. Issue #858 repair moves the record to archive; it does not alter the Accepted architecture.

E2E is `NOT_APPLICABLE`: this task changed architecture/task documentation only and created no executable user or integration journey.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T22:59:53Z
head: 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80
branch: docs/OTERYN-20260808-native-character-portfolio-decision
pr: 859
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - architecture
  - accounts-characters
  - canary-integration
  - testing
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
proven:
  - Repository owner accepted Option A and Issue #857 is closed as completed.
  - PR #859 exact head b3e08b2251a755baddacfe709504227b8534dfb5 passed Agent Governance and CI before merge.
  - PR #859 merged to main as 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80.
  - ADR 0030 is the canonical Accepted Native Character Portfolio architecture decision.
derived:
  - The architecture task is terminal and no longer owns an active execution lease.
unknown:
  - Exact Character Portfolio transport, cache TTL, capability-code vocabulary, entitlement exchange and Canary-to-CharacterId migration mechanics remain deliberately deferred to separately authorized future tasks.
conflicts: []
first_failure:
  marker: none
  evidence: PR #859 exact-head gates passed; the post-merge failure was a task-lifecycle closeout defect resolved by archival under Issue #858.
rejected_hypotheses:
  - The post-merge Agent Governance failure invalidates ADR 0030: run 31225419202 reports only terminal active-task lifecycle findings, not architecture or checkpoint-structure failure.
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260808-native-character-portfolio-context.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
validation:
  - command: PR #859 exact-head Agent Governance
    result: PASS
    evidence: PR #859 records exact head b3e08b2251a755baddacfe709504227b8534dfb5 with Agent Governance PASS.
  - command: PR #859 exact-head CI and architecture validation
    result: PASS
    evidence: PR #859 records CI and all documented architecture validation jobs PASS with zero material diff findings.
  - command: user or integration E2E
    result: NOT_APPLICABLE
    evidence: Documentation-only architecture reconciliation changes no executable user or system journey.
blockers:
  - none
next_action: Archive completed; any runtime implementation or compatibility migration requires a separately authorized future task.
```
