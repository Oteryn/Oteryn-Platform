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

- [ ] One repository-owned classifier covers all 13 baseline change classes and defaults unknown/mixed risk to all affected gates.
- [ ] Deterministic fixtures prove docs-only, governance, backend, frontend, dependency, migration/database, auth/security, payment, gateway, deployment, edge, shared and workflow-self behavior.
- [ ] CI job `test` preserves its identity and skips only when classifier success proves it unaffected.
- [ ] Phase 7, Edge Security Emulation and Platform DB Outage jobs `validate` preserve their identities and fail closed on classifier failure.
- [ ] Game Auth Ticket Concurrency job `concurrency-proof` preserves its identity and fails closed on classifier failure.
- [ ] Workflow-level path filters are not used for required gates.
- [ ] Classified no-op is represented as skipped routing evidence, not product-validation evidence.
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
updated_at: 2026-08-02T13:51:00+02:00
head: 981e93e28b48f24634ba70e8a06dc7af51f71e71
branch: ci/OTERYN-20260802-change-routing
pr: none
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
  - Issue #467 is the sole open tracker found for this exact CI-routing scope.
  - PR #453 proved five heavy workflow families execute for documentation-only changes.
  - GitHub documents that a job skipped by jobs.<job_id>.if reports success, while workflow path filtering can leave required checks pending.
  - Existing terminal job identifiers are test, validate and concurrency-proof.
derived:
  - Job-level conditional routing can preserve existing required check identities without workflow-level path filters.
unknown:
  - Exact repository branch-protection context configuration is not exposed by the current connector.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Workflow-level paths-ignore is safe for required checks.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
validation:
  - command: live workflow and ownership preflight
    result: PASS
    evidence: no overlapping CI-routing issue, PR or indexed active task was found
blockers:
  - none
next_action: Add the deterministic classifier, fixtures and bounded branch-only workflow patch harness, then open a draft PR.
```

## Notes

No application behavior, production environment, database content, payment activation, secret or external repository is in scope. Runtime/browser E2E is not applicable; the real system boundary is GitHub change classification through emitted workflow jobs.
