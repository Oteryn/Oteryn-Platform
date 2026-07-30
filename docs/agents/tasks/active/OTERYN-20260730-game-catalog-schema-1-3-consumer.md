---
task_id: OTERYN-20260730-game-catalog-schema-1-3-consumer
program_id: GAME-CATALOG-PRODUCTION-COMPLETION
related_issue: 330
status: active
agent: "GPT-5.6 Thinking"
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
base_branch: main
created: 2026-07-30T06:17:00Z
updated: 2026-07-30T20:19:00Z
risk: high
---

# Goal

Implement the complete consumer-first Game Catalog schema `1.3.0` support in Oteryn Platform for inactive NPC and NPC shop-offer snapshots. Preserve schemas `1.0.0`-`1.2.0`, reject unknown types fail closed, persist all new records transactionally, expose bounded administrator candidate diagnostics, and keep public projection and every activation/deployment operation outside this task.

# Acceptance criteria

- [ ] Add the complete canonical Draft 2020-12 schema `1.3.0` by retaining all `1.2.0` constraints and adding only `npc`, `npc_buy_offer` and `npc_sell_offer`.
- [ ] Pin exact schema and fixture SHA-256 values without modifying old schema bytes.
- [ ] Register `1.3.0` as an inactive import consumer while unsupported versions still fail closed.
- [ ] Add explicit typed entity and relation dispatch; never map an unknown type to creature or loot persistence.
- [ ] Persist NPC records, currency endpoints, buy/sell relations, runtime paths, exact prices/subtypes and nullable exact storage conditions transactionally.
- [ ] Reject duplicate keys, dangling/wrong-type source, target or currency endpoints, invalid canonical relation identity and count mismatch.
- [ ] Preserve null/unknown facts and keep verified-boundary activation guards unchanged.
- [ ] Extend administrator candidate inspection with typed counts and unknown/unverified counts without adding public NPC/shop routes.
- [ ] Prove failed import leaves no partial snapshot and preserves the prior active snapshot.
- [ ] Add focused schema, validation, persistence, rollback/candidate isolation and authorization tests.
- [ ] Do not perform staging or production import/activation.

# Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
  - config/game-catalog.php
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Application/AdminRead/**
  - app/GameCatalog/Domain/**
  - database/migrations/*game_catalog*.php
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - .github/workflows/game-catalog-contract.yml
modules:
  - GameCatalog import
  - GameCatalog persistence
  - GameCatalog administrator review
dependencies:
  - Platform issue 330
  - merged Platform PR 331
  - merged Platform PR 332
blockers:
  - local edit/build/test capability is not available through the GitHub connector; exact-head CI is the executable validation boundary
cross_repository_tasks:
  - CAN-20260730-game-catalog-npc-runtime-authority
  - CAN-20260730-game-catalog-schema-1-3-producer
```

# Constraints

- Repository writes remain limited to `blakinio/Oteryn-Platform` and `blakinio/canary`.
- Schema `1.3.0` support is consumer-first and inactive by default.
- Public NPC/shop projection is a later independent task.
- No staging, production, secrets, deployment routing or environment mutation is authorized.
- Unknown historical, availability and dynamic player-specific shop state remains unknown.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T20:19:00Z
head: ea5f4bf81430edd42d1438cb3b5bb48274dd8cff
branch: feat/OTERYN-20260730-game-catalog-schema-1-3-consumer
pr: 338
status: validating
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
  - app/GameCatalog/Application/Import/**
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - database/migrations/*game_catalog*.php
  - tests/Feature/GameCatalog/**
  - tools/game-catalog/**
  - .github/workflows/game-catalog-*.yml
proven:
  - PR #338 is open, draft and mergeable on branch feat/OTERYN-20260730-game-catalog-schema-1-3-consumer; the validated handoff parent head is ea5f4bf81430edd42d1438cb3b5bb48274dd8cff.
  - Schema 1.3.0 and its fixture are pinned to SHA-256 0282c0ce4b995e4aded440b148dd4eb8a96a441e9924da182a2df2a0f2eef8a8 and c4fd9b187e001065f68d90f93dc67f71bb2ff745fc43c3e73110d49b23407ce7; Game Catalog Contract run 30522303950 job 90805246098 passed contract validation.
  - The current PR diff contains explicit item, creature and NPC entity dispatch, explicit loot and NPC shop relation dispatch, a reversible NPC/shop migration, an inactive-only activation guard and focused schema 1.3 consumer tests.
  - Game Catalog Contract run 30522303950 job 90805246155 passed Pint and failed PHPStan only in CatalogConfiguration.php lines 79, 83 and 84 because redundant array-shape checks are inferred as always true or always present.
  - Game Catalog 1.3 Consumer run 30522303348 job 90805241151 failed in Validate exact schema and fixture because tools/game-catalog/validate_contract_fixture.py line 285 reads chance_numerator from an NPC shop relation; the MariaDB lifecycle job was skipped.
  - Platform DB Outage Validation, Game Auth Ticket Concurrency, Agent Governance, Acceptance E2E and Visual UX, Edge Security Emulation, Phase 7 Production-Like Validation, Portal Acceptance Contract and Build Synology Staging Images passed on head a985131bcb675f6543430b6efbdef79449073560.
  - The affected trust boundary is administrator-only candidate inspection and transactional catalog persistence; no public NPC/shop route, activation, staging import, production import or deployment operation was added.
  - No secret or production-only configuration is involved; migration rollback is required and encoded in the dedicated read-only workflow but has not executed because its contract dependency failed.
derived:
  - The next repair must directly edit the shared fixture validator and CatalogConfiguration source; pull-request workflows must not mutate task code.
  - Canary producer support and public NPC/shop projection remain separate later tasks, so the merge gate is not met.
unknown:
  - MariaDB migration, rollback and focused schema 1.3 feature-test results because the lifecycle job was skipped.
  - Administrator typed-summary controller, view and authorization-test changes because the temporary helper did not apply them.
  - Cross-repository Canary producer compatibility and live staging or production state.
conflicts: []
first_failure:
  marker: Game Catalog 1.3 Consumer / contract / Validate exact schema and fixture
  evidence: run 30522303348 job 90805241151; tools/game-catalog/validate_contract_fixture.py line 285 raises KeyError chance_numerator for an NPC shop relation
rejected_hypotheses:
  - Generic fixture semantic validation already supports schema 1.3 shop relations: disproven by run 30522303348 job 90805241151.
  - Temporary pull-request write helpers are a reliable source-editing path: the intended validator and admin source patches were not applied; the admin helper was disabled at 5465d35e8d9b087a893bee05077614769d5d9e33.
changed_paths:
  - .github/workflows/game-catalog-admin-preview-fix.yml
  - .github/workflows/game-catalog-v13-consumer.yml
  - .github/workflows/game-catalog-validator-fix.yml
  - app/GameCatalog/Application/Activation/CatalogActivationService.php
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/CatalogSemanticValidator.php
  - app/GameCatalog/Application/Import/ValidatedCatalogSnapshot.php
  - config/game-catalog.php
  - database/migrations/2026_07_30_061800_add_game_catalog_npc_shop_snapshots.php
  - docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
  - resources/schemas/game-catalog/v1.3/game-catalog-snapshot.schema.json
  - tests/Feature/GameCatalog/CatalogV13ConsumerTest.php
  - tests/Fixtures/GameCatalog/v1.3/minimal-snapshot.json
validation:
  - command: Game Catalog Contract / validate-contract / run 30522303950 job 90805246098
    result: PASS
    evidence: schema compilation and registered v1-v1.2 fixture validation passed on exact PR head a985131bcb675f6543430b6efbdef79449073560
  - command: Game Catalog Contract / static-analysis / run 30522303950 job 90805246155
    result: FAIL
    evidence: Pint passed; PHPStan reports only CatalogConfiguration.php lines 79, 83 and 84
  - command: Game Catalog 1.3 Consumer / contract / run 30522303348 job 90805241151
    result: FAIL
    evidence: KeyError chance_numerator at tools/game-catalog/validate_contract_fixture.py line 285; mariadb-lifecycle skipped
  - command: Remaining PR workflow matrix on head a985131bcb675f6543430b6efbdef79449073560
    result: PASS
    evidence: eight non-catalog workflows completed successfully; CI failure is catalog-related
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md --require-checkpoint
    result: PASS
    evidence: Compact Task Handoff run 30579099402 job 90994560993 step Validate active task checkpoint
  - command: python tools/agents/resume.py --task docs/agents/tasks/active/OTERYN-20260730-game-catalog-schema-1-3-consumer.md
    result: PASS
    evidence: Compact Task Handoff run 30579099402 job 90994560993 step Generate compact resume prompt
blockers:
  - No external blocker; current failures are task-code validation failures and local execution remains unavailable outside GitHub Actions.
next_action: Replace the temporary write helpers with direct source edits that add discriminator-based schema 1.3 NPC/shop fixture validation and simplify CatalogConfiguration schema-contract typing, then rerun Game Catalog Contract and Game Catalog 1.3 Consumer on the exact new head.
```
