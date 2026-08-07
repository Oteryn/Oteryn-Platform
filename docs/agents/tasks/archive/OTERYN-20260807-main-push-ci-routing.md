---
task_id: OTERYN-20260807-main-push-ci-routing
issue: 783
status: completed
completed_at: 2026-08-07T10:54:08Z
implementation_pull_request: 786
implementation_head: abbaca25bbd5a0a4f677ac84562fdc544249aa9f
implementation_merge: 7dbb35e2257bd3265d4dc75a1723bf6a315afa80
risk: medium
validation_intensity: HEIGHTENED
self_review: PASS
material_findings: 0
production_activation_authorized: false
ownership: RELEASED_ON_ARCHIVE_MERGE
---

# OTERYN-20260807 main-push CI routing — Completed

## Result

Issue #783 is repaired. Documentation/governance-only pushes to `main` no longer force the runtime-heavy CI classification or create a full Acceptance E2E generation, while product-affecting and ambiguous push ranges remain fail-closed.

The delivered boundary:

- derives main-push CI classification from the exact GitHub `before`/head range;
- fails closed to all heavy gates for missing, zero, empty or unusable push ranges;
- preserves the existing pull-request classifier and manual workflow-dispatch behavior;
- applies the existing product-path Acceptance filter to `push: main`;
- prevents docs-only main pushes from entering and preempting the shared full Acceptance generation;
- adds deterministic regression coverage for docs-only, product, zero/ambiguous and trigger/concurrency behavior.

## Delivery

- Implementation PR: #786.
- Final exact PR head: `abbaca25bbd5a0a4f677ac84562fdc544249aa9f`.
- Current main was incorporated before final validation without changing the six-file repair diff.
- Protected squash merge: `7dbb35e2257bd3265d4dc75a1723bf6a315afa80`.
- Issue #783 closed automatically as `completed` by the merge.

## Exact-head validation

Required and applicable validation on final head `abbaca25bbd5a0a4f677ac84562fdc544249aa9f`:

- CI `31171430222`: PASS;
- Agent Governance `31171430074`: PASS;
- Acceptance E2E and Visual UX `31171430056`: PASS;
- Platform DB Outage Validation `31171430383`: PASS;
- Edge Security Emulation `31171430338`: PASS;
- Game Auth Ticket Concurrency `31171430839`: PASS;
- Phase 7 Production-Like Validation `31171429982`: PASS.

Deep System Validation `31171430368` remained non-required and in progress at the final closeout observation; protected auto-merge had already accepted the exact final head after repository-required checks passed. The same repair diff had previously passed Deep System Validation on implementation head `55aecd772447709355437c44e810f82c2b4fdbf0` via run `31165891386`.

The exact-head full-diff review confirmed only the six owned repair paths differ from the then-current `main`, with no overlap from the intervening main changes.

## Self-review and E2E

Self-review: `PASS`, zero material findings, zero unresolved review threads and zero requested changes.

E2E: `PASS` via Acceptance E2E and Visual UX run `31171430056` on the final PR head. No production, protected-environment or external-service mutation was required or performed.

## Rollback and compatibility

A squash revert of PR #786 restores the previous main-push routing. Pull-request routing and manual full `workflow_dispatch` remain compatible. Ambiguous push ranges deliberately retain fail-closed heavy validation rather than risking false-negative CI suppression.

## Ownership release

This archival closeout removes the durable active-task record for Issue #783. Once this archive change is merged, all Issue #783 repair-path and lifecycle ownership is released.
