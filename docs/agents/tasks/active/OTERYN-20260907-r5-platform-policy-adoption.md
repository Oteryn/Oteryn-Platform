---
task_id: OTERYN-20260907-r5-platform-policy-adoption
governing_issue: 1302
required_reads: []
search_first: []
optional_reads: []
---

# R5 Platform policy adoption

## Goal

Adopt META organization policy 3.0.0 through one immutable binding, reduce duplicated Platform instruction/prompt surfaces, migrate their validators, and reconcile D26 lifecycle without changing product runtime behavior.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T09:02:00Z
head: 3c197b9f513aad2674f2999d0316af4b000cb71e
branch: docs/r5-platform-policy-adoption-1302
pr: 1303
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - README.md
  - .github/workflows/agent-governance.yml
  - docs/agents/AGENTS.md
  - docs/agents/README.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/evals/prompt-contract-v2.json
  - docs/agents/evidence/OTERYN-20260907-r5-platform-instruction-reduction.md
  - docs/agents/prompts/OTERYN-HISTORICAL-WORK-RECONCILIATION.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
  - docs/agents/tasks/archive/OTERYN-20260907-task-inventory-path-d26.md
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - tools/validation/prompt_eval.py
  - tools/validation/test_prompt_eval.py
proven:
  - Protected admission main is 3557085c20512d25576d8884cc54471665784b00 with required platform-gate.
  - META policy 3.0.0 is on protected META main 5ed3f14400af450b5875c091e443da70f2d67ab9.
  - Issue #1302 owns this exact bounded Platform consumer migration and coordinates with broader #1009.
  - D26 code and focused regressions are integrated through PR #1300; only lifecycle archival remains.
  - Draft PR #1303 publishes the complete candidate from Issue #1302.
derived:
  - The provider overlay, binding and consuming validators must change together to avoid retaining prose duplication as a gate.
unknown:
  - Exact final candidate head after this task-record reconciliation, hosted check results, independent review disposition and integration outcome.
conflicts: []
first_failure:
  marker: duplicated-platform-operating-policy
  evidence: Root, bootstrap and nested instructions repeated shared policy while policy_consistency.py required copied status, budget and prose markers.
rejected_hypotheses:
  - Shortening root alone would complete adoption while nested/bootstrap consumers still required the copies.
changed_paths:
  - .github/workflows/agent-governance.yml
  - AGENTS.md
  - README.md
  - docs/agents instruction, prompting, evaluation and lifecycle surfaces listed in owned_paths
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - tools/validation/prompt_eval.py
  - tools/validation/test_prompt_eval.py
validation:
  - command: focused Agent Governance unit suites and validators
    result: PASS
    evidence: 84 unit tests passed, including malicious-validator pre-import rejection and the unchanged Portal Completion scope-manifest contract; prompt contract reported 11 cases, 7 categories, 4 safety-critical cases and model_trials_executed=0; Documentation IA and workflow trigger economy passed.
  - command: bound META central validator with protected-main resolver evidence
    result: PASS
    evidence: META bundle, binding, provider overlay, checkout identity and Platform invariants passed against 5ed3f14400af450b5875c091e443da70f2d67ab9.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: This change affects agent instructions, validators and task lifecycle only; it does not modify product runtime behavior.
blockers: []
next_action: Finish exact-head validation and review; after any later protected integration, archive this task and close Issues #1302 and #1299.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: The task PR is not yet integrated.
source_branch_evidence: Branch docs/r5-platform-policy-adoption-1302 is the active Issue #1302 candidate.
```
