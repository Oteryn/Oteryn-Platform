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
updated_at: 2026-08-05T19:49:00Z
head: ae17585ff5f801fcaebf1796d27cbf4d69dc089e
branch: repair/issue-547
pr: 595
status: validating
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
  - Issue 547 is implementation-authorized, unblocked and exclusively claimed by branch repair/issue-547 and pull request 595.
  - The verified provider-event contract now carries authenticated currency and minor-unit amount facts.
  - Settlement and provider-object mismatches are reconciled before any payment-order transition.
  - Production payments and public webhook activation remain disabled and no forbidden product paths changed.
derived:
  - The bounded implementation satisfies the code-level acceptance shape pending exact-head automated and independent security validation.
unknown:
  - Exact-head automated validation result after this checkpoint correction.
  - Independent security-review disposition for the high-risk financial-integrity change.
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-validation
  evidence: Workflow run 31040823616 rejected unsupported custom keys inside the active-task checkpoint block.
rejected_hypotheses:
  - The product implementation caused the first workflow failure; the failed step was active-task checkpoint validation before product tests ran.
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
  - command: GitHub Actions Agent Governance run 31040823616 on ae17585ff5f801fcaebf1796d27cbf4d69dc089e
    result: FAIL
    evidence: Validate active task checkpoints failed because the checkpoint used fields outside governance contract version 1; this commit removes them from the checkpoint block.
  - command: Exact-head required CI and payment regression suite
    result: NOT_RUN
    evidence: A new exact head is required after correcting the governance checkpoint.
blockers:
  - Independent security review is required before accepting or merging this high-risk payment-integrity repair.
next_action: Validate the corrected exact head in GitHub Actions, resolve any implementation failures, then obtain independent security approval for pull request 595.
```
