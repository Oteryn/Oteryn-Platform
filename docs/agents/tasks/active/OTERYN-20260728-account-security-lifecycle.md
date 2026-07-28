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

- [ ] Current Identity, session, notification, audit and binding implementations are inventoried from exact code and tests before mutation design is finalized.
- [ ] Primary-email change requires current-password or stronger step-up authentication, verifies the new address through a single-use expiring token, notifies the old address, enforces uniqueness and cooldown, supports cancellation/recovery, and prevents replay/concurrent ambiguity.
- [ ] Users can view bounded active web sessions with a current-session marker, non-sensitive device/network summaries and last-activity time.
- [ ] Users can revoke one remote session or all other sessions without accepting browser-supplied ownership as authority; current-session revocation and race behavior are deterministic.
- [ ] Password reset/change, MFA disable, account termination and recovery-key use continue to revoke the appropriate sessions and rotate the current session where required.
- [ ] Account privacy/status controls are Platform-owned, server-authorized, safely defaulted and ready for future public-profile consumers without exposing private account or Canary identifiers.
- [ ] Account termination supports request, grace period, cancellation and idempotent finalization; first delivery does not delete Canary accounts or characters and fails closed around active marketplace/account-recovery operations.
- [ ] A high-assurance recovery key is generated once, stored only as a verifier, rotated/revoked safely, consumed with replay protection and never logged or included in audit metadata.
- [ ] Exceptional Canary unlink/rebind behavior has an explicit reviewed policy and cannot silently transfer ownership; any adopted workflow is audited, MFA/permission guarded and contract-safe.
- [ ] Email-code MFA has an explicit reviewed security decision; exclusion or implementation is durable and machine-visible rather than silently omitted.
- [ ] Security-sensitive actions are CSRF-protected, rate-limited, audited with bounded metadata and covered for IDOR, replay, stale state and concurrency.
- [ ] English/Polish account-security UI covers desktop, tablet and mobile, including success, empty, validation, authorization, expired-token and recovery states.
- [ ] Focused unit, feature, database/concurrency and zero-retry browser acceptance coverage exists for every adopted lifecycle.
- [ ] Module catalog, data ownership, security architecture, operations guidance and product-completeness ledger are updated without claiming production verification.
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
blockers:
  - exact session persistence and binding-state implementation must be discovered before final design
cross_repository_tasks:
  - blakinio/canary remains read-only; no Canary account or character mutation is authorized by this task
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-28T19:21:19Z
head: 0470eb6bcbbebca80927ce8df4fc7507d6413914
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
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
proven:
  - PR #283 is open, draft and mergeable on branch feat/OTERYN-20260728-account-security-lifecycle; the last fully validated functional head is 0470eb6bcbbebca80927ce8df4fc7507d6413914.
  - The branch implements Platform-owned active-session persistence and targeted revocation, verified email change with old-address recovery, privacy controls, non-destructive termination and a verifier-only recovery-key lifecycle.
  - Browser-supplied identifiers do not establish session ownership, recovery keys are stored only as keyed verifiers, and termination preserves the immutable Canary binding and Canary-owned account and character data.
  - Revoked or stale registered sessions are rejected before protected controllers execute and are redirected to the login surface; public routes continue as guest requests.
  - Canary and login-server schema or session compatibility is unchanged; blakinio/canary remains read-only and no cross-repository rollout is required.
  - No secret or production-only credential is committed; rollback is the reversible PR branch plus reversible Platform migrations, and production deployment remains unverified.
  - Exact-head CI run 30390881191 passed Composer validation, dependency audit, Pint, PHPStan and the full PHPUnit suite.
  - Portal Acceptance Contract run 30390881152 passed both strict portal coverage closure and the complete zero-retry account lifecycle against real HTTP, MariaDB, Redis and MailHog.
  - Acceptance E2E and Visual UX run 30390881206 and Phase 7 run 30390881058 passed on exact head 0470eb6bcbbebca80927ce8df4fc7507d6413914.
  - Agent Governance, Synology preflight and image build, DB outage, edge-security and game-auth concurrency workflows all passed on exact head 0470eb6bcbbebca80927ce8df4fc7507d6413914.
  - Account Security Static Diagnostics run 30390881213 produced artifact 8700909486 with PHPStan exit-code 0.
