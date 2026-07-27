---
task_id: OTERYN-20260728-portal-e2e-audit
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
search_first:
  - .github/workflows/*acceptance*.yml
  - scripts/acceptance/**
  - docs/agents/tasks/active/**
optional_reads:
  - docs/testing/E2E_COVERAGE_ROADMAP.md
---

# OTERYN-20260728-portal-e2e-audit

## Goal

Execute a fresh exact-head comprehensive portal E2E audit against the current post-refresh repository state, classify every failure as product, harness, documentation or infrastructure, and persist all confirmed defects and missing capabilities in `docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md`.

## Acceptance criteria

- [ ] A dedicated audit orchestration executes both the existing zero-retry `critical` profile and the existing zero-retry `full` profile on one exact task head.
- [ ] The strict portal ledger/account lifecycle and all module-specific acceptance workflows are executed on that same exact head.
- [ ] Every failed workflow is inspected at job/step/log level before rerun or classification.
- [ ] Confirmed defects, harness limitations, documentation drift and known missing capabilities are recorded in the audit report with severity, evidence and disposition.
- [ ] The final checkpoint names the exact tested SHA and exact run evidence and makes no `PRODUCTION_PROVEN` claim.

## Ownership

```yaml
owned_paths:
  - .github/workflows/portal-e2e-audit.yml
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
modules:
  - testing
  - portal-acceptance
  - agent-governance
dependencies:
  - PR #260 delivered-surface closure
  - PR #262 final portal staging refresh
  - PR #264 final portal container-namespace verification
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T22:12:09Z
head: UNKNOWN
branch: test/OTERYN-20260728-portal-e2e-audit
pr: 265
status: implementing
context_routes:
  - testing
  - web-cms
  - admin-rbac
  - security
  - agent-governance
owned_paths:
  - .github/workflows/portal-e2e-audit.yml
  - docs/testing/PORTAL_E2E_AUDIT_2026-07-28.md
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
  - docs/agents/ACTIVE_WORK.md
proven:
  - Current main head before the synchronized audit implementation is ef6d03e0b7c6ed0ecf40e6e108b81358c9b64b1b from merged PR #264.
  - The delivered-surface contract is strict and Issue #240 is closed through PR #260.
  - The reusable acceptance workflow selects critical whenever the caller event is pull_request, before considering inputs.profile; a pull-request caller therefore cannot request full with the current expression.
derived:
  - A push-triggered audit caller can use the existing workflow_call input to execute full without changing ordinary pull-request critical behavior.
unknown:
  - Exact-head runtime results for the fresh comprehensive audit.
conflicts: []
first_failure:
  marker: E2E-AUD-001
  evidence: .github/workflows/acceptance-validation.yml profile selection overrides workflow_call profile for pull_request callers
rejected_hypotheses:
  - A documentation-only pull request is sufficient to execute the complete E2E matrix.
  - Evidence collected on the superseded ccd45fdce3176bd1da97a264bbbaf19a68c1397b-based task head is valid for the current main head.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260728-portal-e2e-audit.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: synchronized audit workflow not created yet
blockers:
  - none
next_action: Recreate the audit orchestration on the synchronized PR #265 head and observe all exact-head workflows.
```

## Notes

The audit is repository/staging evidence only. Issue #91 remains the production-only gate, and external Canary/login-server repositories remain read-only.
