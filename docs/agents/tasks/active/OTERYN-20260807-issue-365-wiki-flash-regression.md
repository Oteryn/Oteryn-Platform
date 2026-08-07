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
- [ ] Current synchronized exact-head required checks pass.
- [ ] Current exact-head self-review and PR hygiene are clean immediately before merge.
- [ ] PR #837 merges and Issue #365 receives a terminal evidence-backed disposition without claiming an unknown root cause.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T19:55:00+02:00
head: 10c21742649d854daa81d676080130958148fe5c
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
  - Issue #365 remains the repair target and PR #837 is its single active repair PR.
  - The implementation changes only the Wiki browser regression gate; no product runtime path changes.
  - Exact prior head ea4fa0b3721448229df15c389c9dc3f92709f55f passed CI 31202472381, Acceptance E2E 31202471870, Deep System Validation 31202471572, Agent Governance 31202471569, Game Auth Ticket Concurrency 31202471568, Edge Security Emulation 31202471587, Platform DB Outage Validation 31202471515, Phase 7 Production-Like Validation 31202471511, Wiki Reconciliation Acceptance 31202471612, and Portal Acceptance Contract 31202471591.
  - Acceptance E2E on that prior exact head passed zero-retry Chromium smoke, browser portability, responsive, dependency-resilience and keyboard-accessibility profiles.
  - Direct merge of that prior head was rejected because main advanced and GitHub reported mergeable_state behind; no validation failure occurred.
  - The branch is now synchronized onto main 84922e4a24be9759c864b41efd34b1e43634d407 and the same two-path repair is re-applied.
derived:
  - Current-head checks must be allowed to re-establish the protected-branch exact-head gate after synchronization; no additional runtime repair is justified by current evidence.
unknown:
  - Exact historical runtime cause of the intermittent flash loss.
  - Current synchronized-head CI conclusions until workflows complete.
conflicts: []
first_failure:
  marker: protected_branch_behind
  evidence: merge attempt after all prior exact-head checks passed returned `2 of 2 required status checks are expected`; GitHub REST reported mergeable_state `behind`, so the branch was synchronized instead of bypassing protection.
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
  - command: prior exact-head full-diff self-review and PR review hygiene
    result: PASS
    evidence: two changed paths only, zero material findings, zero review threads, zero submitted reviews/requested changes.
blockers: []
next_action: validate the synchronized exact PR head; if required checks and hygiene pass while the branch is current with main, merge PR #837, close Issue #365, then archive this task through lifecycle closeout
```

## Safety

Repository-only acceptance regression. No production, deployment, database, session implementation, authentication, authorization, media runtime, secret, external repository or Canary mutation is authorized.
