---
task_id: OTV2-20260930-n4p3-native-admission-issuer
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeLogin
  - app/GameAuth/Worlds
  - app/GameAuth/NativeRuntimeStatus
optional_reads: []
---

# OTV2-20260930-n4p3-native-admission-issuer

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419). Coordination: Oteryn/Oteryn-Game#162 (lane N4-P). Owner authority: D160 (Platform write scope, #162 comment 5899892092), D164, D171, D172 (owner answer 33c/34b, #162 comment 5905264083).

N4P-3: the private issuer `POST /internal/v1/game-auth/native-admissions` (contract §3.3). It fills `NativeAdmissionScopeResolver` with route selection over `NativeTopologyRegistry` plus fresh, ownership-bound runtime status, fail-closed when stale. It adds the D172 native route record on `game_channels` (testing/preproduction only) and the D171 unverified-ownership mode (config-gated, refused outside testing/preproduction). It also bounds the issuer lock wait, raises the scope-assignment rate-limit default from 60 to 120, and adds a contract amendment recording both modes and their release gates.

Out of scope: the Go Gateway forwarding of `protocol_version: 2` (split to N4P-3b, a separate later PR), the per-AccountId committed-attempt limit (§10, recorded in the amendment), production config/secrets/enablement, LCFA-1, CHAR-NAME-1, Game repository writes and workflow changes.

## Acceptance criteria

- [x] Issuer route behind the Gateway service credential, exact §3.1 wire, §3.2 success body with the Registry endpoint bound to the grant's route_revision, §11.1 error body, no-store.
- [x] Route selection: login-enabled Registry route record, fresh ready ownership-bound report, report route_revision equals the record's, lowest ChannelId, requested channel must be a candidate. Stale, invalid or mismatched status routes nowhere.
- [x] Route record migration (additive, nullable, rollback refused while a record exists) and `publishRouteForPreproduction` (testing/preproduction only, revision stable for an unchanged endpoint, advanced on change).
- [x] D171 mode config-gated, refused outside testing/preproduction, off by default (fail closed).
- [x] InnoDB lock wait timeout bounded for the issuer transaction. Issuer limit is 120/min per credential. Scope-assignment default is 120.
- [x] Contract amendment §17 (33a and 34a release gates). The #1426 epoch finding is recorded in U16.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeLogin/**
  - app/GameAuth/Worlds/NativeRouteRecord.php
  - app/GameAuth/Worlds/NativeRouteRecords.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Http/Controllers/GameAuth/NativeAdmissionController.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Providers/AppServiceProvider.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_210000_add_native_route_record_to_game_channels.php
  - tests/Feature/GameAuth/NativeLogin/**
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20260930-n4p3-native-admission-issuer.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-scope-assignment.md
modules:
  - GameAuth/NativeLogin
  - GameAuth/Worlds
dependencies:
  - "#1426 N4P-ING-SA (merged b0e7b47)"
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T23:59:00Z
head: c575a1a
branch: claude/n4p3-native-admission-issuer
pr: 1427
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeLogin/**
  - app/GameAuth/Worlds/NativeRouteRecord.php
  - app/GameAuth/Worlds/NativeRouteRecords.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Http/Controllers/GameAuth/NativeAdmissionController.php
  - database/migrations/2026_09_30_210000_add_native_route_record_to_game_channels.php
  - tests/Feature/GameAuth/NativeLogin/**
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
proven:
  - No Platform Character read model exists at b0e7b47; without D171 the only fail-closed answer is NATIVE_LOGIN_ROUTE_UNAVAILABLE
  - The resolver takes only the shared epoch lock after ticket/Identity row locks; ingestion takes epoch then assignment/report, so no lock-order cycle is added
derived:
  - "D171 needs a Character world: the mode uses one configured unverified_character_world_id; Game FND-04A §5 rejects a Character in another world"
  - A route record changed after issuance answers ROUTE_UNAVAILABLE on retry (grant would be ROUTE_STALE at admission)
unknown:
  - U3 gameplay CA, U7 production login policy, U8 production topology, U13 retention, U16 epoch raise authority
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Laravel throttle middleware rejected for the issuer; its 429 body is not the §11.1 shape
changed_paths:
  - app/GameAuth/NativeLogin/ (resolver, wire, lock wait; UnavailableNativeAdmissionScopeResolver removed)
  - app/GameAuth/Worlds/ (route record, reader, Registry publish)
  - app/Http/Controllers/GameAuth/NativeAdmissionController.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Providers/AppServiceProvider.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_210000_add_native_route_record_to_game_channels.php
  - tests/Feature/GameAuth/NativeLogin/
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php (repeated-lock-conflict proof now bounds the wait through the issuer setting instead of a session statement the issuer overrides)
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-scope-assignment.md (moved from active)
validation:
  - command: vendor/bin/phpunit tests/Feature/GameAuth and the full suite (sqlite)
    result: PASS
    evidence: local PHP 8.4.19, dependencies from the lock installed from source without phpstan/larastan (scratch manifest; composer.json requires ^8.5); GameAuth 179 tests, 16 skipped; full suite 745 tests, 0 failures, 30 skipped
  - command: php artisan test --filter=GameTicketConcurrencyTest and NativeTopologyConcurrencyTest (MariaDB 10.11 InnoDB, GAME_AUTH_CONCURRENCY_TEST=1, pcntl)
    result: PASS
    evidence: local; 11 + 3 tests pass. With the old session statement (innodb_lock_wait_timeout = 1) instead of the issuer setting, the repeated-lock-conflict test fails because the issuer's own 3 s bound overrides it, which proves the bound is applied. CI runs MariaDB 11.8
  - command: vendor/bin/pint --test on changed PHP files, git diff --check, checkpoint.py --require-checkpoint
    result: PASS
    evidence: local
  - command: composer analyse (PHPStan level 10)
    result: BLOCKED
    evidence: phpstan/phpstan is dist-only and its api.github.com zipball is refused by the session proxy (403); exact-head CI is authoritative
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: the Gateway native branch (N4P-3b) does not forward yet and every switch is default-off; joint E2E is #1419 item 5
blockers:
  - none
next_action: exact-head CI green; control plane freezes the head and routes the required security review
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

Split: the Go Gateway forwarding of the native branch is N4P-3b (separate later PR).
