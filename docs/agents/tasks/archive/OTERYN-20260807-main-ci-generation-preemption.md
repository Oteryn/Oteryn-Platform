---
task_id: OTERYN-20260807-main-ci-generation-preemption
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_REMEDIATION
issue: 848
finding_id: OPA-GOV-0025
status: completed
agent: ChatGPT
branch: repair/issue-848
base_branch: main
base_sha: d331365163ba44acbbb3cfd9e785926aa57ed41a
implementation_authorized: true
claim_protocol_version: 5
coordination_key: workflow:main-ci-generation-preemption
validation_intensity: HEIGHTENED
pr: 854
implementation_head: af80ad0d035172ad9cbe0b24e327551099ea71ad
merge_sha: a1f4a8aaf8d3d43092cbd3b7133d99366093795a
completed_at: 2026-08-07T20:52:09Z
ownership_released: true
---

# OTERYN-20260807-main-ci-generation-preemption

## Terminal result

Issue #848 was repaired and closed by protected squash merge of PR #854.

The core CI concurrency contract now separates event classes:

- pull requests use `github.event.pull_request.number`, preserving cancellation of superseded runs for the same PR;
- pushes use exact `github.sha`, preventing a later documentation-only `main` push from cancelling an earlier product/runtime-required main CI generation;
- `cancel-in-progress` is enabled only for `pull_request` events;
- exact-range push classification, documentation-only runtime-test suppression, and fail-closed ambiguous-range behavior remain unchanged.

## Exact-head evidence

```yaml
validation:
  exact_head: af80ad0d035172ad9cbe0b24e327551099ea71ad
  self_review: PASS_ZERO_MATERIAL_FINDINGS
  unresolved_review_threads: 0
  submitted_change_requests: 0
  checks:
    - name: CI
      run: 31216788225
      result: PASS
    - name: Agent Governance
      run: 31216786885
      result: PASS
    - name: Phase 7 Production-Like Validation
      run: 31216789633
      result: PASS
    - name: Edge Security Emulation
      run: 31216788852
      result: PASS
    - name: Platform DB Outage Validation
      run: 31216788227
      result: PASS
    - name: Game Auth Ticket Concurrency
      run: 31216788855
      result: PASS
  supplemental:
    - name: Deep System Validation
      run: 31216787517
      result: NOT_REQUIRED_FOR_ISSUE_ACCEPTANCE
```

CI run `31216788225` executed the deterministic routing contracts, including `tests/ci/test_push_change_routing.py` and `tests/ci/test_workflow_trigger_economy.py`, before the protected merge.

## Delivery and closeout

```yaml
delivery:
  pr: 854
  merge_sha: a1f4a8aaf8d3d43092cbd3b7133d99366093795a
  issue_state: closed
  issue_closed_at: 2026-08-07T20:52:09Z
  product_runtime_changes: none
  production_mutation: none
  external_repository_mutation: none
ownership:
  state: released
  released_paths:
    - .github/workflows/ci.yml
    - tests/ci/test_workflow_trigger_economy.py
    - tests/ci/test_push_change_routing.py
  coordination_key_released: workflow:main-ci-generation-preemption
next_action: none
```

The former active checkpoint is archived by the lifecycle closeout delivery so no stale ownership remains.
