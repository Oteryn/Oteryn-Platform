---
task_id: OTERYN-20261009-remembered-device-candidate
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# Disabled native remembered-device candidate — terminal archive

## Terminal outcome

Draft PR #1484 (`fix/native-client-login-20261007`) merged into `main` at 2026-10-10T20:51:06Z, delivering this workstream as a default-OFF testing/preproduction candidate. This record is archived by lifecycle closeout. No contract acceptance, runtime migration, client integration, production activation or exact-head independent security review is claimed; those require a new task record.

Root owned publication on branch `fix/native-client-login-20261007`, PR #1484.
Delegated source and metadata workers had disjoint scoped ownership. Authoring only; no
production deployment, runtime migration, DB reset or contract acceptance. Owner
explicitly authorized both Platform and Game client work and asks to avoid
recurring browser redirection without lowering security.

Owned workstream paths: `app/GameAuth/DeviceSessions/**`, the additive
`2026_10_09_220000` migration, DeviceSessions feature tests, candidate contract,
existing `app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php` security action, and
the bounded device-session HTTP adapter paths listed in the checkpoint.

Service candidate is testing/preproduction-only and default OFF. Opaque hashed
device credentials have immutable owner/client/security-generation binding,
absolute/idle expiry, transactional rotation and retained replay lineage. No
HTTP credential enrollment or client token persistence is active. HTTP adapter
source was authored separately and was not independently qualified; the existing
OAuth/ticket revocation semantics remain unchanged.

Independent local security sweep identified an MFA transition gap: successful
MFA enrollment now calls existing RevokeIdentityGameAuthorizations within the
same confirmation transaction. Regression invokes actual Start/Confirm actions
and proves prior OAuth/device credentials cannot issue a native ticket afterward.

Isolated owner-PC Docker SQLite memory verification:41 tests/315 assertions PASS
(DeviceSessions and existing Identity/Mfa suites), affected Pint PASS. No runtime
DB migration occurred. This does not qualify deployment-database locking.

Client vault implementation exists separately in Game; Linux ambiguous namespace
records can be deleted but cannot be loaded/saved. Real OS-vault smoke checks,
client rotation/crash recovery/cross-process locking, secure HTTP adapters,
logout/revocation integration, accepted owning contract and exact-head independent
security review remain pending. Existing first-device browser PKCE remains.

Jira mapping pending; live Platform Issue #1473 lifecycle is tracked separately. No merge readiness or activation claimed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T21:00:00Z
head: e6305777c2f9ec7b9ec588212f49c4fe1cac54ca
branch: fix/native-client-login-20261007
pr: 1484
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - docs/agents/BUILD_TEST_MATRIX.md
owned_paths:
  - app/GameAuth/DeviceSessions/**
  - app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php
  - app/Http/Controllers/GameAuth/DeviceSessions/**
  - app/Http/Middleware/GameAuth/GuardNativeDeviceSessionHttp.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - routes/api.php
  - config/game-auth.php
  - bootstrap/app.php
  - database/migrations/2026_10_09_220000_create_native_device_sessions.php
  - tests/Feature/GameAuth/DeviceSessions/**
  - docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/archive/OTERYN-20261009-remembered-device-candidate.md
proven:
  - Actual Windows browser PKCE, owner character selection and native Game admission completed locally on 2026-10-09 with a normally registered test identity and fenced Game character bootstrap.
  - PR 1484 merged into main at 2026-10-10T20:51:06Z from head e6305777c2f9ec7b9ec588212f49c4fe1cac54ca; governing Issue 1473 lifecycle is tracked separately.
  - The disabled remembered-device service and existing Identity MFA suites passed 41 tests and 315 assertions in isolated owner-PC Docker SQLite memory; affected Pint passed.
  - Real MFA enrollment confirmation invokes existing game-authorization revocation in the same transaction and the regression denies prior OAuth and device credentials afterward.
  - The remembered-device contract remains a candidate; no HTTP credential enrollment, runtime migration, client integration or production activation has occurred.
derived:
  - A separate narrow device credential can reduce repeated browser redirects while preserving first-device browser PKCE and unchanged Passport ticket-family consumption.
unknown:
  - Concrete client lock/journal/HTTPS integration and accepted successor contract review.
  - Deployment-database enrollment, rotation and revocation race behavior; SQLite tests do not prove row-lock concurrency.
  - Complete OS-vault restart behavior, successor-write failure recovery, cross-process serialization and actual client-to-game remembered login.
  - Accepted owning contract and exact-head independent security review before any activation.
conflicts: []
first_failure:
  marker: MISSING_CONTEXT_CHECKPOINT
  evidence: Classification and Agent Governance at remote candidate 549de370 rejected this packet for its missing Context checkpoint and absent machine-readable live branch and PR identity.
rejected_hypotheses:
  - Persisting an existing Passport refresh token would implement remembered login without a contract or server change.
  - In-memory SQLite success proves deployment-database row-lock races.
changed_paths:
  - app/GameAuth/DeviceSessions/**
  - app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php
  - app/Http/Controllers/GameAuth/DeviceSessions/**
  - app/Http/Middleware/GameAuth/GuardNativeDeviceSessionHttp.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - routes/api.php
  - config/game-auth.php
  - bootstrap/app.php
  - database/migrations/2026_10_09_220000_create_native_device_sessions.php
  - tests/Feature/GameAuth/DeviceSessions/**
  - docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/archive/OTERYN-20261009-remembered-device-candidate.md
validation:
  - command: Isolated owner-PC DeviceSessions and existing Identity MFA suites; affected Pint
    result: PASS
    evidence: 41 tests and 315 assertions passed in Docker SQLite memory, including real MFA confirmation invalidating prior OAuth and device credentials; no runtime database migration.
  - command: Actual Windows browser PKCE, native owner character selection and Game admission
    result: PASS
    evidence: Baseline local flow completed on 2026-10-09; it does not prove remembered-device enrollment or resumed login.
  - command: Isolated HTTP/core/MFA/cache suites; scoped Pint and PHPStan level10
    result: PASS
    evidence: 64 tests and 657 assertions; strict Bearer parsing, raw-peer loopback checks and original OAuth ticket consumption verified. Memory fixtures only; candidate OFF.
  - command: Full composer analyse; affected test Pint; expanded isolated directory/status/device/MFA/cache suites
    result: PASS
    evidence: All PHPStan level10 application/test paths pass; 75 tests and 737 assertions pass after task-owned fixture typing fixes. Memory database only; remembered candidate remains OFF.
  - command: Complete remembered-device native client integration
    result: NOT_RUN
    evidence: Concrete exclusive lock, durable journal, HTTPS and monotonic-ticket adapters remain unwired; not claimed by this record.
  - command: PR #1484 protected merge
    result: PASS
    evidence: PR #1484 state closed, merged true, merged_at 2026-10-10T20:51:06Z
blockers:
  - Complete secure-vault native client integration and deployment-database race qualification remain outstanding and require a new task record.
  - Accepted owning contract, deployment-database race evidence and exact-head independent security review are required before any activation.
next_action: Archive this terminal task record; PR #1484 is already merged. Client integration, contract acceptance and security review need a new task record.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: merged feature branch has no retention or recovery purpose after protected merge
source_branch_evidence: PR #1484 merged (REST pulls/1484 state closed, merged true) as cdd562bf52fa98e427227c4ccb52b9023b318f63 at 2026-10-10T20:51:06Z from exact head e6305777c2f9ec7b9ec588212f49c4fe1cac54ca and source branch fix/native-client-login-20261007 is absent (REST branches API returns 404 Branch not found)
```
