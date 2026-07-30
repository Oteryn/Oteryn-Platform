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
- [x] PR #336 merged and this preparation task is archived separately.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/archive/OTERYN-20260730-audit-remediation-agent-preparation.md
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
updated_at: 2026-07-30T06:40:00Z
head: 0364834d5541f512bed9a5dedf9b149f06513c37
branch: docs/OTERYN-20260730-audit-remediation-agent
pr: 336
status: completed
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/archive/OTERYN-20260730-audit-remediation-agent-preparation.md
proven:
  - PR #336 introduced the durable audit-remediation implementation-agent prompt and changed only its prompt and preparation task record.
  - The prompt consumes the merged product audit, frontend integration addendum, machine benchmark ledger, live issues, task records, PRs and exact-SHA evidence.
  - It enforces one bounded task/branch/PR at a time, dynamic ownership checks and explicit prioritization of required gaps.
  - It requires backend/domain behavior, reachable frontend integration and applicable zero-retry browser evidence before a user-facing capability can be marked implemented.
  - It keeps Canary and external repositories read-only without explicit current-task authorization and blocks payment or production activation without required decisions and evidence.
  - Final exact head 0364834d5541f512bed9a5dedf9b149f06513c37 passed all six required workflows.
  - PR #336 was squash-merged as cae57033ddc32f1c243bb19104cf56a5ce71d080.
derived:
  - The remediation agent is ready to be launched after it verifies the live audit/ownership state.
  - The agent must skip any issue already owned by another active task, including rename work while PR #328 remains active.
unknown:
  - Which audit remediation issue will be the first unblocked implementation after PR #315 merges and current ownership is rechecked.
conflicts: []
first_failure:
  marker: none
  evidence: All required exact-final-head workflows passed before merge.
rejected_hypotheses:
  - One agent should implement every audit finding in one PR; rejected because it violates bounded-task, ownership and reviewability rules.
  - A backend-only change may close a user-facing audit finding; rejected by the audit's backend/frontend/integrated-browser evidence rule.
  - The remediation prompt may authorize Canary or production writes implicitly; rejected by repository safety policy.
changed_paths:
  - docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md
  - docs/agents/tasks/archive/OTERYN-20260730-audit-remediation-agent-preparation.md
validation:
  - command: Agent Governance run 30519975463
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
  - command: CI run 30519975489
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
  - command: Edge Security Emulation run 30519975461
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
  - command: Game Auth Ticket Concurrency run 30519975471
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
  - command: Platform DB Outage Validation run 30519975490
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
  - command: Phase 7 Production-Like Validation run 30519975501
    result: PASS
    evidence: exact head 0364834d5541f512bed9a5dedf9b149f06513c37
blockers: []
next_action: Launch the remediation agent from `docs/agents/prompts/OTERYN-PRODUCT-AUDIT-REMEDIATION-AGENT-PROMPT.md` after verifying PR #315 and all live ownership state; no further preparation work remains.
```

## Notes

This task prepared the reusable agent entry point only. It did not implement an audit finding, mutate Canary, select a payment provider or deploy to production.
