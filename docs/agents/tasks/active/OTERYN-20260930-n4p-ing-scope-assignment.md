---
task_id: OTERYN-20260930-n4p-ing-scope-assignment
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeRuntimeStatus
  - tests/Feature/GameAuth/NativeRuntimeStatus
  - tests/Feature/GameAuth/Concurrency
optional_reads:
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
---

# OTERYN-20260930-n4p-ing-scope-assignment

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419). Coordination: Oteryn/Oteryn-Game#162, owner authority Q14 and Q16b. Lock entry `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN`, rollout step 4 (Platform ingestion endpoints and read models, `backward-compatible`). Control-plane ruling on #1425 (comment 5904675686): assignment ingestion is this separate task.

N4P-ING-SA: Platform ingestion endpoint `POST /internal/v1/game-auth/native-scope-assignments` for Game `ReportScopeAssignmentV1` (Game producer `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §3, §5, §6 read-only at Game `main@07cd8bcc3539a1504c1f6f6df3e188f53f39adf0`; login contract §7.2), which writes `native_scope_assignments` so runtime reports can be accepted. Also the #1425 review follow-ups 1-3: early HTTP bounds on both routes, ownership-authority identity in the identity-reuse refusal (both directions), the highest-epoch read under a lock, and a MariaDB lock-order proof.

Out of scope: the Game ops producer and U16 (Game-side epoch storage and operator procedure), the `ListCharactersForAccount` consumer, Gateway route selection (N4P-3), the Registry route record (D3/U3), enabling anything, workflow edits, production, secrets and real certificates.

## Acceptance criteria

- [x] Exact Game wire: 8 members, no unknown/duplicate (also escaped)/missing/null/nested member; UUIDv7 scope, canonical non-zero uint64 epoch and generation kept as decimal strings, `node_identity` in the runtime-status identity grammar, `assigned_at` Unix seconds; body at most 2048 bytes; else `400`, empty body.
- [x] Success body exactly `{"contract_version":1,"result":"accepted|superseded"}`; failures empty: `400`, `401`, `409`, `429`, `503` (Game §4 status set).
- [x] Own purpose identity: only configured ownership-authority identities (trusted-terminator `SSL_CLIENT_*`, TLS 1.3) pass (`401`); per-identity scope list (`401`); `node_identity` must be a runtime-status identity configured for the scope (`409`).
- [x] Identity reuse refused symmetrically (`503`): a runtime-status identity may not be an ownership-authority identity and vice versa, whether or not the other purpose is enabled; both refuse native-evidence and character-bootstrap identities.
- [x] Per scope `(assignment_epoch, ownership_generation)` only moves forward: lower `superseded`, equal with identical content `accepted` (idempotent replay), equal with other content `409` without change; an epoch below the highest epoch seen is `superseded` and never stored.
- [x] Epoch lock row: assignment ingestion takes it exclusively, runtime ingestion shared, before the scope assignment row and the report row; the highest epoch is read under it. MariaDB independent-process proof registered through the existing `--filter=GameTicketConcurrencyTest` step without a workflow edit.
- [x] `EnforceNativeEvidenceHttpBounds` (with a per-route byte limit) on both routes: early `413` on Content-Length or body above 2048, `400` on Transfer-Encoding, Content-Encoding or non-JSON type, `431` on oversized headers.
- [x] Default-off switch `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED`; `503` also while the runtime-status configuration is invalid.
- [x] Forward-only additive migration; `down()` refuses before DDL while assignments exist.
- [x] Integrated N4P-ING-RS packet archived with its closeout.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeRuntimeStatus/**
  - app/Http/Controllers/GameAuth/NativeScopeAssignmentController.php
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - app/Http/Middleware/GameAuth/EnforceNativeEvidenceHttpBounds.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_180000_add_native_assignment_epoch_lock.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/**
  - tests/Feature/GameAuth/Concurrency/NativeRuntimeStatusGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-scope-assignment.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-scope-assignment.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-runtime-status.md
modules:
  - GameAuth/NativeRuntimeStatus
dependencies:
  - "#1425 N4P-ING-RS (merged 328472d0)"
  - "#1420 N4P contract (LOCKED candidate)"
  - "Game runtime-status producer contract on Game main@07cd8bcc"
blockers:
  - none
cross_repository_tasks:
  - "Oteryn/Oteryn-Game read-only (Q14): producer contract §3, §5, §6"
```

Overlap: no open Platform PR touches the owned paths (checked 2026-09-30). `EnforceNativeEvidenceHttpBounds` keeps its native-evidence default; the integrated N4P-ING-RS packet is archived by this task.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T23:59:00Z
head: UNKNOWN
branch: claude/n4p-ing-scope-assignment
pr: none
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeRuntimeStatus/**
  - app/Http/Controllers/GameAuth/NativeScopeAssignmentController.php
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - app/Http/Middleware/GameAuth/EnforceNativeEvidenceHttpBounds.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_180000_add_native_assignment_epoch_lock.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/**
  - tests/Feature/GameAuth/Concurrency/NativeRuntimeStatusGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-scope-assignment.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-scope-assignment.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-runtime-status.md
proven:
  - Game main@07cd8bcc fixes the request wire (§5), the path and per-purpose certificate (§3), the 2048-byte bound (§3), the global epoch ordering and restore reset (§6); Game has no ReportScopeAssignmentV1 producer or fixture yet, so the test wire is the §5 example with a configured node identity
  - U16 (Game-side epoch storage and raise procedure) does not change the wire; Platform consumes the epoch as an opaque canonical uint64
  - With the runtime report's shared epoch lock removed, the MariaDB race accepts a report against an epoch being raised (test fails); with it, the report waits and is 409
derived:
  - "Assignment response reuses the Game §4 envelope and status set; results are accepted (new or identical replay, so an ambiguous delivery replayed with the same content gets a definite result) and superseded (lower key or epoch below the highest)"
  - An equal (epoch, generation) with a different node_identity or assigned_at is 409 and changes nothing (the first committed assignment stays)
  - A node_identity that is not a runtime-status identity configured for the scope is 409 (Game §5 says the ops tool rejects it before reporting)
  - The assignment endpoint needs a valid runtime-status configuration to check node_identity; otherwise 503
  - 413 and 431 from the reused bounds middleware are outside the Game §4 status set; the control plane ruled the reuse, and the Game node treats every failure as not delivered
unknown:
  - U5 measured H/F; U15 per-purpose PKI; U16 Game-side epoch storage and raise procedure
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Deriving the highest epoch under a lock on every assignment row rejected; a shared lock on all rows followed by the scope row's exclusive lock deadlocks two reports for one scope
  - A lazily inserted lock row rejected; concurrent first inserts deadlock on InnoDB, so the migration inserts the one row
changed_paths:
  - app/GameAuth/NativeRuntimeStatus/ (3 new, 4 changed)
  - app/Http/Controllers/GameAuth/NativeScopeAssignmentController.php
  - app/Http/Middleware/GameAuth/ (bounds limit parameter, guard purpose parameter, no-store path)
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_180000_add_native_assignment_epoch_lock.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/ (new assignment test, runtime test 413 case)
  - tests/Feature/GameAuth/Concurrency/NativeRuntimeStatusGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-scope-assignment.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-runtime-status.md (moved from active)
validation:
  - command: vendor/bin/phpunit (full suite, sqlite) and tests/Feature/GameAuth/NativeRuntimeStatus
    result: PASS
    evidence: local PHP 8.4.19 in a scratch copy (composer.json requires ^8.5; dependencies installed from source with platform requirements ignored, phpstan from its locked 2.2.13 tag); 737 tests, 0 failures, 30 skipped; NativeRuntimeStatus 19 tests, 709 assertions
  - command: php artisan test --filter=GameTicketConcurrencyTest (MariaDB 10.11 InnoDB, GAME_AUTH_CONCURRENCY_TEST=1, pcntl)
    result: PASS
    evidence: local; 11 tests (9 existing + 2 new); new tests fail with the runtime shared epoch lock removed; CI runs MariaDB 11.8
  - command: phpstan analyse (level 10, full configuration), pint on changed files, php -l, git diff --check, checkpoint.py --require-checkpoint, source_branch_closeout.py
    result: PASS
    evidence: local offline run on the candidate; exact-head CI is authoritative
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: both endpoints are default-off and nothing routes until N4P-3 route selection; the Game ops producer does not exist yet; joint E2E is #1419 item 5
blockers:
  - none
next_action: exact-head CI; then the control plane freezes the head and routes the security review; after integration archive this packet with the next #1419 delivery task
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```
