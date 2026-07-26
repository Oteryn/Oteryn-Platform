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
  - PR 213 has released all overlapping Identity and Composer paths
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T21:25:00Z
head: dd932d62b16df29687045615aaeb7d0b2cb2a10c
branch: feat/OTERYN-20260726-mfa-qr-enrollment
pr: 214
status: validating
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
  - endroid/qr-code 6.1.3 is locked locally and supports SVG rendering on PHP 8.5 without an external QR endpoint
  - the implementation renders a data URI from the existing provisioning URI and keeps manual entry collapsed as fallback
  - invalid non-TOTP provisioning URIs are rejected before rendering
  - the QR renderer has focused unit coverage and does not embed the original otpauth URI as readable SVG text
  - PR 213 released all overlapping QR-owned paths and recorded PR 214 as a staging prerequisite
  - the temporary Composer workflow generated and validated the lockfile and was removed
  - no production, staging-data, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurred
derived:
  - a separate PR allows QR to reach staging before final-staging closure remains blocked on genuine MFA confirmation
  - local inline rendering avoids disclosing the TOTP secret to an external QR provider
unknown:
  - exact final validated head and merge SHA
  - scan result on the deployed staging page until PR 214 is merged and deployed
conflicts: []
changed_paths:
  - app/Http/Controllers/Identity/Mfa/MfaEnrollmentController.php
  - app/Identity/Mfa/MfaQrCode.php
  - composer.json
  - composer.lock
  - resources/views/identity/mfa/settings.blade.php
  - tests/Unit/Identity/MfaQrCodeTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-enrollment.md
validation:
  - command: Composer dependency generation and validation
    result: PASS
    evidence: pull-request workflow resolved endroid/qr-code 6.1.3, updated composer.json/composer.lock and completed strict Composer validation
blockers: []
next_action: Run all required exact-head PR 214 checks, resolve any failures, then mark ready and merge before deploying the QR-capable SHA to staging.
```
