# Evidence — OTERYN-20260807 payment partial-refund integrity audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `f7abc6096264aee890e0ab475087adeba7265397`
- Prior repair: Issue #547 / PR #595 / merge `5a04d055aa02b74cc741f69713d1ea26c91550c0`
- Parent payment programme: Issue #321

## Primary repository evidence

- `VerifiedProviderEvent` now carries authenticated currency and minor-unit amount; the original OPA-SEC-0001 contract gap is repaired.
- `ProcessPaymentProviderEvent::settlementMismatch()` accepts `payment.partially_refunded` when currency matches and the individual amount is greater than zero and less than the immutable order total.
- `PaymentOrderStateMachine` maps a partial refund to `partially_refunded`; if the order is already in that state, another distinct partial-refund event is classified `NOOP` with `duplicate_state`.
- `PaymentOrder` persists original amount/currency/status/version but no refunded minor-unit accumulator.
- `PaymentOrderTransition` persists state/version/reason but no refund amount.
- Successfully processed provider events retain bounded metadata but do not persist authenticated amount/currency; therefore a second partial-refund NOOP does not preserve its amount in durable Platform financial truth.
- Focused tests contain no two-distinct-partial-refund or concurrent-distinct-partial-refund scenario.

## Accepted contract evidence

ADR 0021 requires amount/currency integrity and append-oriented financial/reconciliation history. It defines a partial refund only as same currency plus a positive amount below total and does not define whether the amount is incremental or cumulative.

Issue #321 requires partially-refunded lifecycle, immutable financial/audit records, refund records and concurrency evidence before production provider activation.

## Finding

`OPA-SEC-0002` / Issue #797 — HIGH / P1 / PROVEN.

A second distinct authenticated partial refund can be acknowledged as processed while the new refunded amount is neither accumulated nor durably represented, leaving no aggregate over-refund invariant.

## Safety

No real provider, payment charge, production webhook, customer data, migration or external system was mutated to establish this finding.
