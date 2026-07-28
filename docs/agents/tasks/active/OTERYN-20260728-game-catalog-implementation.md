---
task_id: OTERYN-20260728-game-catalog-implementation
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/adr/0016-versioned-game-catalog-snapshots.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - resources/schemas/game-catalog/v1/game-catalog-snapshot.schema.json
search_first:
  - active Wiki, route, migration, RBAC, audit, navigation and acceptance ownership
  - open pull requests touching shared Platform integration paths
  - existing localized public routes, Wiki routes, admin permission, MFA, audit and pagination conventions
optional_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260728-game-catalog-implementation

## Goal

Deliver the first production-quality Platform vertical slice of the version-aware Oteryn Game Catalog: shared-contract validation, immutable transactional snapshot import, profile activation and rollback, visibility projections, public item/weapon/creature/loot surfaces and administrative snapshot/visibility inspection without production deployment or activation.

## Acceptance criteria

- [ ] The Platform schema remains byte-identical to Canary schema version `1.0.0` with SHA-256 `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b`.
- [ ] The shared sanitized fixture imports transactionally and proves release-, completeness-, availability- and relation-gated visibility for at least two target releases.
- [ ] Releases, immutable snapshots, profiles, stable entities, identifiers, typed item/creature/loot snapshots, import runs, findings and visibility projections are persisted with reversible migrations.
- [ ] Validate/import/activate/diff/verify CLI commands follow the merged contract; import is inactive by default and failed operations preserve public state.
- [ ] Activation and rollback lock the profile and switch projections transactionally.
- [ ] Public routes are registered before generic Wiki article routing and expose bounded item, weapon, creature and visible loot navigation without leaking internal provenance or findings.
- [ ] Administrator catalogue routes use authentication, confirmed MFA, exact permissions, CSRF for mutations and bounded audit events.
- [ ] Focused migration, schema, semantic, transaction, authorization, public-query and browser acceptance coverage is added.
- [ ] Required exact-head CI passes before readiness; no production deployment or profile activation occurs.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/**
  - config/game-catalog.php
  - database/migrations/*game_catalog*
  - resources/fixtures/game-catalog/**
  - resources/navigation/public/game-catalog.php
  - resources/navigation/admin/game-catalog.php
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - lang/en/game_catalog.php
  - lang/pl/game_catalog.php
  - public/css/game-catalog.css
  - tests/Unit/GameCatalog/**
  - tests/Feature/GameCatalog/**
  - tests/Integration/GameCatalog/**
  - scripts/acceptance/**/*game-catalog*
  - .github/workflows/*game-catalog*
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-implementation.md
  - docs/agents/tasks/deferred/OTERYN-20260728-game-catalog-*.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
modules:
  - GameCatalog
  - Wiki
  - Admin
  - Audit
  - Integration
  - Testing
dependencies:
  - contract: oteryn.game-catalog
  - schema_version: 1.0.0
  - CAN-20260728-game-catalog-exporter
  - parent: OTERYN-20260728-versioned-game-catalog-architecture
blockers:
  - local clone/build/test unavailable because sandbox DNS cannot resolve github.com; implementation validation must use repository CI until a runnable checkout is available
cross_repository_tasks:
  - CAN-20260728-game-catalog-exporter
```

## Coordination

- Rollout order: Canary exporter and shared fixture validation first; Platform importer second; activation/public visibility third; cross-repository E2E fourth.
- Platform PR #270 currently edits shared integration points including `app/Admin/AdminPermission.php`, `app/Localization/LocalizedPublicRouteRegistrar.php`, `app/Providers/AppServiceProvider.php`, shared layouts and public navigation. This task does not claim those files exclusively. Any required edit must be rebased onto current `main` and reconciled narrowly after #270 state is verified.
- The schema is contract-owned and must not be silently changed. A discovered defect blocks dependent implementation and requires coordinated versioned updates in both repositories.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T08:20:51Z
head: 8aa1fc29dd13895efb2a7006204a6b88105e6972
branch: feat/OTERYN-20260728-game-catalog-implementation
pr: none
status: investigating
context_routes:
  - agent-governance
  - architecture
  - public-game-data
  - canary-integration
  - database
  - web-cms
  - admin-rbac
  - security
  - testing
owned_paths:
  - app/GameCatalog/**
  - database/migrations/*game_catalog*
  - resources/fixtures/game-catalog/**
  - resources/views/game-catalog/**
  - routes/modules/game-catalog.php
  - tests/**/GameCatalog/**
  - scripts/acceptance/**/*game-catalog*
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-implementation.md
proven:
  - Platform main is 8aa1fc29dd13895efb2a7006204a6b88105e6972 and contains merged architecture PR 271.
  - Shared contract ID is oteryn.game-catalog, schema version is 1.0.0 and expected content SHA-256 is 099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b.
  - Structured catalogue persistence is separate from editorial Wiki Markdown persistence.
  - Public catalogue routes must precede the generic Wiki slug route.
  - PR 270 overlaps only shared integration paths, not the dedicated GameCatalog module paths.
derived:
  - The implementation can proceed in dedicated paths while shared integration edits remain explicitly coordinated.
  - Public visibility must be materialized per profile and snapshot rather than inferred ad hoc in public controllers.
unknown:
  - exact current framework registration points after PR 270 is merged or rebased
  - exact final Canary loader boundary and staging snapshot contents
  - complete introduced/removed history, availability evidence, sprite source and historical mappings
conflicts:
  - Platform PR 270 edits shared route/provider/admin/navigation/layout integration paths; reconcile before touching those paths
first_failure:
  marker: sandbox-network-unavailable
  evidence: direct git clone failed because github.com DNS could not be resolved
rejected_hypotheses:
  - Store catalogue entities as ordinary Wiki Markdown articles.
  - Import or activate through a browser file upload in the first slice.
  - Guess missing versions, availability, locations or zero-valued attributes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-game-catalog-implementation.md
validation:
  - command: GitHub connector preflight, branch/PR/task overlap search and architecture baseline verification
    result: PASS
    evidence: current main, required architecture commits, open PRs, active work indexes and ownership overlap were inspected
  - command: local clone/build/test
    result: NOT_RUN
    evidence: sandbox DNS cannot resolve github.com
blockers:
  - local runtime validation unavailable until CI or another runnable checkout is used
next_action: Add the shared sanitized fixture and automated schema/hash contract validation without changing schema bytes.
```

## Notes

External wikis are UX and information-architecture references only. Canary final runtime registries and reviewed Oteryn metadata remain authoritative.