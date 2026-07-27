---
task_id: OTERYN-20260727-exhaustive-portal-acceptance
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0008-risk-based-continuous-e2e-validation.md
  - docs/testing/E2E_COVERAGE_ROADMAP.md
  - docs/acceptance/VISUAL_UX_ACCEPTANCE_MATRIX.md
search_first:
  - active tasks and open PRs touching scripts/acceptance, acceptance workflows or testing architecture
  - existing Identity, account, MFA, password, provisioning and character browser scenarios
  - current browser-visible route inventory and module route files
optional_reads:
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/CANARY_DATA_CONTRACT.md
---

# OTERYN-20260727-exhaustive-portal-acceptance

## Goal

Establish a machine-enforced, risk-layered acceptance coverage contract for every currently delivered Oteryn Platform portal surface and state, strengthen the complete account lifecycle browser profile, and provide a standalone implementation-agent prompt for closing the remaining coverage gaps without claiming that automation can prove universal absence of defects.

## Acceptance criteria

- [x] A durable architecture decision defines the route/surface/state/role/viewport/evidence ledger and preserves ADR 0008 layering.
- [x] `TEST_STRATEGY.md` documents the exhaustive delivered-surface acceptance contract and honest evidence limits.
- [x] A versioned machine-readable portal coverage manifest classifies every browser-visible named route as covered, partial, planned or intentionally non-page.
- [x] A deterministic validator rejects malformed, stale, duplicate or unclassified manifest entries and can report strict coverage gaps.
- [x] The acceptance package exposes dedicated coverage-contract and account-lifecycle commands.
- [x] A dedicated workflow executes route classification and the complete account lifecycle with zero retries and secret-safe diagnostics.
- [x] Account lifecycle coverage explicitly includes registration, login/logout, Account Overview, provisioning states/retry, password recovery/change, MFA lifecycle, session revocation and character creation/visibility.
- [x] A human-readable matrix lists delivered surfaces, required dimensions, current evidence and remaining gaps.
- [x] A standalone agent prompt can continue from repository state and close every remaining classified gap in bounded PRs.
- [x] No staging evidence is promoted to `PRODUCTION_PROVEN`; authoritative game login and final production verification remain separate boundaries.
- [x] Manifest validation, account-lifecycle browser execution and all required repository checks pass on the fresh exact implementation head.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/archive/OTERYN-20260727-portal-completeness.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/prompts/OTERYN-EXHAUSTIVE-PORTAL-ACCEPTANCE-AGENT-PROMPT.md
  - scripts/acceptance/package.json
  - scripts/acceptance/coverage/**
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
modules:
  - Testing / Acceptance E2E
  - Identity
  - Accounts / Characters
  - Agent governance
  - Architecture
dependencies:
  - ADR 0008 risk-based continuous E2E validation
  - current exact-SHA Playwright production-like harness
  - PR 246 merged as 9af2624e68061d52f861068976a38fe67abc4b5a
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T18:45:00+02:00
head: e840b6b78de1b659ea1bb4696c4d12f1e1e7022e
branch: test/OTERYN-20260727-exhaustive-portal-acceptance-v2
pr: 247
status: ready
context_routes:
  - agent-governance
  - architecture
  - testing
  - auth-identity
  - accounts-characters
  - security
owned_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/archive/OTERYN-20260727-portal-completeness.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/prompts/OTERYN-EXHAUSTIVE-PORTAL-ACCEPTANCE-AGENT-PROMPT.md
  - scripts/acceptance/package.json
  - scripts/acceptance/coverage/**
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
proven:
  - PR 246 merged the Account Center and character-presentation remediation as 9af2624e68061d52f861068976a38fe67abc4b5a
  - every current named application route is classified once or has a bounded framework-endpoint exclusion
  - the dedicated account profile executes seven registration, login/logout, Account Overview, provisioning, password, MFA, session and character scenarios serially with zero retries
  - acceptance-only registration throttle isolation prevents cross-scenario source-limit contamination while leaving product limiters and within-scenario 429 assertions enabled
  - strict validation is defined but intentionally remains disabled while truthful planned and partial module gaps exist
  - all required PR workflows succeeded on implementation head e840b6b78de1b659ea1bb4696c4d12f1e1e7022e
  - no Canary, login-server, production, router or DSM write occurred
derived:
  - classification completeness can be required immediately without claiming that the planned module packages are already covered
  - the manifest provides the bounded successor order for Downloads, Events, Announcements, Support and Legal, Editorial Media and Wiki reconciliation
unknown:
  - exact future CI duration of each bounded successor module package
conflicts: []
first_failure:
  marker: account-profile-shared-registration-throttle
  evidence: initial parallel and then serial account-profile runs showed later scenarios receiving 429 from the shared synthetic source; the account-only helper now clears isolated acceptance cache before helper-driven registration and run 30285564647 passed all seven zero-retry scenarios
rejected_hypotheses:
  - the conflicted PR 241 could be merged safely without rebasing its files onto current main
  - running the full secret-sensitive suite across every browser and viewport is required for exhaustive defined-surface coverage
  - disabling production rate limiting is an acceptable way to stabilize account acceptance
changed_paths:
  - .github/workflows/portal-acceptance-contract.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/prompts/OTERYN-EXHAUSTIVE-PORTAL-ACCEPTANCE-AGENT-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/tasks/archive/OTERYN-20260727-portal-completeness.md
  - docs/agents/tasks/active/OTERYN-20260727-portal-completeness.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/coverage/validate-portal-coverage.mjs
  - scripts/acceptance/package.json
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
validation:
  - command: Portal Acceptance Contract run 30285564647
    result: PASS
    evidence: route classification and all seven zero-retry account-lifecycle scenarios succeeded on e840b6b78de1b659ea1bb4696c4d12f1e1e7022e
  - command: CI run 30285564271
    result: PASS
    evidence: formatting, static analysis and complete PHPUnit suite succeeded on the same implementation head
  - command: Acceptance E2E and Visual UX run 30285564266
    result: PASS
    evidence: required primary, portability, responsive, resilience, accessibility and visual acceptance succeeded
  - command: Phase 7, governance, outage, concurrency, edge and Synology preflight workflows
    result: PASS
    evidence: runs 30285564402, 30285564255, 30285564665, 30285564516, 30285564524 and 30285564263 succeeded
blockers:
  - none
next_action: Mark PR 247 ready and squash-merge after the docs-only checkpoint commit receives required checks.
```

## Notes

“Exhaustive” means complete against the versioned delivered-surface contract. It does not mean proof that no unknown defect exists, and it does not replace production verification or the separately authorized Platform-to-game login bridge.
