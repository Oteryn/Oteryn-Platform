# Oteryn Portal Architecture and Product Review — 2026-08-10

## Evidence status

Live-state refresh on 2026-08-10 verified protected `main` at `dc9adc7d9246e83c7299d8cf9c161524fb85b2c9`. This document is a dated evidence snapshot; Issues, Pull Requests, task ownership and CI must still be revalidated before future mutation.

Scope: `blakinio/Oteryn-Platform` only. It does not authorize or claim inspection/mutation of Oteryn-v2, Canary or another server/game repository.

Classification used below:

- `PROVEN` — directly supported by current repository/live GitHub evidence;
- `DERIVED` — conclusion from proven facts;
- `UNKNOWN` — evidence is insufficient or environment-specific proof is absent;
- `CONFLICT` — durable sources disagree and require reconciliation.

## Executive verdict

| Statement | Class | Verdict |
|---|---|---|
| Laravel modular-monolith foundation is appropriate | `PROVEN` | Retain. No SPA/microservice/framework rewrite is justified. |
| Repository contains substantial real portal capability | `PROVEN` | Identity/accounts, public portal, CMS/Wiki, GameCatalog, support/moderation, admin/audit, Wallet/Marketplace foundations exist. |
| Portal is globally product-complete | `PROVEN` | No. Canonical portal architecture explicitly rejects that claim. |
| Portal is publicly production-proven | `UNKNOWN` | Repository/staging evidence does not prove the full live edge, restore/rollback, observability and controlled production smoke. |
| Current routing/source-of-truth set is fully synchronized | `CONFLICT` | `ACTIVE_WORK.md` says no active tasks while live `docs/agents/tasks/active/**` contains two blocked records; `PROJECT_STATE.md` is also historically stale in places. |
| Player utility matches mature Tibia/OTS tooling | `DERIVED` | Not yet; Today/LiveOps, native Character Portfolio, Player Companion, World Hub and complete federated knowledge/search remain incomplete/planned. |

**Conclusion:** finish the existing foundation through bounded, risk-first vertical slices. Rebuilding the platform would increase delivery risk without resolving the known gaps.

## What is already good

### Architecture and technology

- Laravel 13 / PHP 8.5 modular monolith is proportionate to the project and current operational shape.
- Blade/server-rendered UI is a maintainable baseline; no evidence justifies replacing it with a separate SPA.
- Module boundaries and application/domain/infrastructure patterns are sufficient to evolve without a repository-wide rewrite.
- Service extraction should require measured independent scaling, failure-isolation, lifecycle or ownership pressure.

### Ownership and trust boundaries

- Platform-owned and game-owned state are intentionally separated.
- Browser-to-game-database shortcuts are rejected.
- Shared writes are expected to use operation-specific contracts rather than broad table access.
- Accepted architecture keeps `PublicPortal` as composition, not source truth.
- Accounts is the authenticated Character Portfolio composition owner while canonical character identity/lifecycle remains game authority under the accepted ADR boundary.

### Security and delivery discipline

The repository has strong foundations around deny-by-default authorization, MFA/session controls, secure password handling, rate limiting, auditability, upload boundaries, durable task ownership, exact-head self-review/CI, applicable real E2E, PR reconciliation and post-merge archival. Production, payment, protected-environment and external-repository authority are deliberately separate from ordinary repository implementation authority.

### Existing product surface

Current code/documentation recognizes substantial capability across:

- homepage, localized navigation, SEO, news, announcements and events;
- downloads and legal/public content;
- characters, guilds, highscores, online/status and deaths;
- registration/login/recovery/MFA/session/account management;
- CMS, Wiki and editorial media;
- support, reports, moderation, admin and audit;
- versioned Game Catalog foundations;
- Wallet and Character Bazaar foundations.

Module/route availability is not the same as complete content, environment proof or public launch.

## Material gaps

### P0 — source-of-truth and lifecycle drift

Live refresh proves:

- `ACTIVE_WORK.md` says no active work, but two task records exist under `docs/agents/tasks/active/**`;
- the active public-domain repair is blocked on external Cloudflare/token/evidence work;
- the active native-auth production-verification record is blocked on later native/runtime/environment gates;
- open PRs are currently only #541 and #338;
- no existing `PORTAL-CLOSEOUT`/portal-completion PR existed before this package.

This drift can misroute agents, obscure ownership and create duplicate work. The portal programme must always prefer live task/PR/Issue state over historical summary prose.

### P1 — proven high-risk findings

Live Issues remain open and implementation-authorized:

1. **#948 — Download Center mutable artifact reference** (`PROVEN`, high/P1): an allowlisted HTTPS non-root path is not sufficient proof of artifact-reference immutability. Repair must enforce a machine-testable immutable-reference invariant without falsely claiming independent checksum verification.
2. **#944 — game-consumed entitlement stale authority** (`PROVEN`, high/P1): Profile-B entitlement evidence lacks a finite stale/offline authority bound. Before future Premium/VIP game enforcement, contract semantics must define finite validity, revision fencing, expiry/revoke precedence and outage/reconnect behaviour.
3. **#941 — personalized Today private-cache isolation** (`PROVEN`, high/P1): any future Today representation containing owner-private PlayerCompanion data needs explicit private/no-shared-cache semantics, guest/authenticated separation and cross-user transition tests.

