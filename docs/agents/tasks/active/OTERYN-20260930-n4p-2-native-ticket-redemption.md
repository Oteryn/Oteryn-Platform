---
task_id: OTERYN-20260930-n4p-2-native-ticket-redemption
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeAdmission
  - app/GameAuth/Tickets
optional_reads: []
---

# OTERYN-20260930-n4p-2-native-ticket-redemption

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419), delivery item 2. Coordination: Oteryn/Oteryn-Game#162, owner authority Q14 (comment 5899892092). Contract: `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` (merged via #1420 as ed6c0d38). Grant issuer: #1421 (`app/GameAuth/NativeAdmission/*`, reused unchanged).

N4P-2: native Game Login Ticket redemption returns the canonical AccountId (UUIDv7), inside the native admission issuer transaction with `attempt_ref` idempotency (contract §4, §6) and the §11.2 error mapping. Behind the existing default-off `game-auth.native_admission.enabled` switch; no route, no HTTP surface.

Out of scope: route selection over `NativeTopologyRegistry` and runtime status (N4P-3), runtime-status and Character-projection ingestion, the issuer HTTP route and its rate limits (§3.3, §10), enabling anything, production, secrets and deployment.

## Acceptance criteria

- [x] Native ticket kind in storage: audience `oteryn-native-game-gateway`, bound to `identities.account_id` and `native_security_generation`, no Canary binding; native and Canary tickets never cross-redeem (the Canary redeem now also refuses a ticket without `canary_account_id`).
- [x] Redemption (§4.2) under row locks inside the issuer transaction; rejection rolls back and leaves the ticket unused.
- [x] `attempt_ref` record (§6.1) with ticket hash, AccountId, character, requested channel, JCS offer digest, signing input, kid, iat, exp; no signature or token stored.
- [x] Retry re-signs byte-identically with decreasing `valid_for_seconds`; changed member is `NATIVE_LOGIN_ATTEMPT_CONFLICT`; after exp (or without a stored signing input) `NATIVE_LOGIN_GRANT_EXPIRED`.
- [x] Stored payload bound to its attempt row before `resign` (#1421 comment 5901930996); a mismatch or unloadable kid is `ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED`.
- [x] `NativeAdmissionGrantContext` built only from redeemed account facts and a `NativeAdmissionScopeResolver` result; the default resolver fails closed (`NATIVE_LOGIN_ROUTE_UNAVAILABLE`) until N4P-3.
- [x] §11.2 error table as `NativeLoginError`, SECURITY_TERMINAL rows collapse to `NATIVE_LOGIN_AUTHENTICATION_REQUIRED`.
- [ ] Issuing native tickets through an OAuth client (§4.1; U1, owner decision) — not in this slice.
- [ ] Reducing retired attempts to audit rows (§6.3; U13 retention) and the issuer route, its rate limits and route selection — N4P-3.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeLogin/**
  - app/GameAuth/Tickets/GameLoginTicket.php
  - app/GameAuth/Tickets/RedeemGameLoginTicket.php
  - app/Providers/AppServiceProvider.php
  - database/migrations/2026_09_30_000100_add_native_game_login_ticket_redemption.php
  - tests/Feature/GameAuth/NativeLogin/**
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-1-native-login-issuer.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-1-native-login-issuer.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-native-gateway-contract.md
  - .gitignore
  - tools/agents/__pycache__/checkpoint.cpython-311.pyc
  - tools/validation/__pycache__/adr_registry.cpython-311.pyc
  - tools/validation/__pycache__/architecture_decision_backlog.cpython-311.pyc
modules:
  - GameAuth/NativeLogin
dependencies:
  - "#1420 contract (merged ed6c0d38)"
  - "#1421 grant issuer (merged 0e9abbf)"
blockers:
  - none
cross_repository_tasks:
  - none
```

Overlap: `OTERYN-20260929-n4p-1-native-login-issuer` owns `app/GameAuth/NativeAdmission/**`; this task only consumes it. No open PR touches the owned paths (checked 2026-09-30).

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T14:00:00Z
head: 3ad7e4f
branch: claude/n4p-2-native-ticket-redemption
pr: 1423
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeLogin/**
  - app/GameAuth/Tickets/GameLoginTicket.php
  - app/GameAuth/Tickets/RedeemGameLoginTicket.php
  - app/Providers/AppServiceProvider.php
  - database/migrations/2026_09_30_000100_add_native_game_login_ticket_redemption.php
  - tests/Feature/GameAuth/NativeLogin/**
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-1-native-login-issuer.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-1-native-login-issuer.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-native-gateway-contract.md
  - .gitignore
  - tools/agents/__pycache__/checkpoint.cpython-311.pyc
  - tools/validation/__pycache__/adr_registry.cpython-311.pyc
  - tools/validation/__pycache__/architecture_decision_backlog.cpython-311.pyc
proven:
  - The Canary RedeemGameLoginTicket compares audience first, so a native ticket (audience oteryn-native-game-gateway, null canary_account_id) is refused there
  - RevokeIdentityGameAuthorizations advances both game_auth_generation and native_security_generation, so a revocation between issuance and redemption fails native redemption
  - Making canary_account_id nullable surfaced (PHPStan) that RedeemGameLoginTicket passed it on unchecked; the Canary redeem now refuses a null value explicitly
  - Mutation check - disabling the payload-to-row binding makes the swapped-signing-input test fail
  - Review repair (PR 1423 review comment) - the migration down() now refuses before any DDL while native attempts or native tickets exist, following 2026_09_26_150000; the empty case still rolls back cleanly
  - Review repair - NativeAdmissionRequest keeps the ticket in a closure behind ticket(); json_encode, var_export and debug output omit it and serialization is refused (residual - Reflection or debug output of the closure itself)
  - Review repair - committed .pyc files removed and __pycache__/ ignored; the integrated #1420 and #1421 packets move to the archive with an archive-pending terminal transition
derived:
  - A concurrent duplicate of the same attempt_ref either blocks on the ticket row lock and finds the ticket consumed by its own attempt_ref, or hits the attempt_ref unique index; both re-read the committed attempt once
unknown:
  - U1 native OAuth client selecting the native ticket kind
  - U13 attempt audit retention
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Issuing native tickets in this slice rejected; the task is redemption only and the issuing OAuth client is U1
  - Extending the Canary RedeemGameLoginTicket rejected; the native kind has a different binding and must never share a branch with Canary
changed_paths:
  - app/GameAuth/NativeLogin/*
  - app/GameAuth/Tickets/GameLoginTicket.php
  - app/GameAuth/Tickets/RedeemGameLoginTicket.php
  - app/Providers/AppServiceProvider.php
  - database/migrations/2026_09_30_000100_add_native_game_login_ticket_redemption.php
  - tests/Feature/GameAuth/NativeLogin/NativeAdmissionAttemptsTest.php
  - tests/Feature/GameAuth/NativeLogin/NativeTicketRedemptionMigrationRollbackTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-1-native-login-issuer.md
  - docs/agents/tasks/archive/OTERYN-20260929-n4p-native-gateway-contract.md
  - .gitignore
  - tools/agents/__pycache__/checkpoint.cpython-311.pyc (deleted)
  - tools/validation/__pycache__/*.pyc (deleted)
validation:
  - command: php artisan test (full suite), php artisan test tests/Feature/GameAuth, composer analyse (PHPStan level 10), vendor/bin/pint --test, composer audit
    result: PASS
    evidence: local PHP 8.4.19 with --ignore-platform-req=php (composer requires 8.5); full suite 708 tests passed, 631 carry the pre-existing missing-.env warning; PHPStan no errors; Pint passed; audit no advisories; exact-head CI is authoritative
  - command: php artisan migrate, migrate:rollback --step=1, migrate (sqlite file)
    result: PASS
    evidence: the new migration applies, rolls back and re-applies
  - command: review repair - php -l on changed PHP files, git diff --check, a standalone NativeAdmissionRequest redaction harness, checkpoint.py, source_branch_closeout.py, documentation_ia.py and the other offline governance scripts
    result: PASS
    evidence: local; harness 9/9 (accessor, json_encode, var_export, print_r, var_dump, serialize and unserialize refused, digest); composer install refused by the session proxy (github.com authentication), so PHPUnit, PHPStan and Pint for the repair run in exact-head CI
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: no route or actor path in this slice; the issuer is reachable only after N4P-3 adds the route
blockers:
  - none
next_action: control plane freezes the repaired head and routes the independent security re-review; once integrated, archive this packet to docs/agents/tasks/archive with the next #1419 delivery task (N4P-3)
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```
