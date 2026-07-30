---
task_id: OTERYN-20260730-game-catalog-schema-1-3-consumer
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
related_issue: 330
status: ready
agent: "GPT-5.6 Thinking"
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
base_branch: main
created: 2026-07-30T06:17:00Z
updated: 2026-07-30T21:17:10Z
risk: high
---

# Goal

Implement the complete consumer-first Game Catalog schema `1.3.0` support in Oteryn Platform for inactive NPC and NPC shop-offer snapshots. Preserve schemas `1.0.0`-`1.2.0`, reject unknown types fail closed, persist all new records transactionally, expose bounded administrator candidate diagnostics, and keep public projection and every activation/deployment operation outside this task.

# Acceptance criteria

- [x] Add the complete canonical Draft 2020-12 schema `1.3.0` by retaining all `1.2.0` constraints and adding only `npc`, `npc_buy_offer` and `npc_sell_offer`.
- [x] Pin exact schema and fixture SHA-256 values without modifying old schema bytes.
- [x] Register `1.3.0` as an inactive import consumer while unsupported versions still fail closed.
- [x] Add explicit typed entity and relation dispatch; never map an unknown type to creature or loot persistence.
- [x] Persist NPC records, currency endpoints, buy/sell relations, runtime paths, exact prices/subtypes and nullable exact storage conditions transactionally.
- [x] Reject duplicate keys, dangling/wrong-type source, target or currency endpoints, invalid canonical relation identity and count mismatch.
- [x] Preserve null/unknown facts and keep verified-boundary activation guards unchanged.
- [x] Extend administrator candidate inspection with typed counts and unknown/unverified counts without adding public NPC/shop routes.
- [x] Prove failed import leaves no partial snapshot and preserves the prior active snapshot.
- [x] Add focused schema, validation, persistence, rollback/candidate isolation and authorization tests.
- [x] Do not perform staging or production import/activation.

# Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - config/game-catalog.php
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Http/Admin/AdminGameCatalogController.php
  - database/migrations/2026_07_30_061800_add_game_catalog_npc_shop_snapshots.php
  - resources/views/game-catalog/admin/snapshot.blade.php
  - tests/Feature/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - .github/workflows/game-catalog-v13-consumer.yml
modules:
  - GameCatalog import
  - GameCatalog persistence
  - GameCatalog administrator review
  - Acceptance browser synchronization
dependencies:
  - Platform issue 330
  - merged Platform PR 331
  - merged Platform PR 332
blockers:
  - Canary schema 1.3 producer compatibility evidence is not part of this Platform consumer task and remains required before merge.
cross_repository_tasks:
  - CAN-20260730-game-catalog-npc-runtime-authority
  - CAN-20260730-game-catalog-schema-1-3-producer
