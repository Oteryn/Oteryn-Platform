---
task_id: OTERYN-20261011-legacy-ids-nullable
governing_issue: 1474
required_reads: []
search_first:
  - database/migrations/2026_10_08_090000_add_native_ids_to_character_bazaar_and_profile_preferences.php
optional_reads: []
---

# OTERYN-20261011 legacy ids nullable

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet C0 (LEGACY-NULLABLE-1) of the removal plan under `docs/agents/programs/`.

One backward-compatible migration makes the legacy-engine id columns of `character_auctions` (seller account, escrow account, `player_id`) and `character_profile_preferences` (legacy player id) nullable, so the later cutover packets (C3 Bazaar, C8 preferences) can write rows that carry only the native ids added by #1477. The legacy unique keys do not guard rows without a legacy id, so the native guards ship in the same migration. No read or write path changes.

## Acceptance criteria

- [x] The four legacy id columns are nullable after `up()`; existing rows, child bids and the legacy unique keys are unchanged.
- [x] Native guard: unique `(identity_id, character_id)` on `character_profile_preferences`.
- [x] Native guard: `character_auctions.active_character_id` (char 36, nullable, unique), the native twin of `active_player_id`; `up()` copies `character_id` into it for rows whose `active_player_id` is set, and refuses before any DDL when existing native ids would violate either guard.
- [x] `down()` requires maintenance mode outside tests, holds one table lock for checks and DDL on MariaDB, and refuses before any DDL while any relaxed column holds NULL.
- [x] Tests in the programme's test directory under `tests/Feature/`: up with existing rows, native-only inserts, both guards, rollback refusal per relaxed column, maintenance-mode rollback, up refusal on duplicates, MariaDB lock order and an independent-process race on both guards.
- [x] The existing native-id rollback test unwinds the new migration first, matching the real rollback order.
- [ ] Legacy token ratchet: tests and docs stay within baseline; the `database` group (+1, the one line naming the relaxed columns) and the `paths` group (+1, the new test file in the programme's test directory) rise and need a baseline decision (see Notes).
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - database/migrations/2026_10_11_120000_relax_legacy_ids_and_add_native_guards.php
  - tests/Feature/*/LegacyIdsNullableMigrationTest.php
  - tests/Feature/*/NativeIdsMigrationRollbackTest.php
  - docs/agents/tasks/active/OTERYN-20261011-legacy-ids-nullable.md
  - docs/agents/tasks/archive/OTERYN-20261011-legacy-ids-nullable.md
modules:
  - marketplace
  - character-profiles
dependencies:
  - "#1477 native ids (merged)"
blockers:
  - legacy token baseline decision for the database and paths groups
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T12:00:00Z
head: UNKNOWN
branch: claude/decanary-legacy-nullable-1-20261011
pr: none
status: validating
context_routes:
  - marketplace
owned_paths:
  - database/migrations/2026_10_11_120000_relax_legacy_ids_and_add_native_guards.php
  - tests/Feature/*/LegacyIdsNullableMigrationTest.php
  - tests/Feature/*/NativeIdsMigrationRollbackTest.php
  - docs/agents/tasks/active/OTERYN-20261011-legacy-ids-nullable.md
  - docs/agents/tasks/archive/OTERYN-20261011-legacy-ids-nullable.md
proven:
  - No application code writes `character_auctions.character_id` or `character_profile_preferences.character_id` yet, so the copy into `active_character_id` and both duplicate checks find nothing on existing data.
  - Application code sets `active_player_id` when a listing is created and clears it on every terminal path (reconcile, recover); C3 must maintain `active_character_id` on the same paths.
derived:
  - Native guards rely on NULL being distinct in unique indexes (MariaDB and SQLite), so legacy rows without a native id are unaffected.
unknown:
  - MariaDB run of the lock-order and race tests: they skip on SQLite and no workflow runs this test directory against MariaDB yet (needs a step in the MariaDB concurrency workflow, outside this packet's paths).
conflicts:
  - Plan section 7 asks every packet to keep each ratchet group from rising; a migration that names the legacy columns and a new file in the programme's test directory cannot.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A generated column derived from `status` for the active-listing guard; rejected because `active_player_id` is set and cleared by code, not by status, and the twin must follow the same rule.
changed_paths:
  - database/migrations/2026_10_11_120000_relax_legacy_ids_and_add_native_guards.php
  - tests/Feature/*/LegacyIdsNullableMigrationTest.php
  - tests/Feature/*/NativeIdsMigrationRollbackTest.php
  - docs/agents/tasks/active/OTERYN-20261011-legacy-ids-nullable.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: FAIL
    evidence: database and paths groups each one above baseline; all other enforced groups within baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-legacy-ids-nullable.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: pint --test, composer analyse, php artisan test on the programme's test directory
    result: NOT_RUN
    evidence: dependencies could not be installed in the authoring session (GitHub archive downloads denied by the egress proxy; source installs too slow); exact-head CI is the first run
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: schema-only migration with no read or write path change; migration tests cover the behaviour
blockers:
  - legacy token baseline decision for the database and paths groups
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

- Ratchet: the migration names the relaxed columns on one line and the new test file sits in the programme's test directory, whose name contains the legacy token. Both rises are inherent to the packet as planned; the baseline file is outside this packet's paths. Options: raise `database` and `paths` by one each in this PR, or land the raise separately first.
- Follow-up for C3: write `active_character_id` wherever `active_player_id` is written or cleared. Follow-up for C8: the preference service keys on `character_id` and relies on the new unique key.
