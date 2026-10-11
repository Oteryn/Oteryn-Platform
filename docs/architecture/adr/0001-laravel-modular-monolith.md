# ADR 0001 — Laravel modular monolith

- Status: Accepted
- Date: 2026-07-18

## Context

Oteryn Platform is intended to replace MyAAC as the long-term web/application platform for Oteryn while legacy engine remains the game server. The project needs secure identity/account handling, CMS/public pages, administration, game-data integration and future business modules.

The initial team/project context favors a coherent first-party application with strong framework support and low operational complexity.

## Decision

Start Oteryn Platform as a **Laravel modular monolith**.

Use Laravel's maintained framework capabilities for routing, validation, authentication primitives, authorization, hashing, sessions, queues/events, database transactions and testing.

Use server-rendered Blade views initially unless a later product requirement justifies a separate frontend architecture.

Internal modules must have explicit responsibilities as defined in `docs/architecture/MODULE_CATALOG.md`.

## Consequences

### Positive

- one primary application stack for web/backend;
- lower deployment and observability complexity than microservices;
- framework-provided security primitives;
- straightforward server-side authorization;
- faster initial replacement of MyAAC;
- modules can still be extracted later if evidence justifies it.

### Negative

- requires discipline to prevent a tightly coupled monolith;
- independent scaling/deployment of modules is not available initially;
- direct shared-database integration can create coupling unless contracts are enforced.

## Rejected alternatives

### Rebuild immediately as microservices

Rejected because service boundaries, scaling needs and operational ownership are not yet proven.

### NestJS as the initial primary backend

Not rejected as a technology in general, but not selected because introducing Node.js/TypeScript alongside the existing PHP web ecosystem does not currently provide enough proven benefit.

### Continue extending MyAAC indefinitely

Rejected as the long-term architecture because Oteryn requires first-party control over identity, authorization, security boundaries and future platform-specific behavior.

## Revisit triggers

Revisit only when there is evidence for:

- independent scaling requirements;
- a module requiring separate security isolation;
- independent deployment/lifecycle requirements;
- a frontend product requirement that materially benefits from SPA/client architecture.