```

# Constraints

- Repository writes for this task remain limited to `blakinio/Oteryn-Platform`; Canary is read-only until separately authorized.
- Schema `1.3.0` support is consumer-first and inactive by default.
- Public NPC/shop projection is a later independent task.
- No staging, production, secrets, deployment routing or environment mutation is authorized.
- Unknown historical, availability and dynamic player-specific shop state remains unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T21:17:10Z
head: b1adb5355871cc7ede579799669d38ca323e3dcc
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
pr: 338
status: ready
context_routes:
  - architecture
  - public-game-data
  - canary-integration
  - database
  - admin-rbac
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - config/game-catalog.php
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Http/Admin/AdminGameCatalogController.php
  - database/migrations/2026_07_30_061800_add_game_catalog_npc_shop_snapshots.php
  - resources/views/game-catalog/admin/snapshot.blade.php
  - tests/Feature/GameCatalog/**
  - tools/game-catalog/validate_contract_fixture.py
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - .github/workflows/game-catalog-v13-consumer.yml
proven:
  - PR #338 is open, draft and mergeable on branch feat/OTERYN-20260730-game-catalog-schema-1-3-consumer; the exact validated code head is b1adb5355871cc7ede579799669d38ca323e3dcc.
  - Schema 1.3.0 and its fixture remain pinned to SHA-256 0282c0ce4b995e4aded440b148dd4eb8a96a441e9924da182a2df2a0f2eef8a8 and c4fd9b187e001065f68d90f93dc67f71bb2ff745fc43c3e73110d49b23407ce7.
  - Discriminator-based fixture validation handles creature_loot, npc_buy_offer and npc_sell_offer explicitly and validates endpoint types, currency identity and canonical shop identity without weakening schemas 1.0.0-1.2.0.
  - CatalogConfiguration normalizes the version-keyed schema contract map before returning a typed contract; bounded Pint and PHPStan pass.
  - NPC and shop-offer persistence is transactional, migration rollback is guarded and tested, invalid candidates leave no partial catalog state, and rejection preserves a previously active snapshot and its projections.
  - Administrator-only snapshot inspection exposes typed entity and relation counts plus unknown-or-unverified counts without adding a public NPC/shop route or activation operation.
  - The schema 1.3 activation guard remains inactive-import-only and focused tests prove zero profile activation, profile entity projection, profile relation projection and activation audit events.
  - Acceptance E2E WebKit publication synchronization now waits for both lifecycle redirects and passed the zero-retry portability profile on the exact validated code head.
  - No staging import, production import, activation, deployment or production-only configuration change was performed.
derived:
  - The Platform consumer implementation is complete and ready for cross-repository compatibility review, but PR #338 must remain draft and unmerged until the separate Canary schema 1.3 producer task proves the pinned contract.
  - Public NPC/shop projection remains a later independent task after producer compatibility and does not belong in this consumer PR.
unknown:
  - Cross-repository Canary producer compatibility with the pinned Platform schema 1.3 consumer.
  - Live staging and production catalog state because no environment import or deployment was authorized.
conflicts: []
first_failure:
  marker: none
  evidence: All required exact-head Platform validation completed successfully on b1adb5355871cc7ede579799669d38ca323e3dcc.
rejected_hypotheses:
  - Generic fixture semantic validation already supported schema 1.3 shop relations; disproven by the original chance_numerator KeyError and repaired with explicit relation dispatch.
  - Temporary pull-request workflow helpers were an acceptable source-editing mechanism; they did not apply the intended source patches and were removed.
  - The WebKit 404 indicated a public Wiki application defect; artifacts showed the lifecycle test advanced without proving redirect completion, and explicit status waits made portability pass.
changed_paths:
  - .github/workflows/game-catalog-v13-consumer.yml
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/CatalogSemanticValidator.php
  - app/GameCatalog/Application/Import/ValidatedCatalogSnapshot.php
  - app/GameCatalog/Http/Admin/AdminGameCatalogController.php
  - config/game-catalog.php
  - database/migrations/2026_07_30_061800_add_game_catalog_npc_shop_snapshots.php
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - resources/views/game-catalog/admin/snapshot.blade.php
  - scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs
  - tests/Feature/GameCatalog/AdminGameCatalogTest.php
  - tests/Feature/GameCatalog/CatalogV13ConsumerTest.php
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - tools/game-catalog/validate_contract_fixture.py
validation:
  - command: Game Catalog 1.3 Consumer / contract / run 30582235285 job 91005055850
    result: PASS
    evidence: Exact schema and fixture validation passed on b1adb5355871cc7ede579799669d38ca323e3dcc.
  - command: Game Catalog 1.3 Consumer / mariadb-lifecycle / run 30582235285 job 91005129110
    result: PASS
    evidence: Bounded Pint and PHPStan, MariaDB migration, focused schema 1.3 tests and guarded rollback passed on the exact validated code head.
  - command: Game Catalog Contract / run 30582235239
    result: PASS
    evidence: Registered catalog contract validation and static analysis passed on the exact validated code head.
  - command: CI / run 30582235362 job 91005056247
    result: PASS
    evidence: Composer validation and audit, formatting, static analysis and the full test suite passed on the exact validated code head.
  - command: Acceptance E2E and Visual UX / run 30582235309 job 91005056015
    result: PASS
    evidence: Chromium smoke, Chromium-Firefox-WebKit portability, responsive, dependency resilience and keyboard accessibility profiles passed; zero Playwright retries were configured.
  - command: Exact-head supporting PR workflow matrix
    result: PASS
    evidence: Agent Governance, Platform DB Outage Validation, Game Auth Ticket Concurrency, Edge Security Emulation, Phase 7 Production-Like Validation, Portal Acceptance Contract and Build Synology Staging Images completed successfully on b1adb5355871cc7ede579799669d38ca323e3dcc.
blockers:
  - Merge gate remains closed pending separate Canary schema 1.3 producer compatibility evidence against the pinned Platform consumer contract.
next_action: Implement and validate the separate Canary schema 1.3 producer task against the pinned Platform consumer contract; do not merge PR #338 until that compatibility evidence exists.
```
