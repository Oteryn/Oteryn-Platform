# Oteryn Platform System Architecture

## Authority and scope

This document owns the current high-level system context, trust boundaries, topology and dependency direction. It is not an exhaustive source for modules, data, security, testing, roadmap or integration contracts.

Start architecture-wide work at `ARCHITECTURE_AUTHORITY.md`. That index defines precedence and routes each concern to its focused canonical owner. Accepted ADRs and operation-specific contracts take precedence within their stated scope.

Sections explicitly labelled **Historical baseline** preserve first-phase planning context. They must not be interpreted as current product exclusions, current implementation state or unresolved facts without checking the focused canonical source and exact implementation evidence.

## Status

Living system-context document. Implementation availability, capability completeness, staging evidence and production proof are separate dimensions and require exact evidence.

## Architectural style

Oteryn Platform starts as a **Laravel modular monolith**.

This means one deployable Laravel application with explicit internal modules and dependency boundaries. The goal is to gain the operational simplicity of one application while avoiding a tightly coupled "everything calls everything" codebase.

Microservices are not the default. A module may be extracted later only when there is proven need for independent scaling, security isolation, lifecycle or ownership.

## System context

```text
                              Internet
                                 |
                                 v
                         Public Edge boundary
                   DNS | TLS | WAF | rate limits
                      Turnstile | admin Access
                                 |
                                 v
                      Reverse proxy / web tier
                                 |
                                 v
+------------------------------------------------------------------+
|                    Oteryn Platform (Laravel)                     |
|                                                                  |
|  Public Web/CMS      Identity/Auth       Accounts/Characters      |
|  Public Game Data    Wiki/EditorialMedia Support/Moderation       |
|  Game Catalog        Wallet/Marketplace  Admin/RBAC/Audit         |
|  Integration         Notifications       Platform API (planned)   |
|  Products/Entitlements (planned)         Payments (planned later) |
|                                                                  |
+---------------------------+-------------------------+--------------+
                            |                         |
                   explicit contracts       app-owned storage
                            |                         |
          +-----------------+-----------------+       |
          |                                   |       |
          v                                   v       v
  login-server / auth path         Canary-compatible DB   cache/queue/mail
          |                                   |
          +-----------------+-----------------+
                            |
                            v
                       Canary server
                    (separate repository)

Cross-cutting repository boundaries:
  Operations/Observability | Quality/E2E | LegalCommerce
```

The diagram is a system-context abstraction, not a complete module inventory or proof of current deployment. Use `MODULE_CATALOG.md`, focused contracts and exact deployment evidence for those questions.

`Wallet` and `Marketplace` represent the implemented Oteryn Coins and Character Bazaar foundation. They are not provider Payments. Regulated provider charging, products/entitlements fulfilment and production activation remain separately owned and gated.

`OperationsObservability`, `PublicEdge`, `QualityE2E` and `LegalCommerce` are explicit ownership boundaries. They do not imply a standalone service or user interface, and repository availability does not prove a particular environment is production-ready.

## Evidence dimensions

Architecture and delivery claims must keep these facts separate:

1. **Repository availability** — at least one bounded capability is merged and validated on `main`.
2. **Capability completeness** — the accepted expected inventory for the module is satisfied.
3. **Environment evidence** — an exact staging or production deployment/identity was validated.
4. **Activation authority** — product, security, legal and operational gates authorize use in that environment.

`AVAILABLE` in `MODULE_CATALOG.md` proves only the first dimension. Open audit findings may coexist with an available module. Neither a diagram entry nor repository validation establishes `PRODUCTION_PROVEN`.

The frozen PR #453 production-completion baseline and later exact merged PRs are evidence inputs, not replacement canonical owners. Current gaps remain tracked by their live Issues, including #365, #488, #489 and #490.

## Trust boundaries

### Boundary A — Internet to edge

