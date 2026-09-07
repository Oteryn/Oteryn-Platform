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
updated_at: 2026-09-07T09:40:00Z
head: 48c19ad5c74ca933e057e705ade7b5b4a1acec12
branch: governance/meta-agent-policy-v3-1301
pr: 1304
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
  - docs/agents/tasks/archive/OTERYN-20260907-task-inventory-path-d26.md
proven:
  - Platform main 3557085c20512d25576d8884cc54471665784b00 includes merged D26 repair #1300.
  - META policy 3.0.0 and the corrected routing/continuation/access machine authorities are bound at protected-main commit 5ed3f14400af450b5875c091e443da70f2d67ab9.
  - Platform admission had no META_AGENT_POLICY_BINDING.json and the central provider validator rejected its parallel-first directive.
  - All ten active reusable prompts are semantically migrated to task/domain deltas with stable named invariants; historical entries remain inert.
  - Published PR #1304 head 48c19ad5c74ca933e057e705ade7b5b4a1acec12 has tree 3934259316b65b251a82710284fa02e15859a62b, byte-identical to reviewed local commit bad2e5a2bcf5193ecb6418a21526d062e49115bc.
  - D26 PR #1300 is merged on protected main as 3557085c20512d25576d8884cc54471665784b00; its terminal Issue #1299 is closed and its stale packet is archived in this lifecycle repair.
  - W5 bounded matched screening accepted all 16/16 baseline/candidate executions across four fresh gpt-5.6-sol medium threads, including three safety repeats per arm and representative remediation-prompt delivery; durable PR evidence is comment 5568582769.
derived:
  - Closed-unmerged PR #1270 was a direct stale AGENTS.md overlap whose intended anti-stall semantics are already represented on later protected main and central policy.
unknown:
  - Exact final candidate CI and merge-queue outcome.
  - Exact-SHA deletion readback for the two terminal validation-canary refs recorded below.
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
  - none for central-policy adoption; external canary cleanup remains separately waiting below.
next_action: Publish the lifecycle, W5 evidence and fixed-trust-anchor repair, then qualify the exact head; pursue the separate reviewed cleanup route recorded below after adoption.
```

## Pending external branch cleanup

Cleanup state: `WAITING_REVIEWED_CLEANUP_PR`. This cleanup debt is not part of the central-policy adoption acceptance claim. Historical Branch Audit run `34104877215`, job `101687493437`, artifact `10011942363` found two terminal validation-only canary branches and fixed their exact remote heads. A fresh 2026-09-07 branch search confirms both named refs still exist. Each has a closed, unmerged PR whose body says it must not merge. Direct connector deletion and authenticated Git publication are unavailable here, but the existing protected-main `terminal-branch-lifecycle` workflow is an authorized exact-SHA route. These interim active claims keep the refs visible to branch hygiene without classifying them as deleted, merged, or exempt until a separate cleanup PR is reviewed and integrated.

```yaml
lock_branch: test/codex-native-publish-canary-20260903-platform
```

- Exact head: `d866d21ca856b925a32a3a5457174906237db199`
- Closed unmerged PR: #1290
- Recovery evidence: the commit changes only `CODEX_NATIVE_PUBLISH_CANARY.md` and remains addressable by the recorded SHA.

```yaml
lock_branch: test/codex-shared-pat-canary-20260903-platform
```

- Exact head: `65126f2b1d24e0fe497e436be99f524104021033`
- Closed unmerged PR: #1291
- Recovery evidence: the commit changes only `CODEX_SHARED_PAT_CANARY.md` and remains addressable by the recorded SHA.

The separate cleanup PR must remove these interim claims, use its PR dry-run artifact to bind the complete live `TERMINAL_CLOSED_UNMERGED` candidate digest in canonical `TERMINAL_BRANCH_DELETION_APPROVAL.json`, and receive normal review before protected integration. The existing main-push workflow then rematerializes the live manifest and refuses active claims, SHA drift, protection, reserved names, open PRs, retention metadata or policy/digest drift before deletion. Absence readback remains required. Ref repointing, merging either canary, changing the controller to force eligibility, or treating this record as a validation exception is forbidden.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository adoption branch; ordinary delete-after-merge applies.
source_branch_evidence: Pending protected merge and source-ref readback.
```

## Notes

This is one cohesive provider-adoption contract. A second writer would overlap the same binding/bootstrap/validator/workflow semantics and add integration cost, so the task uses one agent and one isolated workspace.
