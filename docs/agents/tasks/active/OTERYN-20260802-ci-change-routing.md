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
- [x] Added, copied, modified, renamed and deleted paths participate in classification.
- [x] CI job `test` preserves its identity and skips only when classifier success proves it unaffected.
- [x] Phase 7, Edge Security Emulation and Platform DB Outage jobs `validate` preserve their identities and fail closed on classifier failure.
- [x] Game Auth Ticket Concurrency job `concurrency-proof` preserves its identity and fails closed on classifier failure.
- [x] Workflow-level path filters are not used for required gates.
- [x] Classified no-op is represented as skipped routing evidence, not product-validation evidence.
- [ ] Exact-final-head required checks pass, independent audit has zero open material findings and related PRs are terminal.
- [ ] A real docs/governance-only follow-up proves classifier jobs pass while all five original heavy jobs skip successfully.
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
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-02T14:15:00+02:00
head: 24edd2ccb0e29da8ff570a6f9ffe3fc3228a89e4
branch: ci/OTERYN-20260802-change-routing
pr: 468
status: validating
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
  - Issue #467 and draft PR #468 exclusively own this CI-routing scope.
  - The classifier covers the 13 baseline classes and validates its fixture contract on every invocation.
  - Operational and unknown nested Markdown fail closed; only root Markdown and non-contract docs paths may be docs-only.
  - Git diff classification includes deletions through diff-filter ACMRD.
  - The five original terminal job identifiers remain test, validate and concurrency-proof.
  - Each original job depends on classification and runs on classifier failure to fail closed.
  - Artifact 8833224700 from run 30747105458 contains the five generated workflows and all recorded SHA-256 checks passed.
  - Temporary bootstrap instrumentation and its patch script are removed by the candidate implementation commit.
derived:
  - Job-level conditional routing preserves required job identities without workflow-level path filtering.
  - A docs-only closeout PR can provide real no-op routing proof after implementation merge.
unknown:
  - Final exact-head workflow results for the atomic implementation commit.
  - Exact branch-protection context configuration remains unavailable through the connector.
conflicts: []
first_failure:
  marker: bootstrap-workflow-write-transport
  evidence: GitHub Actions could generate and test workflow changes but could not update the branch because its token lacked workflows permission; artifact plus Git Data API removed that transport dependency.
rejected_hypotheses:
  - Workflow-level paths-ignore is safe for required checks.
  - All nested Markdown is documentation-only.
  - Deletions can be omitted from changed-path classification.
  - Repeating the rejected Actions push could cross the workflow permission boundary.
changed_paths:
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - scripts/ci/classify_changes.py
  - tests/ci/fixtures/change-routing-cases.json
  - tests/ci/test_classify_changes.py
  - docs/agents/tasks/active/OTERYN-20260802-ci-change-routing.md
  - docs/agents/evidence/OTERYN-20260802-ci-change-routing/README.md
validation:
  - command: bootstrap run 30746779996 classifier tests before and after generated patch
    result: PASS
    evidence: deterministic classifier unit suite passed twice and exact generated paths/markers passed
  - command: final generator run 30747105458
    result: PASS
    evidence: workflow generation and visible artifact upload completed successfully
  - command: artifact 8833224700 SHA256SUMS
    result: PASS
    evidence: all five generated workflow files matched their recorded digests
  - command: independent classification boundary audit
    result: PASS
    evidence: operational Markdown, unknown nested Markdown and deletions were added as fail-closed boundaries
  - command: exact implementation-head GitHub Actions
    result: NOT_RUN
    evidence: pending atomic candidate commit
blockers: []
next_action: Persist the atomic implementation commit, verify exact-head classifier and heavy jobs, complete independent PR audit, then merge and run a real docs-only no-op closeout proof.
```

## Notes

No application behavior, production environment, database content, payment activation, secret or external repository is in scope. Runtime/browser E2E is not applicable; the real system boundary is GitHub change classification through emitted workflow jobs.
