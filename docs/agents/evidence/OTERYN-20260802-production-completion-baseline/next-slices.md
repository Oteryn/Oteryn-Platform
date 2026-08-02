# Prioritized programme slices

Parent: #451

## P0 — CI change classification and heavy-gate routing

Outcome:

- documentation/task/evidence PRs run governance/docs validation without full application, production-like, edge, DB-outage or game-auth concurrency internals;
- runtime/security/shared changes retain required heavy gates;
- stable required check names and branch-protection behavior are preserved;
- classifier policy is validated against positive, negative, boundary and deliberately defective fixtures.

Why first:

- it reduces cost and queue pressure for every subsequent slice;
- the defect is directly proven from current workflow triggers and PR #453 execution;
- it is independent of product feature code.

Required execution mode: GitHub API for the branch/commit plus GitHub Actions as the remote validation environment. Codex/local checkout is optional, not required.

## P0 — Issue #365 exact validator closure

- execute the frozen 12-sample differential with request/session instrumentation;
- determine root cause or falsify candidate mechanisms;
- repair only after evidence;
- close temporary PR #412 without merge;
- terminally reconcile PR #381 and Issue #365.

## P0 — private-production operational baseline correction

- exact deployed release identity and image digests;
- private-production classification separate from public exposure;
- delivery-capable mail, session/cache/queue topology and workers;
- centralized logs, metrics, alerts and ownership;
- dated backup/restore and rollback evidence;
- controlled exact-release smoke/E2E.

Dependencies: Issue #91 / PR #405, operator access and protected secrets outside Git.

## P1 — architecture/module catalogue reconciliation

- update stale Wiki, EditorialMedia, Wallet and Marketplace statuses;
- add ProductsEntitlements, LegalCommerce, OperationsObservability, PublicEdge and QualityE2E ownership;
- link roadmap/module entries to machine-readable capability and production-evidence states.

## P1 — provider-neutral payment foundation for Poland/EU, PLN/EUR

- current provider research and ADR/threat model;
- provider interface and sandbox adapter selection;
- orders/payment lifecycle, minor-unit money and currency allowlist;
- signed webhooks, replay/idempotency, immutable event ledger;
- reconciliation, refunds, disputes/chargebacks and admin/fraud controls;
- no live charging.

Dependencies: EU/Poland legal/tax/privacy ownership and separately gated provider credentials.

## P1 — products and entitlements

- premium/VIP and coin packages;
- vouchers/codes;
- idempotent fulfilment;
- entitlement expiry, revocation and history;
- customer/admin EN/PL responsive UI;
- real sandbox/fake-provider E2E.

## P1 — character lifecycle

- restart Issue #324 rename discovery from current `main` and deliver the ADR/contract before implementation;
- continue deletion/restore only through a Canary-owned operation contract;
- implement complete backend/frontend/audit/E2E slices only after operation authority and least privilege are proven.

## P1 — Game Catalog continuation

- resolve PR #338 through the Canary schema 1.3 producer;
- deliver public NPC/shop projections after compatible rollout;
- separately decide spells, quests, achievements, maps and other knowledge modules.
