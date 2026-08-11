---
task_id: OTERYN-20260811-dependabot-actions-major-cleanup
mode: implementation
branch: chore/dependabot-actions-major-cleanup
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260811 remaining Dependabot Actions major cleanup

## Goal

Finish the old #955-#958 dependency queue on current protected `main` without reverting the later #989/#997/#1000 work. Deliver one coherent current-main wave for `docker/build-push-action` 6→7, `actions/setup-node` 4→7, `actions/checkout` 5/6→7 on the declared bot-owned surfaces, and `docker/metadata-action` 5→6.

## Acceptance

- [x] #955-#958 are no longer open standalone bot PRs and their staged replacements were preserved in the canonical cleanup wave.
- [x] Current protected `main` `8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b` is incorporated without reverting #1000 trigger-coupling changes.
- [x] Additional old checkout/setup-node markers introduced by later current-main work were reconciled inside the exact 19 owned workflow files.
- [x] Temporary reconciliation tooling was removed before final readiness.
- [ ] Pass exact-head workflow/runner compatibility validation and final review.
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
updated_at: 2026-08-11T14:13:00Z
head: ea5165b49789fc1f68bedcfd989838230384a321
branch: chore/dependabot-actions-major-cleanup
pr: 1001
status: validating
context_routes:
  - ci-repair
  - testing
owned_paths:
  - the exact workflow/task paths declared above
proven:
  - Protected main remains 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b.
  - Merge/reconciliation commit 760bece1e57198596669cefb85c42f5608352842 preserves #1000 changes while carrying the staged Actions-major wave.
  - Reconciler run 31498690675 transformed exactly four owned files and six stale markers, then its push was rejected solely because GitHub's workflow GITHUB_TOKEN lacks workflow-update authority.
  - The rejected commit object 447b8a325bd78b1735c0c7fdcb02da71f2fedce1 is retained by GitHub and contains exactly those six substitutions; the canonical branch was safely fast-forwarded to it through the authenticated GitHub connector.
  - Reconciler validation found no old checkout@v5/v6, setup-node@v4, build-push-action@v6, or metadata-action@v5 marker remaining in the exact 19 owned workflow files after transformation.
  - Commit ea5165b49789fc1f68bedcfd989838230384a321 removes the temporary source-editing workflow; no temporary reconciliation authority remains in the candidate tree.
  - PR #1003 explicitly defers deep-system-validation.yml until #1001 is terminal; no ownership hijack occurred.
derived:
  - The remaining gate is exact-head CI/review and terminal lifecycle closeout, not further implementation.
unknown:
  - final exact-head CI/review result until the frozen candidate generation completes.
conflicts: []
first_failure:
  marker: reconciler push permission
  evidence: run 31498690675 job 93802769699 rejected workflow-file push because GITHUB_TOKEN lacks workflows permission; transformation step itself passed.
rejected_hypotheses:
  - Reconciler failure means the action-version transformation failed; the transformation step succeeded and the only failed step was remote workflow-file push authorization.
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
blockers:
  - none
next_action: Freeze the candidate, run exact-head CI and final review, squash-merge PR #1001, verify resulting main, then archive and release task ownership.
```
