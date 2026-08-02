# Prioritized programme slices

Parent: #451

## P0 — CI change classification and heavy-gate routing

Outcome:

- documentation/task/evidence PRs run governance/docs validation without full application, production-like, edge, DB-outage or game-auth concurrency execution;
- runtime/security/shared changes retain the required heavy gates;
- stable required check names and branch-protection behavior are preserved;
- path-classifier policy is validated against positive, negative and shared-dependency cases.

Why first:

- it reduces cost and queue pressure for every subsequent slice;
- the defect is directly proven from current workflow triggers;
- it can be implemented independently of product feature code.

Required execution mode: checkout-capable worker for workflow edits and emitted-check validation.

## P0 — private-production operational baseline correction

Outcome:

- exact deployed release identity;
- explicit private-production classification separate from public exposure;
- delivery-capable mail;
- session/cache/queue and worker topology;
- centralized logs, metrics, alerts and ownership;
- dated backup/restore and rollback evidence;
- controlled exact-release smoke/E2E.

Dependencies: Issue #91 / PR #405, operator access and protected secrets outside Git.

## P0 — Issue #365 exact validator closure

Outcome:

- execute the frozen 12-sample differential with required request/session instrumentation;
- determine root cause or falsify candidate mechanisms;
- repair only after evidence;
- close temporary PR #412 without merge;
- terminally reconcile PR #381 and Issue #365.

## P1 — architecture/module catalogue reconciliation

Outcome:

- update stale statuses for Wiki, EditorialMedia, Wallet and Marketplace;
- add ProductsEntitlements, LegalCommerce, OperationsObservability, PublicEdge and QualityE2E ownership;
- link roadmap/module entries to the machine-readable capability and production-evidence states.

This is documentation-only but should follow validation of the baseline report.

## P1 — payment provider-neutral foundation for Poland/EU, PLN/EUR

Outcome:

- payment ADR/threat model;
- provider interface and sandbox adapter selection;
- order/payment state machine;
- minor-unit money representation and currency allowlist;
- signed webhook verification, replay/idempotency and event ledger;
- reconciliation, refunds, disputes/chargebacks and admin/fraud controls;
- no live charging.

Dependencies: current provider research, EU/Poland legal/tax/privacy ownership and checkout-capable implementation worker.

## P1 — products and entitlements

Outcome:

- premium/VIP and coin packages;
- vouchers/codes;
- fulfilment contract and idempotent delivery;
- entitlement expiry, revocation and history;
- customer and admin EN/PL responsive UI;
- real E2E with payment sandbox/fake provider and Canary boundary where applicable.

## P1 — character lifecycle

- restart Issue #324 rename discovery from current `main` and deliver the actual ADR/contract before implementation;
- continue deletion/restore only through the Canary-owned lifecycle contract;
- implement complete backend/frontend/audit/E2E slices only after operation authority and least privilege are proven.

## P1 — Game Catalog continuation

- resolve PR #338 through the Canary schema 1.3 producer;
- then deliver public NPC/shop projections;
- separately decide spells/quests/achievements/maps and other knowledge modules by value and authoritative data availability.