derived:
  - The formatter, PHPStan, PHPUnit, strict coverage ledger, targeted session revocation and complete browser lifecycle blockers are resolved on the validated functional head.
  - The trust boundary remains Platform Identity and registered web-session state; authorization derives the authenticated Identity server-side and security-sensitive mutations revoke or rotate sessions deterministically.
  - The remaining acceptance slice is localization and durable architecture, operations and product-completeness documentation; the current PR changed-file inventory does not include those declared outputs.
unknown:
  - English and Polish account-security UI coverage remains unproven because the declared localization outputs are not present in the current PR changed-file inventory.
  - Required module catalog, data ownership, security architecture, operations guidance and product-completeness updates remain unproven because those files are not present in the current PR changed-file inventory.
  - Production deployment state and production-only configuration remain outside repository evidence.
conflicts: []
first_failure:
  marker: none-on-validated-functional-head
  evidence: All required workflows associated with exact head 0470eb6bcbbebca80927ce8df4fc7507d6413914 concluded success, including CI, Portal Acceptance Contract, Acceptance E2E and Visual UX and Phase 7.
rejected_hypotheses:
  - PHPStan remains blocked after the Pint patch; rejected because exact-head CI and static diagnostics both pass with PHPStan exit-code 0.
  - Targeted or global session revocation may allow a stale authenticated request to execute a protected controller; rejected by the protected-route regression test and complete browser lifecycle evidence.
  - First-slice termination should delete or unlink Canary data; rejected because the immutable binding contract and repository authorization permit no such mutation.
  - Email-code MFA should be added as a second factor; rejected by the durable ADR because email is already the recovery channel.
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
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
validation:
  - command: CI run 30390881191
    result: PASS
    evidence: Composer metadata, dependency audit, Pint, PHPStan and the full PHPUnit suite completed successfully on exact head 0470eb6bcbbebca80927ce8df4fc7507d6413914.
  - command: Account Security Static Diagnostics run 30390881213
    result: PASS
    evidence: artifact 8700909486 records PHPStan exit-code 0 on exact head 0470eb6bcbbebca80927ce8df4fc7507d6413914.
  - command: Portal Acceptance Contract run 30390881152
    result: PASS
    evidence: strict portal coverage closure and the complete zero-retry account lifecycle both completed successfully.
  - command: Acceptance E2E and Visual UX run 30390881206
    result: PASS
    evidence: the exact-head browser acceptance and visual/UX workflow concluded success.
  - command: Phase 7 Production-Like Validation run 30390881058
    result: PASS
    evidence: production-like validation concluded success on the exact functional head.
  - command: Agent Governance run 30390881231, Synology preflight run 30390881050, image build run 30390881200, DB outage run 30390881297, edge-security run 30390881051 and game-auth concurrency run 30390881109
    result: PASS
    evidence: all focused governance, infrastructure and security workflows concluded success on the exact functional head.
blockers:
  - PR #283 remains validating because English/Polish account-security localization and the required architecture, operations and product-completeness documentation have not been closed.
  - Production deployment and production-only configuration remain unverified and outside repository evidence.
next_action: Close the remaining acceptance slice by adding English/Polish account-security localization and updating the module catalog, data ownership, security architecture, operations guidance and product-completeness documentation, then rerun exact-final-head workflows.
```

## Boundaries

This task may disable or terminate Platform web access but cannot delete or silently transfer Canary-owned accounts or characters. Production deployment and direct production verification remain isolated to Issue #91.
