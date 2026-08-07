---
task_id: OTERYN-20260807-main-push-ci-routing-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/BUILD_TEST_MATRIX.md
---

# OTERYN-20260807-main-push-ci-routing-audit

## Goal

Audit current main-push CI and browser-acceptance routing for documentation/governance-only merges and persist any material finding without modifying workflow/runtime implementation.

## Acceptance criteria

- [x] Live ownership and current open PRs were refreshed before selection.
- [x] Current main `17f4d5a0de3f029c036df61d326e369cc53bb0ef` was inspected.
- [x] CI, classifier, Acceptance E2E trigger/concurrency and workflow-economy tests were inspected.
- [x] Live documentation-only main-push behavior was observed without manufacturing a destructive condition.
- [x] Existing Issues #452 and #467 were checked for duplicate/root-cause coverage.
- [x] One material finding was persisted as Issue #783 (`OPA-GOV-0020`).
- [ ] Exact-head documentation/governance CI passes and the audit package is merged/archived.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/reports/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/evidence/OTERYN-20260807-main-push-ci-routing-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - main-push CI routing audit records only
dependencies:
  - Issue #783 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T08:44:00Z
head: 17f4d5a0de3f029c036df61d326e369cc53bb0ef
branch: audit/OTERYN-20260807-main-push-ci-routing
pr: none
status: validating
context_routes:
  - ci
  - architecture
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/reports/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/evidence/OTERYN-20260807-main-push-ci-routing-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - CI uses path-aware base/head classification for pull requests but `--all` for every main push.
  - Acceptance E2E applies path filters only to pull requests; every main push defaults to the full profile.
  - Documentation-only audit merge f72fafd started full Acceptance run 31162272112.
  - Documentation-only lifecycle merge 17f4d5a emitted another Acceptance run 31162564522 and CI run 31162564524.
  - The earlier Acceptance run 31162272112 was cancelled at 2026-08-07T08:39:06Z after the newer docs-only main generation entered the shared main concurrency group.
  - Completed Issue #467 proves PR routing only and does not cover main-push or Acceptance routing.
  - OPA-GOV-0020 is recorded as Issue #783 with risk medium, priority P1 and implementation authorization.
derived:
  - The accepted docs-only heavy-workflow economy invariant is not preserved after merge to main.
  - A docs-only main push can both consume heavy runner capacity and preempt a prior main Acceptance generation.
unknown: []
conflicts:
  - Completed baseline Issue #452 requires docs/task/metadata-only changes not to run unrelated heavy browser/container/application gates, while current main-push routing does so.
first_failure:
  marker: OPA-GOV-0020
  evidence: .github/workflows/ci.yml non-PR classification forces --all and acceptance-validation.yml has an unconditional full-profile push-to-main path.
rejected_hypotheses:
  - Issue #467 already covers the defect; rejected because its stated scope and terminal evidence are pull-request routing across five workflow families, excluding Acceptance main-push routing.
  - Docs-only main pushes merely emit lightweight checks; rejected by live run 31162272112 progressing into full browser acceptance.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/reports/OTERYN-20260807-main-push-ci-routing-audit.md
  - docs/agents/evidence/OTERYN-20260807-main-push-ci-routing-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source static routing inspection on main@17f4d5a0de3f029c036df61d326e369cc53bb0ef
    result: PASS
    evidence: CI, classifier, Acceptance workflow and workflow-economy tests inspected.
  - command: live Actions observation of docs-only main pushes
    result: PASS
    evidence: runs 31162272112, 31162564522 and 31162564524.
  - command: runtime/product E2E for audit deliverable
    result: NOT_APPLICABLE
    evidence: audit package changes documentation/governance records only.
blockers:
  - none
next_action: Open the documentation-only audit PR, validate exact head, merge, archive the task and release ownership.
```
