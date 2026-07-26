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
- [ ] The QR code has a high-contrast background and remains usable on desktop and mobile widths.
- [ ] Manual key entry remains available as an explicit fallback.
- [ ] Invalid non-TOTP provisioning URIs fail closed.
- [ ] Confirmed MFA, disabling MFA and recovery-code behavior remain unchanged.
- [ ] Focused tests, formatter, static analysis, CI and required browser acceptance pass on the exact head.

## Ownership

```yaml
owned_paths:
  - app/Http/Controllers/Identity/Mfa/MfaEnrollmentController.php
  - app/Identity/Mfa/MfaQrCode.php
  - composer.json
  - composer.lock
  - public/css/mfa.css
  - resources/views/identity/layout.blade.php
  - resources/views/identity/mfa/settings.blade.php
  - tests/Feature/Identity/Mfa/MfaWebFlowTest.php
  - tests/Unit/Identity/MfaQrCodeTest.php
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
updated_at: 2026-07-26T21:46:00Z
head: 8cb3b71197f48023706351ebe56073f912cb8b8f
branch: feat/OTERYN-20260726-mfa-qr-enrollment
pr: 214
status: validating
context_routes:
  - agent-governance
  - identity
  - security
  - testing
  - accessibility
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
  - the MFA web-flow regression now requires the local SVG data URI and scan instructions while forbidding the raw otpauth URI in rendered output
  - the view gives the QR code a fixed white backing surface and responsive maximum width for reliable camera scanning on the dark identity theme
  - PR 213 released all overlapping QR-owned paths and recorded PR 214 as a staging prerequisite
  - all temporary Composer, formatter, diagnostic and patch workflows completed their bounded jobs and were removed
  - repository Pint formatting and static analysis passed on the preceding implementation head
  - docs/agents/ACTIVE_WORK.md was restored to trusted main because the final-staging task already owns that coordination path
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
  - public/css/mfa.css
  - resources/views/identity/layout.blade.php
  - resources/views/identity/mfa/settings.blade.php
  - tests/Feature/Identity/Mfa/MfaWebFlowTest.php
  - tests/Unit/Identity/MfaQrCodeTest.php
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-enrollment.md
validation:
  - command: Composer dependency generation and validation
    result: PASS
    evidence: pull-request workflow resolved endroid/qr-code 6.1.3, updated composer.json/composer.lock and completed strict Composer validation
  - command: Repository Pint formatting
    result: PASS
    evidence: workflow run 30221126420 applied the repository formatter and committed the exact changes
  - command: preceding implementation-head static analysis
    result: PASS
    evidence: CI run 30221309298 completed static analysis successfully before the outdated MFA web-flow assertion failed
  - command: focused QR renderer unit test
    result: PASS
    evidence: CI run 30221309298 reported Tests Unit Identity MfaQrCodeTest PASS
blockers: []
next_action: Run all required exact-head PR 214 checks, resolve any failures, then mark ready and merge before deploying the QR-capable SHA to staging.
```
