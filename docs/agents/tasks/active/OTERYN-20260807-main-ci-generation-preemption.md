---
task_id: OTERYN-20260807-main-ci-generation-preemption
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_REMEDIATION
issue: 848
finding_id: OPA-GOV-0025
status: validating
agent: ChatGPT
branch: repair/issue-848
base_branch: main
base_sha: d331365163ba44acbbb3cfd9e785926aa57ed41a
created: 2026-08-07T20:33:00Z
updated: 2026-08-07T20:36:00Z
implementation_authorized: true
claim_protocol_version: 5
coordination_key: workflow:main-ci-generation-preemption
validation_intensity: HEIGHTENED
pr: 854
---

# OTERYN-20260807-main-ci-generation-preemption

## Goal

Repair Issue #848 so a later documentation-only push to `main` cannot cancel an earlier product/runtime-required CI generation, while superseded runs for the same pull request continue to cancel and path-based runtime-test suppression remains intact.

## Ownership

```yaml
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_workflow_trigger_economy.py
  - tests/ci/test_push_change_routing.py
  - docs/agents/tasks/active/OTERYN-20260807-main-ci-generation-preemption.md
  - docs/agents/tasks/archive/OTERYN-20260807-main-ci-generation-preemption.md
forbidden_paths:
  - .github/workflows/agent-governance.yml
  - production systems
  - external repositories
```

## Acceptance

- [x] Main-push CI generations use per-commit identity and cannot cancel a different main commit.
- [x] Pull-request CI generations still share one PR identity and cancel superseded same-PR runs.
- [x] Documentation-only main pushes still classify the exact push range and skip runtime tests.
- [x] Product/runtime and ambiguous push ranges remain fail-closed.
- [x] Deterministic regression coverage locks the concurrency contract.
- [ ] Exact-head CI and Agent Governance pass with zero unresolved review threads.
- [ ] Merge closes Issue #848 and this task is archived with ownership released.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: ChatGPT/OTERYN-20260807-main-ci-generation-preemption
  classified_at: 2026-08-07T20:36:00Z
  risk: medium
  triggers:
    - CI
    - concurrency
    - post-merge runtime validation
  unknown_or_conflict: []
  rationale: The implementation changes CI concurrency semantics. The repository risk gate explicitly requires HEIGHTENED validation for material CI and concurrency changes.
  self_review:
    result: PENDING
    exact_head: none
    evidence:
      - PR #854 effective diff is limited to the CI concurrency block, deterministic concurrency assertions, and this task checkpoint.
```

## Checkpoint

```yaml
checkpoint_version: 2
status: validating
branch: repair/issue-848
pr: 854
implementation_head: 30e0e2323fd5b9b623f3b23df5ccb86d6fc5257c
base_sha: d331365163ba44acbbb3cfd9e785926aa57ed41a
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_workflow_trigger_economy.py
  - tests/ci/test_push_change_routing.py
proven:
  - Issue #848 is PROVEN and implementation_authorized.
  - deterministic branch repair/issue-848 was created successfully from trusted main.
  - the old CI concurrency group used github.ref with unconditional cancellation, allowing different main commits to preempt one another.
  - PR #854 now keys pull_request generations by PR number and push generations by github.sha.
  - cancel-in-progress is now true only for pull_request events.
  - existing exact-range push routing tests remain in the CI classifier and deterministic workflow-economy assertions lock the new concurrency contract.
  - full PR patch review found no product/runtime, production, database, deployment, secret, or external-repository changes.
unknown: []
conflicts: []
next_action: validate the final exact PR head under HEIGHTENED gate, confirm zero unresolved review threads, then merge, verify Issue #848 closure, archive this task, and release ownership.
```
