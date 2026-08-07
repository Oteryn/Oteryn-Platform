---
task_id: OTERYN-20260807-main-push-ci-routing-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
status: completed
completed_at: 2026-08-07T09:04:58Z
audited_main: 17f4d5a0de3f029c036df61d326e369cc53bb0ef
audit_pull_request: 784
audit_head: 7af0adb484daf2c414ae97df51b235d8e2528426
audit_merge: 8478b627609f9d82799bc5866c8ba504d5751f19
finding_issue: 783
finding_id: OPA-GOV-0020
---

# OTERYN-20260807-main-push-ci-routing-audit — Completed

## Result

The bounded continuous-audit package proved one material medium-risk finding: `OPA-GOV-0020`.

Core CI preserves path-aware economy for pull requests but forces all gates for non-PR events, while Acceptance E2E runs a full profile on every push to `main`. Live evidence also proved that a newer documentation-only main generation can cancel an already-running full main Acceptance generation.

The finding is durably handed off as Issue #783 with `priority:P1`, `risk:medium`, `agent:ready` and implementation authorization. This audit package did not implement the repair.

## Delivery

- Audit PR #784 final head: `7af0adb484daf2c414ae97df51b235d8e2528426`.
- PR #784 merged by protected auto-merge as `8478b627609f9d82799bc5866c8ba504d5751f19`.
- Report: `docs/agents/reports/OTERYN-20260807-main-push-ci-routing-audit.md`.
- Evidence: `docs/agents/evidence/OTERYN-20260807-main-push-ci-routing-audit/index.md`.

## Validation

Exact-head validation for PR #784:

- CI run `31164308992`: PASS.
  - `classify-changes`: PASS.
  - required `test`: PASS.
  - `runtime-tests`: correctly SKIPPED because the package changed audit/governance documentation only.
- Agent Governance run `31164310591`: PASS.
- unresolved inline review threads: 0.
- exact effective diff against current base before merge: four audit/governance documentation paths only.

## E2E

`NOT_APPLICABLE` for the audit delivery itself because no executable product behavior changed. Live post-merge workflow behavior was observed from normal documentation-only merges; no synthetic destructive or production condition was created.

## Ownership release

All paths owned by this audit task are released by this archival closeout. Issue #783 is the independent remediation handoff and owns future implementation work under the remediation claim protocol.

The continuous audit programme remains active and must refresh live ownership, Issues, tasks, PRs and current-main deltas before selecting the next non-overlapping domain.
