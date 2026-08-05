---
task_id: OTERYN-20260802-payment-event-core
status: completed
agent: payment-event-core continuation owner
project_lane: payments
track: backend-security
created: 2026-08-02T14:45:00+02:00
completed: 2026-08-05T10:14:00+02:00
archived: 2026-08-05T10:14:00+02:00
product_issue: 470
parent_issue: 321
product_pr: 471
product_head: 16c3d108fe3e2220c15e6eec8b61c635debe31a2
risk: high
owned_paths: []
shared_path_lease: []
feature_scope:
  type: backend_only
  user_facing: false
  backend_required: true
  frontend_required: false
  integration_required: true
  e2e_required: true
execution_mode: github-only
---

# Terminal result

Issue #470 is implemented as a provider-neutral payment event producer slice with fail-closed runtime and production boundaries. PR #471 is the sole delivery PR.

The maximum delivery claim is `producer_complete`. The customer-facing payment feature is not complete, and parent Issue #321 remains open.

# Delivered scope

- additive, reversible payment order, attempt, provider-event, transition and reconciliation persistence;
- integer minor-unit amounts, immutable public IDs, monotonic order versions and unique idempotency keys;
- deterministic non-production HMAC checkout/webhook adapter that refuses production execution;
- signed raw-payload verification before parsing;
- exact replay, conflicting-event and out-of-order reconciliation behavior;
- append-oriented transition and reconciliation evidence without raw payload or secret persistence;
- fail-closed production configuration verification;
- focused SQLite, security and isolated MariaDB concurrency coverage;
- ADR, operations guide and durable evidence record.

# PAY-CORE-001 remediation

`PaymentAvailability::ensureEnabled()` is the first operation in:

- `CreatePaymentOrder::execute()`;
- `CreatePaymentCheckout::execute()`;
- `ProcessPaymentProviderEvent::execute()`.

`PaymentProviderResolver` defers gateway and webhook-verifier construction until after this guard. With the default `payments.enabled=false` and no provider binding, all three actions return `payments_disabled` before provider resolution or persistence.

Regression coverage proves zero payment orders, attempts, transitions, provider events and reconciliation entries are written on the disabled path.

# PHPStan and MariaDB concurrency repair

The two original PHPStan failures in `PaymentEventConcurrencyMariaDbTest.php` were removed by narrowing the by-reference `pcntl_waitpid()` status to `int` before `pcntl_wifexited()` and `pcntl_wexitstatus()`. No suppression or analysis weakening was introduced.

The isolated MariaDB concurrency fixture also closes its administrative PDO connection before `pcntl_fork()`. This prevents child teardown from invalidating the parent connection; final cleanup opens a fresh administrative connection.

# Validation

Exact implementation head `16c3d108fe3e2220c15e6eec8b61c635debe31a2`:

- CI: PASS, including Pint, PHPStan and complete PHPUnit;
- Agent Governance: PASS;
- Portal Exhaustive Audit: PASS;
- Portal Acceptance Contract: PASS;
- Acceptance E2E and Visual UX: PASS;
- Build Synology Staging Images: PASS;
- Error State Acceptance: PASS;
- Support Moderation Acceptance: PASS;
- Game Auth Ticket Concurrency: PASS;
- Edge Security Emulation: PASS;
- Platform DB Outage Validation: PASS;
- Phase 7 Production-Like Validation: PASS;
- Deep System Validation PHP formatting/static-analysis lane: PASS;
- Deep System Validation complete PHP regression and concurrency lane: PASS;
- independent payment security and persistence audit: PASS;
- material findings: 0;
- unresolved review threads: 0.

The archive commit containing this record is the exact-final-head merge gate and must pass every required GitHub Actions check before PR #471 is marked ready and squash-merged.

# Explicitly incomplete consumers

- selected real provider and sandbox proof;
- public signed webhook ingress and edge/rate-limit controls;
- authenticated checkout and payment-history frontend;
- operator reconciliation UI behind exact permission and confirmed MFA;
- Wallet/product/entitlement delivery under Issue #322;
- real refund, dispute and chargeback value reconciliation;
- provider sandbox and production evidence;
- deployment and production activation.

Issue #321 must remain open for these consumers.

# Safety boundary

No real provider credential, charge, customer financial data, public webhook, Wallet mutation, entitlement delivery, Canary mutation, deployment or production activation was performed.

# Ownership release

All task ownership and shared-path leases are released. No follow-up task or repository was created or modified.
