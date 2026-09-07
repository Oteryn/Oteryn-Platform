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
- [x] Exact-head CI, overlap reconciliation, protected integration/readback and lifecycle closeout complete.

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
  - docs/agents/tasks/archive/OTERYN-20260907-task-inventory-path-d26.md
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
updated_at: 2026-09-07T10:02:00Z
head: e8c0e5ea1d2d9faa469ca1b053698427ecdce692
branch: governance/meta-agent-policy-v3-1301
pr: 1304
status: completed
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
  - docs/agents/tasks/archive/OTERYN-20260907-task-inventory-path-d26.md
proven:
  - Platform main 3557085c20512d25576d8884cc54471665784b00 includes merged D26 repair #1300.
  - META policy 3.0.0 and the corrected routing/continuation/access machine authorities are bound at protected-main commit 5ed3f14400af450b5875c091e443da70f2d67ab9.
  - Platform admission had no META_AGENT_POLICY_BINDING.json and the central provider validator rejected its parallel-first directive.
  - All ten active reusable prompts are semantically migrated to task/domain deltas with stable named invariants; historical entries remain inert.
  - Published PR #1304 head 48c19ad5c74ca933e057e705ade7b5b4a1acec12 has tree 3934259316b65b251a82710284fa02e15859a62b, byte-identical to reviewed local commit bad2e5a2bcf5193ecb6418a21526d062e49115bc.
  - D26 PR #1300 is merged on protected main as 3557085c20512d25576d8884cc54471665784b00; its terminal Issue #1299 is closed and its stale packet is archived in this lifecycle repair.
  - W5 bounded matched screening accepted all 16/16 baseline/candidate executions across four fresh gpt-5.6-sol medium threads, including three safety repeats per arm and representative remediation-prompt delivery; durable PR evidence is comment 5568582769.
  - Exact candidate head e8c0e5ea1d2d9faa469ca1b053698427ecdce692 passed Agent Governance, CI, CodeQL and all other selected PR workflows, then #1304 squash-merged through the protected path as main 907546f193e91b0bed2f5f077ab5b874771929ef with tree d844c6c0e568896008ea8053057bcae53fb4b949.
  - The adoption source ref is absent after merge; terminal canary cleanup was transferred to open Issue #1305 as separate non-adoption work.
derived:
  - Closed-unmerged PR #1270 was a direct stale AGENTS.md overlap whose intended anti-stall semantics are already represented on later protected main and central policy.
unknown: []
conflicts: []
first_failure:
  marker: hosted-lifecycle-and-codeql
  evidence: Initial PR #1304 Agent Governance found omitted PR identity and stale D26 packet; CodeQL rejected a candidate-derived META checkout ref.
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
next_action: none
```

## External cleanup handoff

The two terminal validation-canary refs are not adoption acceptance work. Their exact heads, closed-unmerged PR evidence and guarded deletion procedure moved to open Issue #1305 and its dedicated task. No deletion or exemption is claimed here.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository adoption branch; ordinary delete-after-merge applies.
source_branch_evidence: PR #1304 head e8c0e5ea1d2d9faa469ca1b053698427ecdce692 squash-merged as protected main 907546f193e91b0bed2f5f077ab5b874771929ef; live source-ref readback is absent.
```

## Notes

This is one cohesive provider-adoption contract. A second writer would overlap the same binding/bootstrap/validator/workflow semantics and add integration cost, so the task uses one agent and one isolated workspace.
