# PR cleanup actions

## Closed during the 2026-08-02 baseline invocation

- #116 — closed as request-only stale blocked task PR; issue #114 remains open.
- #182 — closed as obsolete historical Liquid20 retry request.
- #189 — closed as obsolete historical Liquid20 attempt/retry record.
- #328 — closed as request-only task/index PR; issue #324 remains open because no rename ADR/contract was delivered.
- #335 — closed as superseded by current `main` restart policies and `Repair Synology Autostart` workflow.
- #387 — closed as superseded by later production-gate evidence and merged public-edge work.

## Rebase actions requested after independent audit

The original baseline omitted seven old, conflicted Dependabot PRs. The following commands were posted on 2026-08-02:

- #222 — `@dependabot rebase`;
- #223 — `@dependabot rebase`;
- #224 — `@dependabot rebase`;
- #226 — `@dependabot rebase`;
- #227 — `@dependabot rebase`;
- #228 — `@dependabot rebase`;
- #229 — `@dependabot rebase`.

PR #225 had already received the same rebase request during the baseline work. Rebase completion is an external event; no polling loop is justified.

## Retained open intentionally

- #222, #223, #224 — Composer dependency updates awaiting refreshed exact-head validation.
- #225 — Go action update awaiting refreshed Game Gateway CI.
- #226, #227, #228, #229 — GitHub Action major updates awaiting runner/input/behavior compatibility validation.
- #338 — blocked by Canary schema 1.3 producer compatibility.
- #381 — active Issue #326/#365 audit and exact validator work.
- #391 — active official Linux client live-reference harness with explicit safety gates.
- #405 — current private-production/public-go-live evidence and unresolved operational blockers.
- #412 — temporary Issue #365 Synology preflight; must close without merge when validation ends.

## Follow-up hygiene

Closed request-only PRs preserve historical checkpoints and must not be reopened merely to reuse old branches. Fresh work starts from current `main`, claims only unowned paths and must make each dependency PR terminal after its own exact-head gate.