Cloudflare may provide TLS termination, WAF, bot mitigation, rate limiting and administrative access controls. Origin access should eventually be restricted so public traffic cannot trivially bypass the edge.

Cloudflare is defense in depth only. The application must remain secure if a request reaches Laravel directly.

The `PublicEdge` module owns the expected DNS, TLS, redirect, HSTS, WAF, tunnel/origin and private-ingress contract and its exact environment evidence. Repository automation or configuration does not by itself prove live edge correctness.

### Boundary B — Edge to Oteryn Platform

Laravel owns HTTP validation, authentication, authorization, CSRF protection, session security, output escaping and application rate limits.

Never trust client-provided identity, role, account ID, character ownership or privilege claims.

### Boundary C — Oteryn Platform to shared game data

Any table also read or written by Canary/login-server is a cross-repository contract.

Before implementation, every shared write path must define:

- owning component;
- allowed writer(s);
- schema fields used;
- transaction/locking behavior;
- compatibility assumptions;
- rollback or migration strategy.

Read-only queries may be optimized independently but still require a documented schema contract.

### Boundary D — Identity to game login

Identity and game-session behavior is governed by accepted authentication ADRs and the matching contracts. This system document does not invent password, ticket, protocol or session compatibility.

The governing principle is one authoritative identity/security policy with explicit contracts to components that create or validate game login sessions.

### Boundary E — gameplay economy to regulated commerce

Wallet and Marketplace own Oteryn Coins reservation/ledger semantics and Character Bazaar policy. They must not silently become payment-provider settlement or product-entitlement authorities.

Provider events, refunds, chargebacks, tax/invoice handling, product fulfilment, expiry and revocation belong to `Payments`, `ProductsEntitlements` and `LegalCommerce` under separate accepted contracts and activation gates.

## Historical baseline — initial modules

The following list preserves the original first-phase module model. It is not the current exhaustive catalogue.

See `MODULE_CATALOG.md` for current responsibility, ownership and capability status.

1. `Identity` — credentials, sessions, verification, MFA, recovery.
2. `Accounts` — player account profile and account-level settings.
3. `Characters` — allowed character lifecycle operations.
4. `PublicGameData` — characters, guilds, highscores, online/status read models.
5. `CMS` — news and managed public content.
6. `Admin` — privileged application operations and RBAC.
7. `Audit` — immutable/security-relevant audit events.
8. `Integration` — Canary/login-server adapters and contract enforcement.
9. `Notifications` — mail and asynchronous user notifications.
10. `Payments` — originally deferred; current payment scope and activation limits are governed by later ADRs and the module catalogue.

## Dependency rules

- HTTP controllers depend on application/domain services, not directly on arbitrary shared tables.
- Security-critical authorization lives in policies/gates/domain rules, not only in UI visibility.
- `Identity` must not depend on payment settlement, product fulfilment or commerce delivery.
- `Accounts` may depend on `Identity` identity references, but must not implement authentication itself.
- `Characters` may mutate Canary-owned/shared data only through a documented integration boundary.
- `PublicGameData` should prefer read-only models/query services and must not become a hidden mutation path.
- `GameCatalog` consumes versioned deterministic external snapshots and must not infer completeness or activate an unknown boundary.
- `Wallet` owns Platform Oteryn Coins balance invariants; `Marketplace` uses Wallet services and operation-specific Character transfer contracts.
- `Payments` and `ProductsEntitlements` may depend on accepted Wallet/product contracts, but Wallet and basic Identity must not depend on provider activation.
- `Admin` invokes the same domain/application services as normal flows with stronger authorization; it must not bypass invariants with raw SQL.
- `Integration` contains compatibility translation, not core business policy.
- `OperationsObservability`, `PublicEdge` and `QualityE2E` prove operational and delivery properties without taking ownership of module business rules.
- `LegalCommerce` owns commerce-specific presentation/retention/refund/tax decision boundaries, not generic CMS publication or payment settlement.

Focused module and contract documents may add stricter rules for their scope.

