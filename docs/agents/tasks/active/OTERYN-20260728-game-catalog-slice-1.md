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

Deliver the Platform half of the first production-quality vertical slice of the version-aware Oteryn Game Catalog: shared contract validation, immutable transactional snapshot import, profile activation and rollback, visibility projections, public item/weapon/creature/loot surfaces, and administrative snapshot/visibility inspection. Production deployment and production profile activation are excluded.

## Acceptance criteria

- [ ] Shared fixture and schema hash validation are byte-identical with `blakinio/canary`.
- [ ] Storage, import, activation, rollback and verification commands satisfy the merged architecture and import contract.
- [ ] Public item, weapon, creature and visible-loot routes are version/completeness/availability gated.
- [ ] Administrator surfaces use exact RBAC permissions, confirmed MFA, CSRF and bounded audit events.
- [ ] Focused, repository-required and cross-repository E2E validation pass on the final head.
- [ ] No production deployment or production profile activation occurs.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - .github/workflows/game-catalog-contract.yml
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
modules:
  - GameCatalog
  - Wiki public navigation integration
  - Admin RBAC/MFA/audit integration
dependencies:
  - CAN-20260728-game-catalog-exporter-slice-1
  - oteryn.game-catalog schema version 1.0.0
blockers:
  - Shared integration paths modified by open PR #270 must not be edited until ownership is reconciled from current main.
  - Local PHP/Node execution is unavailable in the current sandbox; CI evidence is required for executable validation.
cross_repository_tasks:
  - CAN-20260728-game-catalog-exporter-slice-1
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
updated_at: 2026-07-28T10:36:00+02:00
head: 98cd827188490db17aba9e43db02d63b51ec4d70
branch: feat/OTERYN-20260728-game-catalog-slice-1
pr: 272
status: implementing
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
proven:
  - main head is architecture merge commit 8aa1fc29dd13895efb2a7006204a6b88105e6972
  - merged architecture defines contract oteryn.game-catalog schema 1.0.0 and fail-closed visibility
  - Platform and Canary schema files have the same Git blob SHA a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - no open Platform PR is dedicated to Game Catalog
  - open PR #270 modifies shared provider, routing, navigation and layout integration paths
  - draft PR #272 tracks this task
  - sanitized fixture contains two releases, visible/future items, complete/partial creatures and visible/future loot relations
  - shared validator performs pinned hash checks, Draft 2020-12 validation, semantic integrity checks and two-release visibility assertions
  - Game Catalog Contract run 30342833946 passed on head 98cd827188490db17aba9e43db02d63b51ec4d70
  - repository writes are authorized only in blakinio/Oteryn-Platform and blakinio/canary
  - production deployment and production activation are excluded
derived:
  - matching Git blob SHAs prove the two schema files are byte-identical
  - identical fixture and validator bytes plus pinned SHA-256 values create a cross-repository contract gate
  - initial contract/fixture work can proceed without touching PR #270 shared paths
unknown:
  - final integration shape after PR #270 lands or ownership is explicitly reconciled
  - executable local PHP test results because the sandbox cannot clone GitHub or run the repository checkout
  - complete historical content and availability facts listed by the architecture
conflicts:
  - potential future ownership overlap with PR #270 for app/Providers/AppServiceProvider.php, localized route registration, shared layouts and navigation aggregation
first_failure:
  marker: checkpoint-validation
  evidence: Agent Governance run 30342833669 failed after the workflow path was added but before the checkpoint changed_paths list was refreshed
rejected_hypotheses:
  - external wiki data is authoritative
  - imported snapshots activate automatically
  - unknown values may be converted to zero or guessed
changed_paths:
  - .github/workflows/game-catalog-contract.yml
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-slice-1.md
  - tests/Fixtures/GameCatalog/v1/minimal-snapshot.json
  - tools/game-catalog/validate_contract_fixture.py
validation:
  - command: GitHub repository and main-head inspection
    result: PASS
    evidence: main head 8aa1fc29dd13895efb2a7006204a6b88105e6972
  - command: GitHub schema blob comparison
    result: PASS
    evidence: both schema paths resolve to blob a3c239a6d61385edde0b06f72cdf781f4ce58df3
  - command: local synthetic fixture semantic validation
    result: PASS
    evidence: counts, ranges, endpoints, probability/count bounds and 15.20/15.21 visibility assertions passed; fixture SHA-256 c947e461c1ee8f6fbf511c9890b61135d2585d6c16e2e99a0f72dd5a946c2181
  - command: local validator syntax and semantic smoke
    result: PASS
    evidence: Python validator executed against a Draft 2020-12 smoke schema and the exact fixture
  - command: Game Catalog Contract
    result: PASS
    evidence: workflow run 30342833946 on head 98cd827188490db17aba9e43db02d63b51ec4d70
  - command: Agent Governance
    result: FAIL
    evidence: workflow run 30342833669; checkpoint refreshed in this commit
  - command: open PR ownership inspection
    result: PASS_WITH_CONFLICT_RECORDED
    evidence: PR #270 changed-file inventory reviewed
  - command: local checkout/build/test
    result: NOT_RUN
    evidence: sandbox DNS cannot resolve github.com
blockers:
  - shared integration paths remain held pending reconciliation with PR #270
next_action: Inspect renewed PR #272 governance and contract checks, then begin the isolated Platform persistence/import package.
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
