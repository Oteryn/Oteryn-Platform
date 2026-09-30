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

N4P-3: the private issuer `POST /internal/v1/game-auth/native-admissions` (contract §3.3). It fills `NativeAdmissionScopeResolver` with route selection over `NativeTopologyRegistry` plus fresh, ownership-bound runtime status, fail-closed when stale. It adds the D172 native route record on `game_channels` (testing/preproduction only) and the D171 unverified-ownership mode (config-gated, refused outside testing/preproduction). It also bounds the issuer lock wait, enforces the §10 per-AccountId limit (10 committed attempts/min) inside the issuer transaction, raises the scope-assignment rate-limit default from 60 to 120, and adds a contract amendment recording both modes and their release gates.

Out of scope: the Go Gateway forwarding of `protocol_version: 2` (split to N4P-3b, a separate later PR), an operator path for `publishRouteForPreproduction` outside the isolated connection (joint-E2E follow-up under separate authority), production config/secrets/enablement, LCFA-1, CHAR-NAME-1, Game repository writes and workflow changes.

## Acceptance criteria

- [x] Issuer route behind the Gateway service credential, exact §3.1 wire, §3.2 success body with the Registry endpoint bound to the grant's route_revision, §11.1 error body, no-store.
- [x] Route selection: login-enabled Registry route record, fresh ready ownership-bound report, report route_revision equals the record's, lowest ChannelId, requested channel must be a candidate. Stale, invalid or mismatched status routes nowhere.
- [x] Route record migration (additive, nullable, rollback refused while a record exists) and `publishRouteForPreproduction` (testing/preproduction only, revision stable for an unchanged endpoint, advanced on change).
- [x] D171 mode config-gated, refused outside testing/preproduction, off by default (fail closed).
- [x] InnoDB lock wait timeout bounded for the issuer transaction. Issuer limit is 120/min per credential (atomic hit-then-compare). Scope-assignment default is 120.
- [x] Per-AccountId limit of 10 committed attempts/min inside the issuer transaction; a refusal rolls back and the ticket stays unused.
- [x] Security review repair (KEEP, no material blocker): route records unreadable outside testing/preproduction; shared epoch lock taken before the ticket redeem and any non-locking read; IP-looking tls_server_name rejected; MariaDB race of issuance vs an epoch raise; lower-epoch and route-change-after-issuance tests.
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
  - tests/Feature/GameAuth/Concurrency/NativeAdmissionIssuerGameTicketConcurrencyTest.php
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
head: 0a20fd0 (security review repair generation on top); integrated as f880cd744f8846f3efb8e7c1edadf1ea048656ce
branch: claude/n4p3-native-admission-issuer
pr: 1427
status: completed
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
  - tests/Feature/GameAuth/Concurrency/NativeAdmissionIssuerGameTicketConcurrencyTest.php
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
proven:
  - No Platform Character read model exists at b0e7b47; without D171 the only fail-closed answer is NATIVE_LOGIN_ROUTE_UNAVAILABLE
  - "Issuer lock order: attempt row, shared epoch lock, ticket, Identity; ingestion takes epoch, assignment, report, so no cycle exists"
  - Taking the epoch lock after a non-locking read (the per-AccountId count) lets an issuance that waited for an epoch raise grant against the old epoch; the MariaDB race test fails in that order and passes with the lock first
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
  - tests/Feature/GameAuth/Concurrency/NativeAdmissionIssuerGameTicketConcurrencyTest.php (new; issuance vs a scope-assignment epoch raise in both orders)
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php (repeated-lock-conflict proof now bounds the wait through the issuer setting instead of a session statement the issuer overrides)
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-scope-assignment.md (moved from active)
validation:
  - command: vendor/bin/phpunit tests/Feature/GameAuth and the full suite (sqlite)
    result: PASS
    evidence: local PHP 8.4.19, dependencies from the lock installed from source without phpstan/larastan (scratch manifest; composer.json requires ^8.5); review repair generation GameAuth 185 tests, 18 skipped; full suite 751 tests, 0 failures, 32 skipped; the new tests for items 1, 4, 5 and 6 fail against 0a20fd0 app code
  - command: php artisan test --filter=GameTicketConcurrencyTest and NativeTopologyConcurrencyTest (MariaDB 10.11 InnoDB, GAME_AUTH_CONCURRENCY_TEST=1, pcntl)
    result: PASS
    evidence: local; review repair generation 13 + 3 tests pass (2 new issuer-vs-epoch-raise races). Earlier generation 11 + 3. With the old session statement (innodb_lock_wait_timeout = 1) instead of the issuer setting, the repeated-lock-conflict test fails because the issuer's own 3 s bound overrides it, which proves the bound is applied. CI runs MariaDB 11.8
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
next_action: none; PR 1427 integrated as f880cd7 and this record archived by N4P-3b (PR 1429)
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR integration completed
source_branch_evidence: source ref claude/n4p3-native-admission-issuer absent (ls-remote 2026-09-30) after PR 1427 integrated as f880cd744f8846f3efb8e7c1edadf1ea048656ce
```

## Notes

Split: the Go Gateway forwarding of the native branch is N4P-3b (separate later PR).

Security review repair (disposition KEEP, no material blocker), one push on 0a20fd0: (1) `NativeRouteRecords` refuses every read outside testing/preproduction; (2) `NativeAdmissionAttempts::issue` takes the shared epoch lock before `tickets->redeem`, rule "no non-locking read before the epoch lock"; (3) MariaDB race, lower-epoch-after-reset and route-change-after-issuance tests; (4) §10 per-AccountId limit enforced, deferral wording removed; (5) IP-looking `tls_server_name` rejected; (6) atomic hit-then-compare credential limit; (7) out of scope: `publishRouteForPreproduction` works only on the isolated connection, so joint E2E needs an operator path under separate authority.
