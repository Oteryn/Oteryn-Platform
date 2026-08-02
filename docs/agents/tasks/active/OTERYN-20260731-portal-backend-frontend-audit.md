---
task_id: OTERYN-20260731-portal-backend-frontend-audit
policy_version: 2
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
decomposition_decision: phased
related_issues:
  - 326
  - 365
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/index.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/VALIDATOR_VERDICT.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_EXACT_FROZEN_EXECUTION_RUNBOOK.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/ISSUE_365_SYNOLOGY_EXECUTION_ATTEMPTS.md
search_first:
  - live task checkpoint branch exact head PR and CI
  - Issue #326 and Issue #365
  - corrected mechanism evidence before historical comments
---

# OTERYN-20260731-portal-backend-frontend-audit

## Goal

Audit every delivered portal capability across backend, frontend, integration, states, browser evidence and deployment boundaries. Do not implement product findings, merge temporary validation infrastructure or deploy.

## Acceptance criteria

- [x] Freeze the authoritative audit target and environment boundaries.
- [x] Build the delivered-surface, route and capability inventories.
- [x] Classify states, browsers, viewports and deployment evidence.
- [x] Recover strict repository and critical browser artifacts.
- [x] Execute current critical-profile and post-serialization original-flow validation.
- [x] Recover embedded diagnostics and execute generic/source-faithful layout probes.
- [x] Publish the exact frozen-target 12-sample execution runbook.
- [x] Prove the Synology staging runner can build and bootstrap the required environment.
- [x] Generate and install the source-faithful Laravel 13.20.0 `StartSession` observer.
- [ ] Execute a valid exact frozen-target clean/corrupt × immediate/pre-scroll 12-sample matrix with request/session correlation.
- [x] Publish consolidated reports, machine-readable matrices and validator instructions.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260731-portal-backend-frontend-audit.md
  - docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit*.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/**
modules:
  - portal completeness audit
  - audit evidence and validation
  - Wiki Issue #365 evidence
dependencies:
  - Issue #326
  - Issue #365
blockers:
  - external completion of GitHub Actions run 30763456046
cross_repository_tasks: []
```

## Constraints

- Audit-only: no application, route, view/asset, production configuration, migration/model, dependency, deployment or external-repository mutation.
- Browser and framework observers must remain isolated from the frozen source and must never merge.
- Temporary validation PRs must close without merge.
- CI evidence does not imply staging or production deployment.
- The active run must not be retried or replaced after it enters the runtime matrix.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T19:30:00Z
branch: audit/OTERYN-20260731-portal-backend-frontend-audit
pr: 381
status: waiting
phase: validate
session_id: chat-github-20260802-issue365-authorized-continuation
session_role: coordinator-validator
execution_mode: github-actions-synology
project_lane: oteryn-platform-core
task_kind: audit
estimate_confidence: high
decomposition_decision: phased
ci_checks_for_current_head: 2
unchanged_state_checks: 1
identical_failure_retries: 0
context_reconstruction_attempts: 1
stall_warnings: 0
heavy_validation_runs_completed: 10
current_control_head: 8c58035cacb9fd4675d898a1652036fc8b9d4357
current_run: 30763456046
current_job: 91537990755
current_observation_pr: 476
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
owner_authorization:
  source: user instruction "Dzialaj dalej"
  scope: one bounded harness-only repair and exact-frozen validation continuation
  excludes:
    - merge
    - deploy
    - product code mutation
    - production mutation
    - work in another repository
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260731-portal-backend-frontend-audit.md
  - docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit*.md
  - docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/**
proven:
  - frozen audit target is b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
  - normalized audit findings remain 0 HIGH 6 MEDIUM and 1 LOW
  - responsive-mobile flash loss remains intermittently reproduced while durable publication succeeds
  - session serialization is NOT_PROVEN_REMEDIATED and root cause remains UNKNOWN
  - Synology can bootstrap production-like MariaDB Redis and application services
  - source-faithful Laravel 13.20.0 StartSession instrumentation installs and passes syntax validation
  - run 30758971408 reached matrix stage and produced artifact 8837189083 with verified ZIP digest sha256:03ced224c4e14b649f62a77e512821cffc5df679c425610da603137040f66fa0
  - six clean samples in run 30758971408 were invalid technical failures because the Playwright container lacked PHP
  - artifact 8837189083 proves the one-corrupt fixture state is one row with storage_exists true and thumbnail_exists false
  - control head 8c58035cacb9fd4675d898a1652036fc8b9d4357 matches the exact generated Playwright command and exact corrupt fixture state
  - active run 30763456046 uses workers 1 retries 0 and a separately checked-out frozen target
  - no application deployment production or external-repository mutation occurred
derived:
  - run 30758971408 is invalid technical harness evidence and not product evidence
  - the current correction is based on immutable runtime evidence rather than an assumed fixture shape
unknown:
  - whether the current generated validator passes its fail-closed preparation gate
  - request or framework path that removes publication status
  - exact session-lock acquisition and save order during a reproduced sample
  - clean exact-frozen matrix result
  - exactly-one-corrupt-row matrix result
  - causal contribution of integrity-failure responses
  - production release and availability
first_failure:
  marker: responsive-mobile original Wiki publication flash absent after session serialization while durable publication succeeds
  evidence: run 30612399525 attempts 3 and 4 jobs 91343023604 and 91343514611 artifacts 8815383351 and 8815457044
rejected_hypotheses:
  - Synology or Docker availability remains the blocker
  - the Laravel observer still fails to match or install
  - spawnSync php ENOENT is a product defect
  - storage_exists must become false after corrupt-files
  - six technical failures satisfy the matrix gate
  - an unchanged rerun is authorized
validation:
  - command: Issue 365 exact-frozen Synology run 30758971408 job 91526007975
    result: INVALID_TECHNICAL_FAILURE
    evidence: six clean samples failed with php ENOENT and the original corrupt invariant contradicted the captured fixture state
  - command: artifact 8837189083 integrity and content inspection
    result: PASS
    evidence: GitHub ZIP digest verified; one-corrupt media-before.json records count 1 storage_exists true thumbnail_exists false
  - command: current bounded run 30763456046 on control head 8c58035cacb9fd4675d898a1652036fc8b9d4357
    result: WAITING
    evidence: GitHub Actions job 91537990755 is non-terminal; two exact-head state checks consumed
  - command: product and production mutation audit
    result: PASS
    evidence: temporary infrastructure remains unmerged and isolated
blockers:
  - external completion of run 30763456046; no worker should poll it further in this invocation
next_action: when run 30763456046 becomes terminal, inspect it once, verify any artifact, update the audit evidence and Issue 365, close PR 476 without merge, and do not rerun the matrix
```

## Notes

The audit remains `VALIDATED_WITH_CORRECTIONS`. The current exact-frozen run is waiting on GitHub Actions. No merge, deployment or product implementation is authorized.
