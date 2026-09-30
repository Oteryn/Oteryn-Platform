---
task_id: OTERYN-20260930-n4p-ing-runtime-status
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
search_first:
  - app/GameAuth/NativeRuntimeStatus
  - tests/Feature/GameAuth/NativeRuntimeStatus
optional_reads:
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
---

# OTERYN-20260930-n4p-ing-runtime-status

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419). Coordination: Oteryn/Oteryn-Game#162, owner authority Q14 and Q16b. Lock entry `OTV2-20260929-N4P-NATIVE-GATEWAY-LOGIN`, rollout step 4 (Platform ingestion endpoints and read models, `backward-compatible`).

N4P-ING-RS: Platform ingestion endpoint `POST /internal/v1/game-auth/native-runtime-status` and read model for Game `ReportRuntimeStatusV1` (login contract §7.2; Game producer `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` read-only at Game `main@07cd8bcc3539a1504c1f6f6df3e188f53f39adf0`, encoder `apps/game-server/src/native_admission_source/runtime_status.rs`, fixtures `apps/game-server/tests/native_runtime_status.rs`). Heartbeat H = 5 s, freshness F = 15 s (U5 proposal).

Out of scope: `ReportScopeAssignmentV1` ingestion (`native-scope-assignments`), the `ListCharactersForAccount` consumer, Gateway route selection (N4P-3), the Registry route record and `route_revision` computation (D3/U3), enabling anything, workflow edits, production, secrets and real certificates.

## Acceptance criteria

- [x] Exact Game wire: 22 members, no unknown/duplicate (also escaped)/missing/null/nested member, Game grammar per value; uint64 values kept as canonical decimal strings; body at most 2048 bytes; anything else `400` with an empty body.
- [x] Success body exactly `{"contract_version":1,"result":"accepted|refreshed|superseded"}`; failures empty: `400` malformed, `401` identity/scope, `409` assignment or equal-key conflict, `429` rate limit, `503` switch off, misconfiguration or storage failure.
- [x] Per-purpose mTLS identity: only configured runtime-status identities (trusted-terminator `SSL_CLIENT_*`, TLS 1.3) pass; a native-evidence or unknown identity is `401`; a scope outside the identity's list is `401`; a runtime-status identity equal to another purpose's identity fails closed (`503`).
- [x] Ownership binding: accepted only when `(assignment_epoch, scope_ownership_generation)` and the client identity equal the scope's latest assignment and that assignment is in the highest epoch seen; else `409`.
- [x] Monotonic per-scope key `(assignment_epoch, scope_ownership_generation, source_revision)`: lower `superseded`; equal key with other content marks the scope `invalid` (`409`) until a higher key; equal content with newer `observed_at` `refreshed`.
- [x] `observed_at` later than now + clock uncertainty is refused (`400`); evidence `fresh` only while `now - observed_at + uncertainty <= F`, else `stale`; stale, invalid, superseded-owner, lower-epoch or `ready=false` reports never route.
- [x] No endpoint, host, port or SNI is read from reports; the read model record carries none.
- [x] Additive migration; `down()` refuses before DDL while rows exist (precedent `2026_09_26_150000`).
- [x] Default-off switch `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED`.
- [x] Game fixture bytes copied with provenance (`tests/Fixtures/GameAuth/native-runtime-status-v1/provenance.json`).

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeRuntimeStatus/**
  - app/Http/Controllers/GameAuth/NativeRuntimeStatusController.php
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_120000_add_native_runtime_status_read_model.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/**
  - tests/Fixtures/GameAuth/native-runtime-status-v1/**
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2b-redemption-concurrency.md
modules:
  - GameAuth/NativeRuntimeStatus
dependencies:
  - "#1420 N4P contract (LOCKED candidate, Platform ed6c0d38)"
  - "Game runtime-status producer on Game main@07cd8bcc"
blockers:
  - none
cross_repository_tasks:
  - "Oteryn/Oteryn-Game read-only (Q14): producer contract, encoder and fixtures"
```

Overlap: no open Platform PR touches the owned paths (checked 2026-09-30). The integrated N4P-2b packet is archived by this task.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T23:30:00Z
head: UNKNOWN
branch: claude/n4p-ing-runtime-status
pr: none
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeRuntimeStatus/**
  - app/Http/Controllers/GameAuth/NativeRuntimeStatusController.php
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_120000_add_native_runtime_status_read_model.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/**
  - tests/Fixtures/GameAuth/native-runtime-status-v1/**
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2b-redemption-concurrency.md
proven:
  - The Game encoder output (fixture, 753 bytes, sha256 de892e99...) is accepted byte-for-byte and acknowledged with the exact Game ACK bytes
  - Every Game encode() grammar refusal (zero epoch/generation/revision, observed before published, negative time, protocol_major 2, 65-char revision, space in revision, uppercase uuid, quote in decision_identity) is a 400 here; the largest grammar-valid report (uint64 max, i64 max times, 128/64-char strings) parses within 2048 bytes
  - A report is never accepted without a matching assignment row, so no runtime state exists until the assignment ingestion lands
derived:
  - A future observed_at is refused 400 without mutation (contract says "invalid"; the previous report goes stale within F anyway)
  - A report whose identity differs from the assignment's node_identity is 409 (conflict with the current assignment); an identity whose configured scope list lacks the scope is 401
unknown:
  - U5 measured H/F; U15 per-purpose PKI; U16 assignment_epoch storage on the Game side
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reusing ThrottleNativeEvidencePeer/EnforceNativeEvidenceHttpBounds rejected; they are bound to native-evidence config, and 413/431 are not in the Game producer's status set
  - Laravel's named throttle rejected; its 429 has a body and the Game node classifies a non-empty failure body as an invalid response
changed_paths:
  - app/GameAuth/NativeRuntimeStatus/ (7 files)
  - app/Http/Controllers/GameAuth/NativeRuntimeStatusController.php
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_30_120000_add_native_runtime_status_read_model.php
  - tests/Feature/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusIngestionTest.php
  - tests/Fixtures/GameAuth/native-runtime-status-v1/report.json
  - tests/Fixtures/GameAuth/native-runtime-status-v1/provenance.json
  - docs/agents/tasks/active/OTERYN-20260930-n4p-ing-runtime-status.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2b-redemption-concurrency.md (moved from active)
validation:
  - command: vendor/bin/phpunit (full suite, sqlite) and tests/Feature/GameAuth/NativeRuntimeStatus
    result: PASS
    evidence: local PHP 8.4.19 in a scratch copy (composer.json requires ^8.5; dependencies installed from source with the platform check ignored); 727 tests, 0 failures, 28 skipped; new 11 tests, 546 assertions
  - command: phpstan analyse (level 10, full configuration) and pint --test on changed files
    result: PASS
    evidence: phpstan 2.2.13 and larastan v3.10.0 from their locked git tags in the scratch copy (dist downloads refused by the proxy); no errors; Pint passed; exact-head CI is authoritative
  - command: php -l, git diff --check, checkpoint.py --require-checkpoint, source_branch_closeout.py
    result: PASS
    evidence: local offline run on the candidate
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: no actor path yet; the endpoint is default-off and nothing routes until assignment ingestion and N4P-3 route selection exist; joint E2E is #1419 item 5
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
