---
task_id: OTERYN-20260907-r5-platform-policy-adoption
governing_issue: 1302
required_reads: []
search_first:
  - docs/agents/tasks/active/OTERYN-20260907-meta-agent-policy-v3-adoption.md
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
- [ ] Focused validation, exact-head hosted checks and protected integration/closeout are complete.

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
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
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
updated_at: 2026-09-07T11:45:00Z
head: 2c3934fece8e072f23ea33ad82f4b4399fc3b45e
branch: docs/r5-platform-controller-residue-1302
pr: none
status: validating
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
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
proven:
  - Protected main b0f268d682b1f9117140168a3a9d9b86c56ba9cc contains the accepted PR #1304 policy adoption and the completed #1305 cleanup closeout from PRs #1306-#1308.
  - PR #1304 owns the META binding, trusted policy consumer, prompt inventory migration, workflow trust boundary and D26 archive; this task preserves those paths unchanged.
  - Issue #1302 and draft PR #1303 own this remaining bounded R5 delta and coordinate with #1009.
  - Issue #1299 closed externally at 2026-09-07T09:26:20Z; this task does not reactivate or re-close it.
  - Exact PR #1303 head e711378ce6bb3d1c54ba2c33c56f4fe957f261e9 passed Agent Governance run 34112609317 and CI run 34112609162, then merge-group CI run 34117619196 before protected integration as main 2c3934fece8e072f23ea33ad82f4b4399fc3b45e.
  - The post-integration META report at cee7df confirms PR #1303's reduced deltas but identifies copied global controller residue in four documents that the final seven-path reconciliation did not change.
derived:
  - The remaining Platform retry and delivery documents can delegate global execution semantics while retaining local state compatibility, E2E, layer-completeness and resource-hygiene constraints.
unknown:
  - Exact four-file follow-up candidate head, hosted check results and integration outcome.
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
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
validation:
  - command: focused governance and documentation validation
    result: PASS
    evidence: On the four-file follow-up, checkpoint 12, source-closeout 5, task-liveness 25, governing-Issue 10, Documentation IA 5, Control Room 4, authenticated policy consumer 13 and prompt evaluator 8 tests pass; active checkpoints, catalog, authenticated central policy and the 24-case prompt suite validators pass.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Agent instruction and lifecycle documentation only; no executable product behavior changes.
blockers:
  - none
next_action: Validate and publish the four-file specialist-controller follow-up, then run required hosted checks; after protected integration archive this packet and close Issue #1302.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: The bounded specialist-controller follow-up is not integrated.
source_branch_evidence: Branch docs/r5-platform-controller-residue-1302 will carry the remaining Issue #1302 candidate.
```
