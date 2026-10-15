---
task_id: OTERYN-20261015-char-cmd-ucc9
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/CharacterAuthority/CharacterCommandClient.php
optional_reads:
  - tests/Support/Concurrency/ForkedWorkers.php
---

# OTERYN-20261015-char-cmd-ucc9

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

This task closes Game gate U-CC9 of `oteryn-game-character-authority-commands-v1` and prepares the §14 step 4 enablement packet. U-CC9 is the cap on intents Platform commits behind the Game consumer's cursor per 30 s rescan interval; the 300 s TTL proof holds for up to 41 unseen keys. Game closes U-CC9 in its contract text (Oteryn/Oteryn-Game, branch `claude/char-cmd-ucc9-enable`).

## Acceptance criteria

- [x] Platform issuance commits no fresh intent behind the cursor:
  - the issuer lock is taken before the row id (`source_revision`) is drawn and held until the outermost transaction commits;
  - `issue()` refuses an intent whose id is not above every stored key of the issuer, with `CharacterCommandUnavailable` and the error log `character_authority.command_source_revision_behind`;
  - on MariaDB the guard is a locking (current) read.
- [x] PostgreSQL concurrency test: 7 issuers × 24 intents, each inside an outer transaction, against a forward-only consumer that never rescans. The consumer lists every stored intent in key order. With the lock made shared, the test fails.
- [x] PostgreSQL regression: an intent drawn below a stored key (sequence re-seated by `setval`) is refused, stores nothing and logs once.
- [x] Enablement runbook `docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md` lists:
  - the Platform switches;
  - the Game `[platform.character_commands]` node table;
  - the preconditions (U-CC7, certificates, the nginx mTLS route list, compose environment, smoke check), the order, rollback and the backlog decision (40, enforced by Platform).
- [x] The pending backlog defaults to 40, and a configured value above 40 disables commands (`CharacterCommandBacklogBoundTest`).
- [x] No production or deploy configuration changed.

## Ownership

```yaml
owned_paths:
  - app/CharacterAuthority/CharacterCommandClient.php
  - tests/Feature/CharacterAuthority/CharacterCommandIssuanceOrderTest.php
  - tests/Feature/CharacterAuthority/Concurrency/CharacterCommandIssuanceOrderPgsqlTest.php
  - docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md
  - config/character-authority.php
  - app/CharacterAuthority/CharacterCommandWire.php
  - tests/Feature/CharacterAuthority/CharacterCommandBacklogBoundTest.php
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-ucc9.md
modules:
  - app/CharacterAuthority
dependencies:
  - Game contract oteryn-game-character-authority-commands-v1 (GAME-CHAR-CMD-0)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game OTV2-20261015-char-cmd-ucc9 (contract text closing U-CC9)
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Validate with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T09:07:29Z
head: the head reported to the control plane
branch: claude/char-cmd-ucc9
pr: 1632
status: completed
context_routes:
  - character-authority
owned_paths:
  - app/CharacterAuthority/CharacterCommandClient.php
  - tests/Feature/CharacterAuthority/CharacterCommandIssuanceOrderTest.php
  - tests/Feature/CharacterAuthority/Concurrency/CharacterCommandIssuanceOrderPgsqlTest.php
  - docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md
  - config/character-authority.php
  - app/CharacterAuthority/CharacterCommandWire.php
  - tests/Feature/CharacterAuthority/CharacterCommandBacklogBoundTest.php
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-ucc9.md
proven:
  - lockedPendingCount() runs before insertGetId in issue(). Its PostgreSQL advisory xact lock and MariaDB locking count last until the outermost transaction ends, so fresh intents commit in source_revision order.
  - With pg_advisory_xact_lock_shared in place of the exclusive lock, the forward-only consumer test fails; with the exclusive lock it passes.
derived:
  - The Game cursor is a committed listed key, so fresh issuance commits 0 intents behind it per 30 s interval.
  - A forward burst is bounded by the backlog; 300 s TTL less 30 s rescan and 24 s listing at 6 s per key covers 41 keys. Control plane decision (Codex P1 on Game PR 2211) - max_pending_intents defaults to 40, and a value above 41 disables commands (CharacterCommandBacklogBoundTest). Fix round 2 (Codex P1 on Game PR 2211, comment 4281392029): the enforced bound is 40, so `MAX_PENDING_BACKLOG` is 40 and a value above 40 disables commands; the PostgreSQL race test issues 35 intents (7 issuers x 5) to stay below the default ceiling (Codex P1 4281372570).
  - §5.4 reopens (U-CC7, PR #1629) are per restore notice and outside the cap.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A per-interval behind-cursor counter on Platform; rejected because issuance cannot commit behind the cursor at all, so a guard on the invariant is smaller and exact.
changed_paths:
  - app/CharacterAuthority/CharacterCommandClient.php
  - tests/Feature/CharacterAuthority/CharacterCommandIssuanceOrderTest.php
  - tests/Feature/CharacterAuthority/Concurrency/CharacterCommandIssuanceOrderPgsqlTest.php
  - docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md
  - config/character-authority.php
  - app/CharacterAuthority/CharacterCommandWire.php
  - tests/Feature/CharacterAuthority/CharacterCommandBacklogBoundTest.php
  - docs/agents/tasks/archive/OTERYN-20261015-char-cmd-ucc9.md
validation:
  - command: vendor/bin/phpunit tests/Feature/CharacterAuthority tests/Feature/Characters
    result: PASS
    evidence: 36 tests OK (2 PostgreSQL-only skipped on SQLite), including CharacterCommandBacklogBoundTest
  - command: vendor/bin/phpunit --configuration phpunit.pgsql.xml tests/Feature/CharacterAuthority
    result: PASS
    evidence: 18 tests OK on local PostgreSQL 16 with PGSQL_INTEGRATION_HOST set
  - command: vendor/bin/pint --test and vendor/bin/phpstan analyse on the changed PHP files
    result: PASS
    evidence: no findings on the first head; the backlog-bound fix round is checked by CI
blockers:
  - none
next_action: none. Review and merge PR 1632; the owner then follows the enablement runbook.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository squash merge of PR 1632
source_branch_evidence: PR 1632 opened from claude/char-cmd-ucc9 against main; the ref is deleted automatically on merge
```

## Notes

- This PR does not touch the files of PR #1629 or #1631 (U-CC7). The restore-notice switch names in the runbook come from #1629's config.
