---
task_id: OTERYN-20260817-resource-cleanup-closeout
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - resource cleanup closeout EXECUTION_RESOURCE_HYGIENE
optional_reads: []
---

# OTERYN-20260817-resource-cleanup-closeout

## Goal

Persist the owner-mandated autonomous execution-resource cleanup and closeout contract in repository governance and merge it through the protected PR path.

## Acceptance criteria

- [x] Require execution-channel discovery before treating lack of direct SSH/shell as a blocker.
- [x] Require read-only inventory and exact ownership/disposability checks before deletion.
- [x] Preserve shared, persistent, production, runner, Home Assistant, network, database, and UNKNOWN resources by default.
- [x] Require post-delete verification and cleanup of temporary cleanup mechanisms.
- [x] Require terminal `REMOVED / KEPT / VERIFIED / REMAINING / BLOCKER` reporting.
- [ ] Exact-head repository-required checks pass and the PR is squash-merged.

## Ownership

```yaml
owned_paths:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/tasks/active/OTERYN-20260817-resource-cleanup-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260817-resource-cleanup-closeout.md
modules:
  - agent-governance
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T09:27:00Z
head: 8e3ad67a0fcad33403916a6f44c35823413fc93d
branch: docs/resource-cleanup-closeout-contract
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/tasks/active/OTERYN-20260817-resource-cleanup-closeout.md
  - docs/agents/tasks/archive/OTERYN-20260817-resource-cleanup-closeout.md
proven:
  - main was f617120975cb1522cad87d74f8bea37f829b2b64 at task start
  - main requires classify-changes and test status checks
  - root AGENTS.md already makes EXECUTION_RESOURCE_HYGIENE.md mandatory for execution resource work
  - the policy update is documentation/agent-governance only
  - no temporary host/container/runner resources were created by this task
  - repository write scope is blakinio/Oteryn-Platform only
derived:
  - application/browser/container validation is not applicable to this governance-only change
unknown:
  - exact-head CI result
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/tasks/active/OTERYN-20260817-resource-cleanup-closeout.md
validation:
  - command: GitHub protected required checks on PR exact head
    result: NOT_RUN
    evidence: PR not opened yet
  - command: application/browser/container build
    result: NOT_APPLICABLE
    evidence: BUILD_TEST_MATRIX classifies agent-governance/documentation-only changes for lightweight checks only
blockers:
  - none
next_action: open the dedicated PR and inspect exact-head required checks
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository governance PR; branch has no durable post-merge purpose
source_branch_evidence: pending merge and source-ref verification
```

## Notes

The policy deliberately does not expand production, secret, deployment, runner, or cross-repository authority. Temporary execution workflows remain last-resort mechanisms constrained by existing task authority and must be removed before terminal closeout.
