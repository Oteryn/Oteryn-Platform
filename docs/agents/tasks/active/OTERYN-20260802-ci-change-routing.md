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
updated_at: 2026-08-02T14:03:00+02:00
head: 0214c95a39ac9cfaa0010844d3d463c9ac96dc1e
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
  - Classifier tests and the exact five-workflow generated patch passed in bootstrap runs 30746779996 and 30746937197.
  - GitHub Actions cannot push workflow-file changes because its token lacks workflows permission; the failed push made no remote workflow mutation.
  - Artifact run 30746937197 succeeded, but upload-artifact excluded the hidden .github directory and retained only SHA256SUMS.
  - The artifact packaging defect is isolated from classifier and workflow generation correctness.
derived:
  - Packaging generated workflows under a visible workflows directory preserves the validated content and enables connector-side atomic persistence.
unknown:
  - Final exact-head workflow behavior after the generated files are atomically persisted.
conflicts: []
first_failure:
  marker: generated-artifact-hidden-directory-exclusion
  evidence: artifact 8833176101 contained only SHA256SUMS because upload-artifact omitted artifacts/ci-routing/.github/workflows
rejected_hypotheses:
  - The generator failed to produce the five workflow files.
  - Retrying the same hidden artifact path could include .github without changing upload behavior.
changed_paths:
  - .github/workflows/ci-routing-bootstrap.yml
  - scripts/ci/apply_change_routing.py
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_classify_changes.py
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
validation:
  - command: python tests/ci/test_classify_changes.py in bootstrap runs
    result: PASS
    evidence: five tests passed before and after generated mutation
  - command: generated workflow structural audit
    result: PASS
    evidence: exact seven-path generated diff and unique classifier/dependency/fail-closed/test markers passed
  - command: artifact 8833176101 inventory
    result: FAIL
    evidence: only SHA256SUMS was uploaded because the generated workflow directory was hidden
blockers:
  - none
next_action: Regenerate the validated artifact under a visible workflows directory, atomically persist the five workflows with bootstrap files removed, then verify exact-head checks.
```

## Notes

No application behavior, production environment, database content, payment activation, secret or external repository is in scope. Runtime/browser E2E is not applicable; the real system boundary is GitHub change classification through emitted workflow jobs.
