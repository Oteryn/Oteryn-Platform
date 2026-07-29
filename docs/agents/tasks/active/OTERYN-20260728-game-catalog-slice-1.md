---
task_id: OTERYN-20260728-game-catalog-slice-1
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
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
search_first:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - current Wiki routes, navigation, controllers and acceptance tests
  - current RBAC, MFA, audit and administration conventions
optional_reads: []
---

# OTERYN-20260728-game-catalog-slice-1

## Goal

Deliver the Platform half of the first production-quality version-aware Oteryn Game Catalog slice: immutable transactional import, profile activation and rollback, projected public visibility, public item/weapon/creature/loot surfaces and secured administrative inspection. Production deployment and production profile activation are excluded.

## Acceptance criteria

- [x] Shared fixture and schema hash validation are byte-identical with `blakinio/canary`.
- [x] Storage, import, activation, rollback and verification commands satisfy the merged architecture and import contract.
- [x] Public item, weapon, creature and visible-loot routes are version/completeness/availability gated.
- [x] Administrator inspection uses exact RBAC permissions, confirmed MFA and bounded audit history; the delivered surface is read-only, so no mutating browser form requires a CSRF token.
- [x] Focused, repository-required and cross-repository E2E validation pass on the delivered implementation and generated staging-only Canary artifact.
- [x] No production deployment or production profile activation occurs.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - .github/workflows/game-catalog-contract.yml
  - bootstrap/providers.php
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - resources/schemas/game-catalog/v1/**
  - resources/fixtures/game-catalog/v1/**
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - scripts/acceptance/**/*game-catalog*
shared_integration_paths:
  - app/Admin/AdminPermission.php
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - app/Providers/AppServiceProvider.php
  - resources/views/admin/layout.blade.php
  - resources/views/game/layout.blade.php
  - routes/console.php
  - scripts/acceptance/playwright.config.mjs
modules:
  - GameCatalog
  - Wiki public navigation integration
  - Admin RBAC/MFA/audit integration
dependencies:
  - CAN-20260728-game-catalog-exporter
  - oteryn.game-catalog schema version 1.0.0
blockers: []
cross_repository_tasks:
  - CAN-20260728-game-catalog-exporter
```

## Cross-repository contract and rollout

```yaml
contract_id: oteryn.game-catalog
schema_version: 1.0.0
expected_schema_sha256: 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b
platform_schema: resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
canary_schema: schemas/game-catalog/v1/game-catalog-snapshot.schema.json
compatibility: atomic-required
rollout_order:
  - validate byte-identical schema and shared fixture in both repositories
  - deliver Canary deterministic offline exporter
  - deliver Platform inactive transactional importer
  - deliver Platform profile activation and rollback
  - deliver public and administrative surfaces
  - run cross-repository E2E and generate a staging-only snapshot
production_activation: forbidden
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T07:10:00+02:00
head: 077874dac473746ca3c9074d697caf21c7a2775b
branch: feat/OTERYN-20260728-game-catalog-slice-1
pr: 272
status: ready
context_routes:
  - agent-governance
  - architecture
  - database
  - web-cms
  - admin-rbac
  - security
  - testing
  - canary-integration
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - .github/workflows/game-catalog-contract.yml
  - bootstrap/providers.php
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - resources/schemas/game-catalog/v1/**
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tools/game-catalog/**
  - scripts/acceptance/**/*game-catalog*
proven:
  - Canary PR 991 merged first as commit 4ae896d9c6ad33e4193a314f47daeff9ea4ac66b after exact-head CI and Game Catalog runtime smoke passed.
  - Platform and Canary use the coordinated Game Catalog schema v1 contract and sanitized shared fixture.
  - Snapshot import is immutable, transactional and inactive by default; failures preserve the previously active profile.
  - Profile activation, candidate activation and rollback are explicit audited operations with verification and diff support.
  - Public item, weapon, creature, reverse-source and visible-loot pages read only active public projections and preserve unknown or incomplete facts.
  - Administrator snapshot, profile, finding, projection and diff inspection is read-only, permission-gated and confirmed-MFA-gated with bounded audit history.
  - Public and administrator browser acceptance covers desktop, tablet and mobile layouts, Chromium, Firefox and WebKit portability, keyboard navigation and accessibility.
  - Acceptance E2E and Visual UX run 30398956682 passed after the Game Catalog locators and responsive containment were corrected.
  - Portal Acceptance Contract run 30398956738 and repository CI run 30398956735 passed after the public navigation expectation was updated.
  - Exact generated Canary artifact 8714331268 from run 30427617799 has digest sha256:e389915bff1f79e21cbb7b112717550587d3a556afa11e707c0036ba8b2aa5a6 and producer SHA 84b089f9a919bb85773798584e5b0205e2e5895c.
  - Game Catalog Contract run 30428491404 verified the staged payload and passed MariaDB baseline import, activation, candidate import, candidate activation and rollback.
  - The cross-repository staging profile remained public_enabled=false and no production deployment or production profile activation occurred.
derived:
  - The Canary producer and Platform consumer are compatible for schema 1.0.0 item, creature and creature-loot data.
  - The staging artifact is validation evidence only and does not authorize production activation.
unknown:
  - Complete historical introduction, removal, spawn, quest, NPC and availability evidence remains outside this slice.
  - Public sprite sourcing and exact 7.60 compatibility remain deferred.
conflicts: []
first_failure:
  marker: none
  evidence: Focused, repository acceptance and cross-repository MariaDB lifecycle validation are green.
rejected_hypotheses:
  - external wiki data is authoritative
  - imported snapshots activate automatically
  - unknown values may be converted to zero or guessed
  - a working functional route alone proves responsive or accessible acceptance
  - non-unique producer identifiers may be silently repaired by Platform
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - app/GameCatalog/**
  - app/Localization/LocalizedPublicRouteRegistrar.php
  - bootstrap/providers.php
  - config/game-catalog.php
  - database/migrations/*game_catalog*.php
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - scripts/acceptance/**/*game-catalog*
  - scripts/acceptance/playwright.config.mjs
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - tests/Fixtures/GameCatalog/**
  - tests/Unit/GameCatalog/**
  - tools/game-catalog/**
validation:
  - command: Canary exact-head final gate
    result: PASS
    evidence: PR 991 merged after CI run 30429320048 and Game Catalog run 30429319990 passed on final head 1aad762053140b2773825d75dbfc42ce5d13a2f2
  - command: Platform Game Catalog Contract and cross-repository staging lifecycle
    result: PASS
    evidence: run 30428491404 passed schema validation, Pint, PHPStan, exact staged artifact verification and MariaDB import/activate/rollback
  - command: Platform repository CI
    result: PASS
    evidence: run 30398956735 passed on the synchronized implementation head
  - command: Acceptance E2E and Visual UX
    result: PASS
    evidence: run 30398956682 passed public and admin Game Catalog responsive, portability and accessibility coverage
  - command: Portal Acceptance Contract
    result: PASS
    evidence: run 30398956738 passed strict route coverage and portal contract validation
  - command: Agent Governance, Edge Security, DB outage and production-like validation
    result: PASS
    evidence: runs 30398956858, 30398956713, 30398956972 and 30398956774 passed
blockers: []
next_action: Merge Platform PR #272.
```

## Deferred child tasks

- NPC catalogue.
- Quests.
- Spawn and raid availability.
- Map reachability.
- Public sprite sourcing.
- Historical release metadata.
- Backport administration.
- 7.60 compatibility.
