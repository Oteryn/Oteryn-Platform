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

- [ ] A durable architecture decision defines the route/surface/state/role/viewport/evidence ledger and preserves ADR 0008 layering.
- [ ] `TEST_STRATEGY.md` documents the exhaustive delivered-surface acceptance contract and honest evidence limits.
- [ ] A versioned machine-readable portal coverage manifest classifies every browser-visible named route as covered, partial, planned or intentionally non-page.
- [ ] A deterministic validator rejects malformed, stale, duplicate or unclassified manifest entries and can report strict coverage gaps.
- [ ] The acceptance package exposes dedicated coverage-contract and account-lifecycle commands.
- [ ] Critical acceptance executes the complete account lifecycle profile with zero retries and secret-safe diagnostics.
- [ ] Account lifecycle coverage explicitly includes registration, login/logout, Account Overview, provisioning states/retry, password recovery/change, MFA lifecycle, session revocation and character creation/visibility.
- [ ] A human-readable matrix lists delivered surfaces, required dimensions, current evidence and remaining gaps.
- [ ] A standalone agent prompt can continue from repository state and close every remaining classified gap in bounded PRs.
- [ ] No staging evidence is promoted to `PRODUCTION_PROVEN`; authoritative game login and final production verification remain separate boundaries.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/ACTIVE_WORK.md
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
  - .github/workflows/acceptance-validation.yml
modules:
  - Testing / Acceptance E2E
  - Identity
  - Accounts / Characters
  - Agent governance
  - Architecture

dependencies:
  - ADR 0008 risk-based continuous E2E validation
  - current exact-SHA Playwright production-like harness
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T14:20:00Z
head: f5aeb2e80d4692b3ee6309cc3454aa20697721f2
branch: test/OTERYN-20260727-exhaustive-portal-acceptance
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
  - auth-identity
  - accounts-characters
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/architecture/adr/0015-machine-enforced-portal-acceptance-ledger.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/agents/prompts/OTERYN-EXHAUSTIVE-PORTAL-ACCEPTANCE-AGENT-PROMPT.md
  - scripts/acceptance/package.json
  - scripts/acceptance/coverage/**
  - scripts/acceptance/tests/*account*.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/password-recovery-acceptance.spec.mjs
  - scripts/acceptance/tests/password-change-acceptance.spec.mjs
  - scripts/acceptance/tests/mfa-security-acceptance.spec.mjs
  - .github/workflows/acceptance-validation.yml
proven:
  - Current main is f5aeb2e80d4692b3ee6309cc3454aa20697721f2.
  - ACTIVE_WORK records no active repository task.
  - Open PR 218 is operational and does not own acceptance architecture or portal test paths.
  - Existing browser evidence covers registration, login/logout, MFA, password recovery/change, Account Overview provisioning states, retry, character creation and public visibility across several separate specs.
  - The existing strategy intentionally uses lower deterministic layers for concurrency, locking and data-integrity invariants.
derived:
  - A machine-readable coverage ledger can prevent new portal routes or required states from remaining silently unclassified without forcing every invariant into Playwright.
  - The complete account lifecycle should be executable as one dedicated zero-retry critical profile while reusing existing secret-safe specs.
unknown:
  - Exact final CI duration and any route-list normalization adjustment required by the first workflow run.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Running the full secret-sensitive suite across every browser and viewport is not required for exhaustive defined-surface coverage and would conflict with ADR 0008.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260727-exhaustive-portal-acceptance.md
validation:
  - command: repository inspection through GitHub connector
    result: PASS
    evidence: current main, active work, route files, strategy and existing acceptance specs inspected
blockers:
  - none
next_action: Publish the architecture decision, coverage ledger/validator, account-lifecycle profile and implementation-agent prompt, then open a draft PR.
```

## Notes

“Exhaustive” means complete against the versioned delivered-surface contract. It does not mean proof that no unknown defect exists, and it does not replace production verification or the separately authorized Platform-to-game login bridge.
