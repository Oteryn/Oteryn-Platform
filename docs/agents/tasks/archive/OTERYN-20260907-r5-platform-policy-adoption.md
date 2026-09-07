---
task_id: OTERYN-20260907-r5-platform-policy-adoption
governing_issue: 1302
required_reads: []
search_first:
  - docs/agents/tasks/archive/OTERYN-20260907-meta-agent-policy-v3-adoption.md
  - docs/agents/evidence/OTERYN-20260907-meta-agent-policy-v3-adoption.md
optional_reads: []
---

# R5 Platform residual instruction remediation

## Goal

Issue #1302 owns the bounded Platform residuals from the R5 instruction-debt audit. Protected main now contains the central META policy v3 adoption from PR #1304; preserve that implementation and remove only the remaining duplicated execution/review rules and reproduced routing, authority and ownership gaps.

After PR #1303 integrated the seven-path residual, protected META evidence at `Oteryn/Oteryn@cee7df` identified four specialist-controller documents that remained outside that reconciled diff. This follow-up replaces only those four copied global controllers with their already reviewed Platform deltas while preserving the merged root, binding, consumers and prompt/eval implementation.

## Acceptance criteria

- [x] Protected-main PR #1304 binding, trusted consumer, prompt/domain migrations, workflow trust boundary and D26 archive remain unchanged.
- [x] Retry exhaustion is distinguished from missing authority through the bound META state model.
- [x] A protected-main policy change can be reconciled without discarding unaffected work or permitting candidate self-authorization.
- [x] Default instruction loading is bounded to material routes and references.
- [x] Platform delivery, E2E, security and execution-resource cleanup invariants remain available without duplicated global procedures.
- [x] Current live ownership overlap is checked before editing shared paths.
- [x] The seven-path residual from PR #1303 passed exact-head and merge-queue gates and is integrated on protected main.
- [x] `GITHUB_ONLY_EXECUTION.md` no longer grants a competing direct-squash fallback.
- [x] `TERMINAL_ONLY_COMMUNICATION.md` no longer overrides the bound communication policy.
- [x] `AUTONOMOUS_PROGRAM_CONTINUATION.md` and `PROMPTING_HANDOVER.md` retain only Platform-specific programme and handoff deltas.
- [x] Focused validation, exact-head hosted checks and protected integration are complete; this lifecycle-only carrier archives the packet and closes the Issue.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/tasks/archive/OTERYN-20260907-r5-platform-policy-adoption.md
modules:
  - agent-governance
dependencies:
  - Oteryn/Oteryn#142
  - Oteryn/Oteryn-Platform#1009
  - Oteryn/Oteryn-Platform#1304
blockers:
  - none
cross_repository_tasks:
  - META is read-only policy authority; this task writes Platform only.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T11:56:00Z
head: 796209ead741041e8da943fc8c3846672ac96475
branch: docs/r5-platform-policy-closeout-1302
pr: none
status: completed
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/tasks/archive/OTERYN-20260907-r5-platform-policy-adoption.md
proven:
  - Protected main b0f268d682b1f9117140168a3a9d9b86c56ba9cc contains the accepted PR #1304 policy adoption and the completed #1305 cleanup closeout from PRs #1306-#1308.
  - PR #1304 owns the META binding, trusted policy consumer, prompt inventory migration, workflow trust boundary and D26 archive; this task preserves those paths unchanged.
  - Issue #1302 and draft PR #1303 own this remaining bounded R5 delta and coordinate with #1009.
  - Issue #1299 closed externally at 2026-09-07T09:26:20Z; this task does not reactivate or re-close it.
  - Exact PR #1303 head e711378ce6bb3d1c54ba2c33c56f4fe957f261e9 passed Agent Governance run 34112609317 and CI run 34112609162, then merge-group CI run 34117619196 before protected integration as main 2c3934fece8e072f23ea33ad82f4b4399fc3b45e.
  - The post-integration META report at cee7df confirms PR #1303's reduced deltas but identifies copied global controller residue in four documents that the final seven-path reconciliation did not change.
  - Exact PR #1309 head 4e4ed5fc9ed32890d053cb1f930921cf7bab73bc and tree fab9b51c6b122c415238d755599da6f1ae8667fe passed CI run 34118721849, Agent Governance run 34118721863 attempt 2 and independent complete-diff review.
  - PR #1309 passed merge-group CI run 34119020095 and integrated normally through Merge Queue as protected main 796209ead741041e8da943fc8c3846672ac96475.
  - Live exact-ref readback after integration returns no ref for either docs/r5-platform-policy-adoption-1302 or docs/r5-platform-controller-residue-1302.
derived:
  - The remaining Platform retry and delivery documents can delegate global execution semantics while retaining local state compatibility, E2E, layer-completeness and resource-hygiene constraints.
unknown: []
conflicts: []
first_failure:
  marker: post-integration-specialist-controller-residue
  evidence: META report cee7df found that the final PR #1303 reconciliation left four previously reviewed specialist-controller reductions outside its seven-file diff; current main still contains those copied global controllers.
rejected_hypotheses:
  - Reapply the pre-#1304 binding, workflow, validator, prompt-eval or D26 changes; protected main already contains equivalent or stronger accepted implementations.
changed_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md -> docs/agents/tasks/archive/OTERYN-20260907-r5-platform-policy-adoption.md
validation:
  - command: focused governance and documentation validation
    result: PASS
    evidence: On the four-file follow-up, checkpoint 12, source-closeout 5, task-liveness 25, governing-Issue 10, Documentation IA 5, Control Room 4, authenticated policy consumer 13 and prompt evaluator 8 tests pass; active checkpoints, catalog, authenticated central policy and the 24-case prompt suite validators pass.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Agent instruction and lifecycle documentation only; no executable product behavior changes.
  - command: exact-head and merge-group hosted qualification
    result: PASS
    evidence: PR #1309 CI 34118721849, Agent Governance 34118721863 attempt 2 and merge-group CI 34119020095 passed; required platform-gate jobs 101731607498 and 101732571218 passed on the candidate and merge-group generations.
blockers:
  - none
next_action: Merge this lifecycle-only archive carrier through protected Merge Queue, close Issue #1302 from that repository transition, and verify the carrier source ref is absent.
```

## Lifecycle archive carrier

Protected `main` remains authoritative until this lifecycle-only PR merges. The carrier moves the task record from `tasks/active/` to `tasks/archive/` and uses `Closes #1302`; the final merge SHA, terminal Issue state and carrier-ref absence must be recorded in the PR/Issue readback because those post-merge facts cannot truthfully exist in this pre-merge archive commit.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository implementation branches; ordinary delete-after-merge applied after each protected integration.
source_branch_evidence: PR #1303 head e711378ce6bb3d1c54ba2c33c56f4fe957f261e9 merged as 2c3934fece8e072f23ea33ad82f4b4399fc3b45e and PR #1309 head 4e4ed5fc9ed32890d053cb1f930921cf7bab73bc merged as 796209ead741041e8da943fc8c3846672ac96475; live exact-ref readback is absent for both source branches.
```
