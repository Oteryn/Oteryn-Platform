---
task_id: OTERYN-20260728-account-security-lifecycle
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - docs/agents/tasks/active/** for identity, account, session, email, recovery, privacy, termination or binding ownership
  - open pull requests for Issue #276 or overlapping Identity/Accounts/session paths
  - current authentication routes, session persistence, security-event recording, notification and Canary binding state
optional_reads:
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
  - docs/architecture/adr/**identity**
---

# OTERYN-20260728-account-security-lifecycle

## Goal

Close Issue #276 with a secure Platform-owned account lifecycle: confirmed primary-email change, active-session inventory and targeted revocation, privacy/status controls, bounded termination with grace/cancellation/finalization, a high-assurance recovery artifact, and an explicit deny-by-default decision for exceptional Canary binding changes and email-code MFA.

## Acceptance criteria

- [x] Current Identity, session, notification, audit and binding implementations are inventoried from exact code and tests before mutation design is finalized.
- [x] Primary-email change requires current-password or stronger step-up authentication, verifies the new address through a single-use expiring token, notifies the old address, enforces uniqueness and cooldown, supports cancellation/recovery, and prevents replay/concurrent ambiguity.
- [x] Users can view bounded active web sessions with a current-session marker, non-sensitive device/network summaries and last-activity time.
- [x] Users can revoke one remote session or all other sessions without accepting browser-supplied ownership as authority; current-session revocation and race behavior are deterministic.
- [x] Password reset/change, MFA disable, account termination and recovery-key use continue to revoke the appropriate sessions and rotate the current session where required.
- [x] Account privacy/status controls are Platform-owned, server-authorized, safely defaulted and ready for future public-profile consumers without exposing private account or Canary identifiers.
- [x] Account termination supports request, grace period, cancellation and idempotent finalization; first delivery does not delete Canary accounts or characters and fails closed around active marketplace/account-recovery operations.
- [x] A high-assurance recovery key is generated once, stored only as a verifier, rotated/revoked safely, consumed with replay protection and never logged or included in audit metadata.
- [x] Exceptional Canary unlink/rebind behavior has an explicit reviewed policy and cannot silently transfer ownership; any adopted workflow is audited, MFA/permission guarded and contract-safe.
- [x] Email-code MFA has an explicit reviewed security decision; exclusion or implementation is durable and machine-visible rather than silently omitted.
- [x] Security-sensitive actions are CSRF-protected, rate-limited, audited with bounded metadata and covered for IDOR, replay, stale state and concurrency.
- [x] English/Polish account-security UI covers desktop, tablet and mobile, including success, empty, validation, authorization, expired-token and recovery states.
- [x] Focused unit, feature, database/concurrency and zero-retry browser acceptance coverage exists for every adopted lifecycle.
- [x] Module catalog, data ownership, security architecture, operations guidance and product-completeness ledger are updated without claiming production verification.
- [ ] Every required exact-final-head workflow passes before merge.

## Ownership

```yaml
owned_paths:
  - app/Identity/**
  - app/Accounts/**
  - app/Http/Controllers/Identity/**
  - app/Http/Middleware/**Session**
  - app/Http/Requests/Identity/**
  - app/Mail/Identity/**
  - app/Notifications/Identity/**
  - database/migrations/*identity*email*
  - database/migrations/*identity*session*
  - database/migrations/*identity*privacy*
  - database/migrations/*identity*termination*
  - database/migrations/*identity*recovery*
  - routes/**identity**
  - resources/views/identity/**
  - resources/views/account/**
  - lang/en/identity.php
  - lang/pl/identity.php
  - config/identity.php
  - tests/Unit/Identity/**
  - tests/Feature/Identity/**
  - tests/Integration/Identity/**
  - scripts/acceptance/tests/*account*security*
  - scripts/acceptance/coverage/**
  - docs/architecture/adr/*account-security*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/*ACCOUNT*SECURITY*
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
modules:
  - Identity
  - Accounts
  - Audit
  - Notifications
  - Testing
dependencies:
  - current Platform Identity credential and MFA lifecycle
  - current revocable Platform web-session mechanism
  - immutable Identity-to-Canary binding contract
  - test SMTP and existing browser account-lifecycle harness
blockers: []
cross_repository_tasks:
  - blakinio/canary remains read-only; no Canary account or character mutation is authorized by this task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T20:12:16Z
head: 0bd36cf86de2028872690f4d05669cf0ecf1d3cb
branch: feat/OTERYN-20260728-account-security-lifecycle
pr: 283
status: validating
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - accounts-characters
  - canary-integration
  - security
  - web-cms
  - testing
owned_paths:
  - app/Identity/**
  - app/Accounts/**
  - app/Http/Controllers/Identity/**
  - app/Http/Middleware/**Session**
  - app/Http/Requests/Identity/**
  - app/Mail/Identity/**
  - app/Notifications/Identity/**
  - database/migrations/*identity*
  - routes/**identity**
  - resources/views/identity/**
  - resources/views/account/**
  - lang/{en,pl}/identity.php
  - config/identity_security.php
  - tests/**/Identity/**
  - scripts/acceptance/tests/*account*security*
  - scripts/acceptance/coverage/**
  - docs/architecture/adr/*account-security*
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/*ACCOUNT*SECURITY*
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
proven:
  - PR #283 is open and draft on branch feat/OTERYN-20260728-account-security-lifecycle; implementation and durable documentation are complete in the branch tree represented by parent head 0bd36cf86de2028872690f4d05669cf0ecf1d3cb.
  - Platform-owned registered web sessions support bounded inventory plus owner-scoped targeted, current and all-other revocation; stale or revoked sessions are invalidated before protected controllers execute.
  - Primary-email change uses current-password validation, new-address confirmation, previous-address cancellation/recovery, bounded expiry/cooldown, single-use replay denial and global web/game authorization revocation.
  - Account privacy controls default private and are persisted and audited server-side.
  - One active high-assurance recovery key is displayed once, stored only as a keyed verifier and supports rotation, revocation, single use, password/MFA reset and replay denial.
  - Platform account termination uses explicit confirmation, dependency guards, a bounded grace period, cancellation and idempotent finalization while preserving the immutable Canary binding and Canary-owned account and character data.
  - Email-code MFA is intentionally not adopted because email remains the recovery channel; self-service Canary import, unlink, rebind or transfer is intentionally not applicable without a separate reviewed operation contract.
  - English and Polish account-security presentation covers protected, guest and token surfaces, validation/domain errors, notification links and desktop/tablet/mobile responsive states.
  - Module catalog, data ownership, security architecture, account-security operations runbook, portal acceptance matrix, project state and the 43-capability product-completeness ledger are updated without a production claim.
  - Product-completeness account capability status is now 9 implemented, 8 partial, 25 missing and 1 not applicable; required remaining benchmark gaps remain #277, #279 and #280, with #278 required before commerce activation and #281 owning knowledge expansion.
  - Canary and login-server schema or session compatibility is unchanged; blakinio/canary remains read-only and no cross-repository rollout is required.
derived:
  - The implementation, localization and documentation slices requested by Issue #276 are closed in the repository tree; only exact-final-head workflow completion remains before review.
  - Trust derives from authenticated Platform Identity, registered session state and the ready server-resolved binding; no browser identifier establishes Canary or session ownership.
  - Repository and isolated staging-like evidence can support merge review but cannot establish production deployment or PRODUCTION_PROVEN status.
unknown:
  - Exact-final-head workflow conclusions for the checkpoint commit are pending.
  - Production deployment state, production-only configuration and direct production verification remain outside repository evidence and Issue #276.
conflicts: []
first_failure:
  marker: phpstan-test-helper-method-not-found
  evidence: CI run 30393906804 found one PHPStan error because the new test called assertSessionHas on the test class; commit a3e080411baf64dc0c7b73dd61421898a6cc7387 replaced it with a direct typed session assertion, after which focused format/static diagnostics passed on subsequent code and ledger heads.
rejected_hypotheses:
  - Account-security localization can be satisfied by hardcoded Polish copy; rejected because the delivered implementation uses complete EN/PL dictionaries, scoped locale persistence, localized domain/validation outcomes and locale-preserving notification links.
  - A revoked registered session may continue into a protected controller; rejected by feature regression and browser acceptance design that expects redirect to login before controller execution.
  - First-slice termination should delete, unlink or transfer Canary data; rejected by the immutable binding contract, data-ownership boundary and explicit operations runbook.
  - Email-code MFA should be silently treated as a delivered second factor; rejected by the durable security decision that email is already the recovery channel.
changed_paths:
  - app/Identity/**
  - app/Http/Controllers/Identity/**
  - app/Http/Middleware/EnsureIdentitySessionIsCurrent.php
  - app/Http/Requests/Identity/**
  - app/Notifications/Identity/**
  - app/Console/Commands/FinalizeIdentityTerminations.php
  - app/Providers/AccountSecurityServiceProvider.php
  - bootstrap/app.php
  - bootstrap/providers.php
  - database/migrations/*identity*
  - routes/web.php
  - resources/views/identity/**
  - lang/en/identity.php
  - lang/pl/identity.php
  - config/identity_security.php
  - tests/Feature/Identity/**
  - tests/Unit/Identity/**
  - tests/TestCase.php
  - scripts/acceptance/tests/*account*security*
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/coverage/surfaces/identity-account-security-lifecycle.json
  - docs/architecture/adr/0017-account-security-lifecycle.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/ACCOUNT_SECURITY_LIFECYCLE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
  - docs/testing/product-completeness-benchmark.json
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
validation:
  - command: Account Security Format Diagnostics runs through code and ledger head 8acb003378fed7bc343e04d6236616ad4680c398
    result: PASS
    evidence: focused formatting diagnostics concluded success after the localization and ledger changes.
  - command: Account Security Static Diagnostics through code and ledger head 8acb003378fed7bc343e04d6236616ad4680c398
    result: PASS
    evidence: focused PHPStan diagnostics concluded success after the test-helper correction.
  - command: Agent Governance, Synology preflight, image build, DB outage, edge-security and game-auth concurrency through code and ledger head 8acb003378fed7bc343e04d6236616ad4680c398
    result: PASS
    evidence: all six focused governance, infrastructure and security workflows concluded success; longer workflows were superseded by later documentation commits rather than failing.
  - command: exact-final-head workflows for this checkpoint commit
    result: PENDING
    evidence: CI, strict Portal Acceptance Contract, complete zero-retry account lifecycle, E2E/Visual UX, Phase 7 and focused workflows must conclude on the single checkpoint head before review.
blockers:
  - Exact-final-head workflow conclusions are pending for the checkpoint commit.
next_action: Run every required workflow on this checkpoint head; if all pass, record the validated head in the checkpoint, mark PR #283 ready for review and leave review/merge as the sole remaining action.
```

## Boundaries

This task may disable or terminate Platform web access but cannot delete or silently transfer Canary-owned accounts or characters. Production deployment and direct production verification remain isolated to Issue #91.
