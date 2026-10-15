---
task_id: OTERYN-20261015-platform-restore-reconcile-p1
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/CharacterAuthority/CharacterCommandExchange.php
  - app/CharacterAuthority/Restore/GameRestoreReconciler.php
optional_reads: []
---

# OTERYN-20261015 Platform restore reconciliation (PLATFORM-RESTORE-RECONCILE-P1)

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Cross-repository: Game OPERABILITY §3 rulings 9–11 and A3/A4; Game contract `oteryn-game-character-authority-commands-v1` §4.3, §5, §5.4 and U-CC7.

Platform side of a Game restore: accept Game's restore notice, reopen as PENDING every character command intent whose receipt carries `recovery_generation` ≥ the restored generation (by generation only, superseded receipt kept as history), and acknowledge the notice once every reopened operation has exactly one new Game receipt. No cross-database transaction. Default-off.

## Acceptance criteria

- [x] `POST /internal/v1/game-auth/game-restore/notice` behind a dedicated default-off TLS 1.3 mTLS peer guard with rate limit; disabled answers 503; responses are no-store.
- [x] Notice wire is exact (400 on malformed, 401 on another source authority, 409 on same event with other bytes or a stale generation, 413 above 512 bytes).
- [x] Reopen selects by `recovery_generation` ≥ H only; generation < H stays decided; superseded receipt kept with `superseded_at`.
- [x] Reopened intents are listed and read again under their unchanged operation identity and accepted with one new receipt, including an identical republication.
- [x] ACKNOWLEDGED only when every reopened operation has a new receipt; idempotent repeats; a later notice also waits for operations an earlier one left open.
- [x] COMMITTED → REJECTED reversal raises `character_authority.restore_outcome_reversed`.
- [x] Platform-restore self-heal (lost receipt) is tested; Platform's acknowledgement time is stated in the contract and was ratified by the owner (review after the first restore drill).
- [x] Migration rollback is refused while notices exist.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/CharacterAuthority/Restore/**
  - app/CharacterAuthority/CharacterCommandExchange.php
  - app/CharacterAuthority/CharacterCommandSettings.php
  - config/character-authority.php
  - routes/internal.php
  - database/migrations/2026_10_15_120000_create_character_restore_notices.php
  - tests/Feature/CharacterAuthority/GameRestoreReconciliationTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-restore-reconcile-p1.md
  - docs/agents/tasks/archive/OTERYN-20261015-platform-restore-reconcile-p1.md
modules:
  - game-auth
dependencies:
  - OTERYN-20261011-character-cmd-client
  - OTERYN-20261014-char-cmd-wire-1
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game restore step 11 (notice delivery and acknowledgement wait)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T08:00:00Z
head: 2184dfae996884f018f9073b1fba00743c5ba164
branch: claude/platform-restore-reconcile-p1
pr: 1627
status: validating
context_routes:
  - game-auth
owned_paths:
  - app/CharacterAuthority/Restore/**
  - app/CharacterAuthority/CharacterCommandExchange.php
  - app/CharacterAuthority/CharacterCommandSettings.php
  - config/character-authority.php
  - routes/internal.php
  - database/migrations/2026_10_15_120000_create_character_restore_notices.php
  - tests/Feature/CharacterAuthority/GameRestoreReconciliationTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-restore-reconcile-p1.md
  - docs/agents/tasks/archive/OTERYN-20261015-platform-restore-reconcile-p1.md
proven:
  - The intent/receipt store from #1532 already keeps receipt history (`superseded_at`) and stores `recovery_generation` zero-padded to 20 characters, so selection by generation is a text comparison.
  - Receipts have no unique constraint per operation, so one new receipt per reopened operation fits the existing table.
  - Platform never calls Game; every exchange is a Game-initiated mTLS POST, so the acknowledgement is the answer to a repeated notice.
derived:
  - The owner ratified the acknowledgement bound (no Platform wait; outage bounded by the provisional 4 h Platform RTO) on 2026-10-15, for review after the first restore drill (Oteryn/Oteryn-Game#1622 comment 6187301378 item 11).
  - The restore surface has its own default-off switch without an environment limit, because Game admission waits on the acknowledgement even where commands are disabled.
  - Automated compensation of a reversed outcome is deferred to the first settling consumer; until then the reversal raises an alarm.
  - A4 (delivery reconciliation by generation) is recorded as accepted for the future delivery implementation; Platform has no delivery code yet.
unknown:
  - PHP suite, PHPStan and Pint results; composer cannot download packages in the authoring sandbox, so CI is the first run.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Cross-database transaction or Platform calling Game; the contracts forbid both, so reconciliation runs through the unchanged command routes.
  - Selecting reopened intents by time, `decided_at` or the restore point; ruling 10 selects by generation only.
  - Refusing a new receipt equal to the superseded bytes; Game may still hold a decision made before the snapshot within generation H.
  - Gating the restore route on character-authority.commands.enabled; that would leave Game admission closed after a restore on stacks without commands.
changed_paths:
  - app/CharacterAuthority/Restore/GameRestoreNotice.php
  - app/CharacterAuthority/Restore/GameRestoreNoticeSettings.php
  - app/CharacterAuthority/Restore/GameRestoreNoticeWire.php
  - app/CharacterAuthority/Restore/GameRestoreReconciler.php
  - app/CharacterAuthority/Restore/Http/GameRestoreNoticeController.php
  - app/CharacterAuthority/Restore/Http/GuardGameRestoreNoticePeer.php
  - app/CharacterAuthority/CharacterCommandExchange.php
  - app/CharacterAuthority/CharacterCommandSettings.php
  - config/character-authority.php
  - routes/internal.php
  - database/migrations/2026_10_15_120000_create_character_restore_notices.php
  - tests/Feature/CharacterAuthority/GameRestoreReconciliationTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20261015-platform-restore-reconcile-p1.md
validation:
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: php artisan test tests/Feature/CharacterAuthority
    result: NOT_RUN
    evidence: vendor cannot be installed in the authoring sandbox (GitHub dist downloads refused); runs on the exact head in CI
  - command: vendor/bin/phpstan and vendor/bin/pint --test
    result: NOT_RUN
    evidence: same sandbox limitation; CI runs them
blockers:
  - none
next_action: Drive the PR to green CI and a clean Codex review, enable squash auto-merge, then report to CP.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
