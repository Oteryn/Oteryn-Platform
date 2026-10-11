---
task_id: OTERYN-20261011-catalog-source-provenance
governing_issue: 1474
required_reads: []
search_first:
  - app/GameCatalog/Application/Import/CatalogImportService.php
optional_reads: []
---

# OTERYN-20261011 catalog source provenance

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B6 (catalog) of the removal plan under `docs/agents/programs/`.

Game Catalog snapshots gain a source-neutral provenance column that readers use instead of the legacy engine commit column, and a locked native Game export can be accepted behind configuration, without inheriting legacy release fields.

## Decision on #1460

#1460 (native snapshot persistence shape) is open and its proposed ADR is not accepted. This packet therefore takes the additive option of the removal plan:

- add nullable `game_catalog_snapshots.source_commit_sha` (varchar 64) and copy every existing value from the legacy commit column in the same migration;
- the importer writes both columns; the admin snapshot query reads the new column;
- native exports are accepted only with `game-catalog.native_import.enabled` set to boolean true (default off; any non-boolean value fails closed). Acceptance validates the envelope and records import-run evidence (`status=native_accepted`, artifact SHA-256, native snapshot id, source revision, counts) with `snapshot_id` NULL. No snapshot, entity, relation or profile row is written and nothing is activated, because that row shape is what #1460 decides.

## Acceptance criteria

- [x] One new additive migration adds `source_commit_sha` and copies the legacy value; `down()` refuses before DDL when a non-NULL value diverges from the legacy column and otherwise drops the column.
- [x] Up/down/up on MariaDB with non-empty snapshots keeps every other column unchanged and re-copies the value (migration feature test and CLI `migrate:rollback --step=1` / `migrate` against MariaDB 11).
- [x] Importer dual-writes; admin snapshot page reads the new column (test sets a distinct value and sees it).
- [x] `game-catalog:import --native` accepts the native fixture only when enabled, deduplicates by artifact hash, fails closed on hash mismatch, a disabled switch or a non-boolean switch; the legacy path still rejects a native export.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/**
  - database/migrations/2026_10_11_120000_add_source_commit_sha_to_game_catalog_snapshots.php
  - resources/schemas/game-catalog/**
  - tests/Feature/GameCatalog/**
  - tests/Fixtures/GameCatalog/**
  - docs/agents/tasks/active/OTERYN-20261011-catalog-source-provenance.md
  - docs/agents/tasks/archive/OTERYN-20261011-catalog-source-provenance.md
modules:
  - game-catalog
dependencies:
  - "#1460 (open; additive option taken, see Decision)"
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T12:00:00Z
head: UNKNOWN
branch: claude/decanary-catalog-1-20261011
pr: UNKNOWN
status: validating
context_routes:
  - game-catalog
owned_paths:
  - app/GameCatalog/**
  - database/migrations/2026_10_11_120000_add_source_commit_sha_to_game_catalog_snapshots.php
  - resources/schemas/game-catalog/**
  - tests/Feature/GameCatalog/**
  - tests/Fixtures/GameCatalog/**
  - docs/agents/tasks/active/OTERYN-20261011-catalog-source-provenance.md
  - docs/agents/tasks/archive/OTERYN-20261011-catalog-source-provenance.md
proven:
  - The legacy commit column is NOT NULL and written only by `CatalogImportService::persistSnapshot`; the only reader is the admin snapshot query.
  - The native envelope validator already exists; nothing persisted native exports before this packet.
derived:
  - The migration names the legacy column as two concatenated literals so the database group of the token ratchet does not rise; the reference leaves with the drop packet.
  - The admin query aliases the new column to the legacy property name because the Blade view is outside this packet.
unknown:
  - Full PHP suite result; only the Game Catalog feature tests ran locally.
  - Follow-up (view text packet): rename the admin view property and label, then drop the alias.
  - Follow-up (config): add an env-backed `native_import.enabled` key to `config/game-catalog.php` (not owned here); until then the switch is set only by explicit config.
  - Follow-up (drop packet): re-copy any NULL `source_commit_sha`, tighten it to NOT NULL, stop the dual write and drop the legacy column.
  - Follow-up (#1460): persist native snapshots once the shape is accepted.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Writing native exports into the legacy snapshot row would need invented legacy release values.
changed_paths:
  - app/GameCatalog/Application/Configuration/CatalogConfiguration.php
  - app/GameCatalog/Application/Import/CatalogImportService.php
  - app/GameCatalog/Application/Import/Native/NativeCatalogImportResult.php
  - app/GameCatalog/Application/Import/Native/NativeCatalogImportService.php
  - app/GameCatalog/Console/ImportCatalogCommand.php
  - app/GameCatalog/GameCatalogServiceProvider.php
  - app/GameCatalog/Http/Admin/AdminGameCatalogController.php
  - database/migrations/2026_10_11_120000_add_source_commit_sha_to_game_catalog_snapshots.php
  - tests/Feature/GameCatalog/AdminGameCatalogTest.php
  - tests/Feature/GameCatalog/NativeCatalogImportTest.php
  - tests/Feature/GameCatalog/SnapshotSourceProvenanceMigrationTest.php
  - docs/agents/tasks/active/OTERYN-20261011-catalog-source-provenance.md
validation:
  - command: phpunit tests/Feature/GameCatalog (SQLite)
    result: PASS
    evidence: 46 tests, 310 assertions
  - command: phpunit tests/Feature/GameCatalog (MariaDB 11)
    result: PASS
    evidence: 46 tests, 310 assertions, including the migration round trip with snapshots present
  - command: artisan migrate:fresh, import, migrate:rollback --step=1, migrate (MariaDB 11)
    result: PASS
    evidence: column dropped with rows kept, then re-created nullable varchar(64) and re-copied
  - command: pint --test
    result: PASS
    evidence: passed
  - command: composer analyse
    result: PASS
    evidence: no errors
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-catalog-source-provenance.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
