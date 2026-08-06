# Oteryn Platform game-auth topology current-state review

## Review identity

```yaml
review_id: OPA-ARCH-20260806-001
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
reviewed_main: d12a4f4a14db0319a8563cb16b1d92a7b1e117b8
reviewed_at: 2026-08-06T10:13:00Z
classification: documentation_drift
severity: high
confidence: high
runtime_implementation_authorized: false
action_issue: 720
```

## Scope

This rotation reviewed the current game-authentication system context, operation-specific contracts, module ownership and live repository delivery evidence. It did not inspect or change production networking, secrets, deployed component identities, external repositories or active native-protocol implementation PR #542.

Primary sources:

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`;
- `docs/architecture/SYSTEM_ARCHITECTURE.md`;
- `docs/architecture/MODULE_CATALOG.md`;
- `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md`;
- `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`;
- `docs/contracts/GAME_SESSION_CANARY_CONTRACT.md`;
- `docs/contracts/WORLD_REGISTRY_CONTRACT.md`;
- `services/game-gateway/**` on reviewed `main`;
- archived task `OTERYN-20260722-game-gateway-mvp` and merged PR #122 evidence;
- open native-protocol PR #542 path inventory.

## Current state

### PROVEN

1. `services/game-gateway/` exists on reviewed `main` as a separately buildable Go service with a dedicated container definition and internal packages.
2. The completed Phase 4 task and merged PR #122 record a delivered Gateway login flow that:
   - redeems one-time Game Login Tickets through the private Platform Identity boundary;
   - obtains authoritative account/world/character login context through Platform;
   - uses Platform World Registry behavior and account-scoped Canary reads;
   - invokes a Game Session issuer abstraction;
   - fails closed on ambiguous world state and dependency failures.
3. `GAME_SESSION_CANARY_CONTRACT.md` declares the legacy-compatible Game Session contract v1 implemented, bounded-E2E proven and production-activation gated. It separately classifies native contract v2 as a disabled-by-default producer without an Otheryn native consumer claim.
4. `GAME_GATEWAY_IDENTITY_CONTRACT.md` still declares `TARGET CONTRACT — NOT YET IMPLEMENTED`.
5. `AUTH_GAME_LOGIN_CONTRACT.md` still states that Platform is not in the game-authentication path and labels the Identity/Gateway authorization flow as future design direction.
6. `SYSTEM_ARCHITECTURE.md` does not show the separately deployable Game Gateway, private ticket redeem/login-context APIs, World Registry or Game Session issuer in its current system-context diagram.
7. `MODULE_CATALOG.md` describes the login bridge as future work although the bounded Gateway/Identity bridge is merged.
8. Open PR #542 does not modify `GAME_GATEWAY_IDENTITY_CONTRACT.md`, `AUTH_GAME_LOGIN_CONTRACT.md`, `SYSTEM_ARCHITECTURE.md` or `MODULE_CATALOG.md`; it owns a separate native-protocol producer migration and does modify `GAME_SESSION_CANARY_CONTRACT.md` and `WORLD_REGISTRY_CONTRACT.md`.

### DERIVED

- The repository has a delivered Oteryn game-authentication path at source/contract-test level, but the canonical current-state narrative still describes a pre-Gateway architecture.
- The mismatch can misroute agents into duplicating the Gateway/Identity boundary or treating a delivered security boundary as absent.
- Correcting delivery status does not authorize global cutover: direct legacy/native password paths, deployed network exposure, exact service identity, production rollout and native-v2 cross-repository completion remain separate facts.

### UNKNOWN

- Exact currently deployed Gateway, Platform, Canary and client revisions.
- Whether all alternate public password/login-server paths are disabled or network-isolated in the privately operated environment.
- Exact production service-identity transport, secret rotation and private ingress state.
- Production activation and rollback proof for the complete game-authentication topology.
- Final native-v2 producer/consumer/cross-repository E2E outcome of PR #542 and its coordinated repositories.

### CONFLICT

The following accepted/current sources disagree:

| Concern | Source A | Source B | Result |
|---|---|---|---|
| Gateway-to-Identity implementation status | `GAME_GATEWAY_IDENTITY_CONTRACT.md`: not implemented | merged Gateway/Identity source, archived Phase 4 task and PR #122 | documentation drift |
| Platform position in game auth | `AUTH_GAME_LOGIN_CONTRACT.md`: Platform not authoritative/in path | delivered ticket redeem, login context and Gateway orchestration | historical baseline presented as current |
| System topology | `SYSTEM_ARCHITECTURE.md`: web app plus generic login-server path | current repository contains separate Gateway and explicit private contracts | incomplete canonical context |
| Integration ownership | `MODULE_CATALOG.md`: login bridge future | bounded bridge merged | stale module wording |

## Problem and impact

The architecture authority ranks operation-specific contracts and focused canonical architecture documents above programme/task prose. Their stale status therefore has higher routing weight than the merged evidence that disproves it.

Concrete risks:

- duplicate implementation or incompatible redesign of the existing Gateway/Identity boundary;
- incorrect ownership and dependency assumptions during native-protocol work;
- confusion between legacy-compatible Game Session v1, disabled native-v2 producer work and production cutover;
- unsafe security reasoning that either ignores a delivered boundary or overstates production enforcement;
- inaccurate module and topology diagrams used by future agents, audits and rollout planning.

## Invariants

Any correction must preserve all of the following:

1. Repository delivery, cross-repository consumer completion, environment proof and activation authority remain separate evidence dimensions.
2. The Gateway never receives reusable user credentials or direct Platform/Canary database authority.
3. Ticket redeem remains service-authenticated, short-lived, single-use and fail-closed.
4. Legacy-compatible Game Session v1 and native Game Session/protocol v2 remain explicitly separate contracts.
5. Native v2 remains disabled by default until its producer, Otheryn/Rust consumers and cross-repository rollback E2E are complete.
6. No document may claim that alternate password/login-server paths are closed without exact deployment/network evidence.
7. `PRODUCTION_PROVEN=false` remains explicit until exact environment identity, ingress, service credentials, restore/rollback and real E2E are verified.

## Options

### Option A — retain current documents unchanged

Advantages:

- no immediate documentation work;
- avoids touching large historical discovery contracts.

Disadvantages:

- preserves a proven high-authority contradiction;
- keeps agent routing and security analysis unsafe;
- increases the chance of duplicate or incompatible work.

Verdict: rejected.

### Option B — rewrite all game-authentication documents into one new monolithic contract

Advantages:

- one consolidated narrative;
- can remove repeated historical material.

Disadvantages:

- broad, high-risk rewrite;
- risks losing pinned legacy-path evidence and operation-specific semantics;
- creates unnecessary review and compatibility cost while PR #542 is active.

Verdict: rejected.

### Option C — narrow current-state reconciliation in existing canonical owners

Actions:

- change `GAME_GATEWAY_IDENTITY_CONTRACT.md` status and add exact delivered-component evidence plus retained deployment unknowns;
- add a prominent current-state overlay to `AUTH_GAME_LOGIN_CONTRACT.md`, explicitly classifying its old path inventory as a pinned historical baseline and routing active Gateway/Game Session truth to current contracts;
- update the system-context diagram and game-auth boundary text in `SYSTEM_ARCHITECTURE.md`;
- replace the stale `MODULE_CATALOG.md` Integration wording;
- leave PR #542-owned contract paths unchanged until that owner is terminal or explicitly coordinates.

Advantages:

- smallest correction that restores authority coherence;
- preserves historical evidence;
- cleanly separates delivered v1, active native-v2 work and production activation;
- no runtime or workflow mutation.

Disadvantages:

- several canonical files require synchronized review;
- historical contracts remain long and need careful state labels.

Verdict: recommended with high confidence.

## Recommendation

Adopt Option C through Issue #720 as one documentation-only correction package. No ADR is required because this package does not choose a new architecture; it reconciles canonical current-state claims with already accepted contracts and merged implementation evidence.

## Required handoff

```yaml
owner_programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
implementation_issue: 720
recommended_task: OTERYN-20260806-game-auth-topology-canonical-reconciliation
allowed_paths:
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - architecture programme/task lifecycle records
forbidden_paths:
  - app/**
  - services/**
  - database/**
  - routes/**
  - tests/**
  - .github/workflows/**
  - active PR #542 changed paths
  - production and staging systems
  - external repositories
```

## Acceptance and validation expectations

- Every implementation-status statement is tied to exact merged repository evidence.
- Historical legacy-path evidence is retained but cannot present itself as the current complete topology.
- Gateway v1, Game Session v1, native-v2 producer and production activation are separately labelled.
- Current module and system-context wording agrees with operation-specific contracts.
- Contract/document links and status labels pass deterministic documentation/governance checks.
- Exact-head CI passes with zero unresolved review threads.
- Runtime E2E: `NOT_APPLICABLE` because the handoff changes documentation only.

## Review outcome

```yaml
status: finding_confirmed
material_findings: 1
actionable_issue: 720
owner_decision_required: false
runtime_change_required: false
architecture_decision_backlog_change: none
next_action: execute the bounded canonical documentation reconciliation after confirming no new path overlap with active PR #542
```
