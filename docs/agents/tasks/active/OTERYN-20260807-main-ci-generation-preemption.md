---
task_id: OTERYN-20260807-main-ci-generation-preemption
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_REMEDIATION
issue: 848
finding_id: OPA-GOV-0025
status: implementing
agent: ChatGPT
branch: repair/issue-848
base_branch: main
base_sha: d331365163ba44acbbb3cfd9e785926aa57ed41a
created: 2026-08-07T20:33:00Z
updated: 2026-08-07T20:33:00Z
implementation_authorized: true
claim_protocol_version: 5
coordination_key: workflow:main-ci-generation-preemption
validation_intensity: STANDARD
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

- [ ] Main-push CI generations use per-commit identity and cannot cancel a different main commit.
- [ ] Pull-request CI generations still share one PR identity and cancel superseded same-PR runs.
- [ ] Documentation-only main pushes still classify the exact push range and skip runtime tests.
- [ ] Product/runtime and ambiguous push ranges remain fail-closed.
- [ ] Deterministic regression coverage locks the concurrency contract.
- [ ] Exact-head CI and Agent Governance pass with zero unresolved review threads.
- [ ] Merge closes Issue #848 and this task is archived with ownership released.

## Checkpoint

```yaml
checkpoint_version: 1
status: implementing
branch: repair/issue-848
base_sha: d331365163ba44acbbb3cfd9e785926aa57ed41a
owned_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_workflow_trigger_economy.py
  - tests/ci/test_push_change_routing.py
proven:
  - Issue #848 is PROVEN and implementation_authorized.
  - deterministic branch repair/issue-848 was created successfully from trusted main.
  - current ci.yml uses github.ref plus unconditional cancel-in-progress, which gives all main pushes one supersedable generation.
unknown: []
conflicts: []
next_action: replace CI concurrency identity with PR-number-or-push-SHA identity, restrict cancellation to pull_request generations, and add deterministic regression assertions before opening the Issue-owned PR.
```
