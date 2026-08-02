---
task_id: OTERYN-20260802-ci-change-routing
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/evidence/OTERYN-20260802-production-completion-baseline/ci-remediation-acceptance.md
search_first:
  - open CI-routing tasks and pull requests
  - current job identifiers and workflow triggers
optional_reads:
  - docs/agents/ACTIVE_WORK.md
---

# OTERYN-20260802-ci-change-routing

## Goal

Implement deterministic fail-closed pull-request change classification so five over-triggered runtime-heavy workflow families skip only proven-unaffected internals while preserving their existing terminal job/check identities.

## Delivery classification

```yaml
feature_scope:
  type: infrastructure
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: true
  e2e_required: false
implementation_authorized: true
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
```

## Acceptance criteria

- [x] One repository-owned classifier covers all 13 baseline change classes and defaults unknown/mixed risk to all affected gates.
- [x] Deterministic fixtures prove docs-only, governance, backend, frontend, dependency, migration/database, auth/security, payment, gateway, deployment, edge, shared and workflow-self behavior.
- [ ] CI job `test` preserves its identity and skips only when classifier success proves it unaffected.
- [ ] Phase 7, Edge Security Emulation and Platform DB Outage jobs `validate` preserve their identities and fail closed on classifier failure.
- [ ] Game Auth Ticket Concurrency job `concurrency-proof` preserves its identity and fails closed on classifier failure.
- [x] Workflow-level path filters are not used for required gates.
- [x] Classified no-op is represented as skipped routing evidence, not product-validation evidence.
- [ ] Exact-final-head required checks pass, independent audit has zero open material findings and related PRs are terminal.
- [ ] Task is archived and ownership released after merge.

## Ownership

```yaml
owned_paths:
  - scripts/ci/classify_changes.py
  - tests/ci/**
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
  - docs/agents/tasks/archive/OTERYN-20260802-ci-change-routing.md
  - docs/agents/evidence/OTERYN-20260802-ci-change-routing/**
modules:
  - CI policy
  - Testing infrastructure
dependencies:
  - issue #467
  - programme #451
  - baseline PR #453
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-02T13:59:00+02:00
head: 39d557136c7b843596c1ca4b16345f134da6af69
branch: ci/OTERYN-20260802-change-routing
pr: 468
status: implementing
context_routes:
  - testing
  - agent-governance
owned_paths:
  - scripts/ci/classify_changes.py
  - tests/ci/**
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
  - docs/agents/tasks/archive/OTERYN-20260802-ci-change-routing.md
  - docs/agents/evidence/OTERYN-20260802-ci-change-routing/**
proven:
  - Issue #467 and draft PR #468 own this CI-routing scope.
  - The classifier and all deterministic fixtures passed twice in bootstrap run 30746779996 before workflow mutation and after generated workflow mutation.
  - Exact marker checks proved five workflows each received one classifier job, one dependency and one fail-closed step.
  - The generated local commit could not be pushed because GitHub rejected Actions workflow modification without workflows permission.
  - The failed push made no remote branch mutation.
derived:
  - Artifact transfer plus Git Data API is the nearest safe alternative and preserves the validated generated content without protection bypass.
unknown:
  - Final exact-head workflow behavior after the generated files are atomically persisted.
conflicts: []
first_failure:
  marker: bootstrap-workflow-push-permission
  evidence: run 30746779996 job 91493608720 was rejected because the GitHub App cannot update .github/workflows/ci.yml without workflows permission
rejected_hypotheses:
  - The generated patch or classifier tests failed before push.
  - Retrying the same GitHub Actions push could succeed without changing the permission boundary.
changed_paths:
  - .github/workflows/ci-routing-bootstrap.yml
  - scripts/ci/apply_change_routing.py
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_classify_changes.py
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
validation:
  - command: python tests/ci/test_classify_changes.py in run 30746779996 before and after generated patch
    result: PASS
    evidence: five tests passed in both executions
  - command: generated workflow structural audit in run 30746779996
    result: PASS
    evidence: exact seven-path generated diff and unique classifier/dependency/fail-closed markers were verified
  - command: push generated workflows from GitHub Actions
    result: FAIL
    evidence: remote rejected workflow update because the GitHub App lacked workflows permission
blockers:
  - none
next_action: Generate the five validated workflows as an artifact, atomically persist them through Git Data API with temporary instrumentation removed, then verify exact-head checks.
```

## Notes

No application behavior, production environment, database content, payment activation, secret or external repository is in scope. Runtime/browser E2E is not applicable; the real system boundary is GitHub change classification through emitted workflow jobs.
