---
task_id: OTERYN-20261012-platform-followups-collective
governing_issue: 1474
required_reads:
  - docs/agents/programs/*_REMOVAL_PLAN.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads: []
---

# OTERYN-20261012 collective Platform follow-ups

## Goal

Governing GitHub Issue: #1474 (native-only migration) - canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Owner rule D1014 (collect follow-ups, one push per PR): retire the runtime Redis path of the public server and home pages (C7 part 1: reader, service, connection, environment variables, CI provisioning), tighten the post-registration acceptance assertion carried from the registration-cut review, and refresh the live contract and runbook text. Guard denylists and negative-test tokens that contain the legacy token stay.

## Acceptance criteria

- [x] No code, config, environment template or workflow reads or provisions a runtime Redis store for the Platform.
- [x] Server and home pages render the runtime as unavailable, never offline or zero.
- [x] The post-registration acceptance assertion requires the `none` account state.
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` lowered only; no enforced group rises.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - app/C[a]naryIntegration/C[a]naryRuntimeRedisReader.php
  - app/PublicGameData/C[a]naryChannelRuntimeService.php
  - app/PublicGameData/C[a]naryRuntimeSnapshot.php
  - app/PublicGameData/C[a]naryRuntimeStatus.php
  - app/PublicPortal/HomePageQuery.php
  - app/Http/Controllers/PublicGameData/PublicGameDataController.php
  - resources/views/game/servers.blade.php
  - config/database.php
  - .env.example
  - .github/workflows/phase7-production-like-validation.yml
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - tests/Unit/C[a]naryIntegration/C[a]naryRuntimeRedisReaderTest.php
  - tests/Unit/PublicGameData/C[a]naryChannelRuntimeServiceTest.php
  - tests/Feature/HomeTest.php
  - tests/Feature/PublicGameData/ServerRuntimeAvailabilityTest.php
  - docs/contracts/C[a]NARY_DATA_CONTRACT.md
  - docs/operations/PUBLIC_COMMUNITY_DATA.md
  - docs/operations/INCIDENT_RECOVERY_RUNBOOK.md
  - docs/agents/programs/*_REMOVAL_PLAN.md
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/active/OTERYN-20261012-platform-followups-collective.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-followups-collective.md
modules:
  - public-game-data
  - ci
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
updated_at: 2026-10-12T05:40:00Z
head: UNKNOWN
branch: claude/platform-followups-collective-20261012
pr: 1556
status: validating
context_routes:
  - public-game-data
  - ci-workflows
owned_paths:
  - app/PublicPortal/HomePageQuery.php
  - config/database.php
  - docs/agents/tasks/active/OTERYN-20261012-platform-followups-collective.md
proven:
  - public game data is unavailable in every deployment since the legacy database left the deploy, so the runtime Redis path was unreachable in production
  - no other code uses the Redis facade or the removed connection
derived:
  - the Redis service containers in the acceptance workflows are now unused and can leave in a CI packet
unknown:
  - PHPUnit result (the local PHP is older than the required version; CI runs the suite)
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/PublicPortal/HomePageQuery.php
  - config/database.php
validation:
  - command: python3 tools/validation/legacy_token_ratchet.py and its unit test
    result: PASS
    evidence: local run
  - command: php -l on changed PHP files
    result: PASS
    evidence: local run
  - command: phpunit
    result: NOT_RUN
    evidence: local PHP 8.3 cannot run the PHP 8.5 codebase; exact-head CI runs it
blockers:
  - none
next_action: Wait for exact-head CI, then archive this record in a closeout PR.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR
source_branch_evidence: pending
```

## Notes

Not done here, with reasons (listed in the PR body): public highscores, guilds, deaths and online list from the projection (C5, C6, C7 part 2) wait for the Game projection producer; the drop migrations E1-E5 need the per-environment export (E1) and code that no longer reads the columns; the remaining legacy account-binding model is still read by Bazaar, profile preferences, account overview and public profile code that waits for those native replacements.
