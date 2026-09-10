# Oteryn Platform — repository audit continuation ledger

**Status:** `INCOMPLETE`  
**Date:** 2026-09-06  
**Repository:** `Oteryn/Oteryn-Platform`  
**Governing programme:** Issue #451  
**Existing audit PR:** #1294  
**Original audit baseline:** `3b2ea1c7392187d5d22488673073dc8f8305a374`  
**Live `main` observed before this persistence step:** `294e18909b8319695021011ccbeb1386cac32ced`

This file persists work performed after the first audit report in `docs/testing/OTERYN_PLATFORM_REPOSITORY_AUDIT_2026-09-06.md`. It is intentionally a continuation ledger, not a replacement final report.

The owner-requested completion contract has **not** been satisfied. In particular, current-`main` tracked-file reconciliation, all active workflows, all build/test systems, full instruction-debt coverage and all accessible governance surfaces have not yet been completely reconciled. Nothing in this file should be interpreted as a claim of 100% repository coverage or production readiness.

## 1. Evidence-generation warning

The audit ran while `main` was advancing. Evidence therefore belongs to explicit generations:

- most direct production-code reads summarized below were taken from the original audit generation around `3b2ea1c7392187d5d22488673073dc8f8305a374`;
- the existing audit branch before this continuation was `docs/20260906-comprehensive-platform-audit@37484a4de9c611b999285c843f6d45c62a3320b2`;
- current repository instructions and live `main` were re-read at `294e18909b8319695021011ccbeb1386cac32ced` before persisting this ledger;
- findings from the older audit generation are not silently promoted to current-main facts without revalidation.

**UNKNOWN.** A fresh full tracked-file inventory and per-path coverage disposition for `294e18909b8319695021011ccbeb1386cac32ced` has not yet been completed.

## 2. Additional direct coverage completed

The continuation directly inspected the following authored-code batches in addition to the first report's workflow/Gateway/build/security reads.

### Accounts, Canary integration, profiles and character creation

**FACT.** A 31-file batch was read covering:

- `app/Accounts/**`;
- `app/CanaryIntegration/**` relevant to account provisioning, character creation, character transfer, database privilege verification and runtime Redis reads;
- `app/CharacterProfiles/**`;
- `app/Characters/**`.

Important verified properties in that generation:

- Canary read, account-provisioning, character-create and character-transfer operations use distinct connection/principal boundaries;
- dedicated privilege verifiers parse effective grants and reject broader-than-approved privilege shapes;
- character creation uses account-row locking, active-character quota enforcement, duplicate-name recovery and bounded transaction retries;
- character transfer locks participating account rows in deterministic numeric order before locking the player row, checks active/deleted/session state, and supports idempotent already-transferred recovery.

### Routes

**FACT.** All 21 route files present in the inspected generation were read directly, including:

- `routes/web.php`;
- `routes/api.php`;
- `routes/internal.php`;
- `routes/console.php`;
- `routes/localization.php`;
- every `routes/modules/*.php` file in that generation.

Observed state-changing account/admin/security routes generally combine framework auth, scoped permission/MFA middleware where privileged, CSRF through the web stack, and bounded rate limiters for abuse-prone endpoints. This is evidence for the inspected generation only, not a blanket proof for every current-main route.

### Identity, admin and security foundation

**FACT.** A 61-file identity/admin/security batch was read directly. It covered, among other surfaces:

- administrator authorization, role management and audit recording;
- session generation and session registry invalidation;
- password change/reset and game-authorization revocation;
- MFA enrollment, TOTP replay protection, recovery-code consumption and MFA disable/reset;
- email-change confirmation/recovery lifecycle;
- identity recovery keys;
- account termination scheduling/finalization;
- security headers, proxy trust, request correlation and service-credential middleware;
- production configuration verification and rate-limiter configuration.

Verified strengths include:

- login uses a non-secret dummy Argon2id hash for nonexistent accounts to reduce account-enumeration timing differences;
- MFA TOTP consumption uses newer-timestep verification and recovery codes are one-time hashed values;
- sensitive credential changes revoke web and game authorization generations;
- gateway service credentials are compared by SHA-256 using `hash_equals` and support bounded current/previous rotation hashes;
- wildcard trusted proxies are explicitly rejected;
- production configuration verification fails closed on debug/HTTPS/cookie/mail and enabled marketplace/payment invariants.

