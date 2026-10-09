---
task_id: OTERYN-20261009-remembered-device-candidate
governing_issue: 1473
required_reads: []
search_first: []
optional_reads: []
---

# Disabled native remembered-device candidate

Single writer:root. Branch:fix/native-client-login-20261007. Authoring only; no
production deployment, runtime migration, DB reset or contract acceptance. Owner
explicitly authorized both Platform and Game client work and asks to avoid
recurring browser redirection without lowering security.

Owned paths:app/GameAuth/DeviceSessions/**; additive2026_10_09_220000 migration;
DeviceSessions feature tests; candidate contract; existing
app/Identity/Mfa/ConfirmIdentityMfaEnrollment.php security action.

Service candidate is testing/preproduction-only and default OFF. Opaque hashed
device credentials have immutable owner/client/security-generation binding,
absolute/idle expiry, transactional rotation and retained replay lineage. No
HTTP routes, token persistence or existing OAuth/ticket revocation changed.

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

Jira mapping pending; live Platform Issue1473 is open. No merge readiness claimed.
