---
task_id: OTERYN-20260807-main-push-ci-routing
programme_id: OTERYN_PLATFORM_REMEDIATION
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: completed
completed_at: 2026-08-07T11:03:00Z
issue: 783
implementation_branch: repair/issue-783
implementation_pull_request: 786
implementation_head: abbaca25bbd5a0a4f677ac84562fdc544249aa9f
implementation_merge: 7dbb35e2257bd3265d4dc75a1723bf6a315afa80
validation_intensity: HEIGHTENED
---

# OTERYN-20260807 main-push CI routing — Completed

## Result

Issue #783 is repaired and terminally delivered. Main-push CI now classifies the exact GitHub push range through a fail-closed adapter instead of unconditionally enabling all heavy gates. Documentation/governance-only pushes to `main` are excluded from the full Acceptance E2E trigger, while product/runtime paths, pull-request routing, manual dispatch and ambiguous push ranges preserve fail-closed validation behavior.

PR #786 was the single authoritative one-Issue delivery and merged to `main` as `7dbb35e2257bd3265d4dc75a1723bf6a315afa80`. Issue #783 closed as completed at `2026-08-07T10:54:08Z`.

## Delivered paths

- `.github/workflows/ci.yml`
- `.github/workflows/acceptance-validation.yml`
- `scripts/ci/classify_push_changes.py`
- `tests/ci/test_push_change_routing.py`
- `tests/ci/test_workflow_trigger_economy.py`

The final PR diff against base `1ab8d90be35745f8020b2026d6d75ed777ccf76f` contained only the five implementation/test paths above plus this task record.

## Validation and self-review

- exact implementation-head self-review: `PASS`, zero material findings;
- focused routing tests and all eight workflows emitted for implementation head `55aecd772447709355437c44e810f82c2b4fdbf0`: `PASS`;
- final PR head `abbaca25bbd5a0a4f677ac84562fdc544249aa9f`:
  - CI `31171430222`: `PASS`;
  - Agent Governance `31171430074`: `PASS`;
  - Acceptance E2E and Visual UX `31171430056`: `PASS`;
  - Edge Security Emulation `31171430338`: `PASS`;
  - Platform DB Outage Validation `31171430383`: `PASS`;
  - Game Auth Ticket Concurrency `31171430839`: `PASS`;
  - Phase 7 Production-Like Validation `31171429982`: `PASS`.
- PR #786 had zero review submissions and zero inline review threads at terminal reconciliation.
- `Deep System Validation` run `31171430368` was still in progress during post-merge reconciliation and is not used as completion evidence; normal branch protection had already accepted and merged PR #786.

## E2E

`PASS` on the final PR head through Acceptance E2E and Visual UX run `31171430056`. No production, protected-environment or external-repository mutation was performed.

## Rollback and compatibility

Rollback is bounded to reverting merge `7dbb35e2257bd3265d4dc75a1723bf6a315afa80`. Pull-request classification and manual `workflow_dispatch` behavior remain preserved; ambiguous or unusable push ranges enable heavy gates fail-closed.

## Ownership release

```yaml
repair_release:
  issue: 783
  owner: OTERYN-20260807-main-push-ci-routing
  branch: repair/issue-783
  final_head: abbaca25bbd5a0a4f677ac84562fdc544249aa9f
  reason: merged_completed
  released_at: 2026-08-07T11:03:00Z
  next_state: closed
```

All implementation and task paths owned by this repair are released by this archival closeout. There are no blockers, unresolved findings, requested changes, review threads, `UNKNOWN` or `CONFLICT` states remaining for Issue #783.
