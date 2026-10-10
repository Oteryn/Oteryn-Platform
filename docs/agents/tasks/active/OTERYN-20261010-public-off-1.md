---
task_id: OTERYN-20261010-public-off-1
governing_issue: 1474
required_reads: []
search_first:
  - app/PublicGameData/PublicGameDataSource.php
optional_reads: []
---

# OTERYN-20261010 public pages source-off state

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B5 of the removal programme (the removal plan under `docs/agents/programs/`).

Public game-data pages (highscores, profiles, guilds, deaths, servers, online) and the home page world summary render an explicit unavailable state when the legacy game connection is not configured, and issue no legacy connection or runtime Redis query when `PUBLIC_GAME_DATA_SOURCE=native`. The unavailable pages answer 200 for this deliberate off-state; a configured connection that fails keeps the existing 503. This record avoids the legacy token so the ratchet does not count it.

## Acceptance criteria

- [x] Every public game-data page answers 200 with the unavailable view when the connection is absent.
- [x] With `PUBLIC_GAME_DATA_SOURCE=native` a configured connection records no queries and the runtime Redis connection is never opened.
- [x] The home page world summary is UNAVAILABLE in both cases without querying.
- [x] Existing 503 behaviour for failing queries is unchanged.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/PublicGameData/**
  - app/PublicPortal/HomePageQuery.php
  - app/Http/Controllers/PublicGameData/**
  - tests/Feature/PublicGameData/**
  - tests/Feature/HomeTest.php
  - docs/agents/tasks/active/OTERYN-20261010-public-off-1.md
modules:
  - public game data
dependencies:
  - A1 guard (#1490, merged)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T23:30:00Z
head: UNKNOWN
branch: head branch of PR 1491
pr: 1491
status: validating
context_routes:
  - public-game-data
owned_paths:
  - app/PublicGameData/**
  - app/PublicPortal/HomePageQuery.php
  - app/Http/Controllers/PublicGameData/**
  - tests/Feature/PublicGameData/**
  - tests/Feature/HomeTest.php
  - docs/agents/tasks/active/OTERYN-20261010-public-off-1.md
proven: []
derived: []
unknown:
  - Exact-head CI result.
  - Deployed environment does not yet set the source variable (compose file is outside this packet).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/PublicGameData/PublicGameDataSource.php
  - app/PublicGameData/PublicGameDataUnavailable.php
  - app/PublicGameData/ (runtime channel service)
  - app/PublicGameData/GuildIndexQuery.php
  - app/PublicPortal/HomePageQuery.php
  - app/Http/Controllers/PublicGameData/PublicGameDataController.php
  - app/Http/Controllers/PublicGameData/GuildIndexController.php
  - tests/Feature/PublicGameData/PublicGameDataSourceTest.php
  - docs/agents/tasks/active/OTERYN-20261010-public-off-1.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: all groups at or below baseline (docs 1693, runtime 497)
  - command: python tools/agents/checkpoint.py --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: not run in the sandbox; request-level feature tests (including an env-variable-only case) cover the pages; the deployed compose file does not pass the source variable and is outside this packet, so deployed off-state browser proof is deferred to the config-removal packet
blockers:
  - none
next_action: Read exact-head CI and hand the review packet to the lead.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The source switch reads `config('public_game_data.source')` and falls back to the `PUBLIC_GAME_DATA_SOURCE` process environment variable, because `config/` is outside this packet; a config key and `.env.example` entry belong to the config-removal packet. Other consumers of the shared repository (account overview, login context, marketplace, profile preferences) are deliberately not gated here.