### HTTP controllers and requests

**FACT.** A separate 64-file controller/request batch was read directly across admin, identity, character profiles, characters, downloads, Game Auth, public game data, public portal, Player Companion and selected support surfaces.

Observed patterns include object-level identity scoping in account-owned views, explicit request validation, permission middleware on privileged routes and `no-store` handling for several secret-bearing/security-sensitive response surfaces.

### Game authentication

**FACT.** All 27 files in the inspected `app/GameAuth/**` batch were read directly.

The inspected design is:

1. Laravel Passport public OAuth client;
2. PKCE S256 enforcement for public authorization clients;
3. generation binding between identity security state and auth/access tokens;
4. short-lived one-time game login ticket;
5. internal ticket redemption behind a dedicated service credential;
6. login-context projection from Platform world policy plus read-only Canary character data.

One-time ticket redemption is transactional and checks ticket hash, audience, expiry, unused state, current identity generation and current account binding before marking the ticket used.

### Marketplace, payments and wallet

**FACT.** A 49-file Marketplace/Payments/Wallet production batch was read directly.

Verified design characteristics include:

- Bazaar character movement is a recoverable cross-database saga rather than an assumed distributed ACID transaction;
- wallet mutations are lock-based and ledger-backed with idempotency keys;
- bidding releases the previous reservation before reserving the new leading amount inside one Platform DB transaction;
- settlement and return paths have explicit recovery states;
- deterministic test payments are disabled in production;
- payment webhooks use timestamp-bounded HMAC verification and bounded payload sizes;
- provider events persist authenticated facts before state transitions;
- payment-state mismatches, ambiguous provider references and inconsistent refund truth route to reconciliation rather than fabricated success.

### Selected concurrency and integration tests

**FACT.** The following Marketplace tests were read directly during continuation:

- `tests/Feature/Marketplace/CanaryCharacterTransferConcurrencyMariaDbTest.php`;
- `tests/Feature/Marketplace/CanaryCharacterTransferMariaDbIntegrationTest.php`;
- `tests/Feature/Marketplace/MarketplaceAuctionTerminalRecoveryConcurrencyTest.php`;
- `tests/Feature/Marketplace/MarketplaceIdempotencyTest.php`;
- `tests/Feature/Marketplace/MarketplaceModuleTest.php`;
- `tests/Feature/Marketplace/MarketplaceSettlementRecoveryTest.php`.

They cover meaningful ownership races, MariaDB privilege boundaries, idempotency, stale-worker terminal-state regression, escrow recovery and wallet settlement behavior.

**FACT.** The MariaDB integration/concurrency tests skip when their required integration environment variables are absent. Therefore a green general PHP run is not by itself proof that those real-MariaDB paths executed.

## 3. Repository-size evidence captured during the audit session

The audit session captured the following scope metrics while inventorying the repository:

| Scope | Files | Lines |
|---|---:|---:|
| `app/` | 488 | 38,977 |
| `tests/` | 168 | 29,962 |
| `scripts/` | 164 | 27,618 |
| `tools/` | 83 | 21,647 |
| `resources/` | 141 | 9,431 |
| `docs/agents/tasks/archive/` | 354 | 39,455 |
| `docs/agents/prompts/` | 22 | 4,955 |
| `.github/workflows/` | 55 | 11,747 |
| `docs/architecture/` | 88 | 21,154 |
| `docs/contracts/` | 39 | 11,151 |

**FACT.** Fifty migration files were inventoried in the inspected generation.

**UNKNOWN.** These metrics have not yet been replayed and reconciled against live `main@294e18909b8319695021011ccbeb1386cac32ced`; they are preserved as audit-session evidence, not current-main totals.

## 4. Additional finding

### F15 — P2 — recovery-key password path lacks the explicit upper bound used by other password flows

**Domain:** Security / reliability  
**Classification:** FACT + INFERENCE

**FACT.** In the inspected generation:

- `app/Identity/Support/IdentityPasswordPolicy.php` returns `Password::min(12)->letters()->mixedCase()->numbers()->symbols()` and does not configure a maximum;
- `app/Http/Requests/Identity/RecoverWithIdentityKeyRequest.php` validates `password` with `required`, `confirmed` and `IdentityPasswordPolicy::rule()`, but does not add the explicit `max:1024` used by registration, reset and password-change request validators;
- the inspected Laravel 13 `Illuminate\Validation\Rules\Password` implementation emits a `max:` rule only when the rule's own maximum is configured.

