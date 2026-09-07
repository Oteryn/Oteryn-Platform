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

## Acceptance criteria

- [x] Protected-main PR #1304 binding, trusted consumer, prompt/domain migrations, workflow trust boundary and D26 archive remain unchanged.
- [x] Retry exhaustion is distinguished from missing authority through the bound META state model.
- [x] A protected-main policy change can be reconciled without discarding unaffected work or permitting candidate self-authorization.
- [x] Default instruction loading is bounded to material routes and references.
- [x] Platform delivery, E2E, security and execution-resource cleanup invariants remain available without duplicated global procedures.
- [x] Current live ownership overlap is checked before editing shared paths.
- [ ] Focused validation, exact-head hosted checks and protected integration/closeout are complete.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
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
updated_at: 2026-09-07T10:20:00Z
head: 907546f193e91b0bed2f5f077ab5b874771929ef
branch: docs/r5-platform-policy-adoption-1302
pr: 1303
status: implementing
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
proven:
  - Protected main 907546f193e91b0bed2f5f077ab5b874771929ef contains merged PR #1304 and tree d844c6c0e568896008ea8053057bcae53fb4b949.
  - PR #1304 owns the META binding, trusted policy consumer, prompt inventory migration, workflow trust boundary and D26 archive; this task preserves those paths unchanged.
  - Issue #1302 and draft PR #1303 own this remaining bounded R5 delta and coordinate with #1009.
  - Issue #1299 closed externally at 2026-09-07T09:26:20Z; this task does not reactivate or re-close it.
derived:
  - The remaining Platform retry and delivery documents can delegate global execution semantics while retaining local state compatibility, E2E, layer-completeness and resource-hygiene constraints.
unknown:
  - Exact reconciled candidate head, hosted check results and integration outcome.
conflicts: []
first_failure:
  marker: protected-main-overlap
  evidence: PR #1304 merged after the prior candidate qualified and changed the same policy, prompt and consumer paths; prior exact-head qualification cannot qualify the reconciled candidate.
rejected_hypotheses:
  - Reapply the pre-#1304 binding, workflow, validator, prompt-eval or D26 changes; protected main already contains equivalent or stronger accepted implementations.
changed_paths:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/tasks/active/OTERYN-20260907-r5-platform-policy-adoption.md
validation:
  - command: focused governance and documentation validation
    result: NOT_RUN
    evidence: Pending on the reconciled candidate.
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: Agent instruction and lifecycle documentation only; no executable product behavior changes.
blockers:
  - none
next_action: Validate and publish the reconciled exact candidate, then run required hosted checks; after protected integration archive this packet and close Issue #1302.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Draft PR #1303 is not integrated.
source_branch_evidence: Branch docs/r5-platform-policy-adoption-1302 remains the active Issue #1302 candidate.
```
