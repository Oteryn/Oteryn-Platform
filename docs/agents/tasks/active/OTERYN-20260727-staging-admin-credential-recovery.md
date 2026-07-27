---
task_id: OTERYN-20260727-staging-admin-credential-recovery
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/INCIDENT_RECOVERY_RUNBOOK.md
search_first:
  - existing password reset/session revocation implementation
  - existing first-administrator bootstrap evidence
  - active tasks and PRs touching staging workflows
optional_reads:
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
---

# OTERYN-20260727-staging-admin-credential-recovery

## Goal

Execute a one-time, secret-safe credential reset for the existing Synology staging administrator Identity and preserve its confirmed MFA and audited `platform_admin` assignment.

## Acceptance criteria

- [ ] No email address, password, MFA secret, recovery code or private key is committed, logged or posted to GitHub.
- [ ] The encrypted recovery payload can be decrypted only by an ephemeral private key retained on the staging runner.
- [ ] The reset targets exactly one enabled, MFA-confirmed `platform_admin` Identity.
- [ ] The application-owned `IdentityCredentialUpdater::reset` path changes the password, revokes web sessions and game authorizations, and records the security event.
- [ ] Sanitized completion evidence is posted to Issue #248.
- [ ] The temporary workflow, encrypted payload and runner-side key material are removed after success.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-staging-admin-credential-recovery.yml
  - ops/staging-admin-credential-recovery/**
  - docs/agents/tasks/active/OTERYN-20260727-staging-admin-credential-recovery.md
  - docs/agents/tasks/archive/OTERYN-20260727-staging-admin-credential-recovery.md
modules:
  - Identity
  - Admin / RBAC
  - Synology staging operations
dependencies:
  - Issue 248
  - existing Synology staging runner and deployed Platform container
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T16:33:00Z
head: c5ddf82e715e6fb01f8defe6f9052622a55db547
branch: ops/OTERYN-20260727-staging-admin-credential-recovery
pr: 249
status: validating
context_routes:
  - auth-identity
  - admin-rbac
  - security
  - testing
owned_paths:
  - .github/workflows/one-shot-staging-admin-credential-recovery.yml
  - ops/staging-admin-credential-recovery/**
  - docs/agents/tasks/active/OTERYN-20260727-staging-admin-credential-recovery.md
  - docs/agents/tasks/archive/OTERYN-20260727-staging-admin-credential-recovery.md
proven:
  - Synology staging has two enabled Identities and the guarded final-staging workflow bootstrapped the sole enabled MFA-confirmed candidate as the first platform administrator
  - IdentityCredentialUpdater::reset replaces the password, revokes web sessions, revokes game authorizations and records a password-reset completion event
  - active PR 247 owns only its exact portal acceptance workflow and listed acceptance/governance paths; this task uses disjoint paths
  - Issue 248 contains only sanitized execution scope
  - PR 249 contains only the dedicated one-shot workflow and this task record
  - the requested target email and temporary password remain outside repository-visible state
derived:
  - a two-phase ephemeral public-key exchange can deliver an encrypted reset payload without exposing credentials to GitHub
unknown:
  - exact corrected PR 249 validation result
  - exact self-hosted runner result for key generation and credential application
conflicts: []
first_failure:
  marker: Agent Governance run 30285063543 checkpoint-validation
  evidence: checkpoint status validating-keygen was outside the governance contract allowed status set
rejected_hypotheses:
  - committing or logging a temporary plaintext password is acceptable for staging
  - creating a second synthetic administrator is necessary
changed_paths:
  - .github/workflows/one-shot-staging-admin-credential-recovery.yml
  - docs/agents/tasks/active/OTERYN-20260727-staging-admin-credential-recovery.md
validation:
  - command: repository and active ownership inspection
    result: PASS
    evidence: unique workflow, encrypted-payload and task paths do not overlap PR 247 owned paths
  - command: Agent Governance run 30285063543
    result: FAIL
    evidence: unsupported checkpoint status validating-keygen; corrected to validating
  - command: corrected exact-head GitHub checks for PR 249
    result: NOT_RUN
    evidence: pending workflow registration on the corrected head
blockers:
  - none
next_action: Pass corrected exact-head checks, merge PR 249 with the keygen marker, and read the sanitized public key from Issue 248.
```

## Notes

This operation is staging-only. It does not change production, Canary/login-server repositories, MFA state or administrator-role assignment.