**INFERENCE — medium confidence.** An authenticated recovery-key attempt can therefore reach password-policy validation/hashing with a materially larger password than sibling password flows, subject to upstream HTTP/PHP body limits not yet audited for this exact path. This is a resource-consumption inconsistency and potential abuse surface, not a proven denial-of-service vulnerability.

**RECOMMENDATION.** Centralize the maximum password length in `IdentityPasswordPolicy` or apply the same explicit bound to the recovery-key request, then add a boundary regression test. Verify upstream request-size controls before assigning a higher severity.

## 5. Architecture assessment after the broader direct read

**FACT.** The dominant architecture remains a modular Laravel monolith with a separate Go Game Gateway and explicit integration boundaries to Canary/runtime services.

**INFERENCE — high confidence for the inspected generation.** This remains appropriate for the current product shape. The code already contains difficult distributed-state problems at the game/Platform boundary; splitting ordinary Platform domains into more independently deployed services would add failure modes without an identified scaling or ownership requirement.

Particularly strong architecture choices observed:

- security-sensitive identity behavior is concentrated in dedicated domain/service classes rather than controllers;
- Canary privileges are split by operation and mechanically verified;
- cross-system character transfer uses recovery state and idempotency instead of pretending Platform and Canary share one transaction;
- payment truth is provider-authenticated and reconciled before state is accepted;
- Game Auth separates browser/native OAuth bootstrap, one-time authorization and internal service redemption.

Debt/complexity that still deserves scrutiny:

- the Marketplace/Wallet/Canary transfer saga is necessarily complex and requires continued concurrency/recovery testing;
- `WalletMutator` still exposes Marketplace-specific exception coupling noted as F10 in the first report;
- several configuration/policy constants exist in code and config and should be checked systematically for drift;
- current architecture/documentation authority still needs full cross-check against implementation and current open work.

## 6. Test, build and CI status

**FACT.** The repository contains a substantial test system, including unit, feature, real-MariaDB integration/concurrency, browser/E2E and workflow-specific validation surfaces.

**UNKNOWN.** The continuation did not execute a complete current-head PHP suite, Go suite, Playwright matrix, Docker build or migration/rollback matrix.

**UNKNOWN.** Earlier audit evidence directly reviewed only a subset of the 55 workflow files. Therefore the CI domain remains incomplete until every active/reusable workflow is inventoried and assigned an audit disposition.

**RECOMMENDATION.** Final audit verification should use current-main exact-head, risk-routed execution rather than rerunning every heavy suite after every documentation checkpoint. Real MariaDB and browser paths must be reported separately from suites that can skip them.

## 7. Current instruction/governance re-read

At persistence time the following current-main instruction sources were re-read at `294e18909b8319695021011ccbeb1386cac32ced`:

- `AGENTS.md`;
- `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`;
- `docs/agents/CONTEXT_ROUTING.md`;
- `docs/agents/AGENTS.md`;
- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
- `docs/agents/EXECUTION_PROTOCOL.md`;
- `docs/agents/PROJECT_LANES.json`;
- `docs/agents/GOVERNANCE_CONTRACT.json`;
- `docs/agents/ACTIVE_WORK.md`;
- `docs/agents/TERMINAL_ONLY_COMMUNICATION.md`;
- `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`.

**FACT.** Current governance explicitly implements bounded instruction loading and task-routed context rather than declaring every repository document always-loaded.

**INFERENCE.** This is directionally better for context/token efficiency than recursive broad loading, but the dedicated instruction-debt audit remains incomplete. The presence of bounded routing does not by itself prove that all duplicated, stale or model-specific instructions have been removed.

## 8. Working domain ledger

`PARTIAL` below is intentionally a working-only state. Before a final report satisfying the owner's prompt, every applicable domain must be reconciled to its required final status and evidence.

