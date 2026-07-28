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
updated_at: 2026-07-28T13:45:00Z
head: 3993263a002010ac8511a1c6e9fcccfb597adc1c
branch: feat/OTERYN-20260728-account-security-lifecycle
pr: none
status: investigating
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
  - app/Http/Requests/Identity/**
  - app/Mail/Identity/**
  - app/Notifications/Identity/**
  - database/migrations/*identity*
  - resources/views/identity/**
  - resources/views/account/**
  - tests/**/Identity/**
  - scripts/acceptance/tests/*account*security*
  - docs/architecture/adr/*account-security*
  - docs/operations/*ACCOUNT*SECURITY*
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
proven:
  - Issue #276 is the highest-priority required account-security gap produced by the merged product-completeness benchmark.
  - Current main already provides registration, login/logout, password recovery/change, global session revocation, TOTP MFA with recovery codes and an immutable one-to-one Platform Identity to greenfield Canary account binding.
  - The current Accounts contract explicitly excludes account deletion and unlink or rebind unless a separate operation contract is approved.
  - Generic continuation does not authorize Canary repository writes or production verification.
derived:
  - Most adopted lifecycle data can remain Platform-owned, while exceptional binding changes must stay deny-by-default unless exact Canary ownership safety can be proven.
unknown:
  - Exact current session persistence schema and targeted-revocation extension point.
  - Exact Identity model fields, route organization, security-event schema and notification primitives.
  - Whether termination can safely finalize as Platform login disablement while preserving the immutable Canary binding and retained audit records.
conflicts: []
first_failure:
  marker: account-lifecycle-contract-gap
  evidence: the current delivered account contract revokes sessions globally but lacks user-visible session inventory, email change, privacy, termination and high-assurance recovery-key lifecycle
rejected_hypotheses:
  - Delete the bound Canary account during first-slice termination; rejected because no authorized cross-repository deletion contract exists.
  - Add email-code MFA automatically; rejected pending an explicit security decision because email is already the recovery channel.
  - Permit user-driven unlink or rebind from a browser-supplied Canary account identifier; rejected because it would undermine server-authoritative ownership.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-account-security-lifecycle.md
validation:
  - command: active-task and open-PR overlap search
    result: PASS
    evidence: no open PR matched Issue #276 or the account-security lifecycle scope before branch creation
blockers:
  - exact session and binding implementation discovery is required before runtime changes
next_action: Inspect current Identity routes, models, session storage, security events, notifications and account binding state, then record the adopted lifecycle design in an ADR before implementation.
```

## Boundaries

This task may disable or terminate Platform web access but cannot delete or silently transfer Canary-owned accounts or characters. Production deployment and direct production verification remain isolated to Issue #91.
