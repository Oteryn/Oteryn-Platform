---
task_id: OTERYN-20260807-payments-refund-settlement-audit
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
status: validating
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: audit/payments-refund-settlement-integrity-20260807
base_branch: main
base_sha: 152f36c10d765b105bbed77e46c3d6022c4e65a6
pr: 838
production_activation_authorized: false
cross_repository_mutation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
search_first:
  - open audit/remediation Issues touching payment refund truth
  - active tasks and open PRs overlapping Payments
optional_reads: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payments-refund-settlement-audit.md
modules:
  - payments-refund-settlement-integrity
coordination_key: audit:payments-refund-settlement-integrity
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 payments refund settlement audit

## Goal

Independently re-audit the high-risk payment refund settlement slice after the repair of OPA-SEC-0002 / Issue #797, proving whether repeated partial refunds, cumulative refund truth, replay, over-refund rejection and concurrent distinct refund events now preserve durable financial truth without introducing a new material defect.

## Scope and collision check

Audited product paths are read-only under this audit task:

- `app/Payments/Actions/ProcessPaymentProviderEvent.php`
- `app/Payments/PaymentOrderStateMachine.php`
- the payment provider event / transition persistence used by refund processing
- `tests/Feature/Payments/PaymentPartialRefundIntegrityTest.php`
- `tests/Feature/Payments/PaymentPartialRefundConcurrencyMariaDbTest.php`
- `tests/Feature/Payments/PaymentEventCoreTest.php`
- `docs/architecture/adr/0021-provider-neutral-payment-security-core.md`

Live ownership refresh found no open Payments audit/repair PR or active task overlapping this refund-truth slice. PR #826 repaired Issue #797 and is merged. The broader real-provider/product-completion work remains separately owned by Issue #321 and is not duplicated here. Native-auth and native-protocol verification tasks are non-overlapping.

No product/runtime, workflow, dependency, migration or infrastructure mutation is authorized by this task.

## Audit inventory

| Layer | Evidence | Audit result |
| --- | --- | --- |
| Domain action | `ProcessPaymentProviderEvent` | PASS: provider event and order rows are locked; authenticated settlement facts are validated before state mutation. |
| State machine | `PaymentOrderStateMachine` | PASS: `succeeded -> partially_refunded/refunded` and `partially_refunded -> partially_refunded/refunded` are explicit; terminal truth does not regress. |
| Durable refund truth | payment transitions with `verified_refund_amount_minor` and `refunded_total_minor` | PASS: partial events preserve event delta plus cumulative total; legacy partial state without durable history fails closed. |
| Replay/idempotency | provider event identity + payload hash | PASS: exact replay is a no-op and altered replay fails closed. |
| Over-refund / malformed amounts | refund truth validation | PASS: non-positive partial amounts, exact-total partial events and cumulative overshoot reconcile rather than mutate settlement truth. |
| Full refund semantics | ADR 0021 + processor | PASS: `payment.refunded.amount_minor` is cumulative terminal refunded amount and must equal original order amount. |
| Concurrency | MariaDB concurrency test | PASS: distinct concurrent partial-refund events serialize under the locked order and both durable deltas survive. |
| Product/provider completion | Issue #321 | OUT_OF_SCOPE / already tracked: real provider selection and customer-facing production acceptance remain separate pre-production work. |

## Negative-path review

- A second distinct partial-refund event no longer collapses into `duplicate_state`; the processor computes a new cumulative refund total before allowing the same-state transition.
- A repeated identical provider event remains idempotent through the provider-event inbox identity and payload hash.
- A partial refund with `amount_minor <= 0` cannot alter payment truth.
- A partial refund whose cumulative total would reach or exceed the order amount cannot be represented as partial; it is reconciled and a semantic `payment.refunded` event is required for terminal full-refund truth.
- An event currency mismatch, full-refund amount mismatch, provider-object rebinding or unknown order fails closed into reconciliation.
- A legacy `partially_refunded` order without durable cumulative refund history is not guessed from state; it fails closed.
- Concurrent distinct partial refunds are serialized by the locked order row and preserve both deltas.
- Post-terminal partial refunds cannot regress full-refund truth.

## Findings

No new material defect is proven in this bounded refund-settlement slice.

Deduplicated related work:

- OPA-SEC-0002 / Issue #797: repaired by PR #826; this audit verifies the repaired behavior rather than opening a duplicate.
- Issue #321: remains the owner for real payment-provider integration and production/customer-facing acceptance; this audit does not claim that production payments are complete.
- Baseline payment/product inventory Issues remain historical/product-completeness context and are not duplicated by a PASS-only post-repair audit.

## Existing implementation validation evidence

PR #826 implementation head `cd4a47ae025ed397a52441b9c12a8e2f44dd9664` carried successful payment-relevant validation:

- CI run `31190262706`: SUCCESS.
- `composer validate --strict`: PASS.
- `composer audit --no-interaction`: PASS with no security advisories reported.
- Pint: PASS across 703 files.
- PHPStan / `composer analyse`: PASS with no errors.
- Runtime test job: PASS, including `PaymentOrderStateMachineTest`, `PaymentEventConcurrencyMariaDbTest`, `PaymentEventCoreTest`, `PaymentPartialRefundConcurrencyMariaDbTest`, and `PaymentPartialRefundIntegrityTest`.
- Portal Acceptance, Deep System Validation, Game Auth Ticket Concurrency, Platform DB Outage Validation, Edge Security Emulation, Acceptance E2E / Visual UX, Portal Exhaustive Audit, Phase 7 Production-Like Validation, Synology build and Agent Governance all concluded successfully on that implementation head.

The native-protocol contract-audit failure on that historical head was an unrelated CI routing false positive, subsequently tracked and repaired through Issue #829 / PR #834. It is not payment evidence and is not reopened here.

Current PR base is `152f36c10d765b105bbed77e46c3d6022c4e65a6`. The audit originally selected `3109d5e15e98c9c463130dc736db90667ab83c9a`; before PR validation, `main` advanced by one merged portal-evidence change. The `3109d5e...152f36c` delta contains no audited Payments runtime, test or ADR path. Comparing the PR #826 merge `fe5a177af64d28ab4a2780d7ceb629502a257a80` through the current PR base therefore leaves the repaired Payments slice unchanged.

## Acceptance criteria

- [x] Live Issues, active tasks, open PRs and recent `main` changes were refreshed before selecting the domain.
- [x] The selected domain is high-risk, recently repaired and non-overlapping with live owned work.
- [x] Domain action, state machine, persistence truth, replay, negative paths, concurrency and contract semantics were inspected.
- [x] Existing exact implementation-head CI evidence was checked and payment-relevant tests passed.
- [x] Every related known issue was deduplicated against existing ownership.
- [x] No new material finding requiring a new audit Issue was proven.
- [x] No runtime/product fix was made by the auditor.
- [x] Product E2E is evidence-backed by PR #826 acceptance workflows; audit-document E2E is `NOT_APPLICABLE` because this PR changes only durable audit evidence.
- [ ] Exact-head CI / Agent Governance for this audit-record PR pass and PR hygiene is clean before merge.
- [ ] Lifecycle closeout archives this task and updates the continuous-audit programme state after merge.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T17:21:30Z
head: 1c4162e50f9ca98efcb8dbd9fe8f05fdaddea3bc
branch: audit/payments-refund-settlement-integrity-20260807
pr: 838
status: validating
context_routes:
  - agent-governance
  - continuous-audit
  - payments
  - security
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payments-refund-settlement-audit.md
proven:
  - Issue #797 is closed through merged PR #826 and its repaired payment paths are unchanged through the current PR base.
  - Repeated partial refunds are represented as event delta plus locked cumulative refunded total instead of collapsing to a state-only no-op.
  - Exact replay remains idempotent while malformed, over-refund, mismatch and legacy-unknown paths fail closed to reconciliation.
  - MariaDB concurrency evidence serializes distinct partial refunds without losing either refund delta.
  - No new material defect is proven in the bounded post-repair refund-settlement slice.
derived:
  - The repaired core refund-truth contract is internally coherent at the audited PR base, while real-provider production completion remains separately gated by Issue #321.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-run-31201843416
  evidence: first PR-head run failed live liveness because this task still recorded pr none; the same run also reported unrelated terminal PR #821 task archival lag.
rejected_hypotheses:
  - The repaired same-state partial-refund transition still discards a second distinct refund; rejected by source inspection plus integrity and MariaDB concurrency evidence.
  - A full-refund event after prior partial refunds necessarily double-counts canonical refund truth; rejected because ADR 0021 defines the full-refund amount as cumulative terminal truth and `refunded_total_minor` as the canonical cumulative field.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-payments-refund-settlement-audit.md
validation:
  - command: PR #826 exact implementation-head CI run 31190262706
    result: PASS
    evidence: payment-relevant static analysis and runtime tests passed on cd4a47ae025ed397a52441b9c12a8e2f44dd9664.
  - command: Agent Governance run 31201843416
    result: FAIL
    evidence: task PR identity was omitted on the first head and is corrected in this checkpoint; the run also exposed unrelated Issue #491 / PR #821 archival lag on main.
  - command: audit-document product E2E
    result: NOT_APPLICABLE
    evidence: this audit task changes durable evidence only; the audited product implementation already has acceptance E2E evidence on PR #826.
blockers:
  - none
next_action: require exact-head CI and Agent Governance on PR #838; if the unrelated PR #821 archival lag remains, wait for its owning lifecycle closeout rather than mutating that task from this audit
```

## Safety

Repository-only, read-mostly audit. No production deployment, protected-environment operation, secret access, external repository mutation or payment-provider action was performed.