## Data access strategy

Two categories are expected:

### Application-owned data

Examples:

- web sessions when not using framework default storage;
- MFA metadata;
- password recovery/verification metadata where appropriate;
- CMS, Wiki and editorial-media content;
- Wallet, Marketplace and future entitlement records;
- RBAC metadata;
- audit records;
- platform-specific preferences and other Platform-owned module data.

Oteryn Platform owns migrations and lifecycle for these tables unless a focused contract states otherwise.

### Shared/Canary-compatible data

Accounts, players, guilds and game-specific state may cross repository boundaries. Exact ownership and writer rules are defined by `DATA_OWNERSHIP.md` and operation-specific contracts.

No agent may assume shared table names or columns from MyAAC conventions without verifying the actual Oteryn Canary schema and current contract evidence.

## Authentication direction

Target capabilities include:

- framework-backed secure password hashing;
- password migration compatible with the accepted game-login path;
- email verification where product policy requires it;
- MFA/TOTP for security-sensitive users, mandatory for administrators where the accepted security policy requires it;
- secure password reset;
- server-side authorization;
- session revocation after security-sensitive changes;
- rate limiting and abuse controls;
- auditable privileged actions.

Current compatibility and delivery state must be read from accepted authentication ADRs, `docs/contracts/**`, focused security documentation and exact implementation evidence.

## Frontend direction

The original preference was Laravel Blade/server-rendered pages because it reduced moving parts, shared one authorization model and avoided requiring a public SPA API before product need justified it.

Current frontend structure and accepted shell/information architecture are governed by the relevant frontend ADRs, module catalogue and implementation evidence. A framework or topology change that outlives one task requires an ADR.

## Deployment direction

Logical system topology:

```text
Cloudflare / PublicEdge
   |
Origin firewall / reverse proxy
   |
Laravel web instances
   |---- queue workers
   |---- cache/session service
   |---- mail provider
   |
Database reachable only from approved private/application paths
   |
Canary / login-server on explicitly allowed network paths
```

This is an architecture boundary, not proof of a specific live deployment. Production database, cache and game services must not be publicly exposed unless technically unavoidable and explicitly secured and accepted.

## Observability direction

The `OperationsObservability` boundary requires:

- structured application logs;
- request/error correlation IDs;
- authentication/security event logs;
- admin audit trail;
- health/readiness endpoints suitable for infrastructure monitoring;
- metrics for login failure, rate limits, queue failures and critical integrations;
- release identity, alert ownership and documented backup/restore/rollback expectations.

Current availability and production proof belong to exact implementation and operational evidence. Do not log passwords, session tokens, MFA secrets, reset tokens or other credentials.

## Historical baseline — first-release non-goals

The original first-release plan excluded:

- payment processing;
- marketplace/auction systems;
- microservice decomposition;
- complex real-time frontend architecture;
- replacing Canary gameplay/runtime responsibilities;
- silent modifications to Canary or login-server repositories.

This list is historical planning context. It does not override later accepted ADRs, module ownership, contracts or merged implementation. Wallet/Marketplace foundations now exist, while regulated provider Payments, products/entitlements and their activation gates remain separate. The enduring constraints are explicit cross-repository contracts, no silent external mutation and no production activation without its own evidence and authorization.

## Historical baseline — discovery before shared auth/data coding

The first-phase plan required proof of:

1. actual Oteryn Canary account/player/guild schema;
2. password hashing expectations;
3. login-server and/or Canary session authentication path;
4. the component that creates/revokes game sessions;
5. required account flags/status fields;
6. transaction/concurrency expectations for character/account mutations;
7. single-world versus multi-world requirements.

This list remains useful as a discovery checklist, but each item's current state must be established from `DATA_OWNERSHIP.md`, accepted ADRs, operation-specific contracts and exact implementation evidence. Do not preserve an item as `UNKNOWN` merely because this historical section once described it that way.
