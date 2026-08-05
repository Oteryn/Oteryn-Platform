---
task_id: OTERYN-20260805-payment-event-integrity
programme_id: OTERYN_PLATFORM_REMEDIATION
repository: blakinio/Oteryn-Platform
issue: 547
branch: repair/issue-547
pull_request: 595
session_id: chatgpt-20260805T2127+0200
claim_nonce: issue-547-bc9f64ac-20260805T1927Z
coordination_key: module:payment-event-integrity
lease_expires_at: 2026-08-05T21:27:00Z
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
updated_at: 2026-08-05T20:27:00Z
head: 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e
branch: repair/issue-547
pr: 595
status: waiting
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
proven:
  - Issue 547 is implementation-authorized and remains exclusively claimed by branch repair/issue-547 and pull request 595.
  - The verified provider-event contract carries authenticated currency and positive minor-unit amount facts.
  - Settlement and provider-object mismatches are reconciled before any payment-order transition.
  - Provider-object matching resolves the exact same-order and same-provider attempt before inspecting conflicting references.
  - Production payments and public webhook activation remain disabled and no forbidden product paths changed.
  - Merge commit 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e incorporates current main 3efcae79ed55a159f46bb9ffa3904dc81a2a3b1d without path overlap.
  - Required exact-head CI classify-changes and test passed on 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e.
  - Independent AUDIT ONLY Issue 597 is open and agent:ready for a separate validator.
derived:
  - The bounded implementation satisfies the code-level acceptance shape pending only fresh independent security validation.
unknown:
  - Independent security-review disposition for pull request 595 from Issue 597.
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-validation
  evidence: Workflow run 31040823616 rejected unsupported custom keys inside the initial active-task checkpoint block; corrected and subsequent governance runs passed.
rejected_hypotheses:
  - The product implementation caused the initial workflow failure; the failed step was active-task checkpoint validation before product tests ran.
changed_paths:
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
validation:
  - command: GitHub Actions Agent Governance run 31043840961 on 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e
    result: PASS
    evidence: Governance contract tests and all active-task checkpoint validations completed successfully.
  - command: GitHub Actions CI run 31043841017 on 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e
    result: PASS
    evidence: classify-changes, Composer validation, dependency audit, formatting, static analysis and the full automated test suite passed.
  - command: GitHub Actions Edge Security Emulation run 31043841044 on 09fd84ea70a6729e3608fc96fd2d37a21bfbb56e
    result: PASS
    evidence: The repository security-emulation workflow completed successfully.
  - command: Real end-to-end applicability assessment
    result: NOT_APPLICABLE
    evidence: This is an internal backend-only provider-neutral payment event core with no public ingress or customer payment flow; production and webhook activation remain disabled, while feature and MariaDB integration tests exercise the persisted domain outcome.
  - command: Fresh independent security audit
    result: NOT_RUN
    evidence: Issue 597 is open and agent:ready; the implementing session is forbidden from serving as the independent final validator.
blockers:
  - A separate agent or person must complete Issue 597 and submit an approving review with zero open material findings on pull request 595.
next_action: Have a separate agent or person claim Issue 597 and audit pull request 595; remediate any material findings, then merge and perform terminal lifecycle cleanup.
```
