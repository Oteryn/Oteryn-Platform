---
task_id: OTERYN-20260811-dependabot-actions-major-cleanup
mode: implementation
branch: chore/dependabot-actions-major-cleanup
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260811 remaining Dependabot Actions major cleanup

## Goal

Finish the old #955-#958 dependency queue on current protected `main` without reverting the later #989/#997/#1000/#1004 work. Deliver one coherent current-main wave for `docker/build-push-action` 6→7, `actions/setup-node` 4→7, `actions/checkout` 5/6→7 on the declared task-owned surfaces, and `docker/metadata-action` 5→6.

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
checkpoint_version: 1
updated_at: 2026-08-11T15:27:00Z
head: 8af7acc757c26f6db18922cb3c33ef1ab55e5f0b
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
  - Restack 7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18 incorporates protected main with behind_by=0 and changes exactly 19 owned workflows plus this active task record.
  - Reconciler run 31498690675 transformed exactly four owned files and six stale markers; its remote push was rejected solely because the workflow token lacked workflow-file update authority.
  - Retained commit 447b8a325bd78b1735c0c7fdcb02da71f2fedce1 contains those six substitutions and is incorporated in the canonical branch.
  - Reconciliation validation found no old checkout@v5/v6, setup-node@v4, build-push-action@v6, or metadata-action@v5 marker remaining in the exact 19 owned workflow files.
  - Temporary reconciliation authority is absent from the current candidate.
  - The restacked generation passed Agent Governance 31505998998, Portal Exhaustive Trigger Coupling 31505998913, native-protocol checks, multiple acceptance gates, Build Synology Staging Images 31505998973, Platform DB Outage 31505999061, Game Auth Ticket Concurrency 31505999009, Phase 7 31505999056, and additional completed task-relevant workflows before the checkpoint-only repair.
  - Fresh Codex review on 7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18 raised only a stale checkpoint identity finding and no workflow implementation finding.
  - Agent Governance run 31506957791 proved the attempted checkpoint-v2 repair invalid because the repository contract requires checkpoint_version 1 and mandatory field head; this revision repairs exactly that schema defect.
  - PR #1003 continues to defer deep-system-validation.yml until #1001 is terminal; no ownership overlap is being bypassed.
derived:
  - No Actions implementation defect is currently known; the remaining gate is a fresh exact-head validation and review generation after this checkpoint-only schema repair.
unknown:
  - Terminal result of the final post-repair exact-head CI, Deep System/browser validation, and fresh review.
conflicts: []
first_failure:
  marker: checkpoint-contract-mismatch
  evidence: Agent Governance run 31506957791 reported missing checkpoint field head and required checkpoint_version 1 for the prior checkpoint-only revision.
rejected_hypotheses:
  - The Codex P1 indicated a workflow compatibility regression; the finding was confined to stale task checkpoint identity.
  - The checkpoint structure can be upgraded locally to version 2; docs/agents/GOVERNANCE_CONTRACT.json explicitly declares shared checkpoint version 1 and requires head.
  - Reconciler failure means the action-version transformation failed; transformation succeeded and only remote workflow-file push authorization failed.
changed_paths:
  - exact 19 workflow files
  - docs/agents/tasks/active/OTERYN-20260811-dependabot-actions-major-cleanup.md
validation:
  - command: bounded owned-path action-major reconciliation
    result: PASS
    evidence: run 31498690675 transformation step plus retained commit 447b8a325bd78b1735c0c7fdcb02da71f2fedce1.
  - command: current-main restack scope comparison
    result: PASS
    evidence: compare 0375285cfa964a0f0cbdcf56d65d7592ac41298a...7a91f0a6e17cc2f6e1972d2aca0b1c3d6224cd18 is behind_by=0 and lists exactly 19 workflow files plus the active task record.
  - command: final post-repair exact-head required checks
    result: NOT_RUN
    evidence: this checkpoint-only schema repair creates a new exact head and therefore requires a fresh terminal validation generation before merge.
blockers:
  - none
next_action: Require the post-repair exact-head CI, Deep System/browser validation and fresh review to reach terminal success; then squash-merge PR #1001 with expected-head protection, verify protected main, close Issue #691, archive this task, and release deep-system-validation.yml ownership.
```
