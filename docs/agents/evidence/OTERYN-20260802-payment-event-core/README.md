# Payment event core evidence

Issue: #470  
Parent: #321  
Pull request: #471  
Implementation head: `16c3d108fe3e2220c15e6eec8b61c635debe31a2`

## Delivery claim

`producer_complete` is the maximum permitted claim for this slice.

The repository deliverable is a provider-neutral backend/security core:

- additive payment order, attempt, provider-event, transition and reconciliation persistence;
- signed-event verification contract;
- deterministic non-production HMAC adapter;
- exact idempotency and replay/conflict behavior;
- explicit out-of-order reconciliation;
- fail-closed production configuration and runtime availability boundary.

## PAY-CORE-001

Remediated.

A shared `PaymentAvailability` guard rejects all three public payment actions unless `payments.enabled` is exactly `true`. `PaymentProviderResolver` defers provider gateway and webhook verifier resolution until after that guard.

Regression coverage proves that default-disabled configuration with no provider binding:

- rejects order creation with `payments_disabled`;
- rejects checkout creation with `payments_disabled`;
- rejects provider-event processing with `payments_disabled`;
- writes no order, attempt, transition, provider event or reconciliation entry.

## PHPStan and concurrency repairs

The two mixed-argument errors in `PaymentEventConcurrencyMariaDbTest.php` were repaired by validating the by-reference `pcntl_waitpid()` status as `int` before passing it to `pcntl_wifexited()` and `pcntl_wexitstatus()`. No PHPStan suppression was introduced.

The MariaDB concurrency fixture closes its administrative PDO connection before `pcntl_fork()`, preventing child process shutdown from invalidating the parent's cleanup connection. `tearDown()` reconnects with a fresh administrative session.

## Independent audit

Reviewed boundaries:

- disabled execution ordering;
- provider resolution and production refusal;
- signed raw-payload verification before JSON parsing;
- additive/reversible persistence and foreign-key ordering;
- exact event idempotency and conflicting replay behavior;
- monotonic transition versions;
- terminal-state non-regression and reconciliation;
- raw-payload/secret exclusion;
- isolated MariaDB duplicate-event concurrency and cleanup lifecycle.

Material findings: 0.  
Unresolved PR review threads: 0.  
Submitted PR reviews: 0.

## Implementation-head validation

Exact code head `16c3d108fe3e2220c15e6eec8b61c635debe31a2` passed:

- CI, including Pint, PHPStan and complete PHPUnit;
- Agent Governance;
- Portal Exhaustive Audit;
- Portal Acceptance Contract;
- Acceptance E2E and Visual UX;
- Build Synology Staging Images;
- Error State Acceptance;
- Support Moderation Acceptance;
- Game Auth Ticket Concurrency;
- Edge Security Emulation;
- Platform DB Outage Validation;
- Phase 7 Production-Like Validation;
- the PHP formatting/static-analysis and complete PHP regression/concurrency lanes inside Deep System Validation.

The archive head remains the protected merge gate and must pass all required checks before PR #471 is merged.

## Explicitly missing consumers

- selected real provider and sandbox adapter;
- public signed webhook route and deployment rate-limit/edge controls;
- authenticated checkout and payment-history frontend;
- operator reconciliation UI behind exact permission and confirmed MFA;
- wallet/entitlement/product delivery from Issue #322;
- real refund/dispute/chargeback value reconciliation;
- provider sandbox and production evidence.

Issue #321 remains open. No user-facing or production payment feature may be described as complete from this slice.

## Safety boundary

No real charge, provider credential, customer financial data, public webhook, Wallet mutation, entitlement delivery, Canary mutation, deployment or production activation was performed.
