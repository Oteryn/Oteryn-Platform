---
task_id: OTERYN-20261009-remembered-device-candidate
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# Disabled native remembered-device candidate

Root owns publication on branch `fix/native-client-login-20261007`, draft PR #1484.
Delegated source and metadata workers have disjoint scoped ownership. Authoring only; no
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
source is being authored separately and is not yet qualified; the existing
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

Jira mapping pending; live Platform Issue #1473 is open. No merge readiness claimed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-09T10:23:05Z
head: 549de370bbb00e389f0c49fa2b780be623056806
branch: fix/native-client-login-20261007
pr: 1484
status: implementing
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
  - docs/agents/tasks/active/OTERYN-20261009-remembered-device-candidate.md
proven:
  - Draft PR 1484 is open on fix/native-client-login-20261007 at observed remote head 549de370bbb00e389f0c49fa2b780be623056806; governing Issue 1473 remains open.
  - Actual Windows browser PKCE, owner character selection and native Game admission completed locally on 2026-10-09 with a normally registered test identity and fenced Game character bootstrap.
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
  - docs/agents/tasks/active/OTERYN-20261009-remembered-device-candidate.md
validation:
  - command: Repository checkpoint validator for both owned packets; bounded task_liveness.evaluate_task against live GitHub
    result: PASS
    evidence: Both Context checkpoints validate against contract v1 and both owned task records report DRAFT_PR with no findings for live PR 1484; this does not clear unrelated repository-wide CI findings.
  - command: Isolated owner-PC DeviceSessions and existing Identity MFA suites; affected Pint
    result: PASS
    evidence: 41 tests and 315 assertions passed in Docker SQLite memory, including real MFA confirmation invalidating prior OAuth and device credentials; no runtime database migration.
  - command: Actual Windows browser PKCE, native owner character selection and Game admission
    result: PASS
    evidence: Baseline local flow completed on 2026-10-09; it does not prove remembered-device enrollment or resumed login.
  - command: Isolated HTTP/core/MFA/cache suites; scoped Pint and PHPStan level10
    result: PASS
    evidence: 64 tests and657 assertions; strict Bearer parsing, raw-peer loopback checks and original OAuth ticket consumption verified. Memory fixtures only; candidate OFF.
  - command: Complete remembered-device native client integration
    result: NOT_RUN
    evidence: Concrete exclusive lock, durable journal, HTTPS and monotonic-ticket adapters remain unwired.
blockers:
  - Complete secure-vault native client integration and deployment-database race qualification remain outstanding.
  - Accepted owning contract, deployment-database race evidence and exact-head independent security review are required before activation.
  - Existing unrelated LCFA lifecycle and eight trusted-main unexplained branch findings are outside this scoped metadata correction.
next_action: Publish the qualified disabled HTTP successor on the draft authoring branch, then complete native consumer bindings without activating credentials or claiming contract acceptance.
```

The checkpoint head is the observed remote authoring anchor. Ongoing local HTTP
successor source is not frozen or validated by that head, and baseline browser
admission evidence is separate from remembered-device qualification.
