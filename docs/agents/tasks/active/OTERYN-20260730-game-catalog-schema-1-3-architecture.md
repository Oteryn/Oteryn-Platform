---
task_id: OTERYN-20260730-game-catalog-schema-1-3-architecture
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
related_issue: 330
status: active
agent: chatgpt
branch: docs/OTERYN-20260730-game-catalog-schema-1-3-architecture
base_branch: main
created: 2026-07-29T22:25:00Z
updated: 2026-07-29T22:53:00Z
risk: medium
---

# Goal

Define the consumer-first schema `1.3.0` NPC/shop architecture without registering support or changing product/runtime/deployment state.

# Result

- Proposed `npc`, `npc_buy_offer` and `npc_sell_offer` only.
- Preserved immutable schemas `1.0.0`-`1.2.0`.
- Added strict extension proposal SHA-256 `7e9699ecb04bbc777679e22cee9352ae49ff21220eff7294c042358f01d0571e`.
- Added full fixture proposal SHA-256 `b603b4ef1ccbe763d6f5f7565f40d6604027e73101006d359ccfc4aae06f10ca`.
- Kept location/reachability, quests, spawns, public projection and production outside this task.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T22:53:00Z
head: b7f996bb130b09998cfce427eb789f0072047ba8
branch: docs/OTERYN-20260730-game-catalog-schema-1-3-architecture
pr: 332
status: ready
context_routes:
  - architecture
  - public-game-data
  - canary-integration
  - database
  - admin-rbac
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-architecture.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/game-catalog/v1.3/game-catalog-npc-shop-extension.schema.json
  - docs/contracts/game-catalog/v1.3/minimal-snapshot.proposal.json
  - docs/contracts/game-catalog/v1.3/COMPATIBILITY.md
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-architecture.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/game-catalog/v1.3/game-catalog-npc-shop-extension.schema.json
  - docs/contracts/game-catalog/v1.3/minimal-snapshot.proposal.json
  - docs/contracts/game-catalog/v1.3/COMPATIBILITY.md
proven:
  - Current Platform dispatch cannot safely persist NPC/shop types.
  - Final Canary Npcs/NpcType/ShopBlock state is the proposed runtime authority.
  - The extension schema and fixture validate with zero Draft 2020-12 errors.
  - Semantic checks pass and negative duplicate, dangling, identity and count cases fail as expected.
  - Exact-head b7f996bb130b09998cfce427eb789f0072047ba8 passed Agent Governance, CI, Edge Security Emulation, Platform DB Outage Validation, Game Auth Ticket Concurrency and Phase 7 Production-Like Validation.
derived:
  - Platform must implement the complete strict consumer before Canary producer support.
  - Registration does not prove NPC encounterability or item obtainability.
unknown:
  - Safe deterministic Canary registry iteration API.
  - Default storage zero-pair sentinel semantics.
  - Dynamic player-specific shop completeness.
  - Live staging and production state.
conflicts:
  - Issue 301 retains older producer-first and Canary-read-only assumptions superseded by issue 330.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Global item prices are per-NPC offer authority.
  - Registry presence proves encounterability.
  - Quests or spawns belong in schema 1.3.0.
validation:
  - command: Draft202012Validator and semantic fixture checks
    result: PASS
    evidence: valid proposal clean; negative cases detected.
  - command: exact-head b7f996bb130b09998cfce427eb789f0072047ba8 workflow matrix
    result: PASS
    evidence: all six Platform workflows completed successfully.
blockers: []
next_action: Review and merge PR 332, then start the independent Platform schema 1.3 inactive consumer task.
```
