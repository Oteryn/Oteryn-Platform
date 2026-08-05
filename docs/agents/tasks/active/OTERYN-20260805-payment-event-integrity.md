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
updated_at: 2026-08-05T19:52:00Z
head: 4f311efb2ff1ee21d3b03d2b9db398e3bc407efe
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
  - The verified provider-event contract carries authenticated currency and minor-unit amount facts.
  - Settlement and provider-object mismatches are reconciled before any payment-order transition.
  - Provider-object matching resolves the exact same-order and same-provider attempt before inspecting conflicting references.
  - Production payments and public webhook activation remain disabled and no forbidden product paths changed.
  - Agent Governance passed on implementation head 4f311efb2ff1ee21d3b03d2b9db398e3bc407efe.
derived:
  - The bounded implementation satisfies the code-level acceptance shape pending final exact-head automated and independent security validation.
unknown:
  - Exact-head required CI conclusion for the final waiting checkpoint.
  - Independent security-review disposition for the high-risk financial-integrity change.
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-validation
  evidence: Workflow run 31040823616 rejected unsupported custom keys inside the initial active-task checkpoint block; corrected run 31041191905 passed.
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
  - command: GitHub Actions Agent Governance run 31041191905 on 4f311efb2ff1ee21d3b03d2b9db398e3bc407efe
    result: PASS
    evidence: Governance contract tests and all active-task checkpoint validations completed successfully.
  - command: Pull request 595 changed-path and scope inspection
    result: PASS
    evidence: The pull request changes only the declared payment, regression-test, architecture, operations and agent-governance paths; production activation surfaces remain untouched.
  - command: Exact-head required CI and payment regression suite
    result: NOT_RUN
    evidence: GitHub Actions must validate the final waiting-checkpoint head created after this task update.
blockers:
  - Independent security review is required before accepting or merging this high-risk payment-integrity repair.
  - Final exact-head GitHub Actions checks must complete successfully.
next_action: Keep the claim active while pull request 595 completes final exact-head CI and receives independent security approval, then merge and perform terminal lifecycle cleanup.
```
