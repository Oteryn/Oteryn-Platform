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
- [ ] Repository owner accepts/rejects one option and the decision package is reconciled through normal PR/branch lifecycle.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0043-native-game-catalog-persistence-composition.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20261006-native-game-catalog-persistence-architecture.md
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
updated_at: 2026-10-06T22:58:00+02:00
head: UNKNOWN
branch: docs/1460-native-catalog-persistence-architecture
pr: none
status: waiting
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/architecture/adr/0043-native-game-catalog-persistence-composition.md
  - docs/architecture/adr/README.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20261006-native-game-catalog-persistence-architecture.md
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
  - docs/agents/tasks/active/OTERYN-20261006-native-game-catalog-persistence-architecture.md
validation:
  - command: bounded pre-publication structural self-check
    result: PASS
    evidence: proposed lifecycle, ADR inventory, canonical backlog JSON shape and programme active-ID projection were checked before the single branch candidate was created
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: review package changes documentation/governance only and no executable behavior
  - command: repository exact-head CI
    result: NOT_RUN
    evidence: branch-only candidate intentionally has no PR because current connector publication cannot add the future numeric PR identity without a second candidate commit
blockers:
  - repository owner decision on ARCH-DEC-0006
  - PR publication requires a route that can record the numeric PR identity without violating single-candidate connector publication integrity
next_action: Repository owner reviews Issue #1460 and Proposed ADR 0043 and selects Option A, B or C; then reconcile the decision and PR identity through the normal task publication route.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Proposed architecture decision is awaiting owner selection and has no PR yet.
source_branch_evidence: pending
```

## Notes

This task records a proposal only. It does not authorize schema/runtime implementation, native activation/publication, production/protected-environment work or any Oteryn-Game mutation.
