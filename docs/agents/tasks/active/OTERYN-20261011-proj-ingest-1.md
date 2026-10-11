---
task_id: OTERYN-20261011-proj-ingest-1
governing_issue: 1474
required_reads: []
search_first:
  - app/GameAuth/NativeAccountCharacters/NativeAccountCharactersIngestion.php
optional_reads: []
---

# OTERYN-20261011 public projection ingest

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. This is packet C4 (public projection ingest) of the removal plan under `docs/agents/programs/`.

Platform ingests Game Public Projections v1 into read-only read models. The six snapshot families and the per-family watermark arrive on dedicated internal mTLS routes. Ordering follows the Game contract `docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md`, merged in Game PR #1937. A serve-time read model gates every family on its watermark freshness. Public pages (C5-C7) build on that read model in later packets.

## Acceptance criteria

- [x] `POST /internal/v1/game-public/{highscores,character-profile,character-deaths,guild,house,world-online,watermark}` accept only the dedicated public-projection identity:
  - mTLS with TLS 1.3 and `SSL_CLIENT_VERIFY=SUCCESS`;
  - the configured source authority;
  - an identity disjoint from every other internal identity;
  - refusal on other internal routes.
- [x] Wire validation follows contract §4:
  - exact member sets;
  - canonical JSON (no duplicates, escapes, whitespace or `null`);
  - uint64 decimal strings;
  - UUIDv7 ids;
  - names in NFC;
  - list bounds and ordering;
  - depth of at most 5 containers;
  - the per-family byte bounds of §8, with 400 over the bound.
- [x] Ordering follows §6. A lower pair is `superseded`. An equal pair with equal content is an idempotent `accepted` that keeps the first `source_observed_at`. An equal pair with different content is 409, with the first snapshot kept. A higher epoch hides every family until its new-epoch publication arrives.
- [x] Watermarks follow §5.1. They are per family and monotonic within an epoch. Future-dated or inverted times are refused. The family stops being served when `now - complete_through + 1 > 30`, when the watermark is future-dated, or when its epoch is below the highest epoch seen.
- [x] The consumer is default-off: it is enabled only in `testing`/`preproduction` with `game-projections.public.enabled` true, and answers 503 otherwise. `config/game-projections.php` maps the `GAME_PROJECTIONS_PUBLIC_*` env keys, listed in `.env.example`.
- [x] Migration for the three read-model tables. Rollback is refused while Game-published rows exist.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes. Pint, PHPStan and PHPUnit were not runnable in the authoring sandbox: PHP 8.5 is required and composer could not install. The first CI run passed the feature tests on sqlite; its MariaDB migration, Pint and PHPStan findings are fixed.

## Ownership

```yaml
owned_paths:
  - app/GameProjections/**
  - database/migrations/2026_10_11_120000_create_game_public_projection_read_models.php
  - routes/internal.php
  - config/game-projections.php
  - .env.example
  - tests/Feature/GameProjections/**
  - docs/agents/tasks/active/OTERYN-20261011-proj-ingest-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-proj-ingest-1.md
modules:
  - game-projections
dependencies:
  - Game PR 1937 (public projections contract v1)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T19:00:00Z
head: UNKNOWN
branch: claude/decanary-proj-ingest-1-20261011
pr: 1530
status: validating
context_routes:
  - game-auth
owned_paths:
  - app/GameProjections/**
  - database/migrations/2026_10_11_120000_create_game_public_projection_read_models.php
  - routes/internal.php
  - config/game-projections.php
  - .env.example
  - tests/Feature/GameProjections/**
  - docs/agents/tasks/active/OTERYN-20261011-proj-ingest-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-proj-ingest-1.md
proven:
  - All seven contract wire examples (§4.1-§4.6, §5.1) parse with the shipped validators. A standalone script without the framework also showed the duplicate, null, escape, whitespace, extra-member and NFC cases refused, and every list bound accepted at its limit and refused one past it.
derived:
  - Epoch invalidation happens at read time. Entries and watermarks below the highest epoch are not served, so no mass update runs on an epoch raise.
  - A 409 keeps the first snapshot without marking it invalid (contract §6), unlike the LCFA consumer.
  - Over-bound bodies answer 400 as the contract says, not 413 as the LCFA consumer does.
  - A served entry must carry the epoch its family's watermark check validated, so an epoch raise that commits mid-read cannot serve a new-epoch entry under an old-epoch watermark.
unknown:
  - The Game contract lock still marks the projections contract as a candidate revision; the owner decided the PR need not wait for acceptance because the consumer is off by default.
  - Pint, PHPStan and the PHP test suite results; CI is the first run.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/GameProjections/**
  - database/migrations/2026_10_11_120000_create_game_public_projection_read_models.php
  - routes/internal.php
  - config/game-projections.php
  - .env.example
  - tests/Feature/GameProjections/PublicProjectionIngestTest.php
  - docs/agents/tasks/active/OTERYN-20261011-proj-ingest-1.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-proj-ingest-1.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: vendor/bin/pint --test, composer analyse, php artisan test tests/Feature/GameProjections
    result: NOT_RUN
    evidence: composer install was not possible in the authoring sandbox; CI head 53da341c passed the feature tests on sqlite, and its MariaDB index-name, Pint and PHPStan failures are fixed for the next head
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: ingest only; no public page reads the read model until C5-C7
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
