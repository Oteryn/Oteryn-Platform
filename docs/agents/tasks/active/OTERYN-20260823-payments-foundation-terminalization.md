---
task_id: OTERYN-20260823-payments-foundation-terminalization
status: ready
agent: ChatGPT autonomous payments foundation owner
project_lane: oteryn-platform-core
task_kind: implementation
product_issue: 321
successor_issue: 1236
risk: medium
completion_claim: lifecycle_reconciliation
created: 2026-08-23T09:31:50+02:00
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
---

# Payments foundation terminalization

## Goal

Reconcile the now-delivered provider-neutral Payments foundation with live routing and product ownership: make `OTERYN-PAYMENTS-FOUNDATION` terminal, move real-provider/sandbox/production work to Issue #1236, and close Issue #321 only after exact-head documentation/governance validation passes.

## Acceptance criteria

- [x] Canonical foundation prompt cannot start another implementation of completed #321 work.
- [x] Issue #1236 is the explicit successor for real-provider, sandbox, legal/compliance, operational and production-activation gates.
- [x] Canonical project/active-work/ADR/operations documentation distinguishes terminal foundation from blocked real-provider completion.
- [x] Historical foundation evidence remains truthful and points forward without rewriting past state.
- [x] Prompt contract/eval coverage rejects re-execution and preserves payment safety boundaries.
- [x] `git diff --check`, checkpoint validation and applicable docs/governance tests pass.
- [ ] Exact-head required GitHub checks pass with zero unresolved review findings.
- [ ] PR is squash-merged, #321 is closed completed, task is archived and source branches are removed.

## Ownership

```yaml
owned_paths:
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/archive/OTERYN-20260822-payments-foundation.md
  - docs/agents/tasks/active/OTERYN-20260823-payments-foundation-terminalization.md
modules:
  - Payments
  - AgentGovernance
dependencies:
  - PR #1228 merge 788f58c031bf575396231a95b6a9d28afbadb67c
  - PR #1231 closeout 47c626a5e9295dd2a12e8f295d2b57bc0a53f0f9
  - successor Issue #1236
blockers:
  - none
cross_repository_tasks:
  - none
```
## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-23T09:42:46+02:00
head: 0d3be3c70979f257b00360a3fbd0f3c2a8e64622
branch: docs/payments-foundation-terminal-1236
pr: 1238
status: ready
phase: final_exact_head_ci
session_id: chatgpt-20260823T0923+0200
session_role: implementer
execution_mode: remote-terminal-plus-github
execution_reason: isolated clean worktree permits narrow documentation/governance reconciliation while GitHub remains authoritative for Issue and PR state
context_routes:
  - payments
  - agent-governance
  - architecture
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: full
owned_paths:
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/archive/OTERYN-20260822-payments-foundation.md
  - docs/agents/tasks/active/OTERYN-20260823-payments-foundation-terminalization.md
proven:
  - PR #1228 and closeout PR #1231 are merged and the previous foundation task is archived.
  - Issue #1236 exists and owns the unresolved real-provider/sandbox/production gate.
  - No active Payments task or open Payments implementation PR overlaps this reconciliation.
  - Protected main advanced first to 095017ba031bace2794865275c717422d11d82bc and then b5635f262ff2ab859155c8c62af47d05a11f6c6 with only Native Game Catalog task paths; the task branch was rebased after each advance with zero path overlap.
  - PR #1238 is the sole terminalization PR and has zero review submissions and zero review threads.
  - Accidental placeholder Issue #1237 was immediately closed `not_planned` and grants no authority or work ownership.
derived:
  - Re-running the foundation alias would duplicate terminal work; it should become a tombstone/status route rather than redirecting into provider integration.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Issue #321 must remain the permanent owner of real-provider work after the repository foundation is terminal.
changed_paths:
  - docs/agents/prompts/OTERYN-PAYMENTS-FOUNDATION-AGENT.md
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/architecture/adr/0021-provider-neutral-payment-security-core.md
  - docs/operations/PAYMENTS_SECURITY_FOUNDATION.md
  - docs/agents/tasks/archive/OTERYN-20260822-payments-foundation.md
  - docs/agents/tasks/active/OTERYN-20260823-payments-foundation-terminalization.md
validation:
  - command: python tools/validation/prompt_eval.py --suite docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
    result: PASS
    evidence: 11 cases / 11 categories / 4 safety-critical cases; deterministic contract only
  - command: python tools/validation/test_prompt_eval.py
    result: PASS
    evidence: 8 tests passed
  - command: python tools/validation/adr_registry.py
    result: PASS
    evidence: 49 ADRs; preserved legacy duplicate prefixes accepted
  - command: python tools/agents/checkpoint.py current-task --require-checkpoint
    result: PASS
    evidence: context checkpoint validated against contract v1
  - command: git diff --check
    result: PASS
    evidence: no whitespace errors on exact content head 0dbc2097771540c6d74cef6670aabf30a9eb666a
  - command: canonical stale #321 scan
    result: PASS
    evidence: no stale #321-open/incomplete routing markers remain in canonical prompt, registry, state, ADR or operations files
  - command: browser/runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation, governance and architecture-state reconciliation only; no executable Payments behavior changed from PR #1228
blockers:
  - none
next_action: Push the documentation-only readiness checkpoint, mark PR #1238 ready, then verify required exact-head GitHub checks before squash merge.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 0d3be3c70979f257b00360a3fbd0f3c2a8e64622
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - exact 9-path diff inspected after rebase onto main@095017ba031bace2794865275c717422d11d82bc
    - prompt safety contract, prompt evaluator tests, ADR registry, checkpoint validator and diff check PASS
    - canonical stale-routing scan PASS; Issue #1236 exists and remains the explicit blocked successor
    - PR #1238 has zero review submissions and zero review threads
    - rollback is a documentation-only revert to the prior routing state; no runtime or financial data changes
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

## Notes

This task changes no payment runtime, schema, provider integration, production configuration, secrets or customer financial behavior. E2E is `NOT_APPLICABLE` because the delivered browser/payment behavior is unchanged; this task reconciles routing, durable state and Issue ownership only.

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: chatgpt-20260823T0923+0200
  session_started_at: 2026-08-23T09:23:00+02:00
  checkpointed_at: 2026-08-23T09:51:59+02:00
  last_progress_at: 2026-08-23T09:51:59+02:00
  phase: final_exact_head_ci
  exact_head: 0d3be3c70979f257b00360a3fbd0f3c2a8e64622
  pull_request: 1238
  active_operation: required exact-head GitHub CI and resulting authorized squash merge
  external_run_ids:
    - 32626367827
    - 32626367707
  operation_started_at: 2026-08-23T09:51:59+02:00
  wait_deadline_at: 2026-08-23T10:30:11+02:00
  check_generation: current-base-2
  checks_used: 0
  status: waiting
  safe_to_resume: true
  resume_condition: PR #1238 exact-head required checks complete without material findings
  next_action: Observe the aggregate required-check state after the minimum terminal-CI interval; repair any failure, otherwise squash-merge exact head and complete Issue/task closeout.
```
