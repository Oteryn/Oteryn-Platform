---
task_id: OTERYN-20260807-payment-partial-refund-integrity
issue: 797
status: completed
completed_at: 2026-08-07T15:05:36Z
implementation_pull_request: 826
implementation_head: cd4a47ae025ed397a52441b9c12a8e2f44dd9664
implementation_merge: fe5a177af64d28ab4a2780d7ceb629502a257a80
risk: high
validation_intensity: HEIGHTENED
self_review: PASS
material_findings: 0
production_activation_authorized: false
ownership: RELEASED_ON_ARCHIVE_MERGE
---

# OTERYN-20260807 payment partial-refund integrity — Completed

## Result

Issue #797 is repaired. Distinct authenticated partial-refund events no longer lose financial value when an order is already `partially_refunded`.

The delivered payment-core boundary:

- defines `payment.partially_refunded.amount_minor` as an authenticated incremental refund delta;
- records every accepted partial delta and resulting cumulative refunded total on append-oriented, versioned payment-order transitions;
- permits a distinct partial-refund event to create an intentional same-state transition while preserving exact provider-event replay idempotency;
- calculates cumulative refund truth under the existing locked payment-order transaction;
- reconciles missing legacy partial-refund history and any partial delta that would reach or exceed the immutable order amount;
- records `payment.refunded` as cumulative terminal truth equal to the immutable order amount while preserving earlier partial-refund evidence;
- adds an additive migration whose rollback remains compatible with empty repository/test schemas but fails closed if refund-value evidence is populated;
- leaves production/public provider activation disabled and performs no Wallet or Products value delivery.

## Delivery

- Implementation PR: #826.
- Final exact implementation head: `cd4a47ae025ed397a52441b9c12a8e2f44dd9664`.
- Synchronized implementation base/main: `4186c4bcee3a94e2e60911d5902453fcd37962cc`.
- Protected squash merge: `fe5a177af64d28ab4a2780d7ceb629502a257a80`.
- Issue #797 closed automatically as completed by the merge.

## Exact-head validation

Applicable exact-head evidence on `cd4a47ae025ed397a52441b9c12a8e2f44dd9664`:

- CI `31190262706`: PASS. `classify-changes`, `runtime-tests` and required `test` gate passed. Runtime validation included Composer validation/audit, Pint, PHPStan and the complete PHPUnit suite with MariaDB and pcntl enabled.
- Agent Governance `31190259673`: PASS.
- Game Auth Ticket Concurrency `31190258372`: PASS, including existing migration rollback behavior.
- Platform DB Outage Validation `31190259100`: PASS.
- Edge Security Emulation `31190258377`: PASS.
- Phase 7 Production-Like Validation `31190258448`: PASS.
- PR review threads: zero.
- Submitted reviews/requested changes: zero.

The HEIGHTENED full-diff self-review was recorded on PR #826 for the exact final head and returned PASS with zero material findings.

`Native protocol contract audits` run `31190258395` was not applicable to this repair. Its single failing architecture-boundary job rejects any generic `app/`, `database/` or `tests/` change unless the native-protocol producer task is present; the other four native-protocol audit jobs passed. The payment-owned required CI/Governance and migration-compatibility gates passed on the exact head.

## Regression evidence

Deterministic coverage includes:

- first and later distinct partial refunds;
- exact provider-event replay;
- cumulative refund reconstruction;
- arithmetic over-refund reconciliation;
- later terminal full refund;
- amount and currency mismatch;
- out-of-order partial refund;
- legacy `partially_refunded` state without durable refund-value history;
- two concurrent legitimate MariaDB partial refunds whose values must both survive serialization and produce the exact cumulative total.

## E2E and safety

Live-provider/customer E2E is not authorized. The applicable executable boundary is repository/non-production authenticated-provider processing and MariaDB concurrency under the full CI suite.

No provider sandbox, public webhook activation, customer charge/refund, secret, protected environment, Wallet/Products mutation, production activation or external-repository mutation was performed.

## Rollback and compatibility

Application behavior may be reverted only with payment ingress disabled while preserving durable payment evidence. The additive refund columns can be removed by migration rollback when no refund-value evidence is populated; populated authenticated refund evidence makes rollback fail closed instead of deleting financial truth.

## Ownership release

This archival closeout removes the durable active-task lease for Issue #797. Once this archive change is merged, all Issue #797 implementation paths and payment-partial-refund coordination ownership are released.
