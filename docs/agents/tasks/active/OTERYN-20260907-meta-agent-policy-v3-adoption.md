---
task_id: OTERYN-20260907-meta-agent-policy-v3-adoption
governing_issue: 1301
required_reads:
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - tools/agents/policy_consistency.py
optional_reads: []
---

# META organization agent policy v3 adoption

## Goal

Issue #1301 is the canonical lifecycle authority. Adopt `OTERYN_ORGANIZATION_AGENT_POLICY@3.0.0` from immutable META commit `5ed3f14400af450b5875c091e443da70f2d67ab9` through a thin Platform binding, bootstrap, deterministic consumer and prompt inventory without changing product or protected live behavior.

## Acceptance criteria

- [x] The binding resolves the merged immutable META policy and canonical human surfaces.
- [x] Platform keeps its repository, product, security, data and protected-operation boundaries while removing competing organization procedures.
- [x] Active reusable prompts pass the central prompt boundary and historical prompts remain inert.
- [x] Agent Governance consumes the central validator and runs focused provider regressions.
- [x] D26 task-inventory path tests and all affected governance checks pass.
- [x] Before/after source volume, W4 inventory, representative validation and W6 follow-up opportunities are recorded without a token-savings claim.
- [ ] Exact-head CI, overlap reconciliation, protected integration/readback and lifecycle closeout complete.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN-PLATFORM-PARALLEL-WAVE-COORDINATOR.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evidence/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260907-meta-agent-policy-v3-adoption.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#142
  - Oteryn/Oteryn@5ed3f14400af450b5875c091e443da70f2d67ab9
blockers:
  - none
cross_repository_tasks:
  - META is read-only authority; this task writes Platform only.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T10:00:00Z
head: 3557085c20512d25576d8884cc54471665784b00
branch: governance/meta-agent-policy-v3-1301
pr: none
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/META_AGENT_POLICY_BINDING.json
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/DOCUMENTATION_IA_CATALOG.json
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/evals/prompt-contract-v1.json
  - docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  - docs/agents/evals/oteryn-portal-parallel-coordinator-prompt-v1.json
  - docs/agents/prompts/OTERYN_PLATFORM_REMEDIATION_PROGRAM.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
  - docs/agents/prompts/OTERYN-CHARACTER-LIFECYCLE-BARRIER-AGENT.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/prompts/OTERYN-PLATFORM-PARALLEL-WAVE-COORDINATOR.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN-PORTAL-COMPLETION-PARALLEL-COORDINATOR-PROMPT.md
  - docs/agents/evidence/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - tools/agents/policy_consistency.py
  - tools/agents/test_policy_consistency.py
  - .github/workflows/agent-governance.yml
  - docs/agents/tasks/active/OTERYN-20260907-meta-agent-policy-v3-adoption.md
proven:
  - Platform main 3557085c20512d25576d8884cc54471665784b00 includes merged D26 repair #1300.
  - META policy 3.0.0 and the corrected routing/continuation/access machine authorities are bound at protected-main commit 5ed3f14400af450b5875c091e443da70f2d67ab9.
  - Platform admission had no META_AGENT_POLICY_BINDING.json and the central provider validator rejected its parallel-first directive.
  - All ten active reusable prompts are semantically migrated to task/domain deltas with stable named invariants; historical entries remain inert.
derived:
  - PR #1270 is a direct stale AGENTS.md overlap whose intended anti-stall semantics are already represented on later protected main and central policy.
unknown:
  - Exact final candidate CI and merge-queue outcome.
conflicts: []
first_failure:
  marker: central-provider-adoption
  evidence: Missing binding plus forbidden parallel-first wording.
rejected_hypotheses:
  - D26 is not part of central adoption; it is already repaired and remains a regression dependency.
changed_paths:
  - all paths declared in the task ownership block
validation:
  - command: python tools/agents/test_policy_consistency.py
    result: PASS
    evidence: Authenticated consumer regression suite passes 13/13 against the exact 5ed3 META checkout.
  - command: python tools/agents/task_issue_liveness.py path-boundary regressions
    result: PASS
    evidence: D26 invalid task-inventory path coverage remains green in the governing-Issue liveness suite.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Governance instructions, validators and CI consumers only; no application, auth, data, payment or deployment behavior changes.
blockers:
  - none
next_action: Complete full validation, independent review, publish the exact candidate and prove protected integration.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository adoption branch; ordinary delete-after-merge applies.
source_branch_evidence: Pending protected merge and source-ref readback.
```

## Notes

This is one cohesive provider-adoption contract. A second writer would overlap the same binding/bootstrap/validator/workflow semantics and add integration cost, so the task uses one agent and one isolated workspace.
