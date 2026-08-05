# Payment event core evidence

Issue: #470  
Parent: #321  
Pull request: #471

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

A shared `PaymentAvailability` guard now rejects all three public payment actions when `payments.enabled` is not exactly `true`. `PaymentProviderResolver` defers provider gateway and webhook verifier resolution until after that guard.

Regression coverage proves that default-disabled configuration with no provider binding:

- rejects order creation with `payments_disabled`;
- rejects checkout creation with `payments_disabled`;
- rejects provider-event processing with `payments_disabled`;
- writes no order, attempt, transition, provider event or reconciliation entry.

## PHPStan repair

The two mixed-argument errors in `PaymentEventConcurrencyMariaDbTest.php` were repaired by validating the by-reference `pcntl_waitpid()` status as `int` before passing it to `pcntl_wifexited()` and `pcntl_wexitstatus()`. No PHPStan suppression was introduced.

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
- isolated MariaDB duplicate-event concurrency.

Material findings: 0.  
Unresolved PR review threads: 0.  
Submitted PR reviews: 0.

## Validation checkpoint

Code head: `a128c184c5b581c20b7cda1e2e6980c63bf1117a`.

Observed exact-head passes before the documentation checkpoint:

- Agent Governance;
- Portal Exhaustive Audit.

The complete exact-final-head matrix remains a merge gate and will be recorded in the PR/Issue closeout.

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

No real charge, provider credential, customer financial data, public webhook, Wallet mutation, entitlement delivery, Canary mutation, deployment or production activation is performed.
