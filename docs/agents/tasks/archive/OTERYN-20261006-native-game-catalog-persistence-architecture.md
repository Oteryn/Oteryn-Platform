---
task_id: OTERYN-20261006-native-game-catalog-persistence-architecture
governing_issue: 1460
required_reads:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/adr/0034-native-game-catalog-content-ownership.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
search_first:
  - native GameCatalog persistence
  - game_catalog_snapshots Canary
optional_reads: []
---

# OTERYN-20261006-native-game-catalog-persistence-architecture

## Goal

Governing GitHub Issue: #1460 — canonical lifecycle authority for this task.

Produce a decision-ready, non-authoritative Platform Architecture Review package for native GameCatalog snapshot persistence composition. Compare viable alternatives, allocate one Proposed ADR, update the unresolved decision backlog and preserve runtime implementation as unauthorized until repository-owner acceptance.

## Acceptance criteria

- [x] Current persistence/activation/public-read assumptions are grounded in protected-main source evidence.
- [x] At least two viable alternatives plus the status-quo implications are compared across correctness, migration, operability and rollback.
- [x] ADR 0043 is explicitly `Proposed`, never accepted by this task.
- [x] `ARCH-DEC-0006` is recorded as `decision_required` with one owner question and `implementation_authorized: false`.
- [x] Architecture programme projection names exactly the active decision ID.
- [x] No runtime code, migration, workflow, external repository or protected-environment path changes.
- [x] Decision package merged through PR #1487 (owner decision itself carried to the Follow-up section, not claimed here).

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0043-native-game-catalog-persistence-composition.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/archive/OTERYN-20261006-native-game-catalog-persistence-architecture.md
modules:
  - Platform Architecture Review
  - GameCatalog documentation governance
dependencies:
  - Issue #1460
  - ADR 0034
blockers:
  - repository owner decision on Option A/B/C
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T02:00:00Z
head: ef303edb4acbe1e7f194b56b7057f871ede93f19
branch: docs/1460-native-catalog-persistence-architecture
pr: 1487
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/architecture/adr/0043-native-game-catalog-persistence-composition.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/archive/OTERYN-20261006-native-game-catalog-persistence-architecture.md
proven:
  - Protected main at task admission is a834ef54b154c48b119da20c3bdb6fca74178750.
  - Existing Legacy Canary GameCatalog persistence and activation carry mandatory release/runtime assumptions.
  - Native envelope validation is integrated but native persistence/activation is intentionally absent.
derived:
  - Option B best preserves strict Legacy Canary invariants while keeping one Platform lifecycle and an additive migration path.
unknown:
  - Repository owner decision among Option A, B or C.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Dummy Canary provenance is an acceptable native persistence bridge.
  - A second native profile/activation control plane preserves one catalogue lifecycle.
changed_paths:
  - docs/architecture/adr/0043-native-game-catalog-persistence-composition.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/archive/OTERYN-20261006-native-game-catalog-persistence-architecture.md
validation:
  - command: bounded pre-publication structural self-check
    result: PASS
    evidence: proposed lifecycle, ADR inventory, canonical backlog JSON shape and programme active-ID projection were checked before the single branch candidate was created
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: review package changes documentation/governance only and no executable behavior
  - command: repository exact-head CI
    result: NOT_RUN
    evidence: PR 1487 opened; exact-head CI is read by the integrating lead
blockers:
  - repository owner decision on ARCH-DEC-0006
next_action: none for this record; repository owner decision on ARCH-DEC-0006 (Option A, B or C for Issue #1460) and reconciliation of Proposed ADR 0043 remain open and need a new task record (see Follow-up).
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1487 merged (REST pulls/1487 state closed, merged true) as 692c85c32b67d61447f3131835b72ea762e28224 at 2026-10-11T01:12:26Z from exact head ef303edb4acbe1e7f194b56b7057f871ede93f19; source branch absent (REST branches API returns 404)
```

## Notes

This task records a proposal only. It does not authorize schema/runtime implementation, native activation/publication, production/protected-environment work or any Oteryn-Game mutation.

## Follow-up

Follow-up: repository owner decision on ARCH-DEC-0006 (Option A, B or C for Issue #1460) and reconciliation of Proposed ADR 0043 remain open and need a new task record; this record is completed for the PR #1487 decision-package scope only.
