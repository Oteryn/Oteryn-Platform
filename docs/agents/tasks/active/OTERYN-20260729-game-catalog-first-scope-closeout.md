---
task_id: OTERYN-20260729-game-catalog-first-scope-closeout
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
search_first:
  - open pull requests and active tasks overlapping Issue #281 or Game Catalog knowledge capabilities
  - PR #272 exact-head and cross-repository acceptance evidence
  - remaining product-ledger gap references to #281
optional_reads:
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
  - docs/agents/tasks/archive/OTERYN-20260728-game-catalog-slice-1.md
---

# OTERYN-20260729-game-catalog-first-scope-closeout

## Goal

Close Issue #281 for the first server-backed Game Catalog scope already delivered by PR #272, while moving unsupported planned and optional catalogue/knowledge capabilities to explicit follow-up trackers without marking them implemented.

## Acceptance criteria

- [x] Verify that PR #272 satisfies Issue #281's first-scope acceptance for versioned current-server availability, item/creature attributes, exact loot links and public/admin/localization browser lifecycle.
- [x] Create a dedicated tracker for authoritative spell, NPC, quest and achievement catalogues.
- [x] Create a separate discovery tracker for optional maps, hunting tools and server-specific discovery capabilities.
- [x] Reassign every machine-ledger `#281` gap reference to the correct remaining owner without changing delivery status unsupported by evidence.
- [x] Reconcile the human-readable benchmark, PROJECT_STATE and ACTIVE_WORK with the split scope.
- [ ] Validate product-ledger, checkpoint and governance contracts on the exact head.
- [ ] Merge the closeout PR and close Issue #281 without claiming production deployment or completion of deferred capabilities.
- [ ] Archive this task in a separate documentation PR after merge.

## Ownership

```yaml
owned_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md
modules:
  - GameCatalog
  - Wiki
  - Testing
dependencies:
  - PR #272 and archived task OTERYN-20260728-game-catalog-slice-1
  - Issue #301 for structured spell/NPC/quest/achievement expansion
  - Issue #302 for optional knowledge/discovery planning
blockers:
  - none
cross_repository_tasks:
  - none; this closeout does not modify Canary or any external repository
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T15:46:00Z
head: 7f2c37ca2a2447623705f60dffc08a34b9c755d3
branch: chore/OTERYN-20260729-game-catalog-first-scope-closeout
pr: 303
status: validating
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - testing
  - canary-integration
owned_paths:
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md
proven:
  - Issue #281 remains open and its acceptance permits first-scope closure when current server availability, item/creature attributes and loot links are contract-tested with complete public/admin/localization browser evidence.
  - PR #272 delivered immutable versioned item, weapon, creature, reverse-source and visible-loot projections, secured read-only administration and exact cross-repository MariaDB lifecycle evidence.
  - The archived first-slice task records every final Platform and Canary workflow as passing and lists NPC, quests, spawn/map and related extensions as deferred child tasks.
  - Issue #301 now owns authoritative spell, NPC, quest and achievement catalogue expansion.
  - Issue #302 now owns optional map, hunt-tool and server-specific discovery product decisions.
derived:
  - Closing #281 for its delivered first scope is accurate only if all remaining ledger references are moved to #277, #301 or #302 and their current delivery statuses remain unchanged.
unknown:
  - Whether any exact-head repository validator exposes an additional stale #281 ownership reference outside the reconciled files.
conflicts: []
first_failure:
  marker: pending-exact-head-validation
  evidence: Ledger ownership and human-state reconciliation are committed; exact-head repository workflows have not completed yet.
rejected_hypotheses:
  - Closing #281 means spells, NPCs, quests, achievements, maps or hunting tools are implemented.
  - Third-party wiki data can fill missing authoritative server contracts.
  - A closeout task authorizes Canary producer changes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
validation:
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: The temporary closeout reconciler validated all 43 capability records after gap-owner reassignment.
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md --require-checkpoint
    result: PASS
    evidence: The active checkpoint satisfies the shared version-1 contract.
  - command: Required exact-head workflow suite
    result: NOT_RUN
    evidence: Pending normal user-authored cleanup/checkpoint head after temporary reconciler removal.
blockers:
  - none
next_action: Remove the temporary reconciler and workflow, validate the resulting exact head and fix only the first reproducible failure.
```

## Notes

This is a scope and evidence reconciliation task. It does not add catalogue entities, mutate Canary, activate a production profile or claim `PRODUCTION_PROVEN`.
