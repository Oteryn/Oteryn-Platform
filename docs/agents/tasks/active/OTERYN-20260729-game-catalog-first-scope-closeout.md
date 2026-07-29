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
- [x] Validate product-ledger, checkpoint and governance contracts on the exact head.
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
updated_at: 2026-07-29T16:25:00Z
head: b768ec2ee100d4d456da198667318221674cf5d8
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
  - Issue #281 acceptance permits first-scope closure when current server availability, item/creature attributes and loot links are contract-tested with complete public/admin/localization browser evidence.
  - PR #272 delivered immutable versioned item, weapon, creature, reverse-source and visible-loot projections, secured read-only administration and exact cross-repository MariaDB lifecycle evidence.
  - The archived first-slice task records every final Platform and Canary workflow as passing and lists NPC, quests, spawn/map and related extensions as deferred child tasks.
  - Issue #301 owns authoritative spell, NPC, quest and achievement catalogue expansion.
  - Issue #302 owns optional map, hunt-tool and server-specific discovery product decisions.
  - Every former machine-ledger #281 gap reference is reassigned to #277, #301 or #302; delivery statuses remain 20 implemented, 3 partial, 17 missing and 3 not applicable.
  - Relevance counts are derived from the same 43-capability ledger as 22 required, 13 planned, 5 optional/differentiator and 3 not applicable.
  - Run 30468955280 passed the product-completeness validator, checkpoint validator and checkpoint parser tests and confirmed that no #281 gap owner remains in the machine JSON.
  - Main commit b2b2871eed0375e22d48de5dd4947fe29c2bb974 from PR #299 was merged into the feature branch through sync PR #304 without overlap in the five closeout-owned files.
  - Exact head b768ec2ee100d4d456da198667318221674cf5d8 passed all eight required workflows: CI 30469570448, Agent Governance 30469570280, Portal Acceptance Contract 30469570289, Phase 7 Production-Like Validation 30469571639, Platform DB Outage Validation 30469570081, Edge Security Emulation 30469570753, Game Auth Ticket Concurrency 30469569923 and Synology Production Target Preflight 30469570196.
  - Portal Acceptance run 30469570289 passed both strict portal/product ledger validation and the complete zero-retry account lifecycle.
  - Phase 7 run 30469571639 passed production-like schema, privilege, failure, regression, restore and deployment lifecycle validation without a production claim.
derived:
  - Closing #281 for its delivered first scope is accurate because unsupported capabilities remain missing or partial under explicit follow-up ownership rather than being promoted.
  - The closeout changes governance and benchmark ownership only; it does not alter the Game Catalog runtime, schema, producer, activation or public profile.
unknown:
  - Whether the final evidence-only exact head exposes any additional governance issue.
conflicts: []
first_failure:
  marker: none
  evidence: Every required workflow passed at exact head b768ec2ee100d4d456da198667318221674cf5d8; this evidence-only commit requires the final exact-head rerun before merge.
rejected_hypotheses:
  - Closing #281 means spells, NPCs, quests, achievements, maps or hunting tools are implemented.
  - Third-party wiki data can fill missing authoritative server contracts.
  - A closeout task authorizes Canary producer changes.
  - A docs-only closeout establishes production deployment or profile activation.
changed_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
validation:
  - command: node scripts/acceptance/coverage/validate-product-completeness.mjs
    result: PASS
    evidence: Portal Acceptance run 30469570289 validated all 43 capability records and reported no errors after gap-owner reassignment.
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260729-game-catalog-first-scope-closeout.md --require-checkpoint
    result: PASS
    evidence: Agent Governance run 30469570280 validated the active checkpoint against shared contract version 1.
  - command: python tools/agents/test_checkpoint.py
    result: PASS
    evidence: Agent Governance run 30469570280 passed the checkpoint parser tests.
  - command: Required exact-head workflow suite
    result: PASS
    evidence: All eight workflow runs listed under proven completed successfully at b768ec2ee100d4d456da198667318221674cf5d8.
  - command: Required exact-final-head workflow suite
    result: NOT_RUN
    evidence: Pending on this evidence-only checkpoint commit.
blockers:
  - none
next_action: Validate this evidence-only exact head across every required workflow; if all pass, mark PR #303 ready, merge with expected-head protection, verify Issue #281 closure and archive the task.
```

## Notes

This is a scope and evidence reconciliation task. It does not add catalogue entities, mutate Canary, activate a production profile or claim `PRODUCTION_PROVEN`.
