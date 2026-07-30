---
task_id: OTERYN-20260730-audit-remediation-agent-preparation
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
search_first:
  - open audit/remediation tasks and pull requests, especially PR #315 and PR #328
  - existing standalone implementation-agent prompts under docs/agents/prompts
  - open issues linked from the product-completeness audit
optional_reads:
  - docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29.md after PR #315 merges
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md after PR #315 merges
  - docs/testing/PRODUCT_COMPLETENESS_AUDIT_2026-07-29_VALIDATION.md after PR #315 merges
---

# OTERYN-20260730-audit-remediation-agent-preparation

## Goal

Prepare a durable standalone implementation-agent prompt that consumes confirmed findings from the product-completeness audit and delivers them as separate, reviewable remediation tasks without duplicating active work or weakening repository safety gates.

## Acceptance criteria

- [x] A standalone remediation-agent prompt exists under `docs/agents/prompts/`.
- [x] The prompt treats the merged audit report, frontend integration audit, machine benchmark ledger, live issues, task records, PRs and exact-SHA evidence as authoritative rather than chat history.
- [x] The agent must select exactly one bounded remediation issue at a time and create a dedicated task branch, active task record and early draft PR.
- [x] The agent searches active tasks and open PRs before claiming paths and does not overlap PR #328 or another live implementation.
- [x] Required gaps are prioritized over planned and optional findings, with blockers and dependencies respected.
- [x] Writes remain limited to `blakinio/Oteryn-Platform`; Canary and other repositories remain read-only without explicit current-task authorization.
- [x] Payment-provider work, production deployment, production activation and irreversible external mutations remain blocked without explicit authorization and required architecture/security evidence.
- [x] Every implementation slice requires focused tests, exact-final-head CI, truthful benchmark reconciliation and task archival after merge.
- [x] User-facing capability closure requires backend/domain behavior, reachable frontend integration and applicable zero-retry browser evidence.
- [x] The prompt does not authorize a single broad PR covering unrelated audit findings.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260730-audit-remediation-agent-preparation.md
modules:
  - Agent Governance
  - Product Completeness
  - Testing
dependencies:
  - PR #315 product-completeness and frontend integration audit reconciliation
  - PR #328 character-rename contract discovery must not be duplicated
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T06:31:00Z
head: f4d60d7a6b05c0b6eb692341356c987f02fd1b46
branch: docs/OTERYN-20260730-audit-remediation-agent
pr: 336
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260730-audit-remediation-agent-preparation.md
proven:
  - The product-completeness audit PR #315 has an exact-head evidence history, creates bounded remediation issues and now includes a dedicated backend/frontend integration addendum; it is not yet merged.
  - PR #328 actively owns read-only discovery and architecture for the character-rename contract.
  - Repository policy requires one bounded task branch and early draft PR per substantial implementation.
  - PR #336 contains only the standalone remediation prompt and its dedicated preparation task record.
  - The prompt enforces sequential bounded delivery, dynamic ownership checks, three-layer user-facing completion evidence and explicit Canary/payment/production stop conditions.
derived:
  - A remediation coordinator must dispatch one issue at a time rather than own broad runtime paths itself.
  - The durable prompt can be prepared independently because it owns only its prompt and task record.
unknown:
  - Which audit remediation issue will be the first unblocked implementation after PR #315 merges and current ownership is rechecked.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - One agent should implement every audit finding in one PR; rejected because it violates bounded-task, ownership and reviewability rules.
  - A backend-only change may close a user-facing audit finding; rejected by the audit's backend/frontend/integrated-browser evidence rule.
  - The remediation prompt may authorize Canary or production writes implicitly; rejected by repository safety policy.
changed_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260730-audit-remediation-agent-preparation.md
validation:
  - command: focused governance, ownership and Markdown content review
    result: PASS
    evidence: prompt follows the existing standalone-agent pattern, owns only two non-overlapping documentation paths and records exact safety, frontend-integration and merge gates; exact-head GitHub checks are running on PR #336.
blockers:
  - none
next_action: Observe required checks on the exact PR #336 head, fix any governance defect, then merge and archive this preparation task before launching the remediation agent.
```

## Notes

This task prepares the reusable agent entry point only. It does not implement an audit finding, mutate Canary, select a payment provider or deploy to production.