These should stay in the existing Issue-owned `OTERYN_PLATFORM_REMEDIATION` workflow rather than receiving duplicate ownership from the completion programme.

### Production and public edge

Issue #490 remains evidence that Platform API applicability, operations/observability and public-edge boundaries still need explicit current contracts/evidence. Repository/staging tests do not establish production DNS/TLS/WAF/origin/private-ingress topology, effective database/cache/mail configuration, backup/restore, alerts/on-call or controlled production smoke. Production authority remains separate.

### Character/account completion

Product-completeness work still needs current live verification for delete/restore, rename, world transfer, native Character Portfolio and migration away from permanent Canary numeric identities. Server-owned implementation/contract inspection remains outside this Platform-only package unless separately authorized.

### Player-value differentiation

The next major value is integrated utility, not more static pages:

1. `Today` / command-centre composition;
2. authoritative LiveOps projections and history;
3. native Character Portfolio/account context;
4. federated public content search;
5. complete versioned Wiki/GameCatalog inventories;
6. Player Companion vertical slices;
7. World Hub when real multi-world/history inputs justify it.

For Player Companion, useful first-party candidates include loot/session analysis, hunt guidance, equipment/build planning, quest/access tracking and training/EXP calculators. Implement only against authoritative/versioned facts with private-by-default personal data.

## Open PR disposition at live refresh

Only these PRs are open at the persisted refresh; classifications remain provisional until a future executor rechecks full diff, reviews and CI:

| PR | Provisional disposition | Basis |
|---|---|---|
| #541 public-domain checkpoint | `SUPERSEDED` candidate / `NEEDS_DECISION` until revalidated | Draft documentation reconciliation is based on old state. Close only after proving later merged state contains everything needed and preserving unique sanitized environment evidence. |
| #338 NPC shop schema 1.3 consumer | `REBASE/SPLIT` | Valuable direction but old mixed branch/base and producer dependency require a clean current-main reconciliation. Server-side producer inspection is outside Platform-only authority. |

Historical note: dependency-update PRs #952–#958 were part of an earlier 2026-08-10 observation but were no longer open at this live refresh; they must not be routed as current work.

## Benchmark/product comparison

The repository's accepted direction is consistent with the useful parts of mature Tibia/OTS ecosystems: account/community surfaces from official game portals, richer integrated game information from OTS portals such as RubinOT, and practical player tools comparable to TibiaPal. The correct differentiator is first-party integration with authoritative/versioned Oteryn data, not merely recreating a static fan-site layout.

The portal should therefore converge on:

- secure Account Center + Character Portfolio;
- Today/LiveOps as the “what matters now” surface;
- versioned GameCatalog/Wiki as shared truth for UI and tools;
- Player Companion for calculators/planners/private session utility;
- World Hub for multi-world selection/history when applicable;
- safe Wallet/Bazaar/commerce only behind independent security/legal/operational gates.

## Recommended order

1. Reconcile routing/source-of-truth enough that task selection is reliable.
2. Close P1 findings #948, #944 and #941 through existing remediation ownership.
3. Close production/public-edge evidence only with explicit protected-environment authority.
4. Finish core character/account lifecycle and Character Portfolio Platform work.
5. Deliver LiveOps and public Today; private Today only after #941-style isolation gates.
6. Remove Announcements/Events reverse dependency before federated-search provider onboarding; then deliver search.
7. Close expected-content inventories for Wiki/GameCatalog.
8. Deliver Player Companion P0 vertical slices with authoritative/versioned inputs.
9. Add World Hub/community expansion when product inputs justify it.
10. Activate commerce only after payment, entitlement, legal, recovery and operational gates are independently proven.

## Architecture decision

- **Foundation quality:** sound and scalable.
- **Architectural rewrite required:** no.
- **Architectural improvement required:** yes, through bounded ownership/composition/security/completeness gates.
- **Portal globally complete:** no.
- **Player-tools architecture defined:** yes; implementation remains incomplete/planned.
- **Public production proof:** not established by this report.

## Current evidence references

Canonical sources include:

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`;
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- `docs/architecture/MODULE_CATALOG.md`;
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`;
- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`;
- `docs/architecture/SECURITY_ARCHITECTURE.md`;
- `docs/architecture/DATA_OWNERSHIP.md`;
- `docs/agents/PROJECT_STATE.md`;
- `docs/agents/ACTIVE_WORK.md`;
- `docs/agents/tasks/active/**`;
- Issues #941, #944, #948 and #490;
- open PRs #338 and #541 at the 2026-08-10 live refresh.
