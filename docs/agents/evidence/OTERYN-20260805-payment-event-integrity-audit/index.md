# Payment event integrity audit evidence

## Identity

- Programme: `OTERYN_PLATFORM_CONTINUOUS_AUDIT`
- Task: `OTERYN-20260805-payment-event-integrity-audit`
- Repository: `blakinio/Oteryn-Platform`
- Audited base: `3ab77c072dce796b09004c54b649db009a75d524`
- Prior exhaustive audit merge: `cbbd7613cee13cf01931a0ba0f7ac089122132e0`
- Delta: 37 commits on current `main`
- Finding: `OPA-SEC-0001`
- Remediation Issue: #547

## Proven source evidence

| Evidence | Proven fact |
|---|---|
| `app/Payments/Data/VerifiedProviderEvent.php` | The provider-neutral verified event has no currency or minor-unit settlement fields. |
| `app/Payments/Infrastructure/DeterministicTestPaymentProvider.php::verify()` | A signed event is accepted using event ID/type, order public ID and optional object reference; amount and currency are not extracted. |
| `app/Payments/Actions/ProcessPaymentProviderEvent.php::processVerified()` | The order is selected by public ID, provider equality is checked, and the state-machine result is applied without amount/currency comparison or checkout-reference binding. |
| `app/Payments/PaymentOrderStateMachine.php` | `payment.succeeded`, refund, dispute and chargeback event types can directly select financial order states. |
| `tests/Feature/Payments/PaymentEventCoreTest.php::test_signed_success_is_persisted_once_without_raw_payload_or_personal_data()` | A signed payload containing no settlement amount or currency marks a EUR order for 9,999 minor units succeeded. |
| `database/migrations/2026_08_02_124700_create_payment_event_core_tables.php` | Orders persist immutable currency and amount, but provider events persist no corresponding settlement amount/currency evidence. |
| `docs/architecture/adr/0021-provider-neutral-payment-security-core.md` | Amount/currency integrity is explicitly a protected asset. |

## Duplicate and ownership search

- Searched Issues for payment webhook amount, currency, settlement mismatch and provider-object binding: no duplicate root-cause Issue.
- Issue #321 is the parent payment-security programme and remains open.
- Issue #489 is the broad commerce audit/remediation owner.
- Issue #470 and PR #471 are terminal; their task is archived and payment ownership released.
- No active task or open PR was found owning the finding's payment-integrity paths.

## Evidence classification

```yaml
finding_id: OPA-SEC-0001
severity: high
confidence: high
evidence_state: PROVEN
current_exploitability: blocked_by_fail_closed_payment_activation
pre_activation_gate: must_fix
runtime_mutation_by_audit: none
production_mutation: none
external_repository_write: none
```

## Validation boundary

This evidence package is documentation-only. Runtime E2E is `NOT_APPLICABLE` because the auditor made no runtime change. Existing implementation CI proves only the code as written; it does not negate the contract-level mismatch demonstrated above.