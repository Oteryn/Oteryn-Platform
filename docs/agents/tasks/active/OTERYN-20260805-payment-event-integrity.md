---
task_id: OTERYN-20260805-payment-event-integrity
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 547
branch: repair/issue-547
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/REMEDIATION_WORK_CLAIM_PROTOCOL.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
---

# OTERYN-20260805-payment-event-integrity

Repair the verified payment-provider event contract so settlement-changing events cannot mutate immutable payment truth unless authenticated amount, currency and provider-object facts match the target order.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T19:27:00Z
base_head: bc9f64ac78b7f6483a8b0679c422cf772ca20ad6
head: pending
branch: repair/issue-547
issue: 547
pr: pending
status: claiming
session_id: chatgpt-20260805T2127+0200
claim_nonce: issue-547-bc9f64ac-20260805T1927Z
lease_expires_at: 2026-08-05T21:27:00Z
coordination_key: module:payment-event-integrity
context_routes:
  - security
  - database-persistence
  - api-contracts
owned_paths:
  - app/Payments/Data/VerifiedProviderEvent.php
  - app/Payments/Contracts/PaymentWebhookVerifier.php
  - app/Payments/Infrastructure/DeterministicTestPaymentProvider.php
  - app/Payments/Actions/ProcessPaymentProviderEvent.php
  - tests/Feature/Payments/PaymentEventCoreTest.php
  - tests/Feature/Payments/PaymentEventConcurrencyMariaDbTest.php
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/active/OTERYN-20260805-payment-event-integrity.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
forbidden_paths:
  - app/Wallet/**
  - app/Products/**
  - routes/**
  - resources/**
  - .github/workflows/**
  - database/migrations/**
proven:
  - Issue 547 is implementation-authorized, unblocked, parallel-safe and agent-ready.
  - The verified provider-event contract lacks authenticated settlement currency and minor-unit facts.
  - The processor does not compare authenticated settlement facts with the immutable payment order before transition.
  - Production and public webhook activation are outside scope and remain disabled.
derived:
  - Settlement-changing events must fail closed into reconciliation when authenticated order, amount, currency or object binding differs.
unknown:
  - Exact final validation result until branch CI completes.
conflicts: []
first_failure:
  marker: OPA-SEC-0001
  evidence: Issue 547 exact evidence on audited main
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-payment-event-integrity.md
validation: []
blockers: []
next_action: Open the draft PR, activate the Issue claim, then implement the bounded repair and focused regression coverage.
```
