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
updated: 2026-08-07T20:39:00Z
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

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T20:39:00Z
head: c8252419089b2ceb5d5cf48dab25c2676564e88d
branch: repair/issue-848
pr: 854
status: validating
context_routes:
  - ci-build-test
  - architecture-governance
  - dependencies-tooling
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
derived:
  - a pull_request event resolves the concurrency identity to its stable PR number and remains supersedable.
  - a push event has no pull_request number and therefore resolves the concurrency identity to github.sha, isolating each main commit.
unknown: []
conflicts: []
blockers: []
changed_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/active/OTERYN-20260807-main-ci-generation-preemption.md
first_failure:
  marker: agent-governance-checkpoint-schema
  evidence: Agent Governance run 31216560966 first rejected the task because the required Context checkpoint heading was missing; run 31216616236 exposed the remaining required checkpoint fields; run 31216676515 then rejected only unsupported provisional validation result names. The implementation diff itself was not implicated.
rejected_hypotheses:
  - disable cancel-in-progress globally for every CI event
  - key main pushes only by github.ref
  - broaden the repair into push classification or application runtime changes
validation:
  - command: full PR #854 patch self-review
    result: PASS
    evidence: only the CI concurrency block, deterministic regression assertions, and task checkpoint are changed.
  - command: tests/ci/test_push_change_routing.py retained in CI classifier
    result: NOT_RUN
    evidence: final exact-head CI rerun is required after this checkpoint-only correction.
  - command: tests/ci/test_workflow_trigger_economy.py
    result: NOT_RUN
    evidence: final exact-head CI rerun is required after this checkpoint-only correction.
  - command: Agent Governance
    result: NOT_RUN
    evidence: final exact-head governance rerun is required after normalizing checkpoint validation states.
next_action: run final exact-head HEIGHTENED validation, confirm zero unresolved review threads, merge PR #854, verify Issue #848 closure, archive this task, and release ownership.
```
