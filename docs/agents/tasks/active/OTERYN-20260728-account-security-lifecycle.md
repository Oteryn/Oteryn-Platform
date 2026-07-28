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
updated_at: 2026-07-28T17:52:49Z
head: 544912954cbc3c07657182d0923042aaea26f27b
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
  - docs/architecture/adr/*account-security*
  - docs/operations/*ACCOUNT*SECURITY*
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
proven:
  - PR #283 is open, draft and mergeable on branch feat/OTERYN-20260728-account-security-lifecycle at head 544912954cbc3c07657182d0923042aaea26f27b.
  - The branch implements Platform-owned active-session persistence and targeted revocation, verified email change with old-address recovery, privacy controls, non-destructive termination and a verifier-only recovery-key lifecycle.
  - Browser-supplied identifiers do not establish session ownership, recovery keys are stored only as keyed verifiers, and termination preserves the immutable Canary binding and Canary-owned account and character data.
  - Canary and login-server schema or session compatibility is unchanged; blakinio/canary remains read-only and no cross-repository rollout is required.
  - No secret or production-only credential is committed; rollback is the reversible PR branch plus reversible Platform migrations, and production deployment remains unverified.
  - Exact-head format diagnostics run 30368928083 produced one Pint patch limited to tests/Feature/Identity/AccountSecurityLifecycleTest.php.
  - Exact-head static diagnostics run 30368933274 passed; Agent Governance, Synology preflight, image build, DB outage, edge-security and game-auth concurrency workflows also passed on this head.
derived:
  - The current first failure is formatting rather than PHPStan, and the exact artifact patch is the smallest safe next change before broader test diagnosis.
  - The trust boundary remains Platform Identity and web-session state; authorization must continue to derive the authenticated Identity server-side and security-sensitive mutations must revoke or rotate sessions deterministically.
unknown:
  - Whether PHPUnit and focused account-security tests pass after the formatter gate is cleared.
  - The downstream root causes of Phase 7 run 30368933723 and Portal Acceptance Contract run 30368933745 after formatting is fixed.
  - Whether the cancelled acceptance run 30368928203 will expose additional browser, localization or responsive failures on a clean exact head.
  - Production deployment state and production-only configuration remain outside repository evidence.
conflicts: []
first_failure:
  marker: pint-format-account-security-lifecycle-test
  evidence: CI run 30368933735 failed at Check formatting; artifact 8692009680 from diagnostics run 30368928083 imports EmailChangeRejected and replaces two fully-qualified exception references.
rejected_hypotheses:
  - PHPStan is still the current first failure; rejected because exact-head static diagnostics run 30368933274 passed.
  - First-slice termination should delete or unlink Canary data; rejected because the immutable binding contract and repository authorization permit no such mutation.
  - Email-code MFA should be added as a second factor; rejected by the durable ADR because email is already the recovery channel.
changed_paths:
  - app/Identity/**
  - app/Http/Controllers/Identity/**
  - app/Http/Middleware/EnsureIdentitySessionIsCurrent.php
  - app/Http/Requests/Identity/**
  - app/Mail/Identity/**
  - app/Notifications/Identity/**
  - app/Console/Commands/FinalizeIdentityTerminations.php
  - database/migrations/*identity*
  - routes/web.php
  - resources/views/identity/**
  - config/identity_security.php
  - tests/Feature/Identity/AccountSecurityLifecycleTest.php
  - tests/Unit/Identity/**
  - docs/architecture/adr/0017-account-security-lifecycle.md
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
validation:
  - command: Account Security Format Diagnostics run 30368928083
    result: PASS
    evidence: artifact 8692009680 identifies exactly one Pint-managed test file.
  - command: Account Security Static Diagnostics run 30368933274
    result: PASS
    evidence: PHPStan diagnostics completed successfully on exact head 544912954cbc3c07657182d0923042aaea26f27b.
  - command: CI run 30368933735
    result: FAIL
    evidence: Check formatting failed before static analysis and PHPUnit.
  - command: Agent Governance run 30368928038 and infrastructure/security focused workflows on exact head
    result: PASS
    evidence: governance, Synology preflight, image build, DB outage, edge-security and game-auth concurrency concluded success.
  - command: Phase 7 run 30368933723 and Portal Acceptance Contract run 30368933745
    result: FAIL
    evidence: downstream failures remain to inspect after the formatter gate is cleared.
  - command: Acceptance E2E and Visual UX run 30368928203
    result: BLOCKED
    evidence: run was cancelled before exact-head account-security acceptance evidence completed.
blockers:
  - Exact head is not merge-ready because CI currently stops at the Pint formatting gate and downstream validation remains unresolved.
next_action: Apply the exact Pint patch from artifact 8692009680 to tests/Feature/Identity/AccountSecurityLifecycleTest.php, then verify the new exact-head CI progresses past Check formatting.
```

## Boundaries

This task may disable or terminate Platform web access but cannot delete or silently transfer Canary-owned accounts or characters. Production deployment and direct production verification remain isolated to Issue #91.
