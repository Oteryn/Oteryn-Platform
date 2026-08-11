---
task_id: OTERYN-20260811-dependabot-actions-major-cleanup
mode: implementation
branch: chore/dependabot-actions-major-cleanup
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260811 remaining Dependabot Actions major cleanup

## Goal

Finish the old #955-#958 dependency queue on current protected `main` without reverting the later #989/#997/#1000/#1004 work. Deliver one coherent current-main wave for `docker/build-push-action` 6→7, `actions/setup-node` 4→7, `actions/checkout` 5/6→7 on the declared bot-owned surfaces, and `docker/metadata-action` 5→6.

## Acceptance

- [x] #955-#958 are no longer open standalone bot PRs and their staged replacements were preserved in the canonical cleanup wave.
- [x] Current protected `main` `0375285cfa964a0f0cbdcf56d65d7592ac41298a` is incorporated without reverting #1000/#1004 lifecycle and trigger-coupling changes.
- [x] Additional old checkout/setup-node markers introduced by later current-main work were reconciled inside the exact 19 owned workflow files.
- [x] Temporary reconciliation tooling was removed before final readiness.
- [ ] Pass final exact-head workflow/runner compatibility validation and final review.
- [ ] Merge to protected main and archive this task.

## Ownership

```yaml
owned_paths:
  - .github/workflows/acceptance-validation.yml
  - .github/workflows/announcements-acceptance.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/community-data-acceptance.yml
  - .github/workflows/content-scale-acceptance.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/downloads-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/error-state-acceptance.yml
  - .github/workflows/events-acceptance.yml
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/native-protocol-contract.yml
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/support-legal-acceptance.yml
  - .github/workflows/support-moderation-acceptance.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260811-dependabot-actions-major-cleanup.md
  - docs/agents/tasks/archive/OTERYN-20260811-dependabot-actions-major-cleanup.md
modules:
  - github-actions-ci
dependencies:
  - terminal Issue #691 / PR #989 Actions compatibility evidence
blockers:
  - none
cross_repository_tasks:
  - none
```

The temporary `.github/workflows/dependabot-actions-major-reconcile.yml` was task-owned only while reconciling current-main drift and is absent from the final candidate.

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-11T15:24:00Z
implementation_head: 7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18
protected_base: 0375285cfa964a0f0cbdcf56d65d7592ac41298a
branch: chore/dependabot-actions-major-cleanup
pr: 1001
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - the exact workflow/task paths declared above
proven:
  - Protected main is 0375285cfa964a0f0cbdcf56d65d7592ac41298a after lifecycle-only PR #1004 archived the terminal #1000 task.
  - Restack commit 7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18 has protected main 0375285cfa964a0f0cbdcf56d65d7592ac41298a as an ancestor; compare reports behind_by=0.
  - The restacked diff remains exactly 19 owned workflow files plus this active task record; #1004 lifecycle paths do not appear in the PR diff.
  - Reconciler run 31498690675 transformed exactly four owned files and six stale markers, then its push was rejected solely because GitHub's workflow GITHUB_TOKEN lacks workflow-update authority.
  - The retained commit object 447b8a325bd78b1735c0c7fdcb02da71f2fedce1 contains exactly those six substitutions and was incorporated through the authenticated GitHub connector.
  - Reconciler validation found no old checkout@v5/v6, setup-node@v4, build-push-action@v6, or metadata-action@v5 marker remaining in the exact 19 owned workflow files after transformation.
  - Commit ea5165b49789fc1f68bedcfd989838230384a321 removes the temporary source-editing workflow; no temporary reconciliation authority remains in the candidate tree.
  - Fresh restacked generation has already passed Agent Governance 31505998998, CI 31505999069, Portal Exhaustive Audit 31505999033, Portal Exhaustive Acceptance E2E 31505999104, Portal Exhaustive Trigger Coupling 31505998913, Portal Acceptance Contract 31505999551, Build Synology Staging Images 31505998973, and the other completed task-relevant acceptance/security/outage gates observed before this checkpoint refresh.
  - Codex review 4907824928 reviewed restack 7a91f0a6e1 and raised one P1 governance-record finding: refresh this checkpoint to the restacked implementation head/base. This checkpoint implements that finding.
  - PR #1003 explicitly defers deep-system-validation.yml until #1001 is terminal; no ownership hijack occurred.
derived:
  - No workflow implementation defect is currently known; the remaining gate is a fresh exact-head generation after this checkpoint-only correction plus final Codex review.
unknown:
  - terminal result of the final checkpoint-refresh exact-head generation and its fresh review.
conflicts: []
first_failure:
  marker: reconciler push permission
  evidence: run 31498690675 job 93802769699 rejected workflow-file push because GITHUB_TOKEN lacks workflows permission; transformation step itself passed.
rejected_hypotheses:
  - Reconciler failure means the action-version transformation failed; the transformation step succeeded and the only failed step was remote workflow-file push authorization.
  - PR #1004 changes the #1001 product/workflow diff; compare after restack proves the #1004 lifecycle paths are already in base and absent from #1001 diff.
changed_paths:
  - exact 19 workflow files
  - docs/agents/tasks/active/OTERYN-20260811-dependabot-actions-major-cleanup.md
validation:
  - command: bounded owned-path action-major reconciliation
    result: PASS
    evidence: run 31498690675 transformation step plus retained commit 447b8a325bd78b1735c0c7fdcb02da71f2fedce1.
  - command: temporary reconciler removal
    result: PASS
    evidence: commit ea5165b49789fc1f68bedcfd989838230384a321.
  - command: lifecycle-only restack compare
    result: PASS
    evidence: restack 7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18 is behind_by=0 and changes exactly 19 workflows plus the active task record against protected main 0375285cfa964a0f0cbdcf56d65d7592ac41298a.
blockers:
  - none
next_action: Freeze the checkpoint-refresh head, require its exact-head CI and fresh Codex review to reach terminal success, squash-merge PR #1001 with expected-head protection, verify resulting main, close Issue #691, then archive and release task ownership.
```
