# Architecture Decision Records

ADRs record durable architecture decisions that should survive individual tasks and chat sessions.

## Status values

- `Proposed`
- `Accepted`
- `Superseded`
- `Rejected`

## Index

- `0001-laravel-modular-monolith.md` — use a Laravel modular monolith as the initial application architecture.
- `0002-separate-platform-and-canary-repositories.md` — keep Oteryn Platform and Canary separate with explicit contracts.
- `0003-defer-payments-module.md` — defer payment/shop implementation and preserve modular boundary.
- `0015-wiki-launch-content-provisioning.md` — install reviewed bilingual launch content through an attributable, atomic and conflict-safe operator workflow.

When a decision changes, add a new ADR and mark the old one `Superseded` rather than rewriting history silently.
