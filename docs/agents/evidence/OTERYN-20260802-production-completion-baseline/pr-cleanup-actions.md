# PR cleanup actions

## Closed during the 2026-08-02 baseline invocation

- #116 — closed as request-only stale blocked task PR; issue #114 remains open for a fresh scheduled-evidence inspection.
- #182 — closed as obsolete historical Liquid20 retry request.
- #189 — closed as obsolete historical Liquid20 attempt/retry record.
- #328 — closed as request-only task/index PR; issue #324 remains open because no rename ADR/contract was delivered.
- #335 — closed as superseded by current `main` restart policies and `Repair Synology Autostart` workflow.
- #387 — closed as superseded by later production-gate evidence and merged public-edge repair/audit work.

## Retained open intentionally

- #225 — narrow dependency upgrade; requires rebase/recreation and affected Game Gateway CI.
- #338 — required but blocked by Canary schema 1.3 producer compatibility.
- #381 — active Issue #326/#365 audit and exact validator work.
- #391 — active official Linux client live-reference harness with explicit safety gates.
- #405 — current private-production/public-go-live evidence and unresolved operational blockers.
- #412 — temporary Issue #365 Synology preflight; must close without merge when validation ends.

## Follow-up hygiene

- Closed request-only PRs preserve their historical checkpoints in GitHub and must not be reopened merely to reuse old branches.
- Fresh work starts from current `main`, creates a current active task record and claims only unowned paths.
- Every retained PR must be revisited at the next programme barrier; open status is not permanent.
