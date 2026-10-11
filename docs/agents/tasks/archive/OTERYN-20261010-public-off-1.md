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

Public game-data pages (highscores, profiles, guilds, deaths, servers, online) and the home page world summary render an explicit unavailable state when the legacy game connection is not configured, and issue no legacy connection or runtime Redis query when `PUBLIC_GAME_DATA_SOURCE=native`. The unavailable pages answer 200 for this deliberate off-state; a configured connection that fails keeps the existing 503. The ratchet does not count the checkpoint `branch:` line of a task record, because the task check requires the real branch name (decision D989).

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
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/tasks/archive/OTERYN-20261010-public-off-1.md
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
updated_at: 2026-10-10T23:45:00Z
head: 0836d46e4e164186e43fc4f7aae77485bd497b1f
branch: claude/decanary-public-off-1-20261010
pr: 1491
status: completed
context_routes:
  - public-game-data
owned_paths:
  - app/PublicGameData/**
  - app/PublicPortal/HomePageQuery.php
  - app/Http/Controllers/PublicGameData/**
  - tests/Feature/PublicGameData/**
  - tests/Feature/HomeTest.php
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/tasks/archive/OTERYN-20261010-public-off-1.md
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
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/tasks/archive/OTERYN-20261010-public-off-1.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: all groups at or below baseline (docs 1691 under baseline 1693, runtime 497)
  - command: python tools/agents/checkpoint.py docs/agents/tasks/archive/OTERYN-20261010-public-off-1.md --require-checkpoint
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
next_action: none; terminal PR merged and record archived. Deployed off-state browser proof stays deferred to the config-removal packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1491 merged (REST pulls/1491 state closed, merged true) as 46873967e9d193c2d479b30777fd67f0c2d8e0bc at 2026-10-11T03:00:38Z from exact head 0836d46e4e164186e43fc4f7aae77485bd497b1f; source branch (claude/decanary-public-off-1-20261010) absent (REST branches API returns 404)
```

## Notes

The source switch reads `config('public_game_data.source')` and falls back to the `PUBLIC_GAME_DATA_SOURCE` process environment variable, because `config/` is outside this packet; a config key and `.env.example` entry belong to the config-removal packet. Other consumers of the shared repository (account overview, login context, marketplace, profile preferences) are deliberately not gated here.
