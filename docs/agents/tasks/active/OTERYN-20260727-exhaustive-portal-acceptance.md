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
- [ ] Manifest validation, account-lifecycle browser execution and all required repository checks pass on the fresh exact head.

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
updated_at: 2026-07-27T18:05:00+02:00
head: 1a388323de959680d0e3cf7a315c991a920da74a
branch: test/OTERYN-20260727-exhaustive-portal-acceptance-v2
pr: pending
status: validating
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
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
proven:
  - PR 246 merged the Account Center and character-presentation remediation as 9af2624e68061d52f861068976a38fe67abc4b5a
  - the earlier PR 241 implementation was conflict-isolated from current main and its non-overlapping files were transferred onto a fresh branch
  - the ledger architecture classifies delivered surfaces separately from environment proof and preserves lower-layer database and contract evidence
  - the dedicated account profile selects registration, login/logout, Account Overview, provisioning, password, MFA, sessions and character scenarios with zero retries
  - strict validation is defined but intentionally not enabled while truthful planned and partial surface gaps remain
  - no Canary, login-server, production, router or DSM write occurred
derived:
  - classification completeness can become required immediately without dishonestly claiming the planned module packages are already covered
  - the planned and partial manifest records provide the bounded successor order for Issue 240
unknown:
  - exact first validator adjustment required against current main route inventory, if any
  - exact account-lifecycle workflow result and duration on the fresh head
conflicts: []
first_failure:
  marker: none
  evidence: no fresh-branch validation failure observed yet
rejected_hypotheses:
  - the conflicted PR 241 could be merged safely without rebasing its files onto current main
  - running the full secret-sensitive suite across every browser and viewport is required for exhaustive defined-surface coverage
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
  - scripts/acceptance/tests/account-lifecycle-acceptance.spec.mjs
  - scripts/acceptance/tests/account-overview-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
validation:
  - command: transfer the PR 241 implementation onto current main without overwriting PR 246 runtime files
    result: PASS
    evidence: fresh branch is based on main 9af2624e68061d52f861068976a38fe67abc4b5a and changes only the declared acceptance/governance/documentation paths
  - command: manifest validator, account lifecycle and required workflows
    result: NOT_RUN
    evidence: pending fresh pull-request execution
blockers:
  - none
next_action: Open the fresh draft PR, inspect the first failed exact-head workflow and fix its root cause.
```

## Notes

“Exhaustive” means complete against the versioned delivered-surface contract. It does not mean proof that no unknown defect exists, and it does not replace production verification or the separately authorized Platform-to-game login bridge.
