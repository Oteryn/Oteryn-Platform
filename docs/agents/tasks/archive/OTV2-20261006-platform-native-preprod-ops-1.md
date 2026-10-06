---
task_id: OTV2-20261006-platform-native-preprod-ops-1
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/Worlds
  - app/Console/Commands
optional_reads: []
---

# OTV2-20261006-platform-native-preprod-ops-1

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419), item 5 (joint E2E, login contract §14 step 6). Owner authority: answer 1a on Oteryn/Oteryn-Game#1622 (control plane D824/D831, confirmed by D842 Option A). Frozen design: Oteryn/Oteryn-Game `docs/architecture/reviews/OTERYN_GAME_ARCH_PREPROD_ROUTE_PUBLISH_AUTH_2026-10-06.md` §2 (ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1), reassessed against Platform 3896bcd.

PLATFORM-NATIVE-PREPROD-OPS-1: the operator path that the N4P-3 record left out of scope. Two artisan commands, `game-auth:native-route:publish` and `game-auth:native-trust:publish-key`, run only in `testing`/`preproduction` behind one shared disposable-store guard. The predicate of `NativeTopologyRegistry::isolatedConnection()` moves unchanged into `DisposableNativeStore`, which both the registry and the commands call. Both commands also require the retained per-run SQLite file, and the trust command also requires the high-water directory to be canonical and directly beneath that run directory.

Out of scope: `game-auth:native-topology:issue` and `issueForPreproduction` (unchanged), the trust registry rules, migrations, routes, the staging deployment, production enablement, and Game repository writes.

## Acceptance criteria

- [x] `DisposableNativeStore::connection()` is the `isolatedConnection()` predicate moved verbatim, run-directory check included; `isolatedConnection()` only delegates to it, and the existing registry tests pass unchanged.
- [x] Route command: strict option validation, then the guard, the retained-file check, `publishRouteForPreproduction`, and a guarded readback verified through `NativeRouteRecords::fromRow`; it prints `world_id`, `channel_id`, `route_version`, `route_revision`, `native_login_enabled`, and on failure prints only a generic error.
- [x] Trust command: the guard, the retained-file check and the high-water directory check before any write; fresh issuer/profile/purpose only; 32 raw bytes from a regular non-symlink file; prints `key_id` and `profile_version`, or a generic error.
- [x] §2 tests: refusal in local/staging/production; refusal of a MariaDB store, a SQLite file outside a run directory, a symlinked file and a symlinked run directory; refusal of `:memory:` and `oteryn_concurrency`; refusal of high-water directories that are outside the run, in another run, symlinked or under a symlinked parent; the happy path; a version advance; login disabled with the endpoint kept; malformed input refused before any write.
- [x] One line in login contract §17 naming the operator path.

## Ownership

```yaml
owned_paths:
  - app/Console/Commands/PublishNativeRoute.php
  - app/Console/Commands/PublishNativeTrustedKey.php
  - app/GameAuth/Worlds/DisposableNativeStore.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - tests/Feature/GameAuth/NativeTopology/NativeOperatorCommandsTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20261006-platform-native-preprod-ops-1.md
modules:
  - GameAuth/Worlds
dependencies:
  - "Oteryn/Oteryn-Game#1871 (merged)"
blockers:
  - none
cross_repository_tasks:
  - "Oteryn/Oteryn-Game#1622 (owner answer 1a; Game repository read-only here)"
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-06T20:10:00Z
head: 6c439dbffc516969f3620defe07c61ff937e1f37
branch: claude/platform-native-preprod-ops-1
pr: 1469
status: completed
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/Console/Commands/PublishNativeRoute.php
  - app/Console/Commands/PublishNativeTrustedKey.php
  - app/GameAuth/Worlds/DisposableNativeStore.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - tests/Feature/GameAuth/NativeTopology/NativeOperatorCommandsTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20261006-platform-native-preprod-ops-1.md
proven:
  - main 3896bcd unchanged in the owned files since the spec reassessment
  - moved predicate is textually identical to the removed isolatedConnection() body
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Console/Commands/PublishNativeRoute.php
  - app/Console/Commands/PublishNativeTrustedKey.php
  - app/GameAuth/Worlds/DisposableNativeStore.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - tests/Feature/GameAuth/NativeTopology/NativeOperatorCommandsTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20261006-platform-native-preprod-ops-1.md
validation:
  - command: composer format:check
    result: PASS
    evidence: pint passed
  - command: composer analyse
    result: PASS
    evidence: phpstan no errors
  - command: phpunit tests/Feature/GameAuth (PHP 8.5)
    result: PASS
    evidence: 197 tests, 2451 assertions, 18 skipped (MariaDB concurrency profile not provisioned locally); NativeTopologyRegistryTest unchanged and green
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: operator CLI for disposable preproduction runs; the joint E2E (contract §14 step 6) is the separate Game node-boot job that consumes it
blockers:
  - none
next_action: none; PR #1469 merged as 81898fc1 after all exact-head required checks passed, and this task is archived.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1469 is terminal and the same-repository implementation branch has no durable post-merge purpose.
source_branch_evidence: PR #1469 merged from exact head 6c439dbffc516969f3620defe07c61ff937e1f37 and live branch lookup confirms claude/platform-native-preprod-ops-1 is absent.
```

## Notes

Jira synchronization: pending/unknown (no mapped Story).
