# Oteryn Payments Foundation Agent — terminal tombstone

```yaml
prompt_contract:
  version: 1.1
  lifecycle: TERMINAL_DO_NOT_RUN
  changed_surfaces:
    - worker_template
    - payment_safety_routing
    - terminal_alias_routing
  objective: preserve the completed provider-neutral non-production Payments foundation and prevent duplicate implementation
  baseline_version: 1.0
  eval_suite: docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  rollback_version: restore 1.0 only if the merged foundation is proven absent from protected main
owner_alias: OTERYN-PAYMENTS-FOUNDATION
terminal_issue: 321
successor_issue: 1236
```

## Terminal status

`OTERYN-PAYMENTS-FOUNDATION` is **TERMINAL_DO_NOT_RUN**.

The provider-neutral non-production foundation was delivered by PR #1228 and squash merge `788f58c031bf575396231a95b6a9d28afbadb67c`, then archived by PR #1231. Do not create a new Payments implementation task, branch or PR merely because this historical alias is invoked again.

Issue #1236 now owns the unresolved real-provider, provider-sandbox, legal/tax/privacy/receipt, secret-rotation, public-webhook, operational-alerting and production-activation gates.
## Invocation behavior

When the owner invokes or continues this alias:

1. Verify that PR #1228 / merge `788f58c031bf575396231a95b6a9d28afbadb67c` remains reachable from protected `main` and the archived foundation task remains terminal.
2. Verify that no newer accepted decision explicitly reopens the provider-neutral foundation itself.
3. Report the foundation as terminal and identify Issue #1236 as the successor real-provider track when it is still open.
4. Do **not** silently reinterpret this alias as authorization to select a provider, access merchant/provider accounts, use provider credentials, activate public production webhooks, charge customers, refund live money, or mutate Wallet/Entitlements.
5. If terminal evidence is missing or contradicted, classify the state as `CONFLICT` and stop before creating implementation work.

The test adapter cannot be enabled as a real production operator.

## Durable boundary

The completed foundation proves repository/test-adapter behavior only:

- owner-scoped EN/PL payment history and browser-return presentation;
- signed deterministic non-production provider ingress;
- idempotent/order-aware settlement and refund truth;
- exact-permission, confirmed-MFA reconciliation review;
- production fail-closed behavior;
- separation from Wallet and Products/Entitlements;
- zero-retry desktop/tablet/mobile browser evidence for the delivered test path.
It does **not** prove a selected commercial provider, provider sandbox correctness, legal/tax/privacy/receipt readiness, production secret handling, production alerting or live-payment activation. Those are successor gates, primarily Issue #1236; paid value delivery remains separately owned by #322.

## Safety

No invocation of this tombstoned alias grants:

- payment-provider account or merchant-dashboard access;
- credentials, secrets or protected payment data access;
- real charges, refunds, disputes or chargebacks;
- production/protected-environment mutation;
- external repository access;
- owner-funded Codex/OpenAI/API use.

Browser return remains presentation only and never establishes settlement truth. Payment/provider truth remains separate from Wallet and entitlement delivery.

## Terminal response

A normal repeated invocation returns the current terminal status rather than opening work. If Issue #1236 remains unresolved, identify it as the blocked successor without claiming `PRODUCTION_PROVEN` or live-payment completion.

Use the canonical terminal response from `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`; report `RESULT` as a completed non-production foundation with real-provider production work routed to Issue #1236.
