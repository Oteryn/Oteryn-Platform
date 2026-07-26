---
task_id: OTERYN-20260726-mfa-qr-enrollment
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
search_first:
  - active tasks and open pull requests touching Identity MFA enrollment, Composer dependencies or the shared identity view
  - existing MFA provisioning URI, no-store response and CSP boundaries
  - existing authenticator enrollment and recovery-code tests
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-mfa-qr-enrollment

## Goal

Make local QR scanning the primary authenticator-app enrollment path while retaining manual key entry as a collapsed fallback and preserving the existing password, TOTP confirmation, recovery-code, session and no-store boundaries.

## Acceptance criteria

- [ ] Pending MFA enrollment renders a locally generated inline SVG QR code for the existing `otpauth://` provisioning URI.
- [ ] No MFA secret or provisioning URI is sent to an external QR service.
- [ ] Google Authenticator and other TOTP apps can scan the QR code and produce a valid six-digit code.
- [ ] Manual key entry remains available as an explicit fallback.
- [ ] Invalid non-TOTP provisioning URIs fail closed.
- [ ] Confirmed MFA, disabling MFA and recovery-code behavior remain unchanged.
- [ ] Focused unit/feature tests, formatter, static analysis, CI and required acceptance pass on the exact head.

## Ownership

```yaml
owned_paths:
  - app/Http/Controllers/Identity/Mfa/MfaEnrollmentController.php
  - app/Identity/Mfa/MfaQrCode.php
  - composer.json
  - composer.lock
  - resources/views/identity/mfa/settings.blade.php
  - tests/Unit/Identity/MfaQrCodeTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-enrollment.md
  - docs/agents/tasks/archive/OTERYN-20260726-mfa-qr-enrollment.md
modules:
  - Identity
  - Security
  - Testing
  - AgentGovernance
dependencies:
  - existing MFA provisioning and confirmation flow
  - PR 213 must release the same Identity/Composer paths before this PR is reviewed
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T21:18:00Z
head: db97f85121f22b42881084ff82229cde5484a179
branch: feat/OTERYN-20260726-mfa-qr-enrollment
pr: null
status: implementing
context_routes:
  - agent-governance
  - identity
  - security
  - testing
owned_paths:
  - paths listed in Ownership
proven:
  - the existing MFA screen exposes a manual secret and internal otpauth provisioning URI only after enrollment starts
  - the response is private and no-store
  - the global CSP allows data URI images while scripts and connections remain restricted to self
  - endroid/qr-code 6.x supports local SVG rendering on PHP 8.5 without an external QR endpoint
  - the implementation renders a data URI from the existing provisioning URI and keeps manual entry collapsed as fallback
derived:
  - a separate PR is required so QR can reach staging before final-staging closure remains blocked on genuine MFA confirmation
unknown:
  - exact PR number and final validated head
conflicts: []
changed_paths:
  - app/Http/Controllers/Identity/Mfa/MfaEnrollmentController.php
  - app/Identity/Mfa/MfaQrCode.php
  - resources/views/identity/mfa/settings.blade.php
  - tests/Unit/Identity/MfaQrCodeTest.php
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-enrollment.md
validation: []
blockers: []
next_action: Add and lock the local QR dependency, open the bounded PR after PR 213 releases overlapping paths, then run exact-head validation.
```
