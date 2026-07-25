---
task_id: OTERYN-20260724-public-game-statistics
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_GUILD_INDEX_CONTRACT.md
search_first:
  - PR #160
  - docs/agents/tasks/archive/OTERYN-20260724-public-game-statistics.md
optional_reads: []
---

# OTERYN-20260724-public-game-statistics

## Goal

Deliver the smallest complete read-only public-game-statistics capability supported by verified Canary evidence.

## Completion

PR #160 was squash-merged to `main` as `4e44b6414e67c5c54e1f0c91c7b434abe0905adb` after all required workflows passed on immutable PR head `2c044e2636a2751a006ce382ab92be5d57bcc23d`.

Delivered:

- `GET /guilds` through a dedicated PublicGameData query and controller;
- a public allowlist limited to guild name and active-member count;
- active-member aggregation from `guild_membership` joined to `players.deletion = 0`;
- deterministic name/ID ordering with 50-row pagination;
- at most one paginator count query plus one bounded page query;
- explicit HTTP 200 empty and HTTP 503 dependency-unavailable behavior;
- database-enforced read-only boundary evidence and focused privacy/failure tests;
- `docs/contracts/PUBLIC_GUILD_INDEX_CONTRACT.md` pinned to Canary commit `93413bd53e9a40f0ff3c4f55986036b10be44e0f`.

Latest deaths and kill statistics were intentionally not bundled because their semantics and public-exposure rules require separate evidence-backed discovery.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T00:10:00+02:00
head: 4e44b6414e67c5c54e1f0c91c7b434abe0905adb
branch: main
pr: 160
status: ready
context_routes:
  - public-game-data
  - canary-integration
  - database
  - security
  - testing
  - agent-governance
owned_paths:
  - app/Http/Controllers/PublicGameData/GuildIndexController.php
  - app/PublicGameData/GuildIndexQuery.php
  - routes/modules/public-game-statistics.php
  - resources/views/game/guilds/index.blade.php
  - tests/Feature/PublicGameData/GuildIndexTest.php
  - docs/contracts/PUBLIC_GUILD_INDEX_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260724-public-game-statistics.md
proven:
  - Canary commit 93413bd53e9a40f0ff3c4f55986036b10be44e0f defines guilds, guild_membership and players with the keys used by the read query.
  - The existing oteryn_readonly policy grants direct SELECT on guilds, guild_membership and players and rejects broader privileges.
  - The merged projection exposes only guild name and a derived count of memberships joined to players with deletion equal to zero.
  - Pagination is fixed at 50 and ordering is guild name ascending with guild ID as a deterministic tie-breaker.
  - Empty data returns HTTP 200 while Canary query failure returns HTTP 503.
  - All seven required workflow families succeeded on final PR head 2c044e2636a2751a006ce382ab92be5d57bcc23d.
  - PR 160 was squash-merged to main as 4e44b6414e67c5c54e1f0c91c7b434abe0905adb.
derived:
  - No Canary schema, write route or additional database grant was required for the delivered guild index.
unknown:
  - Product meaning of guild points, level, residence and creationdata remains unverified and those fields are not exposed.
conflicts: []
first_failure:
  marker: PHPSTAN_LEVEL_10
  evidence: early PR revisions stopped at static analysis; the final grouped aggregate and response-level assertions passed CI run 30128016764
rejected_hypotheses:
  - Bundle guild index, latest deaths and kill statistics: rejected because only the guild-index semantics were fully proven for this task.
  - Expose free-form MOTD or membership nicknames: rejected to avoid unnecessary moderation and privacy exposure.
changed_paths:
  - app/Http/Controllers/PublicGameData/GuildIndexController.php
  - app/PublicGameData/GuildIndexQuery.php
  - docs/contracts/PUBLIC_GUILD_INDEX_CONTRACT.md
  - resources/views/game/guilds/index.blade.php
  - routes/modules/public-game-statistics.php
  - tests/Feature/PublicGameData/GuildIndexTest.php
validation:
  - command: CI run 30128016764
    result: PASS
    evidence: Composer validation and audit, Pint, PHPStan level 10 and full tests succeeded on final PR head
  - command: Agent Governance run 30128016762
    result: PASS
    evidence: final PR head workflow succeeded
  - command: Phase 7 Production-Like Validation run 30128016763
    result: PASS
    evidence: final PR head workflow succeeded
  - command: Platform DB Outage Validation run 30128016819
    result: PASS
    evidence: final PR head workflow succeeded
  - command: Game Auth Ticket Concurrency run 30128016807
    result: PASS
    evidence: final PR head workflow succeeded
  - command: Acceptance E2E and Visual UX run 30128016765
    result: PASS
    evidence: final PR head workflow succeeded
  - command: Build Synology Staging Images run 30128016760
    result: PASS
    evidence: final PR head workflow succeeded
  - command: local checkout and tests
    result: BLOCKED
    evidence: execution sandbox could not resolve github.com; no local result was claimed
blockers:
  - none
next_action: Use docs/contracts/PUBLIC_GUILD_INDEX_CONTRACT.md as the evidence baseline before proposing another public game-statistics capability.
```

## Notes

The public capability excludes account IDs, owner IDs, balances, MOTD, invitations, wars, disciplinary data, deleted-character names and raw membership identifiers.
