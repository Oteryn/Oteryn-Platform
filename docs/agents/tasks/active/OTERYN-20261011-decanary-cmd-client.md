---
task_id: OTERYN-20261011-decanary-cmd-client
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/CharacterAuthority/CharacterCommandClient.php
  - app/CharacterAuthority/CharacterCommandExchange.php
optional_reads:
  - app/GameAuth/NativeAccountCharacters/NativeAccountCharactersWire.php
---

# OTERYN-20261011 Character Authority command client

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet C1 (DECANARY-CMD-CLIENT-1) of the removal plan under `docs/agents/programs/`.

Platform side of Game contract `oteryn-game-character-authority-commands-v1` (Game PR #1936, revised by #1959): an immutable intent store for `CreateCharacter` and `TransferCharacterOwnership`, keyed by a caller idempotency key and a fresh UUIDv7 `operation_id`; the three Game-initiated mTLS exchanges (pending list, intent read, receipt publication); and a receipt store with history. Platform never calls Game. Disabled by default and limited to `testing`/`preproduction`; the routes are not mounted, so no existing flow changes.

## Acceptance criteria

- [x] `config/character-authority.php`: `commands.enabled` defaults to false; `CharacterCommandSettings::current()` is null unless enabled in `testing`/`preproduction` with a dedicated identity list that shares no subject with another internal purpose, a source authority, TTL 1..300 s, backlog 1..256 and a rate limit.
- [x] Intent bytes follow the contract's member order and canonical JSON; the row id is the `source_revision`; the intent is immutable and every read is byte-identical.
- [x] Retry with the same idempotency key returns the same `operation_id` and bytes (also after a terminal receipt and while the backlog is full); the same key with another binding is a conflict; a new key is a new operation.
- [x] Timeout: an intent past `expires_at_source` stays PENDING (listed and readable) until Game's receipt; Platform never marks it failed. A REJECTED `CHAR_CMD_INTENT_EXPIRED` receipt terminalizes it.
- [x] Receipt replay: identical bytes answer 200 without a write; different bytes, another command or a committed binding that contradicts the intent answer 409 and leave the intent PENDING; no intent is 404; malformed is 400; wrong source authority is 401.
- [x] Peer guard: 503 when disabled or misconfigured, 401 without the dedicated TLS 1.3 client identity, 429 with `Retry-After` per identity; every response is `no-store`.
- [x] Backlog bound (`CHARCMD-PENDING-BACKLOG`) refuses a new intent until a receipt frees a slot.
- [x] Migration rollback is refused while intents or receipts exist.
- [x] Logs carry only operation id, command and result code.
- [ ] Exact-head CI passes (PHP tests, PHPStan and Pint were not runnable in the authoring sandbox: PHP 8.5 and the vendor tree are unavailable).

## Ownership

```yaml
owned_paths:
  - app/CharacterAuthority/**
  - config/character-authority.php
  - database/migrations/2026_10_11_120000_create_character_command_intents_and_receipts.php
  - tests/Feature/CharacterAuthority/**
  - docs/agents/tasks/active/OTERYN-20261011-decanary-cmd-client.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-cmd-client.md
modules:
  - game-auth
dependencies:
  - none
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
branch: claude/decanary-cmd-client-1-20261011
pr: 1532
status: validating
context_routes:
  - game-auth
owned_paths:
  - app/CharacterAuthority/**
  - config/character-authority.php
  - database/migrations/2026_10_11_120000_create_character_command_intents_and_receipts.php
  - tests/Feature/CharacterAuthority/**
  - docs/agents/tasks/active/OTERYN-20261011-decanary-cmd-client.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-cmd-client.md
proven:
  - The Game contract inverts the call direction (Game pulls over mTLS); the Platform "client" is an intent store, three Game-initiated routes and a receipt store.
  - The contract keeps an expired intent PENDING until Game's receipt (§4.1, §5.3), so timeout handling is "never fail locally".
derived:
  - The intent row id doubles as `source_revision`; concurrent issuance can commit revisions out of order, which §5.3 permits and the consumer's bounded rescan covers.
  - Receipt `recovery_generation` is stored zero-padded to 20 digits so a restore can select by generation as text (§5.4); restore reopen itself belongs to PLATFORM-RESTORE-RECONCILE-P1.
unknown:
  - PHP suite, PHPStan and Pint results; composer cannot install here, so CI is the first run.
  - Concurrency of the locking backlog count and the unique-key retry is proven only by design on MariaDB; the suite runs on SQLite.
  - The Game contract is still Candidate (lock entry PENDING_CANONICAL_MERGE); wire changes on acceptance may need a follow-up.
  - Follow-up enablement packet: mount `CharacterCommandRoutes::register()` in `routes/internal.php`, add the env keys to `.env.example`, and make other internal guards refuse character-command subjects.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - An outbound HTTP client with request timeouts and retries; the contract forbids Platform from calling Game.
changed_paths:
  - app/CharacterAuthority/**
  - config/character-authority.php
  - database/migrations/2026_10_11_120000_create_character_command_intents_and_receipts.php
  - tests/Feature/CharacterAuthority/CharacterCommandClientTest.php
  - docs/agents/tasks/active/OTERYN-20261011-decanary-cmd-client.md
validation:
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-decanary-cmd-client.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php artisan test tests/Feature/CharacterAuthority
    result: NOT_RUN
    evidence: PHP 8.5 and vendor unavailable in the authoring sandbox; runs on the exact head in CI
  - command: composer analyse and pint --test
    result: NOT_RUN
    evidence: vendor unavailable in the authoring sandbox; runs on the exact head in CI
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