| Domain | Working status | Evidence / remaining gap |
|---|---|---|
| A Purpose/product intent | PARTIAL | Programme #451 and architecture/product records observed; current implementation-to-goal reconciliation not complete. |
| B System architecture | PARTIAL | Major runtime/domain boundaries directly reconstructed; all directories and current docs not yet reconciled. |
| C Implementation quality | PARTIAL | Large authored-code batches reviewed; whole authored tree not yet covered. |
| D Interfaces/contracts | PARTIAL | Game Auth, Canary DB privileges and selected catalog/payment contracts inspected; complete contract inventory pending. |
| E Data/persistence | PARTIAL | 50 migrations inventoried; complete semantic migration/index/rollback audit pending. |
| F Dependencies/supply chain | PARTIAL | Composer/framework and image-build evidence exists; complete manifest/licensing/vulnerability pass pending current authoritative data. |
| G Security/privacy | PARTIAL | Identity/admin/Game Auth/payment boundaries substantially reviewed; complete route/file/workflow/privacy coverage pending. |
| H Reliability/resilience | PARTIAL | Saga/idempotency/concurrency/recovery paths inspected; execution evidence incomplete. |
| I Performance/resource cost | PARTIAL | CI/build architecture evidence exists; no comprehensive measured runtime/resource benchmark. |
| J Test architecture | PARTIAL | Major test families inventoried and selected tests read; all 168 files and skip/execution matrix not reconciled. |
| K Build system | PARTIAL | Docker/build workflows partly reviewed; full current-head build graph and actual builds pending. |
| L CI/verification | PARTIAL | Only a subset of 55 workflows fully audited in prior pass. |
| M Release/deploy/rollback | PARTIAL | Staging/release evidence reviewed in first pass; fresh end-to-end release/rollback drill pending. |
| N Configuration/infrastructure | PARTIAL | Core config and Synology surfaces partly covered; whole infra/environment drift pass pending. |
| O Observability/operations | PARTIAL | request correlation/config verifiers observed; complete alerts/metrics/restore operations pass pending. |
| P Documentation/knowledge | PARTIAL | high-level/current authority read; large docs corpus not fully reconciled for staleness/duplication. |
| Q Agent instructions/prompts | PARTIAL | current bootstrap/routing sources re-read; full prompts/archive/skills/instruction-debt ledger pending. |
| R Developer experience | PARTIAL | startup/docs findings exist; clean-checkout reproduction pending. |
| S Repository governance | PARTIAL | live PR/branch state accessible; full branch protections/rulesets/Merge Queue remains partly permission-limited/unreconciled. |
| T Current work/drift | PARTIAL | programme #451, PR #1294 and current main drift observed; full open-work overlap reconciliation pending. |
| U Compatibility/portability | PARTIAL | PHP/Go/runtime contracts observed; complete OS/browser/runtime matrix pending. |
| V User-facing quality | PARTIAL | existing browser evidence known; exhaustive current UI/state/a11y/responsive audit pending. |
| W Simplification | PARTIAL | several opportunities identified; final ranked deduplicated simplification pass pending. |

## 9. Exact missing work before a truthful final audit

The audit must continue until at least the following are closed or explicitly `UNVERIFIED` with access reason:

1. Rebuild the complete tracked-file inventory on one fixed current-main SHA and assign every path `DIRECT`, `GROUPED`, `N/A` or `UNVERIFIED`.
2. Reconcile every `.github/workflows/**` active/reusable workflow, including trigger, permissions, Merge Queue, dependency/fan-in and cost semantics.
3. Finish direct authored-code coverage outside the batches already listed, including remaining modules, scripts, tools and frontend code.
4. Semantically audit all migrations/schema/index/rollback behavior rather than inventorying filenames only.
5. Finish test architecture coverage and distinguish tests that actually executed from tests that skipped because external services were absent.
6. Execute the smallest meaningful current-head PHP, Go, static-analysis and build validations needed to confirm material findings.
7. Complete current Docker/build/release/deploy/rollback assessment, including reproducibility and artifact content/provenance.
8. Complete documentation and instruction-debt inventory, including prompts, archives, nested instructions and any accessible skills/hooks.
9. Re-read live rulesets/required checks/Merge Queue/environment protection surfaces to the extent permitted by the connected GitHub identity.
10. Run a separate cross-check pass that searches for unassigned directories, hidden entry points, duplicated config/build paths and claims lacking evidence.
11. Produce the final A–W ledger and tracked-file totals required by the owner prompt.
12. Verify that the audit itself left no unintended tracked mutation other than the explicitly authorized documentation/task persistence branch.

## 10. Persistence state

This continuation is documentation only. It records audit evidence and gaps; it does not repair product code, change CI, alter repository protection, touch production, or claim the audit is complete.

**AUDIT_STATUS: `INCOMPLETE`.**
