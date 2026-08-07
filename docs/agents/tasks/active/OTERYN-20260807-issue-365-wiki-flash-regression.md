---
task_id: OTERYN-20260807-issue-365-wiki-flash-regression
issue: 365
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: validating
risk: medium
validation_intensity: STANDARD
execution_mode: github_only
branch: fix/issue-365-wiki-flash-regression
base_branch: main
pr: 837
production_activation_authorized: false
cross_repository_mutation_authorized: false
owned_paths:
  - scripts/acceptance/tests/admin-wiki-administration.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260807-issue-365-wiki-flash-regression.md
modules:
  - quality_e2e
coordination_key: acceptance:wiki-publication-flash
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 issue 365 Wiki publication flash regression

## Goal

Restore deterministic browser regression coverage for the historically reproduced Wiki publication success flash and fail the clean Wiki administration journey on unexplained HTTP 5xx responses, without inventing an unproven runtime root cause.

## Evidence boundary

- Historical mobile publication-flash loss is proven; its exact historical root cause remains unknown.
- Frozen matrix run `31097086526` passed the publication-flash assertion in all 12 zero-retry samples, including six corrupt-media samples; controlled thumbnail 500s were therefore not sufficient to remove the flash.
- PR #751 separately repaired proven Editorial Media fixture isolation and did not claim that stale thumbnails caused the flash loss.
- No session, authentication, authorization, media-serving, or publication runtime change is part of this task.

## Acceptance criteria

- [x] Standard Wiki administration acceptance explicitly asserts `Wiki article published.` after publish.
- [x] The clean journey fails on any unexplained HTTP >=500 response recorded by existing diagnostics.
- [x] Durable `Published` state and `Unpublish to draft` assertions remain unchanged.
- [x] Existing zero-retry/browser/authentication/MFA/publication semantics remain unchanged.
- [x] Prior exact-head generation `ea4fa0b3721448229df15c389c9dc3f92709f55f` passed all applicable required workflows, including zero-retry Acceptance E2E and Deep System Validation.
- [ ] Current recovered exact-head required checks pass.
- [ ] Current exact-head self-review and PR hygiene are clean immediately before terminal delivery.
- [ ] PR #837 reaches the protected terminal delivery state and Issue #365 receives a bounded evidence-backed disposition without claiming an unknown root cause.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T19:58:00+02:00
head: 1912399cd98218bff63265371a9943900ff1780b
branch: fix/issue-365-wiki-flash-regression
pr: 837
status: validating
context_routes:
  - testing
  - quality-e2e
owned_paths:
  - scripts/acceptance/tests/admin-wiki-administration.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260807-issue-365-wiki-flash-regression.md
proven:
  - Issue #365 remains the repair target and PR #837 is its single repair PR.
  - The implementation changes only the Wiki browser regression gate; no product runtime path changes.
  - The branch currently contains the restored `Wiki article published.` assertion and the clean-journey no-HTTP-5xx assertion.
  - Exact prior head ea4fa0b3721448229df15c389c9dc3f92709f55f passed CI 31202472381, Acceptance E2E 31202471870, Deep System Validation 31202471572, Agent Governance 31202471569, Game Auth Ticket Concurrency 31202471568, Edge Security Emulation 31202471587, Platform DB Outage Validation 31202471515, Phase 7 Production-Like Validation 31202471511, Wiki Reconciliation Acceptance 31202471612, and Portal Acceptance Contract 31202471591.
  - The prior protected delivery attempt was rejected after main advanced; no validation failure occurred.
  - A synchronization transition temporarily made PR #837 appear closed with zero diff; the PR has been reopened against current main and the intended two-path diff is again visible.
derived:
  - Fresh current-head checks are required because the synchronized branch identity changed after the prior successful validation generation.
unknown:
  - Exact historical runtime cause of the intermittent flash loss.
  - Current recovered-head CI conclusions until workflows complete.
conflicts: []
first_failure:
  marker: branch_sync_pr_state_transition
  evidence: Agent Governance run 31204450731 failed only because PR #837 was temporarily terminal while the active task still referenced a future terminal action; the PR was reopened before this checkpoint recovery.
rejected_hypotheses:
  - Treat controlled corrupt-thumbnail responses as the flash-loss root cause; rejected by the 12-sample frozen matrix.
  - Add speculative session/media runtime workaround; rejected because causal evidence is absent.
changed_paths:
  - scripts/acceptance/tests/admin-wiki-administration.spec.mjs
  - docs/agents/tasks/active/OTERYN-20260807-issue-365-wiki-flash-regression.md
validation:
  - command: prior exact-head workflow generation on ea4fa0b3721448229df15c389c9dc3f92709f55f
    result: PASS
    evidence: all ten applicable workflow families completed successfully, including Acceptance E2E and Deep System Validation zero-retry browser coverage.
  - command: branch-content verification after PR-state recovery
    result: PASS
    evidence: current branch still contains both intended acceptance assertions and no runtime change.
blockers: []
next_action: validate the current recovered PR head with required workflows and exact-head hygiene, then complete the protected terminal delivery and lifecycle closeout if all gates pass
```

## Safety

Repository-only acceptance regression. No production, deployment, database, session implementation, authentication, authorization, media runtime, secret, external repository or Canary mutation is authorized.